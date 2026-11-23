//! Shared parser for Chinese AIGC implicit labels (GB 45438-2025 / TC260).
//!
//! The same JSON object appears in several carriers:
//! - EXIF `UserComment` / `ImageDescription` (e.g. MiniMax, Qianfan images)
//! - MP4 `AIGC` metadata key (e.g. Wan, Kling, Vidu, Hailuo videos)
//! - XMP `http://www.tc260.org.cn/ns/AIGC/1.0/` `AIGC` property (e.g. Qwen PNGs)
//!
//! The object is parsed structurally with `serde_json`, so whitespace and field
//! order do not matter. Producer → vendor mappings are empirical observations
//! from sample batches, not published registries; unknown producers and
//! propagator-only labels never yield a vendor.

use serde_json::Value;

use super::AttributionScope;

/// Which kind of media the label was read from. Some producer codes are shared
/// across products (e.g. the same Alibaba code on Qwen images and Wan videos),
/// so the mapping depends on the media kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaKind {
    Image,
    Video,
}

/// A parsed AIGC label.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AigcLabel {
    /// Raw `Label` value (`"1"` means AI-generated).
    pub label: String,
    pub content_producer: Option<String>,
    pub produce_id: Option<String>,
    pub content_propagator: Option<String>,
}

impl AigcLabel {
    /// True if the label declares AI-generated content.
    pub fn is_ai(&self) -> bool {
        self.label.trim() == "1"
    }
}

/// Parse an AIGC label from a JSON string. Accepts either the bare object
/// (`{"Label":"1",...}`) or a wrapper (`{"AIGC":{...}}`), the wrapper also as a
/// JSON-encoded string. Leading/trailing junk outside the outermost braces
/// (NUL padding, EXIF charset prefixes) is ignored.
pub fn parse(text: &str) -> Option<AigcLabel> {
    let start = text.find('{')?;
    let end = text.rfind('}')?;
    if end <= start {
        return None;
    }
    let value: Value = serde_json::from_str(&text[start..=end]).ok()?;
    from_value(&value, 0)
}

fn from_value(value: &Value, depth: u8) -> Option<AigcLabel> {
    if depth > 2 {
        return None;
    }
    let obj = value.as_object()?;
    if let Some(inner) = obj.get("AIGC") {
        return match inner {
            Value::String(s) => parse(s),
            other => from_value(other, depth + 1),
        };
    }
    let label = scalar(obj.get("Label")?)?;
    Some(AigcLabel {
        label,
        content_producer: obj.get("ContentProducer").and_then(scalar),
        produce_id: obj.get("ProduceID").and_then(scalar),
        content_propagator: obj.get("ContentPropagator").and_then(scalar),
    })
}

fn scalar(v: &Value) -> Option<String> {
    let s = match v {
        Value::String(s) => s.trim().to_string(),
        Value::Number(n) => n.to_string(),
        _ => return None,
    };
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

/// Exact ContentProducer values mapped to tools: (value, image tool, video tool).
/// `None` means the code is not mapped for that media kind.
const PRODUCER_EXACT: &[(&str, Option<&str>, Option<&str>)] = &[
    // MiniMax writes its name literally (H3 images, H3/Hailuo videos).
    ("minimax", Some("minimax"), Some("minimax")),
    // Kling 4.0 videos write a literal "kling".
    ("kling", Some("kling"), Some("kling")),
    // Shared by Qwen Image PNGs and Wan videos. Videos keep the historical
    // "wan" tool name; images only narrow to the candidate group.
    ("001191330106MA2CFLDG4R10001", Some("qwen/wan"), Some("wan")),
    // Vidu Q1/Q2/Q3 videos.
    ("001191110108MACC4D63XF10306", None, Some("vidu")),
    // Kling 2.5–3.0 / O1 videos.
    ("001191110108MAKBA45N9M10100", None, Some("kling")),
];

/// ContentProducer prefixes mapped to tools (legacy Qianfan / Qwen images).
const PRODUCER_PREFIXES: &[(&str, &str)] = &[("001191110000802100433B", "qwen")];

/// Map a ContentProducer value to a tool name. Returns `None` for unknown
/// producers so they never yield a definite vendor.
pub fn producer_tool(producer: &str, media: MediaKind) -> Option<&'static str> {
    let p = producer.trim();
    for &(value, image, video) in PRODUCER_EXACT {
        if p.eq_ignore_ascii_case(value) {
            return match media {
                MediaKind::Image => image,
                MediaKind::Video => video,
            };
        }
    }
    PRODUCER_PREFIXES
        .iter()
        .find(|(prefix, _)| p.starts_with(prefix))
        .map(|(_, tool)| *tool)
}

