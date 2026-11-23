use anyhow::Result;
use c2pa::assertions::{Actions, DigitalSourceType};
use c2pa::{Reader, ValidationState};
use serde_json::Value;
use std::fs::File;
use std::path::Path;

use super::{AttributionScope, Confidence, Signal, SignalBuilder, SignalSource};
use crate::known_tools;

/// AI-related digital source types that indicate AI generation.
fn is_ai_source_type(dst: &DigitalSourceType) -> Option<(Confidence, &'static str)> {
    match dst {
        DigitalSourceType::TrainedAlgorithmicMedia => Some((
            Confidence::High,
            "trainedAlgorithmicMedia (fully AI-generated)",
        )),
        DigitalSourceType::CompositeWithTrainedAlgorithmicMedia => Some((
            Confidence::High,
            "compositeWithTrainedAlgorithmicMedia (AI-edited)",
        )),
        DigitalSourceType::CompositeSynthetic => Some((
            Confidence::High,
            "compositeSynthetic (includes AI elements)",
        )),
        DigitalSourceType::AlgorithmicMedia => Some((
            Confidence::Medium,
            "algorithmicMedia (algorithmic, not necessarily AI-trained)",
        )),
        DigitalSourceType::DataDrivenMedia => Some((
            Confidence::Medium,
            "dataDrivenMedia (data-driven generation)",
        )),
        DigitalSourceType::TrainedAlgorithmicData => Some((
            Confidence::Medium,
            "trainedAlgorithmicData (AI-generated data)",
        )),
        _ => None,
    }
}

/// MIME types the C2PA SDK can read from a stream, keyed by sniffed MIME.
/// Files are routed by their *content*, not their extension, so e.g. a JPEG
/// saved as `.png` still has its manifest read.
fn c2pa_stream_format(path: &Path) -> Option<&'static str> {
    let kind = infer::get_from_path(path).ok().flatten()?;
    let fmt = match kind.mime_type() {
        "image/jpeg" => "image/jpeg",
        "image/png" => "image/png",
        "image/webp" => "image/webp",
        "image/gif" => "image/gif",
        "image/tiff" => "image/tiff",
        "image/avif" => "image/avif",
        "image/heif" => "image/heif",
        "image/heic" => "image/heic",
        "image/svg+xml" => "image/svg+xml",
        "video/mp4" => "video/mp4",
        "video/quicktime" => "video/quicktime",
        "video/x-msvideo" => "video/x-msvideo",
        "audio/mpeg" => "audio/mpeg",
        "audio/x-wav" | "audio/wav" => "audio/wav",
        "audio/m4a" | "audio/x-m4a" | "audio/mp4" => "audio/mp4",
        "application/pdf" => "application/pdf",
        _ => return None,
    };
    Some(fmt)
}

fn is_absent(e: &c2pa::Error) -> bool {
    matches!(
        e,
        c2pa::Error::JumbfNotFound | c2pa::Error::UnsupportedType | c2pa::Error::ProvenanceMissing
    )
}

/// Open a C2PA reader. Try the content-sniffed format first; fall back to the
/// path-based reader, which keeps sidecar (`.c2pa`) and extension-based
/// behaviour for formats that cannot be sniffed.
fn open_reader(path: &Path) -> Result<Option<Reader>> {
    let mut stream_err = None;
    if let Some(format) = c2pa_stream_format(path) {
        match File::open(path)
            .map_err(c2pa::Error::from)
            .and_then(|f| Reader::from_stream(format, f))
        {
            Ok(r) => return Ok(Some(r)),
            Err(e) if is_absent(&e) => {}
            Err(e) => stream_err = Some(e),
        }
    }
    match Reader::from_file(path) {
        Ok(r) => Ok(Some(r)),
        Err(e) if is_absent(&e) => match stream_err {
            Some(se) => Err(se.into()),
            None => Ok(None),
        },
        Err(e) => Err(stream_err.unwrap_or(e).into()),
    }
}

