use anyhow::{Context, Result};
use std::path::Path;

use super::{Confidence, Signal, SignalBuilder, SignalSource};

/// Maximum image dimension — higher than invisible watermark detector since we need spatial detail.
const MAX_DIM: u32 = 2048;
/// Images smaller than this are unlikely to have meaningful visible watermarks.
const MIN_DIM: u32 = 200;

// Corner region sizing (fraction of image dimensions)
const CORNER_WIDTH_FRAC: f64 = 0.22;
const CORNER_HEIGHT_FRAC: f64 = 0.12;
const CORNER_MIN_WIDTH_PX: u32 = 80;
const CORNER_MIN_HEIGHT_PX: u32 = 50;

// Text detection thresholds
/// Bright pixels must be this many luminance units above the corner region mean.
const BRIGHT_DELTA: f64 = 35.0;
/// Minimum valid horizontal run length for text-like patterns.
const MIN_RUN_LENGTH: u32 = 3;
/// Maximum valid horizontal run length for text-like patterns.
const MAX_RUN_LENGTH: u32 = 60;
/// A row needs at least this many valid bright runs to count as a "text row".
const MIN_RUNS_PER_ROW: u32 = 2;
/// Minimum fraction of rows within the text bounding box that must be text rows.
const MIN_TEXT_ROWS_FRACTION: f64 = 0.12;
/// Maximum bbox height as fraction of corner region height. Rejects non-compact matches.
const MAX_BBOX_HEIGHT_FRAC: f64 = 0.50;
/// Maximum gap (in rows) between consecutive text rows within a cluster.
const MAX_CLUSTER_GAP: u32 = 3;
/// Minimum bright pixel ratio within the text bounding box.
const MIN_BRIGHT_RATIO: f64 = 0.03;
/// Maximum bright pixel ratio within the text bounding box.
const MAX_BRIGHT_RATIO: f64 = 0.50;
/// Minimum number of text rows to consider a cluster valid.
const MIN_TEXT_ROW_COUNT: u32 = 3;
/// Minimum bounding box dimensions for a valid text cluster.
const MIN_BBOX_WIDTH: u32 = 30;
const MIN_BBOX_HEIGHT: u32 = 12;
/// Bimodal luminance check: minimum difference between cluster centers.
const BIMODAL_CENTER_DIFF: f64 = 50.0;

/// Minimum indicators required to report a detection.
const MIN_INDICATORS: usize = 2;

#[derive(Debug, Clone, Copy, PartialEq)]
enum Corner {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

impl Corner {
    fn name(&self) -> &'static str {
        match self {
            Corner::TopLeft => "top-left",
            Corner::TopRight => "top-right",
            Corner::BottomLeft => "bottom-left",
            Corner::BottomRight => "bottom-right",
        }
    }

    /// Top-left and bottom-right are known positions for Chinese AI disclosure watermarks.
    fn is_known_ai_position(&self) -> bool {
        matches!(self, Corner::TopLeft | Corner::BottomRight)
    }
}

/// Bounding box of detected text cluster within a corner region.
struct TextBbox {
    x0: u32,
    y0: u32,
    x1: u32,
    y1: u32,
}

struct TextCluster {
    start_row: u32,
    end_row: u32,
}

struct TextAnalysis {
    bright_ratio: f64,
    text_row_fraction: f64,
    is_bimodal: bool,
    mean_lum: f64,
}

