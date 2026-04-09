use anyhow::Result;
use exif::{In, Reader, Tag, Value};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use super::{Confidence, Signal, SignalBuilder, SignalSource};
use crate::known_tools;

/// AIGC ContentProducer ID prefixes mapped to tool names (from EXIF UserComment).
/// These are empirically observed enterprise registration numbers.
const EXIF_AIGC_PRODUCER_PREFIXES: &[(&str, &str)] = &[("001191110000802100433B", "qwen")];

/// Camera-specific EXIF tags that real photos typically have.
const CAMERA_TAGS: &[Tag] = &[
    Tag::Make,
    Tag::Model,
    Tag::LensModel,
    Tag::FocalLength,
    Tag::FNumber,
    Tag::ExposureTime,
    Tag::PhotographicSensitivity,
    Tag::Flash,
    Tag::MeteringMode,
    Tag::WhiteBalance,
];

/// Detect AI signals from EXIF metadata.
pub fn detect(path: &Path) -> Result<Vec<Signal>> {
    let file = File::open(path)?;
    let exif = match Reader::new().read_from_container(&mut BufReader::new(file)) {
        Ok(e) => e,
        Err(_) => return Ok(vec![]),
    };

    let mut signals = Vec::new();
    let mut software_matched = false;

    // Check Software tag for known AI tools
    if let Some(field) = exif.get_field(Tag::Software, In::PRIMARY) {
        let sw = field.display_value().to_string().replace('"', "");
        if let Some(tool_name) = known_tools::match_ai_tool(&sw) {
            signals.push(
                SignalBuilder::new(SignalSource::Exif, Confidence::Low, "signal_exif_software")
                    .param("value", &sw)
                    .tool(tool_name)
                    .detail("Software", &sw)
                    .build(),
            );
            software_matched = true;
        }
    }

    // Check Make / Model tags for known AI tools
    for tag in &[Tag::Make, Tag::Model] {
        if let Some(field) = exif.get_field(*tag, In::PRIMARY) {
            let val = field.display_value().to_string().replace('"', "");
            if let Some(tool_name) = known_tools::match_ai_tool(&val) {
                signals.push(
                    SignalBuilder::new(
                        SignalSource::Exif,
                        Confidence::Low,
                        "signal_exif_tag_value",
                    )
                    .param("tag", tag.to_string())
                    .param("value", &val)
                    .tool(tool_name)
                    .detail(tag.to_string(), &val)
                    .build(),
                );
                software_matched = true;
            }
        }
    }

    // Check ImageDescription / UserComment for AI references
    for tag in &[Tag::ImageDescription, Tag::UserComment] {
        if let Some(field) = exif.get_field(*tag, In::PRIMARY) {
            // For UserComment, the display_value() may return hex; decode raw bytes instead.
            let raw_val = decode_field_text(field);
            let val = raw_val.replace('"', "");
            if let Some(tool_name) = known_tools::match_ai_tool(&val) {
                signals.push(
                    SignalBuilder::new(
                        SignalSource::Exif,
                        Confidence::Low,
                        "signal_exif_tag_references_ai",
                    )
                    .param("tag", tag.to_string())
                    .tool(tool_name)
                    .detail(tag.to_string(), &val)
                    .build(),
                );
                software_matched = true;
            } else if raw_val.contains("\"AIGC\"")
                && (raw_val.contains("\"Label\":\"1\"") || raw_val.contains("\"Label\": \"1\""))
            {
                // AIGC JSON label in EXIF (e.g. Qwen images embed AIGC metadata in UserComment)
                let tool = extract_json_field(&raw_val, "ContentProducer").and_then(|cp| {
                    EXIF_AIGC_PRODUCER_PREFIXES
                        .iter()
                        .find(|(prefix, _)| cp.starts_with(prefix))
                        .map(|(_, tool)| tool.to_string())
                });
                signals.push(
                    SignalBuilder::new(
                        SignalSource::Exif,
                        Confidence::Medium,
                        "signal_exif_aigc_label",
                    )
                    .param("tag", tag.to_string())
                    .tool_opt(tool)
                    .detail(tag.to_string(), &val)
                    .build(),
                );
                software_matched = true;
            }
        }
    }

    // Check Artist tag for suspicious patterns
    if let Some(field) = exif.get_field(Tag::Artist, In::PRIMARY) {
        let val = field.display_value().to_string().replace('"', "");
        let is_hex_hash = val.len() >= 32 && val.chars().all(|c| c.is_ascii_hexdigit() || c == '-');
        if is_hex_hash {
            let prefix = &val[..val.len().min(16)];
            signals.push(
                SignalBuilder::new(
                    SignalSource::Exif,
                    Confidence::Low,
                    "signal_exif_artist_hash",
                )
                .param("value", prefix)
                .detail("Artist", &val)
                .build(),
            );
            software_matched = true;
        }
    }

    // Camera absence heuristic
    if software_matched {
        let camera_tag_count = CAMERA_TAGS
            .iter()
            .filter(|&&tag| exif.get_field(tag, In::PRIMARY).is_some())
            .count();

        if camera_tag_count == 0 {
            signals.push(
                SignalBuilder::new(SignalSource::Exif, Confidence::Low, "signal_exif_no_camera")
                    .detail("camera_tags_present", "0")
                    .build(),
            );
        }
    }

    Ok(signals)
}

/// Decode an EXIF field to text. For UserComment (Undefined type),
/// the first 8 bytes are a character code identifier; skip them and
/// decode the remainder as UTF-8.
fn decode_field_text(field: &exif::Field) -> String {
    if let Value::Undefined(ref bytes, _) = field.value {
        if bytes.len() > 8 {
            // Skip 8-byte character code prefix (e.g. "ASCII\0\0\0")
            if let Ok(text) = std::str::from_utf8(&bytes[8..]) {
                return text.trim_end_matches('\0').to_string();
            }
        }
    }
    field.display_value().to_string()
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

/// Dump all EXIF fields for the `info` subcommand.
pub fn dump_info(path: &Path) -> Result<Vec<(String, String)>> {
    let file = File::open(path)?;
    let exif = match Reader::new().read_from_container(&mut BufReader::new(file)) {
        Ok(e) => e,
        Err(_) => return Ok(vec![]),
    };

    let mut fields = Vec::new();
    for field in exif.fields() {
        fields.push((format!("{}", field.tag), field.display_value().to_string()));
    }
    Ok(fields)
}
