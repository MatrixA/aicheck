use anyhow::Result;
use exif::{In, Reader, Tag};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use super::{Confidence, Signal, SignalBuilder, SignalSource};
use crate::known_tools;

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
            let val = field.display_value().to_string().replace('"', "");
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