/// Detect visible AI watermark badges in image corner regions.
///
/// Looks for the characteristic pattern of Chinese AI regulatory watermarks:
/// bright text overlay in corners (e.g., "AI生成", "即梦AI").
/// Uses a bottom-up approach: finds bright text clusters first, then validates.
pub fn detect(path: &Path) -> Result<Vec<Signal>> {
    let img = image::open(path).context("Failed to open image for visible watermark analysis")?;
    let img = if img.width() > MAX_DIM || img.height() > MAX_DIM {
        img.resize(MAX_DIM, MAX_DIM, image::imageops::FilterType::Lanczos3)
    } else {
        img
    };

    let (w, h) = (img.width(), img.height());
    if w < MIN_DIM || h < MIN_DIM {
        return Ok(vec![]);
    }

    let gray = img.to_luma8();
    let debug = std::env::var("AIC_DEBUG").is_ok();

    // Corner region dimensions
    let region_w = ((w as f64 * CORNER_WIDTH_FRAC) as u32)
        .max(CORNER_MIN_WIDTH_PX)
        .min(w / 2);
    let region_h = ((h as f64 * CORNER_HEIGHT_FRAC) as u32)
        .max(CORNER_MIN_HEIGHT_PX)
        .min(h / 2);

    let corners = [
        Corner::TopLeft,
        Corner::TopRight,
        Corner::BottomLeft,
        Corner::BottomRight,
    ];

    let mut signals = Vec::new();

    for &corner in &corners {
        let (ox, oy) = corner_offset(corner, w, h, region_w, region_h);
        let region = extract_region(&gray, ox, oy, region_w, region_h);

        let region_mean = compute_mean(&region);
        let bright_threshold = (region_mean + BRIGHT_DELTA).min(255.0) as u8;

        // Step 1: Find text rows (rows with multiple bright pixel runs)
        let text_rows = find_text_rows(&region, region_w, region_h, bright_threshold);

        let total_text_rows = text_rows.iter().filter(|&&v| v).count() as u32;
        if total_text_rows < MIN_TEXT_ROW_COUNT {
            if debug {
                eprintln!(
                    "  [debug] Visible watermark {}: region_mean={:.1} bright_thr={} text_rows={}",
                    corner.name(),
                    region_mean,
                    bright_threshold,
                    total_text_rows
                );
            }
            continue;
        }

        // Step 2: Split text rows into compact clusters (groups separated by gaps)
        let clusters = find_text_clusters(&text_rows);

        for cluster in &clusters {
            let cluster_h = cluster.end_row - cluster.start_row;
            if cluster_h < MIN_BBOX_HEIGHT
                || cluster_h as f64 > region_h as f64 * MAX_BBOX_HEIGHT_FRAC
            {
                continue;
            }

            // Find bounding box of bright pixels within this cluster's row range
            let bbox = match find_cluster_bbox(
                &region,
                region_w,
                cluster.start_row,
                cluster.end_row,
                bright_threshold,
            ) {
                Some(b) => b,
                None => continue,
            };

            if bbox.x1 - bbox.x0 < MIN_BBOX_WIDTH {
                continue;
            }

            // Step 3: Analyze text characteristics within bounding box
            let analysis = analyze_text_cluster(&region, region_w, &bbox, bright_threshold);

            let mut indicator_count = 0usize;
            let mut indicators = Vec::new();

            if analysis.bright_ratio >= MIN_BRIGHT_RATIO
                && analysis.bright_ratio <= MAX_BRIGHT_RATIO
            {
                indicator_count += 1;
                indicators.push(format!(
                    "bright pixel ratio {:.0}%",
                    analysis.bright_ratio * 100.0
                ));
            }

            if analysis.text_row_fraction >= MIN_TEXT_ROWS_FRACTION {
                indicator_count += 1;
                indicators.push("horizontal text runs".to_string());
            }

            if analysis.is_bimodal {
                indicator_count += 1;
                indicators.push("bimodal luminance".to_string());
            }

            if debug {
                eprintln!(
                    "  [debug] Visible watermark {}: region_mean={:.1} bbox=({},{})..({},{}) bright_ratio={:.3} text_rows={:.3} bimodal={} indicators={}",
                    corner.name(),
                    region_mean,
                    bbox.x0 + ox, bbox.y0 + oy, bbox.x1 + ox, bbox.y1 + oy,
                    analysis.bright_ratio,
                    analysis.text_row_fraction,
                    analysis.is_bimodal,
                    indicator_count
                );
            }

            if indicator_count < MIN_INDICATORS {
                continue;
            }

            let (confidence, msg_key) = if corner.is_known_ai_position() && indicator_count >= 3 {
                (Confidence::Medium, "signal_visible_watermark_badge")
            } else {
                (Confidence::Low, "signal_visible_watermark_generic")
            };

            let indicators_str = indicators.join("; ");

            signals.push(
                SignalBuilder::new(SignalSource::Watermark, confidence, msg_key)
                    .param("corner", corner.name())
                    .param("indicators", &indicators_str)
                    .detail("corner", corner.name())
                    .detail("region_mean_luminance", format!("{:.1}", analysis.mean_lum))
                    .detail(
                        "bright_pixel_ratio",
                        format!("{:.3}", analysis.bright_ratio),
                    )
                    .detail(
                        "text_row_fraction",
                        format!("{:.3}", analysis.text_row_fraction),
                    )
                    .detail("indicator_count", indicator_count.to_string())
                    .build(),
            );

            // One detection per corner is enough
            break;
        }
    }

    Ok(signals)
}

