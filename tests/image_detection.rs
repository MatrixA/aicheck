use assert_cmd::cargo_bin_cmd;
use predicates::prelude::*;

// --- GPT Image (C2PA, HIGH) ---

#[test]
fn gptimage_detected_via_c2pa() {
    cargo_bin_cmd!("aic")
        .args(["--lang", "en", "check", "tests/fixtures/ai_gptimage.png"])
        .assert()
        .success()
        .stdout(predicate::str::contains("C2PA"))
        .stdout(predicate::str::contains("trainedAlgorithmicMedia"))
        .stdout(predicate::str::contains("gpt-4o"));
}

#[test]
fn gptimage_json_output() {
    cargo_bin_cmd!("aic")
        .args([
            "--lang",
            "en",
            "--json",
            "check",
            "tests/fixtures/ai_gptimage.png",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"ai_generated\": true"));
}

#[test]
fn gptimage_1_5_detected_via_c2pa() {
    cargo_bin_cmd!("aic")
        .args([
            "--lang",
            "en",
            "check",
            "tests/fixtures/ai_gptimage_1_5.png",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("C2PA"))
        .stdout(predicate::str::contains("gpt-4o"));
}

// --- Midjourney (XMP + Filename, MEDIUM) ---

#[test]
fn midjourney_1_detected_via_xmp() {
    cargo_bin_cmd!("aic")
        .args([
            "--lang",
            "en",
            "check",
            "tests/fixtures/ai_midjourney_1.png",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("XMP"))
        .stdout(predicate::str::contains("trainedAlgorithmicMedia"));
}

#[test]
fn midjourney_1_json_output() {
    cargo_bin_cmd!("aic")
        .args([
            "--lang",
            "en",
            "--json",
            "check",
            "tests/fixtures/ai_midjourney_1.png",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"ai_generated\": true"));
}

#[test]
fn midjourney_2_detected_via_xmp() {
    cargo_bin_cmd!("aic")
        .args([
            "--lang",
            "en",
            "check",
            "tests/fixtures/ai_midjourney_2.png",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("XMP"));
}

#[test]
fn midjourney_3_detected_via_xmp() {
    cargo_bin_cmd!("aic")
        .args([
            "--lang",
            "en",
            "check",
            "tests/fixtures/ai_midjourney_3.png",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("XMP"));
}

#[test]
fn midjourney_4_detected_via_xmp() {
    cargo_bin_cmd!("aic")
        .args([
            "--lang",
            "en",
            "check",
            "tests/fixtures/ai_midjourney_4.png",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("XMP"));
}

#[test]
fn midjourney_filename_pattern_detected() {
    cargo_bin_cmd!("aic")
        .args([
            "--lang",
            "en",
            "check",
            "tests/fixtures/ai_midjourney_1.png",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("midjourney"));
}

// --- Ideogram (EXIF, LOW) ---

#[test]
fn ideogram_detected_via_exif() {
    cargo_bin_cmd!("aic")
        .args(["--lang", "en", "check", "tests/fixtures/ai_ideogram.png"])
        .assert()
        .success()
        .stdout(predicate::str::contains("EXIF"))
        .stdout(predicate::str::contains("ideogram"));
}

#[test]
fn ideogram_json_output() {
    cargo_bin_cmd!("aic")
        .args([
            "--lang",
            "en",
            "--json",
            "check",
            "tests/fixtures/ai_ideogram.png",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"ai_generated\": true"));
}

// --- Flux (C2PA, HIGH) ---

#[test]
fn flux_pro_detected_via_c2pa() {
    cargo_bin_cmd!("aic")
        .args(["--lang", "en", "check", "tests/fixtures/ai_flux_pro.jpeg"])
        .assert()
        .success()
        .stdout(predicate::str::contains("C2PA"))
        .stdout(predicate::str::contains("flux"));
}

#[test]
fn flux_max_detected_via_c2pa() {
    cargo_bin_cmd!("aic")
        .args(["--lang", "en", "check", "tests/fixtures/ai_flux_max.jpeg"])
        .assert()
        .success()
        .stdout(predicate::str::contains("C2PA"))
        .stdout(predicate::str::contains("flux"));
}

#[test]
fn flux_pro_json_output() {
    cargo_bin_cmd!("aic")
        .args([
            "--lang",
            "en",
            "--json",
            "check",
            "tests/fixtures/ai_flux_pro.jpeg",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"ai_generated\": true"));
}

// --- SeedReam (EXIF/Watermark, LOW-MEDIUM) ---

#[test]
fn seedream_detected_via_exif() {
    cargo_bin_cmd!("aic")
        .args(["--lang", "en", "check", "tests/fixtures/ai_seedream.jpeg"])
        .assert()
        .success()
        .stdout(predicate::str::contains("EXIF"));
}

#[test]
fn seedream_v4_detected_via_watermark() {
    cargo_bin_cmd!("aic")
        .args([
            "--lang",
            "en",
            "check",
            "tests/fixtures/ai_seedream_v4.jpeg",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("WATERMARK"));
}

#[test]
fn seedream_4_5_detected_via_watermark() {
    cargo_bin_cmd!("aic")
        .args([
            "--lang",
            "en",
            "check",
            "tests/fixtures/ai_seedream_4_5.jpeg",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("WATERMARK"));
}

#[test]
fn seedream_v5_detected_via_visible_watermark() {
    cargo_bin_cmd!("aic")
        .args([
            "--lang",
            "en",
            "check",
            "tests/fixtures/ai_seedream_v5.jpeg",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Visible text overlay"));
}

#[test]
fn seedream_json_output() {
    cargo_bin_cmd!("aic")
        .args([
            "--lang",
            "en",
            "--json",
            "check",
            "tests/fixtures/ai_seedream.jpeg",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"ai_generated\": true"));
}

// --- Nano Pro (C2PA + XMP, HIGH) ---

#[test]
fn nano_pro_detected_via_c2pa() {
    cargo_bin_cmd!("aic")
        .args(["--lang", "en", "check", "tests/fixtures/ai_nano_pro.png"])
        .assert()
        .success()
        .stdout(predicate::str::contains("C2PA"))
        .stdout(predicate::str::contains("trainedAlgorithmicMedia"));
}

#[test]
fn nano_pro_json_output() {
    cargo_bin_cmd!("aic")
        .args([
            "--lang",
            "en",
            "--json",
            "check",
            "tests/fixtures/ai_nano_pro.png",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"ai_generated\": true"));
}

// --- Adobe Firefly (C2PA + XMP, HIGH) ---

#[test]
fn firefly_detected_via_c2pa() {
    cargo_bin_cmd!("aic")
        .args(["--lang", "en", "check", "tests/fixtures/ai_firefly.png"])
        .assert()
        .success()
        .stdout(predicate::str::contains("C2PA"))
        .stdout(predicate::str::contains("firefly"));
}

#[test]
fn firefly_detected_via_xmp() {
    cargo_bin_cmd!("aic")
        .args(["--lang", "en", "check", "tests/fixtures/ai_firefly.png"])
        .assert()
        .success()
        .stdout(predicate::str::contains("XMP"))
        .stdout(predicate::str::contains("trainedAlgorithmicMedia"));
}

#[test]
fn firefly_json_output() {
    cargo_bin_cmd!("aic")
        .args([
            "--lang",
            "en",
            "--json",
            "check",
            "tests/fixtures/ai_firefly.png",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"ai_generated\": true"))
        .stdout(predicate::str::contains("\"confidence\": \"high\""));
}

#[test]
fn firefly_info_shows_c2pa_and_xmp() {
    cargo_bin_cmd!("aic")
        .args(["--lang", "en", "info", "tests/fixtures/ai_firefly.png"])
        .assert()
        .success()
        .stdout(predicate::str::contains("C2PA"))
        .stdout(predicate::str::contains("Adobe_Firefly"))
        .stdout(predicate::str::contains("XMP"))
        .stdout(predicate::str::contains("trainedAlgorithmicMedia"));
}

// --- Deep mode tests (slow, require pixel analysis) ---

#[test]
#[ignore] // slow: deep watermark analysis
fn gptimage_deep_shows_watermark() {
    cargo_bin_cmd!("aic")
        .args([
            "--lang",
            "en",
            "check",
            "--deep",
            "tests/fixtures/ai_gptimage.png",
        ])
        .assert()
        .success();
}

#[test]
#[ignore] // slow: deep watermark analysis
fn seedream_v4_deep_invisible_watermark() {
    cargo_bin_cmd!("aic")
        .args([
            "--lang",
            "en",
            "check",
            "--deep",
            "tests/fixtures/ai_seedream_v4.jpeg",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Invisible watermark"));
}

// --- Info command tests ---

#[test]
fn gptimage_info_shows_c2pa() {
    cargo_bin_cmd!("aic")
        .args(["--lang", "en", "info", "tests/fixtures/ai_gptimage.png"])
        .assert()
        .success()
        .stdout(predicate::str::contains("C2PA"));
}

#[test]
fn midjourney_info_shows_xmp() {
    cargo_bin_cmd!("aic")
        .args(["--lang", "en", "info", "tests/fixtures/ai_midjourney_1.png"])
        .assert()
        .success();
}

#[test]
fn ideogram_info_shows_exif() {
    cargo_bin_cmd!("aic")
        .args(["--lang", "en", "info", "tests/fixtures/ai_ideogram.png"])
        .assert()
        .success();
}
