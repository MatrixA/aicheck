use anyhow::Result;
use std::path::Path;

use super::{Confidence, SignalBuilder, Signal, SignalSource};

/// Known filename patterns from AI audio/media generation tools.
const FILENAME_PATTERNS: &[(&str, &str)] = &[
    ("elevenlabs_", "elevenlabs"),
    ("suno_", "suno"),
    ("soundraw_", "soundraw"),
    ("aiva_", "aiva"),
    ("mubert_", "mubert"),
    ("boomy_", "boomy"),
    ("beatoven_", "beatoven"),
    ("dall-e", "dall-e"),
    ("dalle", "dall-e"),
    ("midjourney", "midjourney"),
    ("comfyui", "comfyui"),
    ("stability_", "stable diffusion"),
];

/// Detect AI signals from the filename itself.
pub fn detect(path: &Path) -> Result<Vec<Signal>> {
    let filename = match path.file_name().and_then(|f| f.to_str()) {
        Some(f) => f,
        None => return Ok(vec![]),
    };

    let lower = filename.to_lowercase();
    let mut signals = Vec::new();

    for &(pattern, tool_name) in FILENAME_PATTERNS {
        if lower.contains(pattern) {
            signals.push(
                SignalBuilder::new(SignalSource::Filename, Confidence::Low, "signal_filename_pattern")
                    .param("pattern", pattern)
                    .tool(tool_name)
                    .detail("filename", filename)
                    .build(),
            );
            break;
        }
    }

    if signals.is_empty() && detect_elevenlabs_pattern(&lower) {
        signals.push(
            SignalBuilder::new(SignalSource::Filename, Confidence::Low, "signal_filename_elevenlabs")
                .tool("elevenlabs")
                .detail("filename", filename)
                .build(),
        );
    }

    Ok(signals)
}

fn detect_elevenlabs_pattern(lower: &str) -> bool {
    if !lower.starts_with("elevenlabs_") {
        return false;
    }
    let rest = &lower["elevenlabs_".len()..];
    if rest.len() < 20 {
        return false;
    }
    let bytes = rest.as_bytes();
    bytes[4] == b'-' && bytes[7] == b'-' && bytes[10] == b't' && bytes[13] == b'_' && bytes[16] == b'_'
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_elevenlabs_filename() {
        let path = PathBuf::from("/tmp/ElevenLabs_2026-03-11T04_15_43_Liam - Energetic.mp3");
        let signals = detect(&path).unwrap();
        assert_eq!(signals.len(), 1);
        assert_eq!(signals[0].tool, Some("elevenlabs".to_string()));
        assert_eq!(signals[0].confidence, Confidence::Low);
    }

    #[test]
    fn test_soundraw_filename() {
        let path = PathBuf::from("/tmp/soundraw_track_001.mp3");
        let signals = detect(&path).unwrap();
        assert_eq!(signals.len(), 1);
        assert_eq!(signals[0].tool, Some("soundraw".to_string()));
    }

    #[test]
    fn test_normal_filename_no_match() {
        let path = PathBuf::from("/tmp/my_song.mp3");
        let signals = detect(&path).unwrap();
        assert!(signals.is_empty());
    }

    #[test]
    fn test_midjourney_filename() {
        let path = PathBuf::from("/tmp/midjourney_v6_abc123.png");
        let signals = detect(&path).unwrap();
        assert_eq!(signals.len(), 1);
        assert_eq!(signals[0].tool, Some("midjourney".to_string()));
    }

    #[test]
    fn test_elevenlabs_pattern_detection() {
        assert!(detect_elevenlabs_pattern("elevenlabs_2026-03-11t04_15_43_something"));
        assert!(!detect_elevenlabs_pattern("elevenlabs_short"));
        assert!(!detect_elevenlabs_pattern("something_else"));
    }
}