/// Compute the top-left origin of a corner region.
fn corner_offset(corner: Corner, w: u32, h: u32, rw: u32, rh: u32) -> (u32, u32) {
    match corner {
        Corner::TopLeft => (0, 0),
        Corner::TopRight => (w.saturating_sub(rw), 0),
        Corner::BottomLeft => (0, h.saturating_sub(rh)),
        Corner::BottomRight => (w.saturating_sub(rw), h.saturating_sub(rh)),
    }
}

/// Extract a rectangular region of grayscale pixels.
fn extract_region(gray: &image::GrayImage, ox: u32, oy: u32, rw: u32, rh: u32) -> Vec<u8> {
    let mut region = Vec::with_capacity((rw * rh) as usize);
    for y in oy..oy + rh {
        for x in ox..ox + rw {
            region.push(gray.get_pixel(x, y).0[0]);
        }
    }
    region
}

/// Compute mean luminance of a region.
fn compute_mean(region: &[u8]) -> f64 {
    if region.is_empty() {
        return 128.0;
    }
    let sum: u64 = region.iter().map(|&v| v as u64).sum();
    sum as f64 / region.len() as f64
}

/// Identify which rows contain text-like bright pixel runs.
/// Returns a boolean per row.
fn find_text_rows(region: &[u8], rw: u32, rh: u32, bright_threshold: u8) -> Vec<bool> {
    let mut text_rows = Vec::with_capacity(rh as usize);
    for y in 0..rh {
        let mut run_len = 0u32;
        let mut runs_in_row = 0u32;
        for x in 0..rw {
            if region[(y * rw + x) as usize] >= bright_threshold {
                run_len += 1;
            } else {
                if run_len >= MIN_RUN_LENGTH && run_len <= MAX_RUN_LENGTH {
                    runs_in_row += 1;
                }
                run_len = 0;
            }
        }
        // Check final run
        if run_len >= MIN_RUN_LENGTH && run_len <= MAX_RUN_LENGTH {
            runs_in_row += 1;
        }
        text_rows.push(runs_in_row >= MIN_RUNS_PER_ROW);
    }
    text_rows
}

/// Split text rows into compact clusters separated by gaps of non-text rows.
/// Each cluster is a contiguous (or near-contiguous) group of text rows.
fn find_text_clusters(text_rows: &[bool]) -> Vec<TextCluster> {
    let mut clusters = Vec::new();
    let mut start: Option<u32> = None;
    let mut gap = 0u32;

    for (y, &is_text) in text_rows.iter().enumerate() {
        let y = y as u32;
        if is_text {
            if start.is_none() {
                start = Some(y);
            }
            gap = 0;
        } else if let Some(s) = start {
            gap += 1;
            if gap > MAX_CLUSTER_GAP {
                let end = y - gap;
                if end > s + MIN_TEXT_ROW_COUNT {
                    clusters.push(TextCluster {
                        start_row: s,
                        end_row: end,
                    });
                }
                start = None;
                gap = 0;
            }
        }
    }

    // Final cluster
    if let Some(s) = start {
        let end = text_rows.len() as u32 - gap;
        if end > s + MIN_TEXT_ROW_COUNT {
            clusters.push(TextCluster {
                start_row: s,
                end_row: end,
            });
        }
    }

    // Sort by size (largest first — most likely to be the real watermark)
    clusters.sort_by(|a, b| {
        let a_size = a.end_row - a.start_row;
        let b_size = b.end_row - b.start_row;
        b_size.cmp(&a_size)
    });

    clusters
}

