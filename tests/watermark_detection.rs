use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn watermarked_dwtdct_detected_with_deep() {
    Command::cargo_bin("aic")
        .unwrap()
        .args(["check", "--deep", "tests/fixtures/watermarked_dwtdct.png"])
        .assert()
        .success() // exit 0 = AI detected
        .stdout(predicate::str::contains("WATERMARK"))
        .stdout(predicate::str::contains("Invisible watermark"));
}

#[test]
fn watermarked_dwtdctsvd_detected_with_deep() {
    Command::cargo_bin("aic")
        .unwrap()
        .args(["check", "--deep", "tests/fixtures/watermarked_dwtdctsvd.png"])
        .assert()
        .success()
        .stdout(predicate::str::contains("WATERMARK"));
}

#[test]
fn clean_image_not_detected_with_deep() {
    Command::cargo_bin("aic")
        .unwrap()
        .args(["check", "--deep", "tests/fixtures/clean_synthetic.png"])
        .assert()
        .code(1) // exit 1 = no AI detected
        .stdout(predicate::str::contains("No AI-generation signals detected"));
}

#[test]
fn watermark_not_detected_without_deep() {
    // Without --deep, watermark analysis should not run
    Command::cargo_bin("aic")
        .unwrap()
        .args(["check", "tests/fixtures/watermarked_dwtdct.png"])
        .assert()
        .code(1)
        .stdout(predicate::str::contains("No AI-generation signals detected"));
}

#[test]
fn watermarked_json_output() {
    Command::cargo_bin("aic")
        .unwrap()
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
    Command::cargo_bin("aic")
        .unwrap()
        .args(["info", "tests/fixtures/watermarked_dwtdct.png"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Watermark Analysis"));
}
