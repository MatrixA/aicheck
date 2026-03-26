use assert_cmd::cargo_bin_cmd;
use predicates::prelude::*;

// --- Sora (C2PA, HIGH) ---

#[test]
fn sora_detected_via_c2pa() {
    cargo_bin_cmd!("aic")
        .args(["--lang", "en", "check", "tests/fixtures/ai_sora.mp4"])
        .assert()
        .success()
        .stdout(predicate::str::contains("C2PA"))
        .stdout(predicate::str::contains("sora"));
}

#[test]
fn sora_json_output() {
    cargo_bin_cmd!("aic")
        .args([
            "--lang",
            "en",
            "--json",
            "check",
            "tests/fixtures/ai_sora.mp4",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"ai_generated\": true"));
}

#[test]
fn sora_pro_detected_via_c2pa() {
    cargo_bin_cmd!("aic")
        .args(["--lang", "en", "check", "tests/fixtures/ai_sora_pro.mp4"])
        .assert()
        .success()
        .stdout(predicate::str::contains("C2PA"))
        .stdout(predicate::str::contains("sora"));
}

// --- Kling (MP4 SEI marker, MEDIUM) ---

#[test]
fn kling_detected_via_sei_marker() {
    cargo_bin_cmd!("aic")
        .args(["--lang", "en", "check", "tests/fixtures/ai_kling.mp4"])
        .assert()
        .success()
        .stdout(predicate::str::contains("SEI"))
        .stdout(predicate::str::contains("kling"));
}

#[test]
fn kling_json_output() {
    cargo_bin_cmd!("aic")
        .args([
            "--lang",
            "en",
            "--json",
            "check",
            "tests/fixtures/ai_kling.mp4",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"ai_generated\": true"));
}

#[test]
fn kling_v3_detected_via_sei_marker() {
    cargo_bin_cmd!("aic")
        .args(["--lang", "en", "check", "tests/fixtures/ai_kling_v3.mp4"])
        .assert()
        .success()
        .stdout(predicate::str::contains("kling"));
}

#[test]
fn kling_omni_detected_via_sei_marker() {
    cargo_bin_cmd!("aic")
        .args(["--lang", "en", "check", "tests/fixtures/ai_kling_omni.mp4"])
        .assert()
        .success()
        .stdout(predicate::str::contains("kling"));
}

// --- Vidu (MP4 AIGC label, MEDIUM) ---

#[test]
fn vidu_detected_via_aigc_label() {
    cargo_bin_cmd!("aic")
        .args(["--lang", "en", "check", "tests/fixtures/ai_vidu.mp4"])
        .assert()
        .success()
        .stdout(predicate::str::contains("AIGC"));
}

#[test]
fn vidu_json_output() {
    cargo_bin_cmd!("aic")
        .args([
            "--lang",
            "en",
            "--json",
            "check",
            "tests/fixtures/ai_vidu.mp4",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"ai_generated\": true"));
}

// --- Wan (MP4 AIGC label, MEDIUM) ---

#[test]
fn wan_detected_via_aigc_label() {
    cargo_bin_cmd!("aic")
        .args(["--lang", "en", "check", "tests/fixtures/ai_wan.mp4"])
        .assert()
        .success()
        .stdout(predicate::str::contains("AIGC"));
}

#[test]
fn wan_json_output() {
    cargo_bin_cmd!("aic")
        .args([
            "--lang",
            "en",
            "--json",
            "check",
            "tests/fixtures/ai_wan.mp4",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"ai_generated\": true"));
}

// --- SeedAnce (Video frame watermark, MEDIUM, requires ffmpeg) ---

#[test]
fn seedance_detected_via_video_watermark() {
    cargo_bin_cmd!("aic")
        .args(["--lang", "en", "check", "tests/fixtures/ai_seedance.mp4"])
        .assert()
        .success()
        .stdout(predicate::str::contains("WATERMARK"));
}

#[test]
fn seedance_json_output() {
    cargo_bin_cmd!("aic")
        .args([
            "--lang",
            "en",
            "--json",
            "check",
            "tests/fixtures/ai_seedance.mp4",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"ai_generated\": true"));
}

// --- LTX (Video frame watermark, MEDIUM, requires ffmpeg) ---

#[test]
fn ltx_detected_via_video_watermark() {
    cargo_bin_cmd!("aic")
        .args(["--lang", "en", "check", "tests/fixtures/ai_ltx.mp4"])
        .assert()
        .success()
        .stdout(predicate::str::contains("WATERMARK"));
}

#[test]
fn ltx_json_output() {
    cargo_bin_cmd!("aic")
        .args([
            "--lang",
            "en",
            "--json",
            "check",
            "tests/fixtures/ai_ltx.mp4",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"ai_generated\": true"));
}

// --- Info command tests ---

#[test]
fn sora_info_shows_c2pa() {
    cargo_bin_cmd!("aic")
        .args(["--lang", "en", "info", "tests/fixtures/ai_sora.mp4"])
        .assert()
        .success()
        .stdout(predicate::str::contains("C2PA"));
}

#[test]
fn kling_info_shows_sei_marker() {
    cargo_bin_cmd!("aic")
        .args(["--lang", "en", "info", "tests/fixtures/ai_kling.mp4"])
        .assert()
        .success()
        .stdout(predicate::str::contains("SEI"));
}

#[test]
fn vidu_info_shows_aigc() {
    cargo_bin_cmd!("aic")
        .args(["--lang", "en", "info", "tests/fixtures/ai_vidu.mp4"])
        .assert()
        .success()
        .stdout(predicate::str::contains("AIGC"));
}

// --- Deep mode tests (slow, require ffmpeg + frame extraction) ---

#[test]
#[ignore] // slow: video frame watermark analysis
fn sora_deep_detection() {
    cargo_bin_cmd!("aic")
        .args([
            "--lang",
            "en",
            "check",
            "--deep",
            "tests/fixtures/ai_sora.mp4",
        ])
        .assert()
        .success();
}

#[test]
#[ignore] // slow: video frame watermark analysis
fn kling_deep_detection() {
    cargo_bin_cmd!("aic")
        .args([
            "--lang",
            "en",
            "check",
            "--deep",
            "tests/fixtures/ai_kling.mp4",
        ])
        .assert()
        .success();
}
