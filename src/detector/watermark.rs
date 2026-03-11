use anyhow::{Context, Result};
use std::path::Path;

use super::{Confidence, Signal, SignalBuilder, SignalSource};
use crate::i18n;

const MAX_DIM: u32 = 1024;
const DWT_BLOCK: usize = 4;
const MIN_DIM: usize = 64;
const QUANT_STEP: f64 = 36.0;
const ALT_QUANT_STEPS: &[f64] = &[25.0, 30.0, 40.0, 50.0];
const EMBED_INDICES: &[usize] = &[1, 2, 10, 11];
const MIN_INDICATORS: usize = 2;
const NOISE_ASYMMETRY_THRESHOLD: f64 = 0.08;
const BIT_AGREEMENT_THRESHOLD: f64 = 0.62;

pub fn detect(path: &Path) -> Result<Vec<Signal>> {
    let img = image::open(path).context("Failed to open image for watermark analysis")?;
    let img = if img.width() > MAX_DIM || img.height() > MAX_DIM {
        img.resize(MAX_DIM, MAX_DIM, image::imageops::FilterType::Lanczos3)
    } else {
        img
    };

    let rgba = img.to_rgba8();
    let (width, height) = rgba.dimensions();
    let (w, h) = (width as usize, height as usize);
    if w < MIN_DIM || h < MIN_DIM {
        return Ok(vec![]);
    }

    let debug = std::env::var("AIC_DEBUG").is_ok();
    let mut indicators: Vec<&str> = Vec::new();
    let mut details = Vec::new();

    let channels = extract_rgb_channels(&rgba, w, h);
    let cw = w - (w % 2);
    let ch = h - (h % 2);
    if cw < DWT_BLOCK * 4 || ch < DWT_BLOCK * 4 {
        return Ok(vec![]);
    }

    let channel_pixels: Vec<Vec<f64>> = channels
        .iter()
        .map(|channel| {
            channel
                .iter()
                .take(ch * w)
                .enumerate()
                .filter_map(|(i, &v)| if i % w < cw { Some(v) } else { None })
                .collect()
        })
        .collect();

    let channel_subbands: Vec<DwtSubbands> = channel_pixels
        .iter()
        .map(|px| haar_dwt_2d(px, cw, ch))
        .collect();
    let sub_w = cw / 2;
    let sub_h = ch / 2;

    // Analysis 1: Channel noise asymmetry
    let channel_noises: Vec<f64> = channels
        .iter()
        .map(|c| estimate_noise_level(c, w, h))
        .collect();
    let mean_noise = channel_noises.iter().sum::<f64>() / 3.0;
    if mean_noise > 0.01 {
        let max_noise = channel_noises.iter().cloned().fold(f64::MIN, f64::max);
        let min_noise = channel_noises.iter().cloned().fold(f64::MAX, f64::min);
        let asymmetry = (max_noise - min_noise) / mean_noise;
        if debug {
            eprintln!(
                "  [debug] Watermark noise: R={:.3} G={:.3} B={:.3} asymmetry={:.3}",
                channel_noises[0], channel_noises[1], channel_noises[2], asymmetry
            );
        }
        details.push(("noise_asymmetry".to_string(), format!("{:.3}", asymmetry)));
        if asymmetry > NOISE_ASYMMETRY_THRESHOLD {
            indicators.push("channel noise asymmetry");
        }
    }

    // Analysis 2: Cross-channel bit agreement
    let all_quant_steps: Vec<f64> = std::iter::once(QUANT_STEP)
        .chain(ALT_QUANT_STEPS.iter().copied())
        .collect();
    let mut best_agreement = 0.0f64;
    let mut best_q = 0.0f64;
    for &q_step in &all_quant_steps {
        let channel_bits: Vec<Vec<u8>> = channel_subbands
            .iter()
            .map(|sb| extract_bits(&sb.ll, sub_w, sub_h, q_step, EMBED_INDICES))
            .collect();
        if channel_bits.iter().all(|b| !b.is_empty()) {
            let min_len = channel_bits.iter().map(|b| b.len()).min().unwrap_or(0);
            if min_len > 0 {
                let mut total_agree = 0usize;
                let mut total_compared = 0usize;
                for i in 0..3 {
                    for j in (i + 1)..3 {
                        for (bi, bj) in channel_bits[i]
                            .iter()
                            .zip(channel_bits[j].iter())
                            .take(min_len)
                        {
                            if bi == bj {
                                total_agree += 1;
                            }
                            total_compared += 1;
                        }
                    }
                }
                if total_compared > 0 {
                    let agreement = total_agree as f64 / total_compared as f64;
                    if agreement > best_agreement {
                        best_agreement = agreement;
                        best_q = q_step;
                    }
                }
            }
        }
    }
    if debug {
        eprintln!(
            "  [debug] Watermark cross-channel bit agreement: {:.3} (q={:.0})",
            best_agreement, best_q
        );
    }
    details.push((
        "cross_channel_agreement".to_string(),
        format!("{:.3}", best_agreement),
    ));
    if best_agreement > BIT_AGREEMENT_THRESHOLD {
        indicators.push("cross-channel bit consistency");
        details.push(("best_quant_step".to_string(), format!("{:.0}", best_q)));
    }

    // Analysis 3: DWT residual energy ratio
    let mut energy_ratios = Vec::new();
    for (ch_idx, sb) in channel_subbands.iter().enumerate() {
        let ll_energy: f64 = sb.ll.iter().map(|v| v * v).sum::<f64>();
        let detail_energy: f64 = sb.lh.iter().map(|v| v * v).sum::<f64>()
            + sb.hl.iter().map(|v| v * v).sum::<f64>()
            + sb.hh.iter().map(|v| v * v).sum::<f64>();
        if ll_energy > 0.0 {
            let ratio = detail_energy / ll_energy;
            energy_ratios.push(ratio);
            if debug {
                let ch_name = ["R", "G", "B"][ch_idx];
                eprintln!(
                    "  [debug] Watermark energy ratio ch={}: {:.6}",
                    ch_name, ratio
                );
            }
        }
    }
    if energy_ratios.len() >= 2 {
        let max_ratio = energy_ratios.iter().cloned().fold(f64::MIN, f64::max);
        let min_ratio = energy_ratios.iter().cloned().fold(f64::MAX, f64::min);
        let mean_ratio = energy_ratios.iter().sum::<f64>() / energy_ratios.len() as f64;
        if mean_ratio > 0.0 {
            let ratio_spread = (max_ratio - min_ratio) / mean_ratio;
            details.push((
                "energy_ratio_spread".to_string(),
                format!("{:.4}", ratio_spread),
            ));
            if debug {
                eprintln!(
                    "  [debug] Watermark energy ratio spread: {:.4}",
                    ratio_spread
                );
            }
            if ratio_spread > 0.25 {
                indicators.push("asymmetric DWT energy distribution");
            }
        }
    }

    // Emit signal
    if indicators.len() >= MIN_INDICATORS {
        let strength_key = if indicators.len() >= 3 {
            "signal_watermark_strong"
        } else {
            "signal_watermark_moderate"
        };
        let strength = i18n::t(strength_key, &[]);
        let indicators_str = indicators.join("; ");

        Ok(vec![SignalBuilder::new(
            SignalSource::Watermark,
            Confidence::Low,
            "signal_watermark_detected",
        )
        .param("strength", &strength)
        .param("indicators", &indicators_str)
        .details(details)
        .build()])
    } else {
        Ok(vec![])
    }
}

