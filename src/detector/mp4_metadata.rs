use anyhow::Result;
use std::fs;
use std::path::Path;

use super::{Confidence, Signal, SignalSource};
use crate::known_tools;

/// MP4-specific tool mappings for ambiguous tool names that shouldn't be in the
/// global known_tools list (e.g. "Google" is too broad for general matching but
/// is specifically used by Google Veo in MP4 ©too atoms).
const MP4_TOOL_MAPPINGS: &[(&str, &str, Confidence)] = &[
    ("google", "google veo", Confidence::Medium),
];

/// Known H.264 SEI user_data_unregistered markers embedded by AI video tools.
/// Format: (byte pattern to search for in mdat, tool name)
const SEI_MARKERS: &[(&[u8], &str)] = &[
    // Kling: SEI type 5 (user_data_unregistered) with UUID 91ca6061-4aee-3854-8614-2d5f73f4ae2e
    (b"kling-ai", "kling"),
];

// ---------------------------------------------------------------------------
// MP4 box parsing utilities
// ---------------------------------------------------------------------------

fn read_u32_be(data: &[u8], offset: usize) -> Option<u32> {
    if offset + 4 > data.len() {
        return None;
    }
    Some(u32::from_be_bytes([
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
    ]))
}

struct BoxInfo {
    box_type: [u8; 4],
    content_start: usize,
    box_end: usize,
}

/// Iterate through sibling boxes in the given byte range.
fn find_boxes(data: &[u8], start: usize, end: usize) -> Vec<BoxInfo> {
    let mut boxes = Vec::new();
    let mut pos = start;
    while pos + 8 <= end {
        let size = match read_u32_be(data, pos) {
            Some(s) => s as u64,
            None => break,
        };
        let mut box_type = [0u8; 4];
        box_type.copy_from_slice(&data[pos + 4..pos + 8]);

        let (content_start, actual_size) = if size == 1 {
            // Extended 64-bit size
            if pos + 16 > end {
                break;
            }
            let ext = u64::from_be_bytes([
                data[pos + 8],
                data[pos + 9],
                data[pos + 10],
                data[pos + 11],
                data[pos + 12],
                data[pos + 13],
                data[pos + 14],
                data[pos + 15],
            ]);
            (pos + 16, ext)
        } else if size == 0 {
            // Box extends to end of data
            (pos + 8, (end - pos) as u64)
        } else {
            (pos + 8, size)
        };

        if actual_size < 8 {
            break;
        }

        let box_end = (pos as u64 + actual_size).min(end as u64) as usize;
        boxes.push(BoxInfo {
            box_type,
            content_start,
            box_end,
        });
        pos = box_end;
    }
    boxes
}

/// Find the first box of a given type within a range.
fn get_box(data: &[u8], start: usize, end: usize, box_type: &[u8; 4]) -> Option<(usize, usize)> {
    find_boxes(data, start, end)
        .into_iter()
        .find(|b| &b.box_type == box_type)
        .map(|b| (b.content_start, b.box_end))
}

// ---------------------------------------------------------------------------
// ilst parsing — standard iTunes format
// ---------------------------------------------------------------------------

/// Convert a 4-byte box type to a readable string.
/// iTunes atom types like ©too use 0xa9 which is not valid UTF-8,
/// so we decode as Latin-1 (ISO 8859-1) to preserve the © character.
fn box_type_to_string(box_type: &[u8; 4]) -> String {
    box_type.iter().map(|&b| b as char).collect()
}

/// Parse standard iTunes-style ilst where child atom types are the keys (e.g. ©too).
fn parse_ilst_standard(data: &[u8], start: usize, end: usize) -> Vec<(String, String)> {
    let mut results = Vec::new();
    for item in find_boxes(data, start, end) {
        let key = box_type_to_string(&item.box_type);
        // Look for 'data' sub-atom inside this item
        if let Some((data_cs, data_ce)) = get_box(data, item.content_start, item.box_end, b"data")
        {
            // data atom content: 4 bytes type/flags + 4 bytes locale + value
            if data_ce - data_cs >= 8 {
                let value =
                    String::from_utf8_lossy(&data[data_cs + 8..data_ce]).trim_matches('\0').to_string();
                if !value.is_empty() {
                    results.push((key, value));
                }
            }
        }
    }
    results
}

// ---------------------------------------------------------------------------
// ilst parsing — keys-based format
// ---------------------------------------------------------------------------

