use assert_cmd::cargo_bin_cmd;
use predicates::prelude::*;

#[test]
fn mp4_with_c2pa_is_detected() {
    cargo_bin_cmd!("aic")
        .args(["check", "tests/fixtures/ai_c2pa.mp4"])
        .assert()
        .success() // exit 0 = AI detected
        .stdout(predicate::str::contains("C2PA"))
        .stdout(predicate::str::contains("algorithmicMedia"));
}

#[test]
fn mp4_with_c2pa_json_output() {
    cargo_bin_cmd!("aic")
        .args(["--json", "check", "tests/fixtures/ai_c2pa.mp4"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"ai_generated\": true"))
        .stdout(predicate::str::contains("algorithmicMedia"));
}

#[test]
fn mp4_without_c2pa_not_detected() {
    cargo_bin_cmd!("aic")
        .args(["--lang", "en", "check", "tests/fixtures/no_c2pa.mp4"])
        .assert()
        .code(1) // exit 1 = no AI detected
        .stdout(predicate::str::contains(
            "No AI-generation signals detected",
        ));
}

#[test]
fn mp4_info_shows_c2pa_manifest() {
    cargo_bin_cmd!("aic")
        .args(["info", "tests/fixtures/ai_c2pa.mp4"])
        .assert()
        .success();
}