/// Validation summary that separates "invalid", "valid but signer untrusted"
/// and "trusted".
fn validation_label(reader: &Reader) -> &'static str {
    const UNTRUSTED: &str = "signingCredential.untrusted";
    let untrusted = reader
        .validation_status()
        .map(|s| s.iter().any(|v| v.code() == UNTRUSTED))
        .unwrap_or(false)
        || reader
            .validation_results()
            .and_then(|r| r.active_manifest())
            .map(|m| m.failure().iter().any(|v| v.code() == UNTRUSTED))
            .unwrap_or(false);
    match reader.validation_state() {
        ValidationState::Invalid => "invalid",
        ValidationState::Trusted => "trusted",
        _ if untrusted => "valid_untrusted",
        _ => "valid",
    }
}

/// Detect AI signals from C2PA manifests.
pub fn detect(path: &Path) -> Result<Vec<Signal>> {
    let reader = match open_reader(path)? {
        Some(r) => r,
        None => return Ok(vec![]),
    };

    let validation = validation_label(&reader);
    let active_label = reader.active_label().map(|s| s.to_string());
    let mut signals = Vec::new();

    for (label, manifest) in reader.manifests() {
        let is_active = active_label.as_deref() == Some(label.as_str());
        let start = signals.len();
        check_manifest(manifest, &mut signals);
        for s in &mut signals[start..] {
            s.details
                .push(("c2pa_validation".into(), validation.into()));
            if !is_active {
                s.details.push(("manifest".into(), "ingredient".into()));
                // Ingredient history does not describe the current asset.
                s.scope = None;
            }
        }
    }

    // Model attribution is read from the active manifest only, and never from
    // a manifest whose signature/integrity checks fail.
    if validation != "invalid" {
        if let Some(manifest) = reader.active_manifest() {
            if let Ok(json) = serde_json::to_value(manifest) {
                if std::env::var("AIC_DEBUG").is_ok() {
                    eprintln!(
                        "  [debug] C2PA active manifest: {}",
                        serde_json::to_string(&json).unwrap_or_default()
                    );
                }
                for attr in extract_model_attributions(&json) {
                    signals.push(attr.into_signal(validation));
                }
            }
        }
    }

    Ok(signals)
}

/// Extract AI signals from a single C2PA manifest.
fn check_manifest(manifest: &c2pa::Manifest, signals: &mut Vec<Signal>) {
    // Check claim_generator for known AI tools
    if let Some(cg) = manifest.claim_generator() {
        if let Some(tool_name) = known_tools::match_ai_tool(cg) {
            signals.push(
                SignalBuilder::new(
                    SignalSource::C2pa,
                    Confidence::High,
                    "signal_c2pa_claim_generator",
                )
                .param("value", cg)
                .tool(tool_name)
                .scope(AttributionScope::Provider)
                .evidence_at("claim_generator", cg)
                .detail("claim_generator", cg)
                .build(),
            );
        } else if cg.to_lowercase().contains("google") {
            // Google C2PA manifests (e.g. "Google C2PA Core Generator Library")
            // indicate AI-generated content from Google Imagen/Gemini.
            signals.push(
                SignalBuilder::new(
                    SignalSource::C2pa,
                    Confidence::Medium,
                    "signal_c2pa_claim_generator",
                )
                .param("value", cg)
                .tool("google ai")
                .scope(AttributionScope::Provider)
                .evidence_at("claim_generator", cg)
                .detail("claim_generator", cg)
                .build(),
            );
        }
    }

    // Check claim_generator_info. Only the descriptive fields (name, version)
    // are matched, not icons or other nested data.
    if let Some(info_list) = &manifest.claim_generator_info {
        for info in info_list {
            let info_json = serde_json::to_string(info).unwrap_or_default();
            let name = info.name.as_str();
            let version = info.version.as_deref().unwrap_or("");
            let text = format!("{} {}", name, version);
            if let Some(tool_name) = known_tools::match_ai_tool(&text) {
                signals.push(
                    SignalBuilder::new(
                        SignalSource::C2pa,
                        Confidence::High,
                        "signal_c2pa_claim_generator_info",
                    )
                    .tool(tool_name)
                    .scope(AttributionScope::Provider)
                    .evidence_at("claim_generator_info.name", name)
                    .detail("claim_generator_info", &info_json)
                    .build(),
                );
            } else if name.to_lowercase().contains("google") {
                signals.push(
                    SignalBuilder::new(
                        SignalSource::C2pa,
                        Confidence::Medium,
                        "signal_c2pa_claim_generator_info",
                    )
                    .tool("google ai")
                    .scope(AttributionScope::Provider)
                    .evidence_at("claim_generator_info.name", name)
                    .detail("claim_generator_info", &info_json)
                    .build(),
                );
            }
        }
    }

    // Check actions assertions for digitalSourceType
    let mut checked_labels = Vec::new();
    for label in &[Actions::LABEL, "c2pa.actions.v2"] {
        if checked_labels.iter().any(|l: &&str| l == label) {
            continue;
        }
        if let Ok(actions) = manifest.find_assertion::<Actions>(label) {
            checked_labels.push(label);
            for action in actions.actions() {
                if let Some(src_type) = action.source_type() {
                    if let Some((confidence, desc)) = is_ai_source_type(src_type) {
                        let mut details = vec![
                            ("action".into(), action.action().to_string()),
                            ("digitalSourceType".into(), desc.to_string()),
                        ];
                        let agent_text = action.software_agent().map(software_agent_text);
                        if let Some(sw) = action.software_agent() {
                            let sw_str = serde_json::to_string(sw).unwrap_or_default();
                            details.push(("softwareAgent".into(), sw_str));
                        }
                        let tool = agent_text
                            .as_deref()
                            .and_then(known_tools::match_ai_tool)
                            .map(|s| s.to_string());
                        let scope = if tool.is_some() {
                            AttributionScope::Provider
                        } else {
                            AttributionScope::AiOnly
                        };

                        signals.push(
                            SignalBuilder::new(
                                SignalSource::C2pa,
                                confidence,
                                "signal_c2pa_digital_source_type",
                            )
                            .param("value", desc)
                            .tool_opt(tool)
                            .scope(scope)
                            .evidence_at(
                                format!("{}.digitalSourceType", action.action()),
                                desc.split(' ').next().unwrap_or(desc),
                            )
                            .details(details)
                            .build(),
                        );
                    }
                }
            }
            break;
        }
    }
}

