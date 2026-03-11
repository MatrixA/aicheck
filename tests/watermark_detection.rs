use assert_cmd::cargo_bin_cmd;
use predicates::prelude::*;

#[test]
fn watermarked_dwtdct_detected_with_deep() {
    cargo_bin_cmd!("aic")
        .args([
            "--lang",
            "en",
            "check",
            "--deep",
            "tests/fixtures/watermarked_dwtdct.png",
        ])
        .assert()
        .success() // exit 0 = AI detected
        .stdout(predicate::str::contains("WATERMARK"))
        .stdout(predicate::str::contains("Invisible watermark"));
}

#[test]
fn watermarked_dwtdctsvd_detected_with_deep() {
    cargo_bin_cmd!("aic")
        .args([
            "check",
            "--deep",
            "tests/fixtures/watermarked_dwtdctsvd.png",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("WATERMARK"));
}

#[test]
fn clean_image_not_detected_with_deep() {
    cargo_bin_cmd!("aic")
        .args([
            "--lang",
            "en",
            "check",
            "--deep",
            "tests/fixtures/clean_synthetic.png",
        ])
        .assert()
        .code(1) // exit 1 = no AI detected
        .stdout(predicate::str::contains(
            "No AI-generation signals detected",
        ));
}

#[test]
fn watermark_auto_fallback_when_no_metadata() {
    // Without --deep, watermark analysis runs as fallback when no metadata signals found
    cargo_bin_cmd!("aic")
        .args(["check", "tests/fixtures/watermarked_dwtdct.png"])
        .assert()
        .success()
        .stdout(predicate::str::contains("WATERMARK"));
}

#[test]
fn watermark_skipped_when_metadata_found() {
    // Watermark should not auto-run when metadata signals already exist
    cargo_bin_cmd!("aic")
        .args(["check", "tests/fixtures/ai_xmp.jpg"])
        .assert()
        .success()
        .stdout(predicate::str::contains("XMP"))
        .stdout(predicate::str::contains("WATERMARK").not());
}

#[test]
fn watermarked_json_output() {
    cargo_bin_cmd!("aic")
        .args([
            "--json",
            "check",
            "--deep",
            "tests/fixtures/watermarked_dwtdct.png",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"ai_generated\": true"))
        .stdout(predicate::str::contains("\"watermark\""));
}

#[test]
fn watermark_info_command() {
    cargo_bin_cmd!("aic")
        .args([
            "--lang",
            "en",
            "info",
            "tests/fixtures/watermarked_dwtdct.png",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Watermark Analysis"));
}
