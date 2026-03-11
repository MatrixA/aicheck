use colored::Colorize;
use serde::Serialize;

use aicheck::detector::{Confidence, FileReport};
use aicheck::i18n;

#[derive(Serialize)]
struct JsonOutput<'a> {
    files: &'a [FileReport],
    summary: Summary,
}

#[derive(Serialize)]
struct Summary {
    total: usize,
    ai_detected: usize,
    high: usize,
    medium: usize,
    low: usize,
}

fn make_summary(reports: &[FileReport]) -> Summary {
    Summary {
        total: reports.len(),
        ai_detected: reports.iter().filter(|r| r.ai_generated).count(),
        high: reports
            .iter()
            .filter(|r| r.overall_confidence == Confidence::High)
            .count(),
        medium: reports
            .iter()
            .filter(|r| r.overall_confidence == Confidence::Medium)
            .count(),
        low: reports
            .iter()
            .filter(|r| r.overall_confidence == Confidence::Low)
            .count(),
    }
}

pub fn print_human(reports: &[FileReport]) {
    for (i, report) in reports.iter().enumerate() {
        if i > 0 {
            println!();
        }
        println!("{}", report.path.display().to_string().bold());

        if let Some(err) = &report.error {
            println!("  {} {}", "ERROR".red().bold(), err);
            continue;
        }

        if report.signals.is_empty() {
            println!("  {}", i18n::t("output_no_signals", &[]).dimmed());
            continue;
        }

        for signal in &report.signals {
            let conf_str = format!("{:<6}", signal.confidence.localized());
            let colored_conf = match signal.confidence {
                Confidence::High => conf_str.red().bold(),
                Confidence::Medium => conf_str.yellow().bold(),
                Confidence::Low => conf_str.blue(),
                Confidence::None => conf_str.dimmed(),
            };
            let source = format!("{}", signal.source);
            let desc = signal.localized_description();
            print!("  {} {}: {}", colored_conf, source.dimmed(), desc);
            if let Some(tool) = &signal.tool {
                print!(" [{}]", tool.green());
            }
            println!();
        }

        // Verdict
        let verdict = if report.ai_generated {
            let label = match report.overall_confidence {
                Confidence::High => i18n::t("verdict_ai_generated", &[]).red().bold(),
                Confidence::Medium => i18n::t("verdict_likely_ai", &[]).yellow().bold(),
                Confidence::Low => i18n::t("verdict_possibly_ai", &[]).blue(),
                Confidence::None => i18n::t("verdict_unknown", &[]).dimmed(),
            };
            format!(
                "  Verdict: {} (confidence: {})",
                label, report.overall_confidence
            )
        } else {
            format!(
                "  Verdict: {}",
                i18n::t("verdict_not_detected", &[]).green()
            )
        };
        println!("{}", verdict);
    }

    // Summary for batch
    if reports.len() > 1 {
        let summary = make_summary(reports);
        println!();
        println!(
            "{}",
            i18n::t(
                "output_summary",
                &[
                    ("detected", &summary.ai_detected.to_string()),
                    ("total", &summary.total.to_string()),
                    ("high", &summary.high.to_string()),
                    ("medium", &summary.medium.to_string()),
                    ("low", &summary.low.to_string()),
                ],
            )
            .bold()
        );
    }
}

pub fn print_json(reports: &[FileReport]) {
    let output = JsonOutput {
        summary: make_summary(reports),
        files: reports,
    };
    match serde_json::to_string_pretty(&output) {
        Ok(json) => println!("{}", json),
        Err(e) => eprintln!(
            "{}",
            i18n::t("error_json_serialize", &[("err", &e.to_string())])
        ),
    }
}

/// Print info dump for a single file.
pub fn print_info(
    report: &FileReport,
    xmp_props: &[(String, String)],
    exif_fields: &[(String, String)],
    mp4_meta: &[(String, String)],
    id3_tags: &[(String, String)],
    wav_meta: &[(String, String)],
) {
    println!("{}", report.path.display().to_string().bold());
    if let Some(mime) = &report.mime_type {
        println!(
            "  {}",
            i18n::t("output_type_label", &[("mime", mime.as_str())])
        );
    }
    println!();

    // C2PA section
    let c2pa_signals: Vec<_> = report
        .signals
        .iter()
        .filter(|s| matches!(s.source, aicheck::detector::SignalSource::C2pa))
        .collect();
    if !c2pa_signals.is_empty() {
        println!("{}", i18n::t("info_c2pa_header", &[]).cyan().bold());
        for signal in &c2pa_signals {
            println!("  {}", signal.localized_description());
            for (key, val) in &signal.details {
                println!("    {}: {}", key.dimmed(), val);
            }
        }
        println!();
    }

    // XMP section
    if !xmp_props.is_empty() {
        println!("{}", i18n::t("info_xmp_header", &[]).cyan().bold());
        for (key, val) in xmp_props {
            println!("  {}: {}", key, val);
        }
        println!();
    }

    // EXIF section
    if !exif_fields.is_empty() {
        println!("{}", i18n::t("info_exif_header", &[]).cyan().bold());
        for (key, val) in exif_fields {
            println!("  {}: {}", key, val);
        }
        println!();
    }

    // MP4 Metadata section
    if !mp4_meta.is_empty() {
        println!("{}", i18n::t("info_mp4_header", &[]).cyan().bold());
        for (key, val) in mp4_meta {
            println!("  {}: {}", key, val);
        }
        println!();
    }

    // ID3 Tags section
    if !id3_tags.is_empty() {
        println!("{}", i18n::t("info_id3_header", &[]).cyan().bold());
        for (key, val) in id3_tags {
            println!("  {}: {}", key, val);
        }
        println!();
    }

    // WAV Metadata section
    if !wav_meta.is_empty() {
        println!("{}", i18n::t("info_wav_header", &[]).cyan().bold());
        for (key, val) in wav_meta {
            println!("  {}: {}", key, val);
        }
        println!();
    }

    // Watermark section
    let wm_signals: Vec<_> = report
        .signals
        .iter()
        .filter(|s| matches!(s.source, aicheck::detector::SignalSource::Watermark))
        .collect();
    if !wm_signals.is_empty() {
        println!("{}", i18n::t("info_watermark_header", &[]).cyan().bold());
        for signal in &wm_signals {
            println!("  {}", signal.localized_description());
            for (key, val) in &signal.details {
                println!("    {}: {}", key.dimmed(), val);
            }
        }
        println!();
    }

    if c2pa_signals.is_empty()
        && xmp_props.is_empty()
        && exif_fields.is_empty()
        && mp4_meta.is_empty()
        && id3_tags.is_empty()
        && wav_meta.is_empty()
        && wm_signals.is_empty()
    {
        println!("{}", i18n::t("info_no_metadata", &[]).dimmed());
    }
}
