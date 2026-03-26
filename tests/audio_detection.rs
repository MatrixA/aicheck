use assert_cmd::cargo_bin_cmd;
use predicates::prelude::*;

// --- Suno 2 (ID3 + Filename, MEDIUM) ---

#[test]
fn suno_2_detected_via_id3() {
    cargo_bin_cmd!("aic")
        .args(["--lang", "en", "check", "tests/fixtures/ai_suno_2.mp3"])
        .assert()
        .success()
        .stdout(predicate::str::contains("ID3"))
        .stdout(predicate::str::contains("suno"));
}

#[test]
fn suno_2_json_output() {
    cargo_bin_cmd!("aic")
        .args([
            "--lang",
            "en",
            "--json",
            "check",
            "tests/fixtures/ai_suno_2.mp3",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"ai_generated\": true"))
        .stdout(predicate::str::contains("suno"));
}

#[test]
fn suno_2_info_shows_id3_tags() {
    cargo_bin_cmd!("aic")
        .args(["--lang", "en", "info", "tests/fixtures/ai_suno_2.mp3"])
        .assert()
        .success()
        .stdout(predicate::str::contains("ID3"));
}

// --- ElevenLabs (C2PA, HIGH) ---

#[test]
fn elevenlabs_detected_via_c2pa() {
    cargo_bin_cmd!("aic")
        .args(["--lang", "en", "check", "tests/fixtures/ai_elevenlabs.mp3"])
        .assert()
        .success()
        .stdout(predicate::str::contains("C2PA"))
        .stdout(predicate::str::contains("elevenlabs"));
}

#[test]
fn elevenlabs_json_output() {
    cargo_bin_cmd!("aic")
        .args([
            "--lang",
            "en",
            "--json",
            "check",
            "tests/fixtures/ai_elevenlabs.mp3",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"ai_generated\": true"));
}

#[test]
fn elevenlabs_info_shows_c2pa() {
    cargo_bin_cmd!("aic")
        .args(["--lang", "en", "info", "tests/fixtures/ai_elevenlabs.mp3"])
        .assert()
        .success()
        .stdout(predicate::str::contains("C2PA"));
}
