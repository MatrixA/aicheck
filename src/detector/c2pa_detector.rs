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

    let manifest = match reader.active_manifest() {
        Some(m) => m,
        None => return Ok(vec![]),
    };

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
                    description: format!("claim_generator_info references AI tool"),
                    tool: Some(tool_name.to_string()),
                    details: vec![("claim_generator_info".into(), info_json)],
                });
            }
        }
    }

    // Check actions assertions for digitalSourceType
    // Try both versioned and unversioned labels
    for label in &[Actions::LABEL, "c2pa.actions.v2"] {
        if let Ok(actions) = manifest.find_assertion::<Actions>(label) {
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
                            description: format!(
                                "digitalSourceType = {}",
                                desc
                            ),
                            tool: action
                                .software_agent()
                                .and_then(|sw| {
                                    let sw_str =
                                        serde_json::to_string(sw).unwrap_or_default();
                                    known_tools::match_ai_tool(&sw_str)
                                        .map(|s| s.to_string())
                                }),
                            details,
                        });
                    }
                }
            }
        }
    }

    Ok(signals)
}
