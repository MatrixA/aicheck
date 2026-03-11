//! # AICheck
//!
//! Detect AI-generated content via provenance signals — offline, with no API keys.
//!
//! AICheck analyzes images, videos, audio, and documents for signs of AI generation
//! by inspecting multiple metadata sources:
//!
//! - **C2PA manifests** — cryptographically signed provenance (highest confidence)
//! - **XMP / IPTC metadata** — embedded creator tool references
//! - **EXIF data** — camera and software fields
//! - **Container metadata** — MP4 iTunes atoms, ID3 tags, WAV RIFF chunks
//! - **Invisible watermarks** — DWT-DCT pixel-level analysis
//! - **Spectral analysis** — FFT-based frequency patterns in audio
//! - **Filename patterns** — known AI tool naming conventions
//!
//! ## Quick start
//!
//! ```rust,no_run
//! use std::path::Path;
//! use aicheck::detector;
//!
//! let report = detector::run_all_detectors(Path::new("image.png"), false);
//! println!("AI generated: {}", report.ai_generated);
//! for signal in &report.signals {
//!     println!("  [{:?}] {} (confidence: {:?})", signal.source, signal.description, signal.confidence);
//! }
//! ```

pub mod detector;
pub mod i18n;
pub mod known_tools;
pub mod scanner;

rust_i18n::i18n!("locales", fallback = "en");