/// Find the bounding box of bright pixels within a row range.
fn find_cluster_bbox(
    region: &[u8],
    rw: u32,
    start_row: u32,
    end_row: u32,
    bright_threshold: u8,
) -> Option<TextBbox> {
    let mut min_x = rw;
    let mut max_x = 0u32;

    for y in start_row..end_row {
        for x in 0..rw {
            if region[(y * rw + x) as usize] >= bright_threshold {
                min_x = min_x.min(x);
                max_x = max_x.max(x);
            }
        }
    }

    if max_x > min_x {
        let pad_x = ((max_x - min_x) / 10).max(2);
        let pad_y = ((end_row - start_row) / 10).max(1);
        Some(TextBbox {
            x0: min_x.saturating_sub(pad_x),
            y0: start_row.saturating_sub(pad_y),
            x1: (max_x + pad_x).min(rw),
            y1: (end_row + pad_y).min(region.len() as u32 / rw),
        })
    } else {
        None
    }
}

/// Analyze text characteristics within the text cluster bounding box.
fn analyze_text_cluster(
    region: &[u8],
    rw: u32,
    bbox: &TextBbox,
    bright_threshold: u8,
) -> TextAnalysis {
    let bw = bbox.x1 - bbox.x0;
    let bh = bbox.y1 - bbox.y0;

    // Bright pixel ratio within bbox
    let mut bright_count = 0u32;
    let mut sum_lum = 0u64;
    let total = bw * bh;
    for y in bbox.y0..bbox.y1 {
        for x in bbox.x0..bbox.x1 {
            let v = region[(y * rw + x) as usize];
            sum_lum += v as u64;
            if v >= bright_threshold {
                bright_count += 1;
            }
        }
    }
    let bright_ratio = if total > 0 {
        bright_count as f64 / total as f64
    } else {
        0.0
    };
    let mean_lum = if total > 0 {
        sum_lum as f64 / total as f64
    } else {
        0.0
    };

    // Text row fraction within bbox
    let mut text_rows_in_bbox = 0u32;
    for y in bbox.y0..bbox.y1 {
        let mut run_len = 0u32;
        let mut runs = 0u32;
        for x in bbox.x0..bbox.x1 {
            if region[(y * rw + x) as usize] >= bright_threshold {
                run_len += 1;
            } else {
                if run_len >= MIN_RUN_LENGTH && run_len <= MAX_RUN_LENGTH {
                    runs += 1;
                }
                run_len = 0;
            }
        }
        if run_len >= MIN_RUN_LENGTH && run_len <= MAX_RUN_LENGTH {
            runs += 1;
        }
        if runs >= MIN_RUNS_PER_ROW {
            text_rows_in_bbox += 1;
        }
    }
    let text_row_fraction = if bh > 0 {
        text_rows_in_bbox as f64 / bh as f64
    } else {
        0.0
    };

    // Bimodal luminance check
    let is_bimodal = check_bimodal(region, rw, bbox, bright_threshold);

    TextAnalysis {
        bright_ratio,
        text_row_fraction,
        is_bimodal,
        mean_lum,
    }
}

