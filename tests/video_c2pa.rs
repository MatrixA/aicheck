use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn mp4_with_c2pa_is_detected() {
    Command::cargo_bin("aic")
        .unwrap()
        .args(["check", "tests/fixtures/ai_c2pa.mp4"])
        .assert()
        .success() // exit 0 = AI detected
        .stdout(predicate::str::contains("C2PA"))
        .stdout(predicate::str::contains("algorithmicMedia"));
}

#[test]
fn mp4_with_c2pa_json_output() {
    Command::cargo_bin("aic")
        .unwrap()
        .args(["--json", "check", "tests/fixtures/ai_c2pa.mp4"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"ai_generated\": true"))
        .stdout(predicate::str::contains("algorithmicMedia"));
}

#[test]
fn mp4_without_c2pa_not_detected() {
    Command::cargo_bin("aic")
        .unwrap()
        .args(["check", "tests/fixtures/no_c2pa.mp4"])
        .assert()
        .code(1) // exit 1 = no AI detected
        .stdout(predicate::str::contains("No AI-generation signals detected"));
}

#[test]
fn mp4_info_shows_c2pa_manifest() {
    Command::cargo_bin("aic")
        .unwrap()
        .args(["info", "tests/fixtures/ai_c2pa.mp4"])
        .assert()
        .success();
}
