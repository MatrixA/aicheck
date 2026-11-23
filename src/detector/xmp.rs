use anyhow::Result;
use std::fs;
use std::path::Path;

use super::{aigc, AttributionScope, Confidence, Signal, SignalBuilder, SignalSource};
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
    "Credit",
];

/// TC260 AIGC implicit-label namespace (GB 45438-2025).
const TC260_AIGC_NS: &str = "http://www.tc260.org.cn/ns/AIGC/1.0/";

/// Find the prefixes bound to `namespace` via `xmlns:prefix="namespace"`.
fn namespace_prefixes(xml: &str, namespace: &str) -> Vec<String> {
    let mut prefixes = Vec::new();
    let mut rest = xml;
    while let Some(idx) = rest.find("xmlns:") {
        let after = &rest[idx + 6..];
        let Some(eq) = after.find('=') else { break };
        let prefix = after[..eq].trim();
        let value_part = after[eq + 1..].trim_start();
        if let Some(quote) = value_part
            .chars()
            .next()
            .filter(|c| *c == '"' || *c == '\'')
        {
            if let Some(end) = value_part[1..].find(quote) {
                if &value_part[1..1 + end] == namespace
                    && !prefix.is_empty()
                    && !prefixes.iter().any(|p| p == prefix)
                {
                    prefixes.push(prefix.to_string());
                }
            }
        }
        rest = &after[eq + 1..];
    }
    prefixes
}

/// Read the TC260 `AIGC` property (element text or attribute) for any prefix
/// bound to the TC260 namespace, XML-unescaped.
fn extract_tc260_aigc(xml: &str) -> Option<String> {
    for prefix in namespace_prefixes(xml, TC260_AIGC_NS) {
        let qname = format!("{}:AIGC", prefix);
        // Element form: <p:AIGC>...</p:AIGC>
        let open = format!("<{}", qname);
        let close = format!("</{}>", qname);
        let mut search = xml;
        while let Some(start) = search.find(&open) {
            let after = &search[start + open.len()..];
            if after.starts_with('>') || after.starts_with(char::is_whitespace) {
                if let Some(gt) = after.find('>') {
                    if let Some(end) = after[gt + 1..].find(&close) {
                        let text = after[gt + 1..gt + 1 + end].trim();
                        if !text.is_empty() {
                            return Some(unescape_xml(text));
                        }
                    }
                }
            }
            search = after;
        }
        // Attribute form: p:AIGC="..." or p:AIGC='...'
        for quote in ['"', '\''] {
            let attr = format!("{}={}", qname, quote);
            if let Some(start) = xml.find(&attr) {
                let vs = start + attr.len();
                if let Some(end) = xml[vs..].find(quote) {
                    return Some(unescape_xml(&xml[vs..vs + end]));
                }
            }
        }
    }
    None
}

fn unescape_xml(s: &str) -> String {
    quick_xml::escape::unescape(s)
        .map(|c| c.into_owned())
        .unwrap_or_else(|_| s.to_string())
}

fn is_mp4_container(data: &[u8]) -> bool {
    data.len() >= 8 && &data[4..8] == b"ftyp"
}

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

    // TC260 AIGC implicit label (e.g. Qwen Image PNGs). Parsed as JSON.
    if let Some(value) = extract_tc260_aigc(&xml) {
        if let Some(label) = aigc::parse(&value).filter(|l| l.is_ai()) {
            let media = if is_mp4_container(&data) {
                aigc::MediaKind::Video
            } else {
                aigc::MediaKind::Image
            };
            let attr = aigc::attribute(&label, media);
            let mut b = SignalBuilder::new(
                SignalSource::Xmp,
                Confidence::Medium,
                "signal_xmp_aigc_label",
            )
            .tool_opt(attr.tool.map(str::to_string))
            .scope(attr.scope)
            .evidence_at(format!("XMP.TC260.AIGC.{}", attr.field), attr.raw_value)
            .detail("TC260 AIGC", &value);
            if let Some(ref pid) = label.produce_id {
                b = b.detail("ProduceID", pid.as_str());
            }
            signals.push(b.build());
        }
    }

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
                    .scope(AttributionScope::AiOnly)
                    .evidence_at("XMP.DigitalSourceType", &value)
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
            .scope_opt(tool.as_ref().map(|_| AttributionScope::Provider))
            .tool_opt(tool)
            .evidence_at("XMP.AISystemUsed", &value)
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
                .scope(AttributionScope::Provider)
                .evidence_at("XMP.CreatorTool", &value)
                .detail("CreatorTool", &value)
                .build(),
            );
        }
    }

    // Check Credit field (e.g. photoshop:Credit="Made with Google AI")
    if let Some(value) = extract_property(&xml, "Credit") {
        if let Some(tool_name) = known_tools::match_ai_tool(&value) {
            signals.push(
                SignalBuilder::new(SignalSource::Xmp, Confidence::Medium, "signal_xmp_credit")
                    .param("value", &value)
                    .tool(tool_name)
                    .scope(AttributionScope::Provider)
                    .evidence_at("XMP.Credit", &value)
                    .detail("Credit", &value)
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
    if let Some(value) = extract_tc260_aigc(&xml) {
        props.push(("TC260 AIGC".to_string(), value));
    }
    for prop_name in XMP_AI_PROPERTIES {
        if let Some(value) = extract_property(&xml, prop_name) {
            props.push((prop_name.to_string(), value));
        }
    }

    Ok(props)
}

#[cfg(test)]
mod tests {
    use super::*;

    const QWEN_LIKE: &str = r#"<x:xmpmeta xmlns:x="adobe:ns:meta/"><rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"><rdf:Description rdf:about="" xmlns:TC260="http://www.tc260.org.cn/ns/AIGC/1.0/"><TC260:AIGC>{&quot;Label&quot;:&quot;1&quot;,&quot;ContentProducer&quot;:&quot;001191330106MA2CFLDG4R10001&quot;,&quot;ProduceID&quot;:&quot;p1&quot;}</TC260:AIGC></rdf:Description></rdf:RDF></x:xmpmeta>"#;

    #[test]
    fn tc260_element_form() {
        let v = extract_tc260_aigc(QWEN_LIKE).unwrap();
        let l = aigc::parse(&v).unwrap();
        assert!(l.is_ai());
        assert_eq!(
            aigc::producer_tool(
                l.content_producer.as_deref().unwrap(),
                aigc::MediaKind::Image
            ),
            Some("qwen/wan")
        );
    }

    #[test]
    fn tc260_attribute_form_with_other_prefix() {
        let xml = r#"<rdf:RDF><rdf:Description xmlns:aigc='http://www.tc260.org.cn/ns/AIGC/1.0/' aigc:AIGC="{&quot;Label&quot;: &quot;1&quot;}"/></rdf:RDF>"#;
        let v = extract_tc260_aigc(xml).unwrap();
        assert!(aigc::parse(&v).unwrap().is_ai());
    }

    #[test]
    fn tc260_requires_namespace() {
        let xml = r#"<rdf:RDF><rdf:Description xmlns:TC260="http://example.com/other/"><TC260:AIGC>{"Label":"1"}</TC260:AIGC></rdf:Description></rdf:RDF>"#;
        assert!(extract_tc260_aigc(xml).is_none());
    }
}
