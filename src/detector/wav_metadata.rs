use anyhow::Result;
use std::fs;
use std::path::Path;

use super::{Confidence, Signal, SignalSource};
use crate::known_tools;

/// TTS-typical sample rates (not used in professional music/speech recording).
/// 22050 and 24000 Hz are the most common TTS model output rates.
const TTS_SAMPLE_RATES: &[u32] = &[22050, 24000, 16000];

// ---------------------------------------------------------------------------
// RIFF/WAV parsing
// ---------------------------------------------------------------------------

/// Parsed WAV format information.
pub(crate) struct WavFmt {
    pub channels: u16,
    pub sample_rate: u32,
    pub bits_per_sample: u16,
}

/// Parsed WAV file: format + INFO metadata + raw PCM data range.
#[allow(dead_code)]
pub(crate) struct WavFile {
    pub fmt: WavFmt,
    pub info_entries: Vec<(String, String)>,
    pub pcm_start: usize,
    pub pcm_end: usize,
}

/// Parse a WAV file and extract format info, LIST/INFO metadata, and data chunk location.
pub(crate) fn parse_wav_full(data: &[u8]) -> Option<WavFile> {
    let (fmt, info_entries, pcm_start, pcm_end) = parse_wav_inner(data)?;
    Some(WavFile { fmt, info_entries, pcm_start, pcm_end })
}

/// Parse a WAV file and extract format info + LIST/INFO metadata.
fn parse_wav(data: &[u8]) -> Option<(WavFmt, Vec<(String, String)>)> {
    let (fmt, info, _, _) = parse_wav_inner(data)?;
    Some((fmt, info))
}

fn parse_wav_inner(data: &[u8]) -> Option<(WavFmt, Vec<(String, String)>, usize, usize)> {
    // Minimum: RIFF(4) + size(4) + WAVE(4) + fmt chunk header(8) + fmt data(16) = 36
    if data.len() < 36 {
        return None;
    }
    if &data[0..4] != b"RIFF" || &data[8..12] != b"WAVE" {
        return None;
    }

    let mut fmt = None;
    let mut info_entries = Vec::new();
    let mut data_start = 0usize;
    let mut data_end = 0usize;
    let mut pos = 12; // After "WAVE"

    while pos + 8 <= data.len() {
        let chunk_id = &data[pos..pos + 4];
        let chunk_size = u32::from_le_bytes([
            data[pos + 4],
            data[pos + 5],
            data[pos + 6],
            data[pos + 7],
        ]) as usize;
        let chunk_data_start = pos + 8;
        let chunk_data_end = (chunk_data_start + chunk_size).min(data.len());

        if chunk_id == b"fmt " && chunk_size >= 16 {
            let d = &data[chunk_data_start..chunk_data_end];
            fmt = Some(WavFmt {
                channels: u16::from_le_bytes([d[2], d[3]]),
                sample_rate: u32::from_le_bytes([d[4], d[5], d[6], d[7]]),
                bits_per_sample: u16::from_le_bytes([d[14], d[15]]),
            });
        } else if chunk_id == b"data" {
            data_start = chunk_data_start;
            data_end = chunk_data_end;
        } else if chunk_id == b"LIST" && chunk_size >= 4 {
            let list_type = &data[chunk_data_start..chunk_data_start + 4];
            if list_type == b"INFO" {
                // Parse INFO sub-chunks
                let mut sub_pos = chunk_data_start + 4;
                while sub_pos + 8 <= chunk_data_end {
                    let sub_id = std::str::from_utf8(&data[sub_pos..sub_pos + 4])
                        .unwrap_or("????")
                        .to_string();
                    let sub_size = u32::from_le_bytes([
                        data[sub_pos + 4],
                        data[sub_pos + 5],
                        data[sub_pos + 6],
                        data[sub_pos + 7],
                    ]) as usize;
                    let sub_data_start = sub_pos + 8;
                    let sub_data_end = (sub_data_start + sub_size).min(chunk_data_end);

                    if sub_data_start < sub_data_end {
                        let value = String::from_utf8_lossy(&data[sub_data_start..sub_data_end])
                            .trim_matches('\0')
                            .to_string();
                        if !value.is_empty() {
                            info_entries.push((sub_id, value));
                        }
                    }

                    // Sub-chunks are word-aligned
                    sub_pos = sub_data_start + ((sub_size + 1) & !1);
                }
            }
        }

        // Chunks are word-aligned
        pos = chunk_data_start + ((chunk_size + 1) & !1);
    }

    fmt.map(|f| (f, info_entries, data_start, data_end))
}

