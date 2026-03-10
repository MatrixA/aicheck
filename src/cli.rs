use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "aic",
    version,
    about = "Detect AI-generated content via provenance signals"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,

    /// Output as JSON
    #[arg(long, global = true)]
    pub json: bool,

    /// Quiet mode: only set exit code, no output
    #[arg(short, long, global = true)]
    pub quiet: bool,

    /// Disable colored output
    #[arg(long, global = true)]
    pub no_color: bool,
}

#[derive(Subcommand)]
pub enum Command {
    /// Analyze files for AI-generation signals
    Check(CheckArgs),
    /// Dump all provenance metadata for a file
    Info(InfoArgs),
}

#[derive(clap::Args)]
pub struct CheckArgs {
    /// Files or directories to analyze
    #[arg(required = true)]
    pub paths: Vec<PathBuf>,

    /// Recurse into directories
    #[arg(short = 'r', long)]
    pub recursive: bool,

    /// Minimum confidence level to report
    #[arg(long, value_enum, default_value = "low")]
    pub min_confidence: ConfidenceFilter,

    /// Enable deep pixel analysis (invisible watermark detection)
    #[arg(long)]
    pub deep: bool,
}

#[derive(clap::Args)]
pub struct InfoArgs {
    /// File to inspect
    pub file: PathBuf,
}

#[derive(Clone, ValueEnum)]
pub enum ConfidenceFilter {
    Low,
    Medium,
    High,
}

impl ConfidenceFilter {
    pub fn to_confidence(&self) -> crate::detector::Confidence {
        match self {
            ConfidenceFilter::Low => crate::detector::Confidence::Low,
            ConfidenceFilter::Medium => crate::detector::Confidence::Medium,
            ConfidenceFilter::High => crate::detector::Confidence::High,
        }
    }
}
