use anyhow::Result;
use std::fs;
use std::path::Path;

use super::{Confidence, SignalBuilder, Signal, SignalSource};
use crate::known_tools;

const MP4_TOOL_MAPPINGS: &[(&str, &str, Confidence)] = &[
    ("google", "google veo", Confidence::Medium),
];

const SEI_MARKERS: &[(&[u8], &str)] = &[
    (b"kling-ai", "kling"),
];

fn read_u32_be(data: &[u8], offset: usize) -> Option<u32> {
    if offset + 4 > data.len() { return None; }
    Some(u32::from_be_bytes([data[offset], data[offset+1], data[offset+2], data[offset+3]]))
}

struct BoxInfo { box_type: [u8; 4], content_start: usize, box_end: usize }

fn find_boxes(data: &[u8], start: usize, end: usize) -> Vec<BoxInfo> {
    let mut boxes = Vec::new();
    let mut pos = start;
    while pos + 8 <= end {
        let size = match read_u32_be(data, pos) { Some(s) => s as u64, None => break };
        let mut box_type = [0u8; 4];
        box_type.copy_from_slice(&data[pos + 4..pos + 8]);
        let (content_start, actual_size) = if size == 1 {
            if pos + 16 > end { break; }
            let ext = u64::from_be_bytes([data[pos+8],data[pos+9],data[pos+10],data[pos+11],data[pos+12],data[pos+13],data[pos+14],data[pos+15]]);
            (pos + 16, ext)
        } else if size == 0 { (pos + 8, (end - pos) as u64) }
        else { (pos + 8, size) };
        if actual_size < 8 { break; }
        let box_end = (pos as u64 + actual_size).min(end as u64) as usize;
        boxes.push(BoxInfo { box_type, content_start, box_end });
        pos = box_end;
    }
    boxes
}

fn get_box(data: &[u8], start: usize, end: usize, box_type: &[u8; 4]) -> Option<(usize, usize)> {
    find_boxes(data, start, end).into_iter().find(|b| &b.box_type == box_type).map(|b| (b.content_start, b.box_end))
}

fn box_type_to_string(box_type: &[u8; 4]) -> String { box_type.iter().map(|&b| b as char).collect() }

fn parse_ilst_standard(data: &[u8], start: usize, end: usize) -> Vec<(String, String)> {
    let mut results = Vec::new();
    for item in find_boxes(data, start, end) {
        let key = box_type_to_string(&item.box_type);
        if let Some((data_cs, data_ce)) = get_box(data, item.content_start, item.box_end, b"data") {
            if data_ce - data_cs >= 8 {
                let value = String::from_utf8_lossy(&data[data_cs + 8..data_ce]).trim_matches('\0').to_string();
                if !value.is_empty() { results.push((key, value)); }
            }
        }
    }
    results
}

fn parse_keys(data: &[u8], start: usize, end: usize) -> Vec<String> {
    if end - start < 8 { return vec![]; }
    let count = match read_u32_be(data, start + 4) { Some(c) => c as usize, None => return vec![] };
    let mut keys = Vec::with_capacity(count);
    let mut offset = start + 8;
    for _ in 0..count {
        if offset + 8 > end { break; }
        let key_size = match read_u32_be(data, offset) { Some(s) => s as usize, None => break };
        if key_size < 8 || offset + key_size > end { break; }
        let name = String::from_utf8_lossy(&data[offset + 8..offset + key_size]).to_string();
        keys.push(name);
        offset += key_size;
    }
    keys
}

fn parse_ilst_keyed(data: &[u8], keys: &[String], ilst_start: usize, ilst_end: usize) -> Vec<(String, String)> {
    let mut results = Vec::new();
    for item in find_boxes(data, ilst_start, ilst_end) {
        let idx = u32::from_be_bytes(item.box_type) as usize;
        let key_name = if idx > 0 && idx <= keys.len() { keys[idx - 1].clone() } else { format!("idx:{}", idx) };
        if let Some((data_cs, data_ce)) = get_box(data, item.content_start, item.box_end, b"data") {
            if data_ce - data_cs >= 8 {
                let value = String::from_utf8_lossy(&data[data_cs + 8..data_ce]).trim_matches('\0').to_string();
                if !value.is_empty() { results.push((key_name, value)); }
            }
        }
    }
    results
}