/// Software agent as plain text (name + version), without JSON punctuation.
fn software_agent_text(sw: &c2pa::assertions::SoftwareAgent) -> String {
    match serde_json::to_value(sw) {
        Ok(Value::String(s)) => s,
        Ok(Value::Object(o)) => {
            let name = o.get("name").and_then(Value::as_str).unwrap_or("");
            let version = o.get("version").and_then(Value::as_str).unwrap_or("");
            format!("{} {}", name, version).trim().to_string()
        }
        _ => String::new(),
    }
}

/// A model / version / vendor declaration read from the active manifest.
#[derive(Debug, Clone, PartialEq)]
struct ModelAttribution {
    tool: &'static str,
    scope: AttributionScope,
    model: Option<String>,
    field: String,
    raw_value: String,
    /// Other fields in the same manifest that declare the same value.
    corroborated_by: Vec<String>,
}

impl ModelAttribution {
    fn into_signal(self, validation: &str) -> Signal {
        let shown = self.model.clone().unwrap_or_else(|| self.raw_value.clone());
        let alias = self.model.as_deref().and_then(product_alias);
        let mut b = SignalBuilder::new(
            SignalSource::C2pa,
            Confidence::High,
            "signal_c2pa_model_declared",
        )
        .param("scope", self.scope.as_str())
        .param("value", shown)
        .tool(self.tool)
        .scope(self.scope)
        .evidence_at(self.field.clone(), self.raw_value.clone())
        .detail(self.field, self.raw_value)
        .detail("c2pa_validation", validation);
        if let Some(m) = self.model {
            b = b.model(m);
        }
        for c in self.corroborated_by {
            b = b.detail("corroborated_by", c);
        }
        if let Some(alias) = alias {
            b = b.detail("known_product_alias", alias);
        }
        b.build()
    }
}

/// Product names for backend model IDs observed in sample batches. The raw ID
/// is always reported; this alias is informational only.
fn product_alias(model: &str) -> Option<&'static str> {
    Some(match model {
        "doubao-seedream-4-0" => "Seedream 4.0",
        "doubao-seedream-4-5" => "Seedream 4.5",
        "doubao-seedream-5-0" => "Seedream 5.0 Lite",
        "doubao-seedream-5-0-pro" => "Seedream 5.0 Pro",
        "doubao-seedream-5-5" => "Seedream 5.5",
        "doubao-seedance-2-0" => "Seedance 2.0",
        "doubao-seedance-2-0-fast" => "Seedance 2.0 Fast",
        "doubao-seedance-2-0-mini" => "Seedance 2.0 Mini",
        "doubao-seedance-2-5" => "Seedance 2.5",
        "doubao-seedance-2-5-flash" => "Seedance 2.5 Flash",
        _ => return None,
    })
}

