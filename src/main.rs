mod cli;
mod detector;
mod known_tools;
mod output;
mod scanner;

use clap::Parser;
use cli::{Cli, Command};
use colored::control::set_override;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args = Cli::parse();

    if args.no_color {
        set_override(false);
    }

    match args.command {
        Command::Check(ref check_args) => cmd_check(&args, check_args),
        Command::Info(ref info_args) => cmd_info(info_args),
    }
}

fn cmd_check(args: &Cli, check_args: &cli::CheckArgs) -> ExitCode {
    let files = match scanner::discover_files(&check_args.paths, check_args.recursive) {
        Ok(f) => f,
        Err(e) => {
            if !args.quiet {
                eprintln!("Error: {}", e);
            }
            return ExitCode::from(2);
        }
    };

    if files.is_empty() {
        if !args.quiet {
            eprintln!("No supported files found.");
        }
        return ExitCode::from(2);
    }

    let min_confidence = check_args.min_confidence.to_confidence();

    let reports: Vec<_> = files
        .iter()
        .map(|f| {
            let mut report = detector::run_all_detectors(f, check_args.deep);
            // Filter signals below min confidence
            report.signals.retain(|s| s.confidence >= min_confidence);
            // Recompute overall confidence after filtering
            report.overall_confidence = report
                .signals
                .iter()
                .map(|s| s.confidence)
                .max()
                .unwrap_or(detector::Confidence::None);
            report.ai_generated = report.overall_confidence > detector::Confidence::None;
            report
        })
        .collect();

    if args.quiet {
        // Quiet mode: just exit code
    } else if args.json {
        output::print_json(&reports);
    } else {
        output::print_human(&reports);
    }

    // Exit 0 if any AI detected, 1 if none
    if reports.iter().any(|r| r.ai_generated) {
        ExitCode::from(0)
    } else {
        ExitCode::from(1)
    }
}

fn cmd_info(info_args: &cli::InfoArgs) -> ExitCode {
    let path = &info_args.file;

    if !path.exists() {
        eprintln!("Error: {} not found", path.display());
        return ExitCode::from(2);
    }

    let report = detector::run_all_detectors(path, true);
    let xmp_props = detector::xmp::dump_info(path).unwrap_or_default();
    let exif_fields = detector::exif::dump_info(path).unwrap_or_default();

    output::print_info(&report, &xmp_props, &exif_fields);

    ExitCode::from(0)
}