/// Parse keys box to get key names.
fn parse_keys(data: &[u8], start: usize, end: usize) -> Vec<String> {
    // keys content: 4 bytes version/flags + 4 bytes entry_count + entries
    if end - start < 8 {
        return vec![];
    }
    let count = match read_u32_be(data, start + 4) {
        Some(c) => c as usize,
        None => return vec![],
    };

    let mut keys = Vec::with_capacity(count);
    let mut offset = start + 8;
    for _ in 0..count {
        if offset + 8 > end {
            break;
        }
        let key_size = match read_u32_be(data, offset) {
            Some(s) => s as usize,
            None => break,
        };
        if key_size < 8 || offset + key_size > end {
            break;
        }
        // 4 bytes size + 4 bytes namespace + name
        let name = String::from_utf8_lossy(&data[offset + 8..offset + key_size]).to_string();
        keys.push(name);
        offset += key_size;
    }
    keys
}

/// Parse keys-based ilst where items reference keys by 1-based index.
fn parse_ilst_keyed(
    data: &[u8],
    keys: &[String],
    ilst_start: usize,
    ilst_end: usize,
) -> Vec<(String, String)> {
    let mut results = Vec::new();
    for item in find_boxes(data, ilst_start, ilst_end) {
        let idx = u32::from_be_bytes(item.box_type) as usize;
        let key_name = if idx > 0 && idx <= keys.len() {
            keys[idx - 1].clone()
        } else {
            format!("idx:{}", idx)
        };

        // Look for 'data' sub-atom
        if let Some((data_cs, data_ce)) = get_box(data, item.content_start, item.box_end, b"data")
        {
            if data_ce - data_cs >= 8 {
                let value =
                    String::from_utf8_lossy(&data[data_cs + 8..data_ce]).trim_matches('\0').to_string();
                if !value.is_empty() {
                    results.push((key_name, value));
                }
            }
        }
    }
    results
}

// ---------------------------------------------------------------------------
// Extraction: navigate moov > udta > meta > ilst
// ---------------------------------------------------------------------------

fn extract_ilst_entries(data: &[u8]) -> Vec<(String, String)> {
    let moov = match get_box(data, 0, data.len(), b"moov") {
        Some(m) => m,
        None => return vec![],
    };
    let udta = match get_box(data, moov.0, moov.1, b"udta") {
        Some(u) => u,
        None => return vec![],
    };
    let meta = match get_box(data, udta.0, udta.1, b"meta") {
        Some(m) => m,
        None => return vec![],
    };

    // meta is a full box: skip 4 bytes version/flags
    let meta_content = meta.0 + 4;
    if meta_content >= meta.1 {
        return vec![];
    }

    // Check for keys box (keys-based format)
    let keys_box = get_box(data, meta_content, meta.1, b"keys");
    let ilst = match get_box(data, meta_content, meta.1, b"ilst") {
        Some(i) => i,
        None => return vec![],
    };

    if let Some((keys_start, keys_end)) = keys_box {
        let keys = parse_keys(data, keys_start, keys_end);
        parse_ilst_keyed(data, &keys, ilst.0, ilst.1)
    } else {
        parse_ilst_standard(data, ilst.0, ilst.1)
    }
}

// ---------------------------------------------------------------------------
// Detection methods
// ---------------------------------------------------------------------------

/// Method A: Match ilst tool/software values against known AI tools.
fn detect_ilst_tools(entries: &[(String, String)]) -> Vec<Signal> {
    let mut signals = Vec::new();

    // Keys to check for tool matching (standard iTunes atom names + keys-based names)
    let tool_keys: &[&str] = &["\u{a9}too", "\u{a9}swr", "encoder", "tool", "software"];

    for (key, value) in entries {
        let is_tool_key = tool_keys.iter().any(|tk| key.eq_ignore_ascii_case(tk));
        if !is_tool_key {
            continue;
        }

        let label = match key.as_str() {
            "\u{a9}too" => "Encoding Tool",
            "\u{a9}swr" => "Software",
            _ => key.as_str(),
        };

        // First try global known_tools match
        if let Some(tool_name) = known_tools::match_ai_tool(value) {
            signals.push(Signal {
                source: SignalSource::Mp4Metadata,
                confidence: Confidence::Medium,
                description: format!("{} matches AI tool: {}", label, value),
                tool: Some(tool_name.to_string()),
                details: vec![(key.clone(), value.clone())],
            });
            continue;
        }

        // Then try MP4-specific mappings (case-insensitive exact match)
        let lower = value.to_lowercase();
        for &(pattern, mapped_tool, confidence) in MP4_TOOL_MAPPINGS {
            if lower == pattern {
                signals.push(Signal {
                    source: SignalSource::Mp4Metadata,
                    confidence,
                    description: format!("{} matches AI tool: {}", label, value),
                    tool: Some(mapped_tool.to_string()),
                    details: vec![(key.clone(), value.clone())],
                });
                break;
            }
        }
    }

    signals
}