// ---------------------------------------------------------------------------
// Detection
// ---------------------------------------------------------------------------

/// Detect AI signals from WAV RIFF metadata and audio characteristics.
pub fn detect(path: &Path) -> Result<Vec<Signal>> {
    let data = fs::read(path)?;
    let (fmt, info_entries) = match parse_wav(&data) {
        Some(r) => r,
        None => return Ok(vec![]),
    };

    let mut signals = Vec::new();

    // Method A: Check LIST/INFO chunks for AI tool references
    let tool_keys = ["ISFT", "ICMT", "IART", "IENG", "IPRD", "IGNR"];
    for (key, value) in &info_entries {
        if tool_keys.contains(&key.as_str()) {
            if let Some(tool_name) = known_tools::match_ai_tool(value) {
                signals.push(Signal {
                    source: SignalSource::WavMetadata,
                    confidence: Confidence::Medium,
                    description: format!("WAV INFO {} matches AI tool: {}", key, value),
                    tool: Some(tool_name.to_string()),
                    details: vec![(key.clone(), value.clone())],
                });
            }
        }
    }

    // Method B: Audio characteristics heuristic for TTS detection
    // TTS hallmarks: mono + non-standard sample rate (22050/24000/16000 Hz)
    let is_tts_rate = TTS_SAMPLE_RATES.contains(&fmt.sample_rate);
    let is_mono = fmt.channels == 1;

    if is_mono && is_tts_rate {
        signals.push(Signal {
            source: SignalSource::WavMetadata,
            confidence: Confidence::Low,
            description: format!(
                "Audio characteristics suggest TTS: mono {}Hz {}bit",
                fmt.sample_rate, fmt.bits_per_sample
            ),
            tool: None,
            details: vec![
                ("channels".to_string(), fmt.channels.to_string()),
                ("sample_rate".to_string(), format!("{}Hz", fmt.sample_rate)),
                ("bits_per_sample".to_string(), fmt.bits_per_sample.to_string()),
            ],
        });
    }

    Ok(signals)
}

