use colored::Colorize;
use serde::Serialize;

use crate::detector::{Confidence, FileReport};

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
            println!("  {}", "No AI-generation signals detected.".dimmed());
            continue;
        }

        for signal in &report.signals {
            let conf_str = format!("{:<6}", signal.confidence.to_string());
            let colored_conf = match signal.confidence {
                Confidence::High => conf_str.red().bold(),
                Confidence::Medium => conf_str.yellow().bold(),
                Confidence::Low => conf_str.blue(),
                Confidence::None => conf_str.dimmed(),
            };
            let source = format!("{}", signal.source);
            print!("  {} {}: {}", colored_conf, source.dimmed(), signal.description);
            if let Some(tool) = &signal.tool {
                print!(" [{}]", tool.green());
            }
            println!();
        }

        // Verdict
        let verdict = if report.ai_generated {
            let label = match report.overall_confidence {
                Confidence::High => "AI-generated".red().bold(),
                Confidence::Medium => "Likely AI-generated".yellow().bold(),
                Confidence::Low => "Possibly AI-generated".blue(),
                Confidence::None => "Unknown".dimmed(),
            };
            format!(
                "  Verdict: {} (confidence: {})",
                label,
                report.overall_confidence
            )
        } else {
            format!("  Verdict: {}", "Not detected as AI-generated".green())
        };
        println!("{}", verdict);
    }

    // Summary for batch
    if reports.len() > 1 {
        let summary = make_summary(reports);
        println!();
        println!(
            "{}",
            format!(
                "Results: {}/{} files with AI signals ({} HIGH, {} MEDIUM, {} LOW)",
                summary.ai_detected, summary.total, summary.high, summary.medium, summary.low
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
    println!("{}", serde_json::to_string_pretty(&output).unwrap());
}

/// Print info dump for a single file.
pub fn print_info(report: &FileReport, xmp_props: &[(String, String)], exif_fields: &[(String, String)]) {
    println!("{}", report.path.display().to_string().bold());
    if let Some(mime) = &report.mime_type {
        println!("  Type: {}", mime);
    }
    println!();

    // C2PA section
    let c2pa_signals: Vec<_> = report
        .signals
        .iter()
        .filter(|s| matches!(s.source, crate::detector::SignalSource::C2pa))
        .collect();
    if !c2pa_signals.is_empty() {
        println!("{}", "=== C2PA Manifest ===".cyan().bold());
        for signal in &c2pa_signals {
            println!("  {}", signal.description);
            for (key, val) in &signal.details {
                println!("    {}: {}", key.dimmed(), val);
            }
        }
        println!();
    }

    // XMP section
    if !xmp_props.is_empty() {
        println!("{}", "=== XMP Metadata ===".cyan().bold());
        for (key, val) in xmp_props {
            println!("  {}: {}", key, val);
        }
        println!();
    }

    // EXIF section
    if !exif_fields.is_empty() {
        println!("{}", "=== EXIF Data ===".cyan().bold());
        for (key, val) in exif_fields {
            println!("  {}: {}", key, val);
        }
        println!();
    }

    // Watermark section
    let wm_signals: Vec<_> = report
        .signals
        .iter()
        .filter(|s| matches!(s.source, crate::detector::SignalSource::Watermark))
        .collect();
    if !wm_signals.is_empty() {
        println!("{}", "=== Watermark Analysis ===".cyan().bold());
        for signal in &wm_signals {
            println!("  {}", signal.description);
            for (key, val) in &signal.details {
                println!("    {}: {}", key.dimmed(), val);
            }
        }
        println!();
    }

    if c2pa_signals.is_empty() && xmp_props.is_empty() && exif_fields.is_empty() && wm_signals.is_empty() {
        println!("{}", "No provenance metadata found.".dimmed());
    }
}