fn extract_rgb_channels(rgba: &image::RgbaImage, w: usize, h: usize) -> [Vec<f64>; 3] {
    let mut r = Vec::with_capacity(w * h);
    let mut g = Vec::with_capacity(w * h);
    let mut b = Vec::with_capacity(w * h);
    for y in 0..h {
        for x in 0..w {
            let pixel = rgba.get_pixel(x as u32, y as u32);
            r.push(pixel[0] as f64);
            g.push(pixel[1] as f64);
            b.push(pixel[2] as f64);
        }
    }
    [r, g, b]
}

fn estimate_noise_level(channel: &[f64], width: usize, height: usize) -> f64 {
    if width < 3 || height < 3 {
        return 0.0;
    }
    let mut laplacian_values = Vec::new();
    for y in 1..height - 1 {
        for x in 1..width - 1 {
            let center = channel[y * width + x];
            let top = channel[(y - 1) * width + x];
            let bottom = channel[(y + 1) * width + x];
            let left = channel[y * width + (x - 1)];
            let right = channel[y * width + (x + 1)];
            let lap = (4.0 * center - top - bottom - left - right).abs();
            laplacian_values.push(lap);
        }
    }
    if laplacian_values.is_empty() {
        return 0.0;
    }
    laplacian_values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median = laplacian_values[laplacian_values.len() / 2];
    median / 0.6745
}

fn extract_bits(
    ll_subband: &[f64],
    width: usize,
    height: usize,
    quant_step: f64,
    coeff_indices: &[usize],
) -> Vec<u8> {
    let blocks_x = width / DWT_BLOCK;
    let blocks_y = height / DWT_BLOCK;
    if blocks_x * blocks_y < 32 {
        return vec![];
    }
    let mut bits = Vec::new();
    for by in 0..blocks_y {
        for bx in 0..blocks_x {
            let mut block = [0.0f64; 16];
            for row in 0..DWT_BLOCK {
                for col in 0..DWT_BLOCK {
                    let y = by * DWT_BLOCK + row;
                    let x = bx * DWT_BLOCK + col;
                    if y < height && x < width {
                        block[row * DWT_BLOCK + col] = ll_subband[y * width + x];
                    }
                }
            }
            apply_2d_dct_ortho(&mut block, DWT_BLOCK);
            for &idx in coeff_indices {
                if idx < 16 {
                    let coeff = block[idx];
                    let q = (coeff / quant_step).round() as i64;
                    bits.push((q.abs() % 2) as u8);
                }
            }
        }
    }
    bits
}