/// Method B: Detect China AIGC labeling standard metadata.
fn detect_aigc_label(entries: &[(String, String)]) -> Vec<Signal> {
    let mut signals = Vec::new();

    for (key, value) in entries {
        if !key.eq_ignore_ascii_case("AIGC") {
            continue;
        }

        // Parse JSON-like content for Label field
        // The value is JSON like {"Label":"1", ...}
        // We do simple string matching to avoid adding a JSON dependency for this one field.
        let has_ai_label = value.contains("\"Label\":\"1\"") || value.contains("\"Label\": \"1\"");
        if !has_ai_label {
            continue;
        }

        // Try to extract ProduceID for tool identification
        let produce_id = extract_json_field(value, "ProduceID");
        let description = if let Some(ref pid) = produce_id {
            format!("AIGC label indicates AI-generated content (ProduceID: {})", pid)
        } else {
            "AIGC label indicates AI-generated content".to_string()
        };

        let mut details = vec![("AIGC".to_string(), value.clone())];
        if let Some(pid) = &produce_id {
            details.push(("ProduceID".to_string(), pid.clone()));
        }

        signals.push(Signal {
            source: SignalSource::Mp4Metadata,
            confidence: Confidence::Medium,
            description,
            tool: None,
            details,
        });
    }

    signals
}

/// Simple JSON field extractor (avoids serde_json dependency for this one use).
fn extract_json_field(json: &str, field: &str) -> Option<String> {
    let pattern = format!("\"{}\"", field);
    let idx = json.find(&pattern)?;
    let after = &json[idx + pattern.len()..];
    // Skip whitespace and colon
    let after = after.trim_start();
    let after = after.strip_prefix(':')?;
    let after = after.trim_start();
    // Extract quoted value
    let after = after.strip_prefix('"')?;
    let end = after.find('"')?;
    Some(after[..end].to_string())
}