fn extract_ilst_entries(data: &[u8]) -> Vec<(String, String)> {
    let moov = match get_box(data, 0, data.len(), b"moov") { Some(m) => m, None => return vec![] };
    let udta = match get_box(data, moov.0, moov.1, b"udta") { Some(u) => u, None => return vec![] };
    let meta = match get_box(data, udta.0, udta.1, b"meta") { Some(m) => m, None => return vec![] };
    let meta_content = meta.0 + 4;
    if meta_content >= meta.1 { return vec![]; }
    let keys_box = get_box(data, meta_content, meta.1, b"keys");
    let ilst = match get_box(data, meta_content, meta.1, b"ilst") { Some(i) => i, None => return vec![] };
    if let Some((keys_start, keys_end)) = keys_box {
        let keys = parse_keys(data, keys_start, keys_end);
        parse_ilst_keyed(data, &keys, ilst.0, ilst.1)
    } else {
        parse_ilst_standard(data, ilst.0, ilst.1)
    }
}

fn detect_ilst_tools(entries: &[(String, String)]) -> Vec<Signal> {
    let mut signals = Vec::new();
    let tool_keys: &[&str] = &["\u{a9}too", "\u{a9}swr", "encoder", "tool", "software"];
    for (key, value) in entries {
        let is_tool_key = tool_keys.iter().any(|tk| key.eq_ignore_ascii_case(tk));
        if !is_tool_key { continue; }
        let label = match key.as_str() {
            "\u{a9}too" => "Encoding Tool",
            "\u{a9}swr" => "Software",
            _ => key.as_str(),
        };
        if let Some(tool_name) = known_tools::match_ai_tool(value) {
            signals.push(
                SignalBuilder::new(SignalSource::Mp4Metadata, Confidence::Medium, "signal_mp4_tool_match")
                    .param("label", label).param("value", value.as_str())
                    .tool(tool_name).detail(key.as_str(), value.as_str()).build(),
            );
            continue;
        }
        let lower = value.to_lowercase();
        for &(pattern, mapped_tool, confidence) in MP4_TOOL_MAPPINGS {
            if lower == pattern {
                signals.push(
                    SignalBuilder::new(SignalSource::Mp4Metadata, confidence, "signal_mp4_tool_match")
                        .param("label", label).param("value", value.as_str())
                        .tool(mapped_tool).detail(key.as_str(), value.as_str()).build(),
                );
                break;
            }
        }
    }
    signals
}

fn detect_aigc_label(entries: &[(String, String)]) -> Vec<Signal> {
    let mut signals = Vec::new();
    for (key, value) in entries {
        if !key.eq_ignore_ascii_case("AIGC") { continue; }
        let has_ai_label = value.contains("\"Label\":\"1\"") || value.contains("\"Label\": \"1\"");
        if !has_ai_label { continue; }
        let produce_id = extract_json_field(value, "ProduceID");
        let signal = if let Some(ref pid) = produce_id {
            SignalBuilder::new(SignalSource::Mp4Metadata, Confidence::Medium, "signal_mp4_aigc_label_id")
                .param("id", pid.as_str())
                .detail("AIGC", value.as_str())
                .detail("ProduceID", pid.as_str())
                .build()
        } else {
            SignalBuilder::new(SignalSource::Mp4Metadata, Confidence::Medium, "signal_mp4_aigc_label")
                .detail("AIGC", value.as_str())
                .build()
        };
        signals.push(signal);
    }
    signals
}

fn extract_json_field(json: &str, field: &str) -> Option<String> {
    let pattern = format!("\"{}\"", field);
    let idx = json.find(&pattern)?;
    let after = &json[idx + pattern.len()..];
    let after = after.trim_start();
    let after = after.strip_prefix(':')?;
    let after = after.trim_start();
    let after = after.strip_prefix('"')?;
    let end = after.find('"')?;
    Some(after[..end].to_string())
}

