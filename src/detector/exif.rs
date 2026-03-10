use anyhow::Result;
use exif::{In, Reader, Tag};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use super::{Confidence, Signal, SignalSource};
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
        Err(_) => return Ok(vec![]), // No EXIF — not an error
    };

    let mut signals = Vec::new();
    let mut software_matched = false;

    // Check Software tag for known AI tools
    if let Some(field) = exif.get_field(Tag::Software, In::PRIMARY) {
        let sw = field.display_value().to_string().replace('"', "");
        if let Some(tool_name) = known_tools::match_ai_tool(&sw) {
            signals.push(Signal {
                source: SignalSource::Exif,
                confidence: Confidence::Low,
                description: format!("Software = \"{}\"", sw),
                tool: Some(tool_name.to_string()),
                details: vec![("Software".into(), sw)],
            });
            software_matched = true;
        }
    }

    // Check ImageDescription / UserComment for AI references
    for tag in &[Tag::ImageDescription, Tag::UserComment] {
        if let Some(field) = exif.get_field(*tag, In::PRIMARY) {
            let val = field.display_value().to_string().replace('"', "");
            if let Some(tool_name) = known_tools::match_ai_tool(&val) {
                signals.push(Signal {
                    source: SignalSource::Exif,
                    confidence: Confidence::Low,
                    description: format!("{} references AI tool", tag),
                    tool: Some(tool_name.to_string()),
                    details: vec![(tag.to_string(), val)],
                });
                software_matched = true;
            }
        }
    }

    // Camera absence heuristic — only flag if Software also matched
    if software_matched {
        let camera_tag_count = CAMERA_TAGS
            .iter()
            .filter(|&&tag| exif.get_field(tag, In::PRIMARY).is_some())
            .count();

        if camera_tag_count == 0 {
            signals.push(Signal {
                source: SignalSource::Exif,
                confidence: Confidence::Low,
                description: "No camera metadata (Make, Model, lens, exposure) found".to_string(),
                tool: None,
                details: vec![("camera_tags_present".into(), "0".into())],
            });
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
        fields.push((
            format!("{}", field.tag),
            field.display_value().to_string(),
        ));
    }
    Ok(fields)
}