fn str_at<'a>(v: &'a Value, key: &str) -> Option<&'a str> {
    v.get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
}

fn is_trained_algorithmic(dst: &str) -> bool {
    let tail = dst.rsplit('/').next().unwrap_or(dst);
    tail == "trainedAlgorithmicMedia" || tail == "compositeWithTrainedAlgorithmicMedia"
}

/// Extract model attributions from the JSON form of the active manifest.
///
/// Only `c2pa.created` actions declaring trained-algorithmic media are used,
/// and vendor-specific fields are only read when the claim generator belongs
/// to that vendor.
fn extract_model_attributions(manifest: &Value) -> Vec<ModelAttribution> {
    let mut claims: Vec<String> = Vec::new();
    if let Some(cg) = str_at(manifest, "claim_generator") {
        claims.push(cg.to_string());
    }
    if let Some(list) = manifest
        .get("claim_generator_info")
        .and_then(Value::as_array)
    {
        for info in list {
            if let Some(n) = str_at(info, "name") {
                claims.push(n.to_string());
            }
        }
    }
    let claim_has = |needle: &str| {
        let n = needle.to_lowercase();
        claims.iter().any(|c| c.to_lowercase().contains(&n))
    };

    let assertions: Vec<&Value> = manifest
        .get("assertions")
        .and_then(Value::as_array)
        .map(|a| a.iter().collect())
        .unwrap_or_default();
    let assertion_data = |label: &str| -> Vec<&Value> {
        assertions
            .iter()
            .filter(|a| {
                a.get("label")
                    .and_then(Value::as_str)
                    .map(|l| l == label || l.starts_with(&format!("{}.v", label)))
                    .unwrap_or(false)
            })
            .filter_map(|a| a.get("data"))
            .collect()
    };

    let mut out: Vec<ModelAttribution> = Vec::new();
    let mut actions_seen = false;
    for label in ["c2pa.actions.v2", "c2pa.actions"] {
        for data in assertion_data(label) {
            if actions_seen && label == "c2pa.actions" {
                break;
            }
            let Some(actions) = data.get("actions").and_then(Value::as_array) else {
                continue;
            };
            actions_seen = true;
            for action in actions {
                if str_at(action, "action") != Some("c2pa.created") {
                    continue;
                }
                if !str_at(action, "digitalSourceType")
                    .map(is_trained_algorithmic)
                    .unwrap_or(false)
                {
                    continue;
                }
                let (name, version) = match action.get("softwareAgent") {
                    Some(Value::String(s)) => (Some(s.trim()), None),
                    Some(o @ Value::Object(_)) => (str_at(o, "name"), str_at(o, "version")),
                    _ => (None, None),
                };
                if let Some(a) = attribute_created_action(name, version, action, &claim_has) {
                    if !out.contains(&a) {
                        out.push(a);
                    }
                }
            }
        }
    }

    // Cross-check vendor-specific assertions in the same manifest. They are
    // recorded as corroboration of the same file, not as separate evidence.
    for a in &mut out {
        match a.tool {
            "flux" => {
                for d in assertion_data("c2pa.ai-disclosure") {
                    if let Some(m) = str_at(d, "modelName") {
                        if Some(m) == a.model.as_deref() {
                            a.corroborated_by
                                .push(format!("c2pa.ai-disclosure.modelName={}", m));
                        }
                    }
                }
            }
            "ideogram" => {
                for d in assertion_data("com.ideogram.generation") {
                    if let Some(v) = str_at(d, "system_version") {
                        if v == a.raw_value {
                            a.corroborated_by
                                .push(format!("com.ideogram.generation.system_version={}", v));
                        }
                    }
                }
            }
            _ => {}
        }
    }

    // BFL files may only carry the model in the AI disclosure assertion.
    if claim_has("Black Forest Labs") && !out.iter().any(|a| a.tool == "flux") {
        for d in assertion_data("c2pa.ai-disclosure") {
            if let Some(m) = str_at(d, "modelName") {
                if m.to_lowercase().starts_with("flux") {
                    out.push(ModelAttribution {
                        tool: "flux",
                        scope: AttributionScope::Series,
                        model: Some(m.to_string()),
                        field: "c2pa.ai-disclosure.modelName".into(),
                        raw_value: m.to_string(),
                        corroborated_by: vec![],
                    });
                    break;
                }
            }
        }
    }

    out
}

