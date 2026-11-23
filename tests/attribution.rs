//! Model attribution, evidence levels and content-sniffed C2PA reading.
//!
//! The `attr_*` fixtures are unmodified generator outputs.
//! `attr_seedream_5_0_lite_jpeg_named.png` is a JPEG that was delivered with a
//! `.png` name; it must still have its C2PA manifest read.

use assert_cmd::cargo_bin_cmd;
use serde_json::Value;

fn check_json(path: &str) -> Value {
    let out = cargo_bin_cmd!("aic")
        .args(["--lang", "en", "--json", "check", path])
        .output()
        .expect("run aic");
    serde_json::from_slice(&out.stdout).expect("valid JSON")
}

fn file(v: &Value) -> &Value {
    &v["files"][0]
}

// --- P0: C2PA is read by content type, not by extension ---

#[test]
fn seedream_jpeg_with_png_name_reads_c2pa_model_id() {
    let v = check_json("tests/fixtures/attr_seedream_5_0_lite_jpeg_named.png");
    let f = file(&v);
    assert_eq!(f["mime_type"], "image/jpeg");
    assert_eq!(f["ai_generated"], true);
    assert_eq!(f["overall_confidence"], "high");
    assert_eq!(f["evidence_level"], "declared");
    let a = &f["attribution"];
    assert_eq!(a["scope"], "model_id");
    assert_eq!(a["tool"], "seedream");
    assert_eq!(a["model"], "doubao-seedream-5-0");
    assert_eq!(a["evidence_field"], "c2pa.created.parameters.model_name");
    assert_eq!(a["raw_value"], "doubao-seedream-5-0");
}

#[test]
fn same_bytes_with_correct_suffix_give_same_model() {
    let dir = tempfile::tempdir().unwrap();
    let dst = dir.path().join("seedream.jpg");
    std::fs::copy("tests/fixtures/attr_seedream_5_0_lite_jpeg_named.png", &dst).unwrap();
    let a = check_json("tests/fixtures/attr_seedream_5_0_lite_jpeg_named.png");
    let b = check_json(dst.to_str().unwrap());
    assert_eq!(file(&a)["attribution"], file(&b)["attribution"]);
    assert_eq!(
        file(&a)["overall_confidence"],
        file(&b)["overall_confidence"]
    );
}

#[test]
fn c2pa_jpeg_renamed_to_png_still_detected() {
    let dir = tempfile::tempdir().unwrap();
    let dst = dir.path().join("renamed.png");
    std::fs::copy("tests/fixtures/image_gemini-3-pro-image-preview.jpg", &dst).unwrap();
    let v = check_json(dst.to_str().unwrap());
    let f = file(&v);
    assert_eq!(f["overall_confidence"], "high");
    assert_eq!(f["attribution"]["tool"], "google ai");
}

// --- P0: model fields from the active manifest ---

#[test]
fn openai_software_agent_version_is_model_id() {
    let v = check_json("tests/fixtures/attr_gpt_image_2.png");
    let a = &file(&v)["attribution"];
    assert_eq!(a["scope"], "model_id");
    assert_eq!(a["tool"], "openai");
    assert_eq!(a["model"], "gpt-image-2");
    assert_eq!(a["evidence_field"], "c2pa.created.softwareAgent.version");
}

#[test]
fn c2pa_validation_state_is_reported() {
    let v = check_json("tests/fixtures/attr_gpt_image_2.png");
    let signals = file(&v)["signals"].as_array().unwrap();
    let states: Vec<String> = signals
        .iter()
        .flat_map(|s| s["details"].as_array().cloned().unwrap_or_default())
        .filter(|d| d[0] == "c2pa_validation")
        .filter_map(|d| d[1].as_str().map(str::to_string))
        .collect();
    assert!(!states.is_empty());
    assert!(states
        .iter()
        .all(|s| ["invalid", "valid_untrusted", "valid", "trusted"].contains(&s.as_str())));
}

// --- P1: unified AIGC label parsing ---

#[test]
fn minimax_exif_aigc_label_gives_vendor() {
    let v = check_json("tests/fixtures/attr_minimax_h3_image.jpeg");
    let a = &file(&v)["attribution"];
    assert_eq!(a["scope"], "provider");
    assert_eq!(a["tool"], "minimax");
    assert_eq!(a["raw_value"], "MiniMax");
}

#[test]
fn qwen_png_tc260_xmp_label_is_candidate_group_not_wan() {
    let v = check_json("tests/fixtures/attr_qwen_image_3_0_pro.png");
    let f = file(&v);
    let a = &f["attribution"];
    assert_eq!(a["scope"], "provider");
    assert_eq!(a["tool"], "qwen/wan");
    assert_eq!(a["raw_value"], "001191330106MA2CFLDG4R10001");
    assert!(a["evidence_field"]
        .as_str()
        .unwrap()
        .starts_with("XMP.TC260.AIGC"));
}

#[test]
fn wan_video_keeps_wan_attribution() {
    let v = check_json("tests/fixtures/ai_wan.mp4");
    assert_eq!(file(&v)["attribution"]["tool"], "wan");
}

#[test]
fn vidu_video_producer_code_maps_to_vidu() {
    let v = check_json("tests/fixtures/ai_vidu.mp4");
    let a = &file(&v)["attribution"];
    assert_eq!(a["tool"], "vidu");
    assert_eq!(a["raw_value"], "001191110108MACC4D63XF10306");
}

// --- P2: SEI markers verified structurally ---

#[test]
fn kling_sei_marker_is_a_declaration() {
    let v = check_json("tests/fixtures/ai_kling.mp4");
    let f = file(&v);
    assert_eq!(f["evidence_level"], "declared");
    assert_eq!(f["attribution"]["evidence_field"], "MP4.SEI");
    assert_eq!(f["attribution"]["tool"], "kling");
}

// --- P1: heuristic-only results ---

#[test]
fn watermark_only_result_is_heuristic_without_attribution() {
    let out = cargo_bin_cmd!("aic")
        .args([
            "--lang",
            "en",
            "--json",
            "check",
            "tests/fixtures/watermarked_dwtdct.png",
        ])
        .output()
        .unwrap();
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    let f = file(&v);
    // Backwards compatible boolean is unchanged.
    assert_eq!(f["ai_generated"], true);
    assert_eq!(f["evidence_level"], "heuristic");
    assert!(f.get("attribution").is_none());
    assert_eq!(v["summary"]["heuristic_only"], 1);
    assert_eq!(v["summary"]["declared"], 0);
}

#[test]
fn heuristic_only_verdict_is_labelled() {
    cargo_bin_cmd!("aic")
        .args([
            "--lang",
            "en",
            "check",
            "tests/fixtures/watermarked_dwtdct.png",
        ])
        .assert()
        .success()
        .stdout(predicates::str::contains("heuristic signals only"));
}

#[test]
fn clean_file_has_no_evidence() {
    let v = check_json("tests/fixtures/real_photo.jpg");
    assert_eq!(file(&v)["evidence_level"], "none");
}