/// Check if the bounding box has a bimodal luminance distribution (background + text).
fn check_bimodal(region: &[u8], rw: u32, bbox: &TextBbox, threshold: u8) -> bool {
    let mut dark_sum = 0u64;
    let mut dark_count = 0u64;
    let mut bright_sum = 0u64;
    let mut bright_count = 0u64;

    for y in bbox.y0..bbox.y1 {
        for x in bbox.x0..bbox.x1 {
            let v = region[(y * rw + x) as usize] as u64;
            if v >= threshold as u64 {
                bright_sum += v;
                bright_count += 1;
            } else {
                dark_sum += v;
                dark_count += 1;
            }
        }
    }

    if dark_count == 0 || bright_count == 0 {
        return false;
    }

    let dark_mean = dark_sum as f64 / dark_count as f64;
    let bright_mean = bright_sum as f64 / bright_count as f64;
    let total = (dark_count + bright_count) as f64;
    let smaller = dark_count.min(bright_count) as f64;

    // Two clusters far apart, smaller cluster is 3-50% of total
    (bright_mean - dark_mean) > BIMODAL_CENTER_DIFF
        && smaller / total >= 0.03
        && smaller / total <= 0.50
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{GrayImage, Luma};

    /// Save image to a temp file with .png extension so image::open() can detect the format.
    fn save_tmp_png(img: &GrayImage) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.png");
        img.save(&path).unwrap();
        dir
    }

    /// Create a test image with bright "text" runs in a corner.
    fn make_image_with_text_overlay(
        width: u32,
        height: u32,
        bg_lum: u8,
        text_lum: u8,
        text_x: u32,
        text_y: u32,
        text_w: u32,
        text_h: u32,
    ) -> GrayImage {
        let mut img = GrayImage::from_pixel(width, height, Luma([bg_lum]));

        // Simulate text runs (alternating bright/dark horizontal segments)
        for y in text_y..text_y + text_h {
            if y >= height {
                break;
            }
            let mut x = text_x + 2;
            while x + 5 < text_x + text_w && x + 5 < width {
                // Bright run of ~4px
                for dx in 0..4 {
                    img.put_pixel(x + dx, y, Luma([text_lum]));
                }
                x += 7; // Gap between runs
            }
        }

        img
    }

    #[test]
    fn test_detects_text_in_top_left() {
        // 800x600 image, gray background, white text overlay in top-left
        let img = make_image_with_text_overlay(800, 600, 100, 220, 10, 10, 100, 30);
        let dir = save_tmp_png(&img);

        let signals = detect(&dir.path().join("test.png")).unwrap();
        assert!(
            !signals.is_empty(),
            "Expected visible watermark detection for text in top-left"
        );
        let desc = &signals[0].description;
        assert!(
            desc.contains("top-left"),
            "Expected top-left corner, got: {}",
            desc
        );
    }

    #[test]
    fn test_detects_text_in_bottom_right() {
        // Bright text in bottom-right corner
        let img = make_image_with_text_overlay(800, 600, 80, 230, 690, 565, 100, 25);
        let dir = save_tmp_png(&img);

        let signals = detect(&dir.path().join("test.png")).unwrap();
        assert!(
            !signals.is_empty(),
            "Expected visible watermark detection for text in bottom-right"
        );
    }

    #[test]
    fn test_no_detection_on_uniform_image() {
        let img = GrayImage::from_pixel(800, 600, Luma([128]));
        let dir = save_tmp_png(&img);

        let signals = detect(&dir.path().join("test.png")).unwrap();
        assert!(
            signals.is_empty(),
            "Uniform image should not trigger detection"
        );
    }

    #[test]
    fn test_no_detection_on_small_image() {
        let img = GrayImage::from_pixel(100, 100, Luma([128]));
        let dir = save_tmp_png(&img);

        let signals = detect(&dir.path().join("test.png")).unwrap();
        assert!(signals.is_empty(), "Small image should be skipped");
    }

    #[test]
    fn test_mean_computation() {
        let data = vec![100u8; 100];
        let mean = compute_mean(&data);
        assert!((mean - 100.0).abs() < 0.01);
    }
}