fn attribute_created_action(
    name: Option<&str>,
    version: Option<&str>,
    action: &Value,
    claim_has: &dyn Fn(&str) -> bool,
) -> Option<ModelAttribution> {
    let name = name?;
    let lname = name.to_lowercase();

    // Volcengine Ark (Seedream / Seedance): parameters.model_name is the
    // backend model ID.
    if lname.starts_with("volcengine") {
        let model = action
            .get("parameters")
            .and_then(|p| str_at(p, "model_name"))?;
        let lm = model.to_lowercase();
        let tool = if lm.contains("seedance") {
            "seedance"
        } else if lm.contains("seedream") {
            "seedream"
        } else {
            "volcengine"
        };
        return Some(ModelAttribution {
            tool,
            scope: AttributionScope::ModelId,
            model: Some(model.to_string()),
            field: "c2pa.created.parameters.model_name".into(),
            raw_value: model.to_string(),
            corroborated_by: vec![],
        });
    }

    // OpenAI: softwareAgent {name: "API", version: "<model>"}. A bare
    // "gpt-image" only supports the vendor.
    if claim_has("OpenAI") && name == "API" {
        let v = version?;
        let lv = v.to_lowercase();
        let specific = lv.starts_with("gpt-image-") || lv.starts_with("dall-e-");
        return Some(ModelAttribution {
            tool: "openai",
            scope: if specific {
                AttributionScope::ModelId
            } else {
                AttributionScope::Provider
            },
            model: specific.then(|| v.to_string()),
            field: "c2pa.created.softwareAgent.version".into(),
            raw_value: v.to_string(),
            corroborated_by: vec![],
        });
    }

    // Black Forest Labs: softwareAgent name is the series (FLUX.2, FLUX.3).
    if claim_has("Black Forest Labs") && lname.starts_with("flux") {
        return Some(ModelAttribution {
            tool: "flux",
            scope: AttributionScope::Series,
            model: Some(name.to_string()),
            field: "c2pa.created.softwareAgent.name".into(),
            raw_value: name.to_string(),
            corroborated_by: vec![],
        });
    }

    // Luma: softwareAgent name is the backend (e.g. Uni-1.1).
    if claim_has("Luma") {
        return Some(ModelAttribution {
            tool: "luma",
            scope: AttributionScope::Backend,
            model: Some(name.to_string()),
            field: "c2pa.created.softwareAgent.name".into(),
            raw_value: name.to_string(),
            corroborated_by: vec![],
        });
    }

    // Ideogram: softwareAgent {name: "Ideogram", version: "V_4_0"}.
    if claim_has("Ideogram") && lname == "ideogram" {
        let v = version?;
        return Some(ModelAttribution {
            tool: "ideogram",
            scope: AttributionScope::Backend,
            model: Some(v.to_string()),
            field: "c2pa.created.softwareAgent.version".into(),
            raw_value: v.to_string(),
            corroborated_by: vec![],
        });
    }

    // Kling: vendor only.
    if lname == "kling ai" || lname == "kling" {
        return Some(ModelAttribution {
            tool: "kling",
            scope: AttributionScope::Provider,
            model: None,
            field: "c2pa.created.softwareAgent.name".into(),
            raw_value: name.to_string(),
            corroborated_by: vec![],
        });
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn manifest(claim: &str, agent: Value, params: Value, extra: Vec<Value>) -> Value {
        let mut assertions = vec![json!({
            "label": "c2pa.actions.v2",
            "data": {"actions": [{
                "action": "c2pa.created",
                "digitalSourceType": "http://cv.iptc.org/newscodes/digitalsourcetype/trainedAlgorithmicMedia",
                "softwareAgent": agent,
                "parameters": params,
            }]}
        })];
        assertions.extend(extra);
        json!({"claim_generator_info": [{"name": claim}], "assertions": assertions})
    }

    #[test]
    fn volcengine_model_name() {
        let m = manifest(
            "Volcengine",
            json!({"name": "Volcengine_Ark_CN", "version": "1.0.0"}),
            json!({"model_name": "doubao-seedream-5-0"}),
            vec![],
        );
        let a = extract_model_attributions(&m);
        assert_eq!(a.len(), 1);
        assert_eq!(a[0].scope, AttributionScope::ModelId);
        assert_eq!(a[0].tool, "seedream");
        assert_eq!(a[0].model.as_deref(), Some("doubao-seedream-5-0"));
        assert_eq!(
            product_alias("doubao-seedream-5-0"),
            Some("Seedream 5.0 Lite")
        );
    }

    #[test]
    fn openai_version_specific_and_generic() {
        let m = manifest(
            "OpenAI Media Service API",
            json!({"name": "API", "version": "gpt-image-2"}),
            json!({}),
            vec![],
        );
        let a = extract_model_attributions(&m);
        assert_eq!(a[0].scope, AttributionScope::ModelId);
        assert_eq!(a[0].model.as_deref(), Some("gpt-image-2"));

        let m = manifest(
            "OpenAI Media Service API",
            json!({"name": "API", "version": "gpt-image"}),
            json!({}),
            vec![],
        );
        let a = extract_model_attributions(&m);
        assert_eq!(a[0].scope, AttributionScope::Provider);
        assert_eq!(a[0].model, None);
        assert_eq!(a[0].raw_value, "gpt-image");
    }

    #[test]
    fn flux_series_with_disclosure_corroboration() {
        let m = manifest(
            "Black Forest Labs API",
            json!({"name": "FLUX.2"}),
            json!({}),
            vec![json!({"label": "c2pa.ai-disclosure", "data": {"modelName": "FLUX.2"}})],
        );
        let a = extract_model_attributions(&m);
        assert_eq!(a.len(), 1);
        assert_eq!(a[0].scope, AttributionScope::Series);
        assert_eq!(a[0].corroborated_by.len(), 1);
    }

    #[test]
    fn ideogram_backend_and_kling_provider() {
        let m = manifest(
            "Ideogram",
            json!({"name": "Ideogram", "version": "V_4_0"}),
            json!({}),
            vec![
                json!({"label": "com.ideogram.generation", "data": {"system_name": "Ideogram", "system_version": "V_4_0"}}),
            ],
        );
        let a = extract_model_attributions(&m);
        assert_eq!(a[0].scope, AttributionScope::Backend);
        assert_eq!(a[0].model.as_deref(), Some("V_4_0"));
        assert_eq!(a[0].corroborated_by.len(), 1);

        let m = manifest(
            "ks-c2pa-service",
            json!({"name": "Kling AI"}),
            json!({}),
            vec![],
        );
        let a = extract_model_attributions(&m);
        assert_eq!(a[0].scope, AttributionScope::Provider);
        assert_eq!(a[0].tool, "kling");
    }

    #[test]
    fn vendor_fields_require_vendor_claim() {
        // "API" agent outside an OpenAI claim is not attributed.
        let m = manifest(
            "Some Other Service",
            json!({"name": "API", "version": "gpt-image-2"}),
            json!({}),
            vec![],
        );
        assert!(extract_model_attributions(&m).is_empty());
    }

    #[test]
    fn non_created_or_non_ai_actions_ignored() {
        let m = json!({
            "claim_generator_info": [{"name": "Volcengine"}],
            "assertions": [{"label": "c2pa.actions", "data": {"actions": [
                {"action": "c2pa.edited", "digitalSourceType": "http://cv.iptc.org/newscodes/digitalsourcetype/trainedAlgorithmicMedia",
                 "softwareAgent": {"name": "Volcengine_Ark_CN"}, "parameters": {"model_name": "x"}},
                {"action": "c2pa.created", "digitalSourceType": "http://cv.iptc.org/newscodes/digitalsourcetype/digitalCapture",
                 "softwareAgent": {"name": "Volcengine_Ark_CN"}, "parameters": {"model_name": "y"}}
            ]}}]
        });
        assert!(extract_model_attributions(&m).is_empty());
    }
}
