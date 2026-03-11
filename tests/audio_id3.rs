use assert_cmd::cargo_bin_cmd;
use predicates::prelude::*;

#[test]
fn suno_mp3_detected_as_ai() {
    cargo_bin_cmd!("aic")
        .args(["check", "tests/fixtures/ai_suno.mp3"])
        .assert()
        .success() // exit 0 = AI detected
        .stdout(predicate::str::contains("suno"))
        .stdout(predicate::str::contains("ID3"));
}

#[test]
fn suno_mp3_json_output() {
    cargo_bin_cmd!("aic")
        .args(["--json", "check", "tests/fixtures/ai_suno.mp3"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"ai_generated\": true"))
        .stdout(predicate::str::contains("suno"));
}

#[test]
fn suno_mp3_info_shows_id3_tags() {
    cargo_bin_cmd!("aic")
        .args(["info", "tests/fixtures/ai_suno.mp3"])
        .assert()
        .success()
        .stdout(predicate::str::contains("ID3 Tags"))
        .stdout(predicate::str::contains("WOAS"))
        .stdout(predicate::str::contains("COMM"));
}