fn detect_sei_markers(data: &[u8]) -> Vec<Signal> {
    let mut signals = Vec::new();
    let mdat = match get_box(data, 0, data.len(), b"mdat") { Some(m) => m, None => return signals };
    let scan_end = mdat.1.min(mdat.0 + 1_048_576);
    let scan_data = &data[mdat.0..scan_end];
    for &(marker, tool_name) in SEI_MARKERS {
        if scan_data.windows(marker.len()).any(|w| w == marker) {
            let marker_str = String::from_utf8_lossy(marker);
            signals.push(
                SignalBuilder::new(SignalSource::Mp4Metadata, Confidence::Medium, "signal_mp4_sei_watermark")
                    .param("marker", &*marker_str)
                    .tool(tool_name)
                    .detail("SEI marker", &*marker_str)
                    .build(),
            );
        }
    }
    signals
}

pub fn detect(path: &Path) -> Result<Vec<Signal>> {
    let data = fs::read(path)?;
    if get_box(&data, 0, data.len().min(64), b"ftyp").is_none() { return Ok(vec![]); }
    let entries = extract_ilst_entries(&data);
    let mut signals = Vec::new();
    signals.extend(detect_ilst_tools(&entries));
    signals.extend(detect_aigc_label(&entries));
    signals.extend(detect_sei_markers(&data));
    Ok(signals)
}

pub fn dump_info(path: &Path) -> Result<Vec<(String, String)>> {
    let data = fs::read(path)?;
    if get_box(&data, 0, data.len().min(64), b"ftyp").is_none() { return Ok(vec![]); }
    let mut props = extract_ilst_entries(&data);
    let mdat = get_box(&data, 0, data.len(), b"mdat");
    if let Some((mdat_start, mdat_end)) = mdat {
        let scan_end = mdat_end.min(mdat_start + 1_048_576);
        let scan_data = &data[mdat_start..scan_end];
        for &(marker, tool_name) in SEI_MARKERS {
            if scan_data.windows(marker.len()).any(|w| w == marker) {
                props.push(("SEI watermark".to_string(), format!("{} ({})", String::from_utf8_lossy(marker), tool_name)));
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
        assert_eq!(extract_json_field(json, "ProduceID"), Some("abc-123".to_string()));
        assert_eq!(extract_json_field(json, "Missing"), None);
        let json2 = r#"{"Label": "1", "ProduceID": "xyz"}"#;
        assert_eq!(extract_json_field(json2, "Label"), Some("1".to_string()));
        assert_eq!(extract_json_field(json2, "ProduceID"), Some("xyz".to_string()));
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
        let entries = vec![("AIGC".to_string(), r#"{"Label":"1","ProduceID":"test-123"}"#.to_string())];
        let signals = detect_aigc_label(&entries);
        assert_eq!(signals.len(), 1);
        assert_eq!(signals[0].confidence, Confidence::Medium);
        assert!(signals[0].description.contains("AIGC") || signals[0].msg_key.contains("aigc"));
        assert!(signals[0].description.contains("test-123") || signals[0].msg_params.iter().any(|(_, v)| v == "test-123"));
    }

    #[test]
    fn test_detect_aigc_label_not_ai() {
        let entries = vec![("AIGC".to_string(), r#"{"Label":"0","ProduceID":"test"}"#.to_string())];
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
        let value = b"Google";
        let data_atom_size: u32 = 8 + 8 + value.len() as u32;
        let item_size: u32 = 8 + data_atom_size;
        let ilst_size: u32 = 8 + item_size;
        let mut buf = Vec::new();
        buf.extend_from_slice(&ilst_size.to_be_bytes());
        buf.extend_from_slice(b"ilst");
        buf.extend_from_slice(&item_size.to_be_bytes());
        buf.extend_from_slice(&[0xa9, b't', b'o', b'o']);
        buf.extend_from_slice(&data_atom_size.to_be_bytes());
        buf.extend_from_slice(b"data");
        buf.extend_from_slice(&[0, 0, 0, 1]);
        buf.extend_from_slice(&[0, 0, 0, 0]);
        buf.extend_from_slice(value);
        let entries = parse_ilst_standard(&buf, 8, buf.len());
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].0, "\u{a9}too");
        assert_eq!(entries[0].1, "Google");
    }
}
