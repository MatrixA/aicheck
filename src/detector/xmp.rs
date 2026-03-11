use anyhow::Result;
use std::fs;
use std::path::Path;

use super::{Confidence, Signal, SignalBuilder, SignalSource};
use crate::known_tools;

/// IPTC DigitalSourceType URIs/names that indicate AI generation.
const AI_SOURCE_TYPES: &[(&str, &str)] = &[
    ("trainedAlgorithmicMedia", "trainedAlgorithmicMedia"),
    (
        "compositeWithTrainedAlgorithmicMedia",
        "compositeWithTrainedAlgorithmicMedia",
    ),
    ("algorithmicMedia", "algorithmicMedia"),
    ("compositeSynthetic", "compositeSynthetic"),
    ("dataDrivenMedia", "dataDrivenMedia"),
    ("trainedAlgorithmicData", "trainedAlgorithmicData"),
];

/// XMP property names we search for in the raw XML.
const XMP_AI_PROPERTIES: &[&str] = &[
    "DigitalSourceType",
    "AISystemUsed",
    "AISystemVersionUsed",
    "AIPromptInformation",
    "CreatorTool",
];

/// Extract raw XMP XML from a file's bytes.
fn extract_xmp_xml(data: &[u8]) -> Option<String> {
    let begin_marker = b"<x:xmpmeta";
    let end_marker = b"</x:xmpmeta>";
    let begin_marker2 = b"<xmpmeta";
    let end_marker2 = b"</xmpmeta>";
    let begin_marker3 = b"<rdf:RDF";
    let end_marker3 = b"</rdf:RDF>";

    for (begin, end) in [
        (&begin_marker[..], &end_marker[..]),
        (&begin_marker2[..], &end_marker2[..]),
        (&begin_marker3[..], &end_marker3[..]),
    ] {
        if let Some(start_pos) = find_subsequence(data, begin) {
            if let Some(end_pos) = find_subsequence(&data[start_pos..], end) {
                let xml_end = start_pos + end_pos + end.len();
                if let Ok(xml) = std::str::from_utf8(&data[start_pos..xml_end]) {
                    return Some(xml.to_string());
                }
            }
        }
    }
    None
}

fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

fn extract_property(xml: &str, prop_name: &str) -> Option<String> {
    for prefix in &["Iptc4xmpExt:", "xmp:", "dc:", "photoshop:", ""] {
        let open_tag = format!("<{}{}", prefix, prop_name);
        if let Some(start) = xml.find(&open_tag) {
            let after_tag = &xml[start + open_tag.len()..];
            if let Some(gt_pos) = after_tag.find('>') {
                let content_start = gt_pos + 1;
                let close_tag = format!("</{}{}>", prefix, prop_name);
                if let Some(end_pos) = after_tag.find(&close_tag) {
                    if end_pos > content_start {
                        let value = after_tag[content_start..end_pos].trim();
                        if !value.is_empty() {
                            return Some(value.to_string());
                        }
                    }
                }
            }
        }
    }

    for prefix in &["Iptc4xmpExt:", "xmp:", "dc:", "photoshop:", ""] {
        let attr = format!("{}{}=\"", prefix, prop_name);
        if let Some(start) = xml.find(&attr) {
            let val_start = start + attr.len();
            if let Some(end) = xml[val_start..].find('"') {
                let value = &xml[val_start..val_start + end];
                if !value.is_empty() {
                    return Some(value.to_string());
                }
            }
        }
    }

    None
}

/// Detect AI signals from XMP/IPTC metadata embedded in the file.
pub fn detect(path: &Path) -> Result<Vec<Signal>> {
    let data = fs::read(path)?;
    let search_data = if data.len() > 1_048_576 {
        &data[..1_048_576]
    } else {
        &data
    };

    let xml = match extract_xmp_xml(search_data) {
        Some(x) => x,
        None => return Ok(vec![]),
    };

    let mut signals = Vec::new();

    if let Some(value) = extract_property(&xml, "DigitalSourceType") {
        for (name, pattern) in AI_SOURCE_TYPES {
            if value.contains(pattern) {
                signals.push(
                    SignalBuilder::new(
                        SignalSource::Xmp,
                        Confidence::Medium,
                        "signal_xmp_digital_source_type",
                    )
                    .param("value", *name)
                    .detail("DigitalSourceType", &value)
                    .build(),
                );
                break;
            }
        }
    }

    if let Some(value) = extract_property(&xml, "AISystemUsed") {
        let tool = known_tools::match_ai_tool(&value).map(|s| s.to_string());
        signals.push(
            SignalBuilder::new(
                SignalSource::Xmp,
                Confidence::Medium,
                "signal_xmp_ai_system_used",
            )
            .param("value", &value)
            .tool_opt(tool)
            .detail("AISystemUsed", &value)
            .build(),
        );
    }

    if let Some(value) = extract_property(&xml, "AIPromptInformation") {
        signals.push(
            SignalBuilder::new(
                SignalSource::Xmp,
                Confidence::Medium,
                "signal_xmp_ai_prompt",
            )
            .detail("AIPromptInformation", &value)
            .build(),
        );
    }

    if let Some(value) = extract_property(&xml, "CreatorTool") {
        if let Some(tool_name) = known_tools::match_ai_tool(&value) {
            signals.push(
                SignalBuilder::new(
                    SignalSource::Xmp,
                    Confidence::Medium,
                    "signal_xmp_creator_tool",
                )
                .param("value", &value)
                .tool(tool_name)
                .detail("CreatorTool", &value)
                .build(),
            );
        }
    }

    Ok(signals)
}

/// Dump all XMP properties for the `info` subcommand.
pub fn dump_info(path: &Path) -> Result<Vec<(String, String)>> {
    let data = fs::read(path)?;
    let search_data = if data.len() > 1_048_576 {
        &data[..1_048_576]
    } else {
        &data
    };

    let xml = match extract_xmp_xml(search_data) {
        Some(x) => x,
        None => return Ok(vec![]),
    };

    let mut props = Vec::new();
    for prop_name in XMP_AI_PROPERTIES {
        if let Some(value) = extract_property(&xml, prop_name) {
            props.push((prop_name.to_string(), value));
        }
    }

    Ok(props)
}
