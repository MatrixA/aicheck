use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn suno_mp3_detected_as_ai() {
    Command::cargo_bin("aic")
        .unwrap()
        .args(["check", "tests/fixtures/ai_suno.mp3"])
        .assert()
        .success() // exit 0 = AI detected
        .stdout(predicate::str::contains("suno"))
        .stdout(predicate::str::contains("ID3"));
}

#[test]
fn suno_mp3_json_output() {
    Command::cargo_bin("aic")
        .unwrap()
        .args(["--json", "check", "tests/fixtures/ai_suno.mp3"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"ai_generated\": true"))
        .stdout(predicate::str::contains("suno"));
}

#[test]
fn suno_mp3_info_shows_id3_tags() {
    Command::cargo_bin("aic")
        .unwrap()
        .args(["info", "tests/fixtures/ai_suno.mp3"])
        .assert()
        .success()
        .stdout(predicate::str::contains("ID3 Tags"))
        .stdout(predicate::str::contains("WOAS"))
        .stdout(predicate::str::contains("COMM"));
}
