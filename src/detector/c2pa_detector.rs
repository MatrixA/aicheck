use anyhow::Result;
use c2pa::assertions::{Actions, DigitalSourceType};
use c2pa::Reader;
use std::path::Path;

use super::{Confidence, Signal, SignalSource};
use crate::known_tools;

/// AI-related digital source types that indicate AI generation.
fn is_ai_source_type(dst: &DigitalSourceType) -> Option<(Confidence, &'static str)> {
    match dst {
        DigitalSourceType::TrainedAlgorithmicMedia => {
            Some((Confidence::High, "trainedAlgorithmicMedia (fully AI-generated)"))
        }
        DigitalSourceType::CompositeWithTrainedAlgorithmicMedia => Some((
            Confidence::High,
            "compositeWithTrainedAlgorithmicMedia (AI-edited)",
        )),
        DigitalSourceType::CompositeSynthetic => {
            Some((Confidence::High, "compositeSynthetic (includes AI elements)"))
        }
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

/// Detect AI signals from C2PA manifests.
pub fn detect(path: &Path) -> Result<Vec<Signal>> {
    let reader = match Reader::from_file(path) {
        Ok(r) => r,
        Err(c2pa::Error::JumbfNotFound) => return Ok(vec![]),
        Err(c2pa::Error::UnsupportedType) => return Ok(vec![]),
        Err(e) => return Err(e.into()),
    };

    let mut signals = Vec::new();

    // Check all manifests — active manifest may not contain the AI generation action
    // (e.g., GPT images have a parent manifest with c2pa.created + digitalSourceType
    // and a child manifest with c2pa.opened)
    for manifest in reader.manifests().values() {
        check_manifest(manifest, &mut signals);
    }

    Ok(signals)
}

/// Extract AI signals from a single C2PA manifest.
fn check_manifest(manifest: &c2pa::Manifest, signals: &mut Vec<Signal>) {
    // Check claim_generator for known AI tools
    if let Some(cg) = manifest.claim_generator() {
        if let Some(tool_name) = known_tools::match_ai_tool(cg) {
            signals.push(Signal {
                source: SignalSource::C2pa,
                confidence: Confidence::High,
                description: format!("claim_generator matches AI tool: {}", cg),
                tool: Some(tool_name.to_string()),
                details: vec![("claim_generator".into(), cg.to_string())],
            });
        }
    }

    // Check claim_generator_info
    if let Some(info_list) = &manifest.claim_generator_info {
        for info in info_list {
            let info_json = serde_json::to_string(info).unwrap_or_default();
            if let Some(tool_name) = known_tools::match_ai_tool(&info_json) {
                signals.push(Signal {
                    source: SignalSource::C2pa,
                    confidence: Confidence::High,
                    description: "claim_generator_info references AI tool".to_string(),
                    tool: Some(tool_name.to_string()),
                    details: vec![("claim_generator_info".into(), info_json)],
                });
            }
        }
    }

    // Check actions assertions for digitalSourceType
    // Use the actual assertion labels present in the manifest to avoid duplicates
    let mut checked_labels = Vec::new();
    for label in &[Actions::LABEL, "c2pa.actions.v2"] {
        // Skip if we already found signals from this assertion (v1/v2 can overlap)
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
                        if let Some(sw) = action.software_agent() {
                            let sw_str = serde_json::to_string(sw).unwrap_or_default();
                            details.push(("softwareAgent".into(), sw_str));
                        }

                        signals.push(Signal {
                            source: SignalSource::C2pa,
                            confidence,
                            description: format!("digitalSourceType = {}", desc),
                            tool: action.software_agent().and_then(|sw| {
                                let sw_str = serde_json::to_string(sw).unwrap_or_default();
                                known_tools::match_ai_tool(&sw_str).map(|s| s.to_string())
                            }),
                            details,
                        });
                    }
                }
            }
            // If we found actions with v1 label, skip v2 (they return same data)
            break;
        }
    }
}
