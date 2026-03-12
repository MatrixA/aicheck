pub mod audio_spectral;
pub mod c2pa_detector;
pub mod exif;
pub mod filename;
pub mod id3_metadata;
pub mod mp4_metadata;
pub mod png_text;
pub mod watermark;
pub mod wav_metadata;
pub mod xmp;

use serde::Serialize;
use std::path::{Path, PathBuf};

use crate::i18n;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Confidence {
    None,
    Low,
    Medium,
    High,
}

impl Confidence {
    /// Localized display string for human output.
    pub fn localized(&self) -> String {
        match self {
            Confidence::None => i18n::t("confidence_none", &[]),
            Confidence::Low => i18n::t("confidence_low", &[]),
            Confidence::Medium => i18n::t("confidence_medium", &[]),
            Confidence::High => i18n::t("confidence_high", &[]),
        }
    }
}

impl std::fmt::Display for Confidence {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.localized())
    }
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SignalSource {
    C2pa,
    Xmp,
    Exif,
    Watermark,
    AudioSpectral,
    Filename,
    Id3Metadata,
    Mp4Metadata,
    PngText,
    WavMetadata,
}

impl std::fmt::Display for SignalSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SignalSource::C2pa => write!(f, "C2PA"),
            SignalSource::Xmp => write!(f, "XMP"),
            SignalSource::Exif => write!(f, "EXIF"),
            SignalSource::Watermark => write!(f, "WATERMARK"),
            SignalSource::AudioSpectral => write!(f, "SPEC"),
            SignalSource::Filename => write!(f, "FILE"),
            SignalSource::Id3Metadata => write!(f, "ID3"),
            SignalSource::Mp4Metadata => write!(f, "MP4"),
            SignalSource::PngText => write!(f, "PNG"),
            SignalSource::WavMetadata => write!(f, "WAV"),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Signal {
    pub source: SignalSource,
    pub confidence: Confidence,
    /// Always English — used for JSON serialization.
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub details: Vec<(String, String)>,
    /// Translation key for localized output.
    #[serde(skip)]
    pub msg_key: String,
    /// Translation parameters for localized output.
    #[serde(skip)]
    pub msg_params: Vec<(String, String)>,
}

impl Signal {
    /// Render the localized description for human output.
    pub fn localized_description(&self) -> String {
        if self.msg_key.is_empty() {
            return self.description.clone();
        }
        let params: Vec<(&str, &str)> = self
            .msg_params
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();
        i18n::t(&self.msg_key, &params)
    }
}

/// Builder for creating Signal with both English description and i18n key.
pub struct SignalBuilder {
    source: SignalSource,
    confidence: Confidence,
    msg_key: String,
    msg_params: Vec<(String, String)>,
    tool: Option<String>,
    details: Vec<(String, String)>,
}

impl SignalBuilder {
    pub fn new(source: SignalSource, confidence: Confidence, key: &str) -> Self {
        Self {
            source,
            confidence,
            msg_key: key.to_string(),
            msg_params: Vec::new(),
            tool: None,
            details: Vec::new(),
        }
    }

    pub fn param(mut self, name: &str, value: impl Into<String>) -> Self {
        self.msg_params.push((name.to_string(), value.into()));
        self
    }

    pub fn tool(mut self, tool: impl Into<String>) -> Self {
        self.tool = Some(tool.into());
        self
    }

    pub fn tool_opt(mut self, tool: Option<String>) -> Self {
        self.tool = tool;
        self
    }

    pub fn details(mut self, details: Vec<(String, String)>) -> Self {
        self.details = details;
        self
    }

    pub fn detail(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.details.push((key.into(), value.into()));
        self
    }

    pub fn build(self) -> Signal {
        let params: Vec<(&str, &str)> = self
            .msg_params
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();
        let description = i18n::t_en(&self.msg_key, &params);
        Signal {
            source: self.source,
            confidence: self.confidence,
            description,
            tool: self.tool,
            details: self.details,
            msg_key: self.msg_key,
            msg_params: self.msg_params,
        }
    }
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
    /// Detected creation software (informational, not AI-related).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub software_info: Vec<(String, String)>,
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
            software_info: Vec::new(),
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
            software_info: Vec::new(),
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
            if std::env::var("AIC_DEBUG").is_ok() {
                eprintln!("  [debug] C2PA: {}", e);
            }
        }
    }

    // MP4 metadata detector (ilst atoms, AIGC labels, SEI watermarks)
    match mp4_metadata::detect(path) {
        Ok(sigs) => signals.extend(sigs),
        Err(e) => {
            if std::env::var("AIC_DEBUG").is_ok() {
                eprintln!("  [debug] MP4 Metadata: {}", e);
            }
        }
    }

    // ID3 metadata detector (audio files: MP3 ID3v2 tags)
    match id3_metadata::detect(path) {
        Ok(sigs) => signals.extend(sigs),
        Err(e) => {
            if std::env::var("AIC_DEBUG").is_ok() {
                eprintln!("  [debug] ID3 Metadata: {}", e);
            }
        }
    }

    // WAV metadata detector (RIFF INFO chunks + audio heuristics)
    match wav_metadata::detect(path) {
        Ok(sigs) => signals.extend(sigs),
        Err(e) => {
            if std::env::var("AIC_DEBUG").is_ok() {
                eprintln!("  [debug] WAV Metadata: {}", e);
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

    // PNG text chunk detector
    if mime_type.as_deref() == Some("image/png") {
        match png_text::detect(path) {
            Ok(sigs) => signals.extend(sigs),
            Err(e) => {
                if std::env::var("AIC_DEBUG").is_ok() {
                    eprintln!("  [debug] PNG text: {}", e);
                }
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

    // Filename pattern detector
    match filename::detect(path) {
        Ok(sigs) => signals.extend(sigs),
        Err(e) => {
            if std::env::var("AIC_DEBUG").is_ok() {
                eprintln!("  [debug] Filename: {}", e);
            }
        }
    }

    // Audio spectral analysis (WAV files — frequency cutoff, spectral flatness)
    if deep || signals.is_empty() {
        match audio_spectral::detect(path) {
            Ok(sigs) => signals.extend(sigs),
            Err(e) => {
                if std::env::var("AIC_DEBUG").is_ok() {
                    eprintln!("  [debug] Audio Spectral: {}", e);
                }
            }
        }
    }

    // Watermark detector — pixel-level analysis
    let is_video = mime_type
        .as_deref()
        .map(|m| m.starts_with("video/"))
        .unwrap_or(false);

    if deep || signals.is_empty() {
        if is_video {
            // Video: extract frames and analyze
            match watermark::detect_video(path) {
                Ok(sigs) => signals.extend(sigs),
                Err(e) => {
                    if std::env::var("AIC_DEBUG").is_ok() {
                        eprintln!("  [debug] Watermark (video): {}", e);
                    }
                }
            }
        } else {
            // Image: direct pixel analysis
            match watermark::detect(path) {
                Ok(sigs) => signals.extend(sigs),
                Err(e) => {
                    if std::env::var("AIC_DEBUG").is_ok() {
                        eprintln!("  [debug] Watermark: {}", e);
                    }
                }
            }
        }
    }

    let mut report = FileReport::from_signals(path.to_path_buf(), mime_type, signals);

    // Collect informational software metadata
    if let Ok(sw) = mp4_metadata::detect_software(path) {
        report.software_info.extend(sw);
    }

    report
}