/// Dump WAV metadata for the `info` subcommand.
pub fn dump_info(path: &Path) -> Result<Vec<(String, String)>> {
    let data = fs::read(path)?;
    let (fmt, info_entries) = match parse_wav(&data) {
        Some(r) => r,
        None => return Ok(vec![]),
    };

    let mut props = Vec::new();
    props.push(("Sample Rate".to_string(), format!("{}Hz", fmt.sample_rate)));
    props.push(("Channels".to_string(), fmt.channels.to_string()));
    props.push(("Bits Per Sample".to_string(), fmt.bits_per_sample.to_string()));

    for (key, value) in info_entries {
        let label = match key.as_str() {
            "ISFT" => "Software (ISFT)",
            "ICMT" => "Comment (ICMT)",
            "IART" => "Artist (IART)",
            "IENG" => "Engineer (IENG)",
            "IPRD" => "Product (IPRD)",
            "IGNR" => "Genre (IGNR)",
            "INAM" => "Name (INAM)",
            "ICRD" => "Date (ICRD)",
            other => other,
        };
        props.push((label.to_string(), value));
    }

    Ok(props)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a minimal valid WAV file in memory.
    fn make_wav(channels: u16, sample_rate: u32, bits_per_sample: u16, info_chunks: &[(&str, &str)]) -> Vec<u8> {
        let byte_rate = sample_rate * channels as u32 * bits_per_sample as u32 / 8;
        let block_align = channels * bits_per_sample / 8;
        // Tiny data chunk: 100 samples of silence
        let data_size = 100u32 * block_align as u32;

        let mut buf = Vec::new();

        // Build INFO list if needed
        let mut info_buf = Vec::new();
        if !info_chunks.is_empty() {
            info_buf.extend_from_slice(b"INFO");
            for &(key, value) in info_chunks {
                let val_bytes = value.as_bytes();
                let padded_len = ((val_bytes.len() + 1 + 1) & !1) as u32; // +1 for null, word-align
                info_buf.extend_from_slice(key.as_bytes());
                info_buf.extend_from_slice(&padded_len.to_le_bytes());
                info_buf.extend_from_slice(val_bytes);
                info_buf.push(0); // null terminator
                if (val_bytes.len() + 1) % 2 != 0 {
                    info_buf.push(0); // padding
                }
            }
        }

        let fmt_size = 16u32;
        let list_chunk_size = if info_buf.is_empty() { 0 } else { 8 + info_buf.len() as u32 };
        let riff_size = 4 + 8 + fmt_size + 8 + data_size + list_chunk_size;

        // RIFF header
        buf.extend_from_slice(b"RIFF");
        buf.extend_from_slice(&riff_size.to_le_bytes());
        buf.extend_from_slice(b"WAVE");

        // fmt chunk
        buf.extend_from_slice(b"fmt ");
        buf.extend_from_slice(&fmt_size.to_le_bytes());
        buf.extend_from_slice(&1u16.to_le_bytes()); // PCM
        buf.extend_from_slice(&channels.to_le_bytes());
        buf.extend_from_slice(&sample_rate.to_le_bytes());
        buf.extend_from_slice(&byte_rate.to_le_bytes());
        buf.extend_from_slice(&block_align.to_le_bytes());
        buf.extend_from_slice(&bits_per_sample.to_le_bytes());

        // LIST/INFO chunk
        if !info_buf.is_empty() {
            buf.extend_from_slice(b"LIST");
            buf.extend_from_slice(&(info_buf.len() as u32).to_le_bytes());
            buf.extend_from_slice(&info_buf);
        }

        // data chunk
        buf.extend_from_slice(b"data");
        buf.extend_from_slice(&data_size.to_le_bytes());
        buf.extend_from_slice(&vec![0u8; data_size as usize]);

        buf
    }

    #[test]
    fn test_parse_wav_basic() {
        let wav = make_wav(1, 24000, 16, &[]);
        let (fmt, info) = parse_wav(&wav).unwrap();
        assert_eq!(fmt.channels, 1);
        assert_eq!(fmt.sample_rate, 24000);
        assert_eq!(fmt.bits_per_sample, 16);
        assert!(info.is_empty());
    }

    #[test]
    fn test_parse_wav_with_info() {
        let wav = make_wav(2, 44100, 16, &[("ISFT", "Test Software")]);
        let (fmt, info) = parse_wav(&wav).unwrap();
        assert_eq!(fmt.channels, 2);
        assert_eq!(fmt.sample_rate, 44100);
        assert_eq!(info.len(), 1);
        assert_eq!(info[0].0, "ISFT");
        assert_eq!(info[0].1, "Test Software");
    }

    #[test]
    fn test_tts_heuristic_mono_24k() {
        let wav = make_wav(1, 24000, 16, &[]);
        let tmp = tempfile::NamedTempFile::new().unwrap();
        fs::write(tmp.path(), &wav).unwrap();
        let signals = detect(tmp.path()).unwrap();
        assert_eq!(signals.len(), 1);
        assert_eq!(signals[0].confidence, Confidence::Low);
        assert!(signals[0].description.contains("TTS"));
    }

    #[test]
    fn test_no_tts_heuristic_stereo_44k() {
        let wav = make_wav(2, 44100, 16, &[]);
        let tmp = tempfile::NamedTempFile::new().unwrap();
        fs::write(tmp.path(), &wav).unwrap();
        let signals = detect(tmp.path()).unwrap();
        assert!(signals.is_empty());
    }

    #[test]
    fn test_info_tool_match() {
        let wav = make_wav(2, 44100, 16, &[("ISFT", "ElevenLabs TTS")]);
        let tmp = tempfile::NamedTempFile::new().unwrap();
        fs::write(tmp.path(), &wav).unwrap();
        let signals = detect(tmp.path()).unwrap();
        assert_eq!(signals.len(), 1);
        assert_eq!(signals[0].tool, Some("elevenlabs".to_string()));
        assert_eq!(signals[0].confidence, Confidence::Medium);
    }

    #[test]
    fn test_not_wav() {
        let data = b"not a wav file at all";
        assert!(parse_wav(data).is_none());
    }

    #[test]
    fn test_dump_info() {
        let wav = make_wav(1, 24000, 16, &[("ISFT", "MyTool"), ("ICMT", "A comment")]);
        let tmp = tempfile::NamedTempFile::new().unwrap();
        fs::write(tmp.path(), &wav).unwrap();
        let props = dump_info(tmp.path()).unwrap();
        assert!(props.iter().any(|(k, v)| k == "Sample Rate" && v == "24000Hz"));
        assert!(props.iter().any(|(k, v)| k.contains("ISFT") && v == "MyTool"));
        assert!(props.iter().any(|(k, v)| k.contains("ICMT") && v == "A comment"));
    }
}