/// Method C: Scan mdat for known H.264 SEI watermark markers.
fn detect_sei_markers(data: &[u8]) -> Vec<Signal> {
    let mut signals = Vec::new();

    let mdat = match get_box(data, 0, data.len(), b"mdat") {
        Some(m) => m,
        None => return signals,
    };

    // Scan first 1MB of mdat for performance
    let scan_end = mdat.1.min(mdat.0 + 1_048_576);
    let scan_data = &data[mdat.0..scan_end];

    for &(marker, tool_name) in SEI_MARKERS {
        if scan_data.windows(marker.len()).any(|w| w == marker) {
            signals.push(Signal {
                source: SignalSource::Mp4Metadata,
                confidence: Confidence::Medium,
                description: format!(
                    "H.264 SEI watermark: {}",
                    String::from_utf8_lossy(marker)
                ),
                tool: Some(tool_name.to_string()),
                details: vec![(
                    "SEI marker".to_string(),
                    String::from_utf8_lossy(marker).to_string(),
                )],
            });
        }
    }

    signals
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Detect AI signals from MP4 container metadata.
pub fn detect(path: &Path) -> Result<Vec<Signal>> {
    let data = fs::read(path)?;

    // Quick check: is this an MP4/MOV file? (ftyp box should be near the start)
    if get_box(&data, 0, data.len().min(64), b"ftyp").is_none() {
        return Ok(vec![]);
    }

    let entries = extract_ilst_entries(&data);

    let mut signals = Vec::new();
    signals.extend(detect_ilst_tools(&entries));
    signals.extend(detect_aigc_label(&entries));
    signals.extend(detect_sei_markers(&data));

    Ok(signals)
}

/// Dump all MP4 metadata for the `info` subcommand.
pub fn dump_info(path: &Path) -> Result<Vec<(String, String)>> {
    let data = fs::read(path)?;

    if get_box(&data, 0, data.len().min(64), b"ftyp").is_none() {
        return Ok(vec![]);
    }

    let mut props = extract_ilst_entries(&data);

    // Also report SEI markers found
    let mdat = get_box(&data, 0, data.len(), b"mdat");
    if let Some((mdat_start, mdat_end)) = mdat {
        let scan_end = mdat_end.min(mdat_start + 1_048_576);
        let scan_data = &data[mdat_start..scan_end];

        for &(marker, tool_name) in SEI_MARKERS {
            if scan_data.windows(marker.len()).any(|w| w == marker) {
                props.push((
                    "SEI watermark".to_string(),
                    format!("{} ({})", String::from_utf8_lossy(marker), tool_name),
                ));
            }
        }
    }

    Ok(props)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_json_field() {
        let json = r#"{"Label":"1","ProduceID":"abc-123","Other":"val"}"#;
        assert_eq!(extract_json_field(json, "Label"), Some("1".to_string()));
        assert_eq!(
            extract_json_field(json, "ProduceID"),
            Some("abc-123".to_string())
        );
        assert_eq!(extract_json_field(json, "Missing"), None);

        // With spaces
        let json2 = r#"{"Label": "1", "ProduceID": "xyz"}"#;
        assert_eq!(extract_json_field(json2, "Label"), Some("1".to_string()));
        assert_eq!(
            extract_json_field(json2, "ProduceID"),
            Some("xyz".to_string())
        );
    }

    #[test]
    fn test_detect_ilst_tools_known_tool() {
        let entries = vec![("\u{a9}too".to_string(), "Runway Gen-3".to_string())];
        let signals = detect_ilst_tools(&entries);
        assert_eq!(signals.len(), 1);
        assert_eq!(signals[0].tool, Some("runway".to_string()));
        assert_eq!(signals[0].confidence, Confidence::Medium);
    }

    #[test]
    fn test_detect_ilst_tools_mp4_mapping() {
        let entries = vec![("\u{a9}too".to_string(), "Google".to_string())];
        let signals = detect_ilst_tools(&entries);
        assert_eq!(signals.len(), 1);
        assert_eq!(signals[0].tool, Some("google veo".to_string()));
    }

    #[test]
    fn test_detect_ilst_tools_no_match() {
        let entries = vec![("\u{a9}too".to_string(), "Lavf60.16.100".to_string())];
        let signals = detect_ilst_tools(&entries);
        assert!(signals.is_empty());
    }

    #[test]
    fn test_detect_aigc_label() {
        let entries = vec![(
            "AIGC".to_string(),
            r#"{"Label":"1","ProduceID":"test-123"}"#.to_string(),
        )];
        let signals = detect_aigc_label(&entries);
        assert_eq!(signals.len(), 1);
        assert_eq!(signals[0].confidence, Confidence::Medium);
        assert!(signals[0].description.contains("AIGC"));
        assert!(signals[0].description.contains("test-123"));
    }

    #[test]
    fn test_detect_aigc_label_not_ai() {
        let entries = vec![(
            "AIGC".to_string(),
            r#"{"Label":"0","ProduceID":"test"}"#.to_string(),
        )];
        let signals = detect_aigc_label(&entries);
        assert!(signals.is_empty());
    }

    #[test]
    fn test_detect_keyed_encoder() {
        let entries = vec![("encoder".to_string(), "Sora v2".to_string())];
        let signals = detect_ilst_tools(&entries);
        assert_eq!(signals.len(), 1);
        assert_eq!(signals[0].tool, Some("sora".to_string()));
    }

    #[test]
    fn test_parse_standard_ilst() {
        // Construct a minimal standard ilst: ilst > ©too > data > "Google"
        let value = b"Google";
        let data_atom_size: u32 = 8 + 8 + value.len() as u32; // header + flags+locale + value
        let item_size: u32 = 8 + data_atom_size;
        let ilst_size: u32 = 8 + item_size;

        let mut buf = Vec::new();
        // ilst header
        buf.extend_from_slice(&ilst_size.to_be_bytes());
        buf.extend_from_slice(b"ilst");
        // ©too item header
        buf.extend_from_slice(&item_size.to_be_bytes());
        buf.extend_from_slice(&[0xa9, b't', b'o', b'o']);
        // data atom header
        buf.extend_from_slice(&data_atom_size.to_be_bytes());
        buf.extend_from_slice(b"data");
        // type flags + locale
        buf.extend_from_slice(&[0, 0, 0, 1]); // flags: UTF-8 text
        buf.extend_from_slice(&[0, 0, 0, 0]); // locale
        // value
        buf.extend_from_slice(value);

        let entries = parse_ilst_standard(&buf, 8, buf.len());
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].0, "\u{a9}too");
        assert_eq!(entries[0].1, "Google");
    }
}