/// Attribution derived from a label: (tool, scope, evidence field suffix, raw value).
pub struct LabelAttribution {
    pub tool: Option<&'static str>,
    pub scope: AttributionScope,
    pub field: &'static str,
    pub raw_value: String,
}

/// Compute the attribution for an AI label. A known producer yields a provider
/// attribution; an unknown producer, or a label with only a propagator, yields
/// a generic AI declaration.
pub fn attribute(label: &AigcLabel, media: MediaKind) -> LabelAttribution {
    if let Some(cp) = &label.content_producer {
        let tool = producer_tool(cp, media);
        return LabelAttribution {
            tool,
            scope: if tool.is_some() {
                AttributionScope::Provider
            } else {
                AttributionScope::AiOnly
            },
            field: "ContentProducer",
            raw_value: cp.clone(),
        };
    }
    LabelAttribution {
        tool: None,
        scope: AttributionScope::AiOnly,
        field: "Label",
        raw_value: label.label.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_wrapped_and_bare_objects() {
        let a =
            parse(r#"{"AIGC":{"Label":"1","ContentProducer":"MiniMax","ProduceID":"x"}}"#).unwrap();
        let b =
            parse(r#"{ "ProduceID" : "x" ,  "ContentProducer":"MiniMax", "Label": "1" }"#).unwrap();
        assert_eq!(a, b);
        assert!(a.is_ai());
        assert_eq!(a.content_producer.as_deref(), Some("MiniMax"));
    }

    #[test]
    fn parses_numeric_label_and_padding() {
        let l = parse("ASCII\0\0\0{\"Label\":1,\"ContentProducer\":\"kling\"}\0\0").unwrap();
        assert!(l.is_ai());
    }

    #[test]
    fn parses_string_wrapped_label() {
        let l = parse(r#"{"AIGC":"{\"Label\":\"1\",\"ContentProducer\":\"MiniMax\"}"}"#).unwrap();
        assert_eq!(l.content_producer.as_deref(), Some("MiniMax"));
    }

    #[test]
    fn rejects_invalid() {
        assert!(parse("not json").is_none());
        assert!(parse(r#"{"Other":"1"}"#).is_none());
        assert!(!parse(r#"{"Label":"0"}"#).unwrap().is_ai());
    }

    #[test]
    fn shared_alibaba_code_depends_on_media() {
        let code = "001191330106MA2CFLDG4R10001";
        assert_eq!(producer_tool(code, MediaKind::Video), Some("wan"));
        assert_eq!(producer_tool(code, MediaKind::Image), Some("qwen/wan"));
    }

    #[test]
    fn video_only_codes_and_prefixes() {
        assert_eq!(
            producer_tool("001191110108MACC4D63XF10306", MediaKind::Video),
            Some("vidu")
        );
        assert_eq!(
            producer_tool("001191110108MACC4D63XF10306", MediaKind::Image),
            None
        );
        assert_eq!(
            producer_tool("001191110000802100433B0000", MediaKind::Image),
            Some("qwen")
        );
        assert_eq!(producer_tool("UNKNOWN", MediaKind::Video), None);
    }

    #[test]
    fn unknown_or_propagator_only_is_ai_only() {
        let l = parse(r#"{"Label":"1","ContentPropagator":"MiniMax"}"#).unwrap();
        let a = attribute(&l, MediaKind::Video);
        assert!(a.tool.is_none());
        assert_eq!(a.scope, AttributionScope::AiOnly);

        let l = parse(r#"{"Label":"1","ContentProducer":"999"}"#).unwrap();
        let a = attribute(&l, MediaKind::Video);
        assert!(a.tool.is_none());
        assert_eq!(a.scope, AttributionScope::AiOnly);
        assert_eq!(a.raw_value, "999");
    }
}