fn apply_2d_dct_ortho(block: &mut [f64], size: usize) {
    let n = size as f64;
    for row in 0..size {
        let start = row * size;
        let input: Vec<f64> = block[start..start + size].to_vec();
        for k in 0..size {
            let mut sum = 0.0;
            for (i, val) in input.iter().enumerate() {
                sum += val
                    * (std::f64::consts::PI * (2.0 * i as f64 + 1.0) * k as f64 / (2.0 * n)).cos();
            }
            let scale = if k == 0 {
                (1.0 / n).sqrt()
            } else {
                (2.0 / n).sqrt()
            };
            block[start + k] = sum * scale;
        }
    }
    for col in 0..size {
        let input: Vec<f64> = (0..size).map(|row| block[row * size + col]).collect();
        for k in 0..size {
            let mut sum = 0.0;
            for (i, val) in input.iter().enumerate() {
                sum += val
                    * (std::f64::consts::PI * (2.0 * i as f64 + 1.0) * k as f64 / (2.0 * n)).cos();
            }
            let scale = if k == 0 {
                (1.0 / n).sqrt()
            } else {
                (2.0 / n).sqrt()
            };
            block[k * size + col] = sum * scale;
        }
    }
}

struct DwtSubbands {
    ll: Vec<f64>,
    lh: Vec<f64>,
    hl: Vec<f64>,
    hh: Vec<f64>,
}

fn haar_dwt_2d(data: &[f64], width: usize, height: usize) -> DwtSubbands {
    let half_w = width / 2;
    let half_h = height / 2;
    let inv_sqrt2 = 1.0 / std::f64::consts::SQRT_2;
    let mut row_low = vec![0.0; half_w * height];
    let mut row_high = vec![0.0; half_w * height];
    for y in 0..height {
        for x in 0..half_w {
            let a = data[y * width + 2 * x];
            let b = data[y * width + 2 * x + 1];
            row_low[y * half_w + x] = (a + b) * inv_sqrt2;
            row_high[y * half_w + x] = (a - b) * inv_sqrt2;
        }
    }
    let mut ll = vec![0.0; half_w * half_h];
    let mut lh = vec![0.0; half_w * half_h];
    let mut hl = vec![0.0; half_w * half_h];
    let mut hh = vec![0.0; half_w * half_h];
    for x in 0..half_w {
        for y in 0..half_h {
            let a_low = row_low[2 * y * half_w + x];
            let b_low = row_low[(2 * y + 1) * half_w + x];
            ll[y * half_w + x] = (a_low + b_low) * inv_sqrt2;
            lh[y * half_w + x] = (a_low - b_low) * inv_sqrt2;
            let a_high = row_high[2 * y * half_w + x];
            let b_high = row_high[(2 * y + 1) * half_w + x];
            hl[y * half_w + x] = (a_high + b_high) * inv_sqrt2;
            hh[y * half_w + x] = (a_high - b_high) * inv_sqrt2;
        }
    }
    DwtSubbands { ll, lh, hl, hh }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_haar_dwt_2d_identity() {
        let data = vec![100.0; 16];
        let result = haar_dwt_2d(&data, 4, 4);
        for v in &result.lh {
            assert!(v.abs() < 1e-10);
        }
        for v in &result.hl {
            assert!(v.abs() < 1e-10);
        }
        for v in &result.hh {
            assert!(v.abs() < 1e-10);
        }
        assert!(result.ll[0] > 0.0);
    }

    #[test]
    fn test_haar_dwt_2d_edge() {
        let mut data = vec![0.0; 64];
        for y in 0..8 {
            for x in (1..8).step_by(2) {
                data[y * 8 + x] = 200.0;
            }
        }
        let result = haar_dwt_2d(&data, 8, 8);
        let hl_energy: f64 = result.hl.iter().map(|v| v * v).sum();
        assert!(hl_energy > 0.0);
    }

    #[test]
    fn test_dct_ortho_energy_preservation() {
        let mut block = [
            1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0,
        ];
        let energy_before: f64 = block.iter().map(|x| x * x).sum();
        apply_2d_dct_ortho(&mut block, 4);
        let energy_after: f64 = block.iter().map(|x| x * x).sum();
        assert!(
            (energy_before - energy_after).abs() < 0.1,
            "before={:.1}, after={:.1}",
            energy_before,
            energy_after
        );
    }

    #[test]
    fn test_noise_level_constant() {
        let data = vec![128.0; 100 * 100];
        let noise = estimate_noise_level(&data, 100, 100);
        assert!(noise < 0.1, "got {}", noise);
    }

    #[test]
    fn test_extract_bits_deterministic() {
        let data = vec![42.0; 128 * 128];
        let bits1 = extract_bits(&data, 128, 128, 36.0, &[1, 2]);
        let bits2 = extract_bits(&data, 128, 128, 36.0, &[1, 2]);
        assert_eq!(bits1, bits2);
        assert!(!bits1.is_empty());
    }
}
