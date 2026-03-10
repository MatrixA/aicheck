pub mod c2pa_detector;
pub mod exif;
pub mod watermark;
pub mod xmp;

use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Confidence {
    None,
    Low,
    Medium,
    High,
}

impl std::fmt::Display for Confidence {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Confidence::None => write!(f, "NONE"),
            Confidence::Low => write!(f, "LOW"),
            Confidence::Medium => write!(f, "MEDIUM"),
            Confidence::High => write!(f, "HIGH"),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SignalSource {
    C2pa,
    Xmp,
    Exif,
    Watermark,
}

impl std::fmt::Display for SignalSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SignalSource::C2pa => write!(f, "C2PA"),
            SignalSource::Xmp => write!(f, "XMP"),
            SignalSource::Exif => write!(f, "EXIF"),
            SignalSource::Watermark => write!(f, "WATERMARK"),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Signal {
    pub source: SignalSource,
    pub confidence: Confidence,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub details: Vec<(String, String)>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FileReport {
    pub path: PathBuf,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    pub signals: Vec<Signal>,
    pub overall_confidence: Confidence,
    pub ai_generated: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl FileReport {
    pub fn from_signals(path: PathBuf, mime_type: Option<String>, signals: Vec<Signal>) -> Self {
        let overall_confidence = signals
            .iter()
            .map(|s| s.confidence)
            .max()
            .unwrap_or(Confidence::None);
        let ai_generated = overall_confidence > Confidence::None;
        FileReport {
            path,
            mime_type,
            signals,
            overall_confidence,
            ai_generated,
            error: None,
        }
    }

    #[allow(dead_code)]
    pub fn from_error(path: PathBuf, error: String) -> Self {
        FileReport {
            path,
            mime_type: None,
            signals: vec![],
            overall_confidence: Confidence::None,
            ai_generated: false,
            error: Some(error),
        }
    }
}

/// Run all detectors on a file and return a combined report.
/// When `deep` is true, also runs pixel-level watermark analysis.
pub fn run_all_detectors(path: &Path, deep: bool) -> FileReport {
    let mime_type = infer::get_from_path(path)
        .ok()
        .flatten()
        .map(|t| t.mime_type().to_string());

    let mut signals = Vec::new();

    // C2PA detector — highest confidence
    match c2pa_detector::detect(path) {
        Ok(sigs) => signals.extend(sigs),
        Err(e) => {
            // C2PA errors are non-fatal (file may just not have a manifest)
            if std::env::var("AIC_DEBUG").is_ok() {
                eprintln!("  [debug] C2PA: {}", e);
            }
        }
    }

    // XMP detector
    match xmp::detect(path) {
        Ok(sigs) => signals.extend(sigs),
        Err(e) => {
            if std::env::var("AIC_DEBUG").is_ok() {
                eprintln!("  [debug] XMP: {}", e);
            }
        }
    }

    // EXIF detector
    match exif::detect(path) {
        Ok(sigs) => signals.extend(sigs),
        Err(e) => {
            if std::env::var("AIC_DEBUG").is_ok() {
                eprintln!("  [debug] EXIF: {}", e);
            }
        }
    }

    // Watermark detector — pixel-level analysis (opt-in via --deep)
    if deep {
        match watermark::detect(path) {
            Ok(sigs) => signals.extend(sigs),
            Err(e) => {
                if std::env::var("AIC_DEBUG").is_ok() {
                    eprintln!("  [debug] Watermark: {}", e);
                }
            }
        }
    }

    FileReport::from_signals(path.to_path_buf(), mime_type, signals)
}
