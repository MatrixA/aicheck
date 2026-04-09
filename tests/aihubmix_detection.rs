use assert_cmd::cargo_bin_cmd;
use predicates::prelude::*;

// =============================================================================
// Images — C2PA (HIGH): Google Imagen
// =============================================================================

#[test]
fn google_imagen_4_detected_via_c2pa() {
    cargo_bin_cmd!("aic")
        .args([
            "--lang",
            "en",
            "check",
            "tests/fixtures/image_google-imagen-4.0-generate-001.png",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("C2PA"))
        .stdout(predicate::str::contains("google ai"));
}

#[test]
fn google_imagen_3_detected_via_c2pa() {
    cargo_bin_cmd!("aic")
        .args([
            "--lang",
            "en",
            "check",
            "tests/fixtures/image_google-imagen-3.0-generate-002.png",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("C2PA"))
        .stdout(predicate::str::contains("google ai"));
}

#[test]
fn google_imagen_4_ultra_detected_via_c2pa() {
    cargo_bin_cmd!("aic")
        .args([
            "--lang",
            "en",
            "check",
            "tests/fixtures/image_google-imagen-4.0-ultra-generate-001.png",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("C2PA"));
}

#[test]
fn google_imagen_4_fast_detected_via_c2pa() {
    cargo_bin_cmd!("aic")
        .args([
            "--lang",
            "en",
            "check",
            "tests/fixtures/image_google-imagen-4.0-fast-generate-001.png",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("C2PA"));
}

// =============================================================================
// Images — C2PA (HIGH): Gemini
// =============================================================================

#[test]
fn gemini_2_5_flash_detected_via_c2pa() {
    cargo_bin_cmd!("aic")
        .args([
            "--lang",
            "en",
            "check",
            "tests/fixtures/image_gemini-2.5-flash-image.png",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("C2PA"))
        .stdout(predicate::str::contains("google ai"));
}

#[test]
fn gemini_3_pro_detected_via_c2pa() {
    cargo_bin_cmd!("aic")
        .args([
            "--lang",
            "en",
            "check",
            "tests/fixtures/image_gemini-3-pro-image-preview.jpg",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("C2PA"))
        .stdout(predicate::str::contains("google ai"));
}

#[test]
fn gemini_3_1_flash_detected_via_c2pa() {
    cargo_bin_cmd!("aic")
        .args([
            "--lang",
            "en",
            "check",
            "tests/fixtures/image_gemini-3.1-flash-image-preview.jpg",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("C2PA"));
}

#[test]
fn gemini_3_1_flash_free_detected_via_c2pa() {
    cargo_bin_cmd!("aic")
        .args([
            "--lang",
            "en",
            "check",
            "tests/fixtures/image_gemini-3.1-flash-image-preview-free.png",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("C2PA"))
        .stdout(predicate::str::contains("google ai"));
}

#[test]
fn gemini_2_0_flash_detected_via_c2pa() {
    cargo_bin_cmd!("aic")
        .args([
            "--lang",
            "en",
            "check",
            "tests/fixtures/image_gemini-2.0-flash-preview-image-generation.png",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("C2PA"));
}

// =============================================================================
// Images — C2PA (HIGH): OpenAI
// =============================================================================

#[test]
fn openai_gpt_image_1_detected_via_c2pa() {
    cargo_bin_cmd!("aic")
        .args([
            "--lang",
            "en",
            "check",
            "tests/fixtures/image_openai-gpt-image-1.png",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("C2PA"))
        .stdout(predicate::str::contains("openai"));
}

#[test]
fn openai_gpt_image_1_5_detected_via_c2pa() {
    cargo_bin_cmd!("aic")
        .args([
            "--lang",
            "en",
            "check",
            "tests/fixtures/image_openai-gpt-image-1.5.png",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("C2PA"))
        .stdout(predicate::str::contains("openai"));
}

#[test]
fn openai_gpt_image_1_mini_detected_via_c2pa() {
    cargo_bin_cmd!("aic")
        .args([
            "--lang",
            "en",
            "check",
            "tests/fixtures/image_openai-gpt-image-1-mini.png",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("C2PA"))
        .stdout(predicate::str::contains("openai"));
}

#[test]
fn openai_dall_e_3_detected_via_c2pa() {
    cargo_bin_cmd!("aic")
        .args([
            "--lang",
            "en",
            "check",
            "tests/fixtures/image_openai-dall-e-3.png",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("C2PA"))
        .stdout(predicate::str::contains("openai"));
}

#[test]
fn openai_dall_e_2_detected_via_filename() {
    cargo_bin_cmd!("aic")
        .args([
            "--lang",
            "en",
            "check",
            "tests/fixtures/image_openai-dall-e-2.png",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("dall-e"));
}

#[test]
fn gpt_4o_image_detected_via_c2pa() {
    cargo_bin_cmd!("aic")
        .args([
            "--lang",
            "en",
            "check",
            "tests/fixtures/image_gpt-4o-image.png",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("C2PA"));
}

#[test]
fn gpt_4o_image_vip_detected_via_c2pa() {
    cargo_bin_cmd!("aic")
        .args([
            "--lang",
            "en",
            "check",
            "tests/fixtures/image_gpt-4o-image-vip.png",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("C2PA"));
}

#[test]
fn web_gpt_image_detected_via_c2pa() {
    cargo_bin_cmd!("aic")
        .args([
            "--lang",
            "en",
            "check",
            "tests/fixtures/image_web-gpt-image-1.5.png",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("C2PA"));
}

// =============================================================================
// Images — C2PA (HIGH): BFL Flux
// =============================================================================

#[test]
fn flux_2_pro_detected_via_c2pa() {
    cargo_bin_cmd!("aic")
        .args([
            "--lang",
            "en",
            "check",
            "tests/fixtures/image_bfl-flux-2-pro.jpg",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("C2PA"))
        .stdout(predicate::str::contains("flux"));
}

#[test]
fn flux_2_flex_detected_via_c2pa() {
    cargo_bin_cmd!("aic")
        .args([
            "--lang",
            "en",
            "check",
            "tests/fixtures/image_bfl-flux-2-flex.jpg",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("C2PA"))
        .stdout(predicate::str::contains("flux"));
}

// =============================================================================
// Images — EXIF: Ideogram
// =============================================================================

#[test]
fn ideogram_v3_detected_via_exif() {
    cargo_bin_cmd!("aic")
        .args([
            "--lang",
            "en",
            "check",
            "tests/fixtures/image_ideogram-v3.png",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("ideogram"));
}

#[test]
fn ideogram_v2a_turbo_detected_via_exif() {
    cargo_bin_cmd!("aic")
        .args([
            "--lang",
            "en",
            "check",
            "tests/fixtures/image_ideogram-v_2a_turbo.png",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("ideogram"));
}

// =============================================================================
// Images — EXIF AIGC: Qwen
// =============================================================================

#[test]
fn qwen_image_detected_via_exif_aigc() {
    cargo_bin_cmd!("aic")
        .args([
            "--lang",
            "en",
            "check",
            "tests/fixtures/image_qianfan-qwen-image.jpg",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("AIGC"))
        .stdout(predicate::str::contains("qwen"));
}

#[test]
fn qwen_image_edit_detected_via_exif_aigc() {
    cargo_bin_cmd!("aic")
        .args([
            "--lang",
            "en",
            "check",
            "tests/fixtures/image_qianfan-qwen-image-edit.jpg",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("AIGC"))
        .stdout(predicate::str::contains("qwen"));
}

// =============================================================================
// Images — Watermark (MEDIUM): Doubao Seedream
// =============================================================================

#[test]
fn seedream_4_5_detected_via_watermark() {
    cargo_bin_cmd!("aic")
        .args([
            "--lang",
            "en",
            "check",
            "tests/fixtures/image_doubao-doubao-seedream-4-5.jpg",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("watermark").or(predicate::str::contains("Watermark")));
}

#[test]
fn seedream_5_lite_detected_via_watermark() {
    cargo_bin_cmd!("aic")
        .args([
            "--lang",
            "en",
            "check",
            "tests/fixtures/image_doubao-doubao-seedream-5.0-lite.jpg",
        ])
        .assert()
        .success();
}

// =============================================================================
// Images — Watermark (MEDIUM): BFL Flux 1.1
// =============================================================================

#[test]
fn flux_1_1_pro_detected_via_watermark() {
    cargo_bin_cmd!("aic")
        .args([
            "--lang",
            "en",
            "check",
            "tests/fixtures/image_bfl-flux-1.1-pro.jpg",
        ])
        .assert()
        .success();
}

// =============================================================================
// Videos — C2PA: Sora
// =============================================================================

#[test]
fn sora_2_detected_via_c2pa() {
    cargo_bin_cmd!("aic")
        .args(["--lang", "en", "check", "tests/fixtures/video_sora-2.mp4"])
        .assert()
        .success()
        .stdout(predicate::str::contains("C2PA"))
        .stdout(predicate::str::contains("sora"));
}

#[test]
fn sora_2_pro_detected() {
    cargo_bin_cmd!("aic")
        .args([
            "--lang",
            "en",
            "check",
            "tests/fixtures/video_sora-2-pro.mp4",
        ])
        .assert()
        .success();
}

// =============================================================================
// Videos — MP4 metadata: Google Veo
// =============================================================================

#[test]
fn veo_3_detected_via_mp4_metadata() {
    cargo_bin_cmd!("aic")
        .args(["--lang", "en", "check", "tests/fixtures/video_veo-3.mp4"])
        .assert()
        .success()
        .stdout(predicate::str::contains("google veo"));
}

#[test]
fn veo_2_detected_via_mp4_metadata() {
    cargo_bin_cmd!("aic")
        .args([
            "--lang",
            "en",
            "check",
            "tests/fixtures/video_veo-2.0-generate-001.mp4",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("google veo"));
}

#[test]
fn veo_3_1_detected_via_mp4_metadata() {
    cargo_bin_cmd!("aic")
        .args(["--lang", "en", "check", "tests/fixtures/video_veo3.1.mp4"])
        .assert()
        .success()
        .stdout(predicate::str::contains("google veo"));
}

// =============================================================================
// Videos — AIGC label: Wan (tool=wan)
// =============================================================================

#[test]
fn wan_2_6_t2v_detected_via_aigc() {
    cargo_bin_cmd!("aic")
        .args([
            "--lang",
            "en",
            "check",
            "tests/fixtures/video_wan2.6-t2v.mp4",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("AIGC"))
        .stdout(predicate::str::contains("wan"));
}

#[test]
fn wan_2_7_t2v_detected_via_aigc() {
    cargo_bin_cmd!("aic")
        .args([
            "--lang",
            "en",
            "check",
            "tests/fixtures/video_wan2.7-t2v.mp4",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("AIGC"))
        .stdout(predicate::str::contains("wan"));
}

#[test]
fn wan_2_2_i2v_detected_via_aigc() {
    cargo_bin_cmd!("aic")
        .args([
            "--lang",
            "en",
            "check",
            "tests/fixtures/video_wan2.2-i2v-plus.mp4",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("AIGC"));
}

#[test]
fn wan_2_5_t2v_detected_via_aigc() {
    cargo_bin_cmd!("aic")
        .args([
            "--lang",
            "en",
            "check",
            "tests/fixtures/video_wan2.5-t2v-preview.mp4",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("AIGC"));
}

// =============================================================================
// Videos — Watermark: Doubao Seedance
// =============================================================================

#[test]
fn seedance_2_0_detected_via_watermark() {
    cargo_bin_cmd!("aic")
        .args([
            "--lang",
            "en",
            "check",
            "tests/fixtures/video_doubao-seedance-2-0-260128.mp4",
        ])
        .assert()
        .success();
}

#[test]
fn seedance_1_0_pro_detected_via_watermark() {
    cargo_bin_cmd!("aic")
        .args([
            "--lang",
            "en",
            "check",
            "tests/fixtures/video_doubao-seedance-1-0-pro-250528.mp4",
        ])
        .assert()
        .success();
}

// =============================================================================
// Videos — Watermark: Jimeng
// =============================================================================

#[test]
fn jimeng_3_pro_detected_via_watermark() {
    cargo_bin_cmd!("aic")
        .args([
            "--lang",
            "en",
            "check",
            "tests/fixtures/video_jimeng-3.0-pro.mp4",
        ])
        .assert()
        .success();
}

#[test]
fn jimeng_3_1080p_detected_via_watermark() {
    cargo_bin_cmd!("aic")
        .args([
            "--lang",
            "en",
            "check",
            "tests/fixtures/video_jimeng-3.0-1080p.mp4",
        ])
        .assert()
        .success();
}

// =============================================================================
// Audio — WAV TTS heuristic
// =============================================================================

#[test]
fn gemini_tts_detected_via_wav_heuristic() {
    cargo_bin_cmd!("aic")
        .args([
            "--lang",
            "en",
            "check",
            "tests/fixtures/audio_gemini-2.5-flash-preview-tts.wav",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("TTS"));
}

#[test]
fn gemini_pro_tts_detected_via_wav_heuristic() {
    cargo_bin_cmd!("aic")
        .args([
            "--lang",
            "en",
            "check",
            "tests/fixtures/audio_gemini-2.5-pro-preview-tts.wav",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("TTS"));
}

#[test]
fn gpt_4o_audio_preview_detected_via_wav_heuristic() {
    cargo_bin_cmd!("aic")
        .args([
            "--lang",
            "en",
            "check",
            "tests/fixtures/audio_gpt-4o-audio-preview.wav",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("TTS"));
}

// =============================================================================
// Audio — Not detected (OpenAI TTS MP3 — known blind spot)
// =============================================================================

#[test]
fn openai_tts_1_not_detected() {
    cargo_bin_cmd!("aic")
        .args(["--lang", "en", "check", "tests/fixtures/audio_tts-1.mp3"])
        .assert()
        .code(1)
        .stdout(predicate::str::contains(
            "No AI-generation signals detected",
        ));
}

#[test]
fn openai_gpt_4o_mini_tts_not_detected() {
    cargo_bin_cmd!("aic")
        .args([
            "--lang",
            "en",
            "check",
            "tests/fixtures/audio_gpt-4o-mini-tts.mp3",
        ])
        .assert()
        .code(1)
        .stdout(predicate::str::contains(
            "No AI-generation signals detected",
        ));
}

// =============================================================================
// JSON output — representative batch
// =============================================================================

#[test]
fn google_imagen_json_output() {
    cargo_bin_cmd!("aic")
        .args([
            "--lang",
            "en",
            "--json",
            "check",
            "tests/fixtures/image_google-imagen-4.0-generate-001.png",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"ai_generated\": true"))
        .stdout(predicate::str::contains("\"confidence\": \"high\""))
        .stdout(predicate::str::contains("google ai"));
}

#[test]
fn wan_video_json_output() {
    cargo_bin_cmd!("aic")
        .args([
            "--lang",
            "en",
            "--json",
            "check",
            "tests/fixtures/video_wan2.6-t2v.mp4",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"ai_generated\": true"))
        .stdout(predicate::str::contains("wan"));
}

#[test]
fn qwen_image_json_output() {
    cargo_bin_cmd!("aic")
        .args([
            "--lang",
            "en",
            "--json",
            "check",
            "tests/fixtures/image_qianfan-qwen-image.jpg",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"ai_generated\": true"))
        .stdout(predicate::str::contains("qwen"));
}

#[test]
fn veo_video_json_output() {
    cargo_bin_cmd!("aic")
        .args([
            "--lang",
            "en",
            "--json",
            "check",
            "tests/fixtures/video_veo-3.mp4",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"ai_generated\": true"))
        .stdout(predicate::str::contains("google veo"));
}

#[test]
fn ideogram_json_output_aihubmix() {
    cargo_bin_cmd!("aic")
        .args([
            "--lang",
            "en",
            "--json",
            "check",
            "tests/fixtures/image_ideogram-v3.png",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"ai_generated\": true"))
        .stdout(predicate::str::contains("ideogram"));
}
