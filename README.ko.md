> [English](README.md) | [简体中文](README.zh-CN.md) | [Deutsch](README.de.md) | [日本語](README.ja.md) | **한국어** | [हिन्दी](README.hi.md) | [Español](README.es.md)

<div align="center">

# AICheck

**AI 생성 콘텐츠를 감지합니다. 오프라인. API 키 불필요. 설정 불필요.**

[![CI](https://github.com/MatrixA/aicheck/actions/workflows/ci.yml/badge.svg)](https://github.com/MatrixA/aicheck/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/aicheck)](https://crates.io/crates/aicheck)
[![License: AGPL-3.0](https://img.shields.io/badge/license-AGPL--3.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.86%2B-orange.svg)](https://www.rust-lang.org/)

</div>

*화제의 그 이미지 — AI인가 진짜인가?*
*이 영상은 어떤 모델로 생성되었나?*
*이 사진의 메타데이터를 신뢰할 수 있나?*

AICheck는 파일 메타데이터와 보이지 않는 워터마크를 분석하여 이런 질문에 답합니다. API 키 불필요, 네트워크 불필요, 설정 불필요.

**10가지 감지 방법** · **51개 AI 도구** · **16가지 파일 형식** · **3단계 신뢰도** · **네트워크 요청 제로**

---

## ⚡ 빠른 시작

```bash
cargo install aicheck
```

> Rust 1.86+ 필요. 소스에서 빌드: `cargo install --path .`

```bash
aic check photo.jpg
```

```
photo.jpg
  HIGH   C2PA: digitalSourceType = trainedAlgorithmicMedia (fully AI-generated)
  HIGH   C2PA: claim_generator matches AI tool: DALL-E 3/OpenAI [dall-e]
  MEDIUM XMP: AISystemUsed = DALL-E 3 [dall-e]
  Verdict: AI-generated (confidence: HIGH)

real_photo.jpg
  No AI-generation signals detected.
```

---

## 🔍 작동 원리

```
                              파일 입력
                                 |
     +------+------+------+------+------+------+------+
     |      |      |      |      |      |      |      |
     v      v      v      v      v      v      v      v
  [C2PA] [XMP]  [EXIF] [PNG]  [MP4]  [ID3]  [WAV] [파일명]
  HIGH  MEDIUM  LOW    LOW   MEDIUM MEDIUM MEDIUM   LOW
     |      |      |      |      |      |      |      |
     +--+---+--+---+--+---+--+---+--+---+--+---+--+---+
        |                                          |
        v                                          v
  메타데이터 신호 감지?                      신호 없음?
        |                                          |
        v                                          v
   [ 판정 ]               [ 보이지 않는 워터마크 / 오디오 스펙트럼 분석 ]
                            DWT-DCT 또는 FFT 분석
                            신뢰도: LOW
                                       |
                                       v
                                    [ 판정 ]
```

### 감지 방법

**C2PA 매니페스트 (HIGH 신뢰도)** — 암호화 서명된 출처 증명. C2PA 매니페스트에 "DALL-E로 제작"이라고 되어 있다면, 이것이 메타데이터가 제공할 수 있는 가장 권위 있는 신호입니다. `digitalSourceType`과 `claim_generator`를 읽습니다. 이미지, 영상, 오디오(예: ElevenLabs)에 대응.

**XMP/IPTC 메타데이터 (MEDIUM 신뢰도)** — 표준 사진 메타데이터: `DigitalSourceType`, `AISystemUsed`, `AIPromptInformation`, `CreatorTool`. 신뢰할 수 있지만 서명되지 않음 — 위조하거나 제거할 수 있습니다.

**MP4 컨테이너 메타데이터 (MEDIUM 신뢰도)** — iTunes 스타일 아톰(`©too`, `©swr`), AIGC 라벨(중국 표준, JSON `ProduceID` 포함), H.264 SEI 워터마크 마커(예: Kling)를 분석합니다. 다른 방법이 놓치는 비디오 컨테이너에 내장된 AI 신호를 감지합니다.

**ID3 오디오 메타데이터 (MEDIUM 신뢰도)** — MP3 파일의 ID3v2 태그를 읽습니다: 코멘트 프레임(COMM), URL 프레임(WOAS/WOAF/WXXX), 텍스트 프레임(TENC/TPUB/TXXX). Suno 같은 AI 오디오 플랫폼을 감지합니다(내장 URL과 "made with suno" 코멘트를 통해).

**WAV 컨테이너 메타데이터 (MEDIUM/LOW 신뢰도)** — RIFF LIST/INFO 청크(ISFT, ICMT, IART)에서 AI 도구 참조를 분석합니다. TTS 특유의 오디오 특성도 표시합니다: 모노 채널 + 비표준 샘플레이트(16kHz, 22050Hz, 24000Hz).

**EXIF 휴리스틱 (LOW 신뢰도)** — `Software` 태그가 알려진 AI 도구와 일치하고 일반적인 카메라 필드(Make, Model, GPS, 초점 거리)가 없으면 AI 생성일 가능성이 높습니다. 해시 형태의 Artist 태그도 감지합니다.

**PNG 텍스트 청크 (LOW 신뢰도)** — `tEXt`와 `iTXt` 청크에서 Software, Comment, Description, Source, Author, parameters, prompt 키워드의 AI 도구 참조를 스캔합니다.

**파일명 패턴 (LOW 신뢰도)** — 파일명을 알려진 AI 도구의 명명 규칙과 대조합니다(예: ElevenLabs 타임스탬프 형식 `ElevenLabs_YYYY-MM-DDTHH_MM_SS_*`, Suno/SoundRaw 접두사, 파일명의 Midjourney/DALL-E).

**오디오 스펙트럼 분석 (LOW 신뢰도)** — FFT 기반 WAV 오디오 분석: 하드 주파수 컷오프(나이퀴스트 이하에 에너지 집중)와 TTS/AI 합성에 전형적인 비정상적 스펙트럼 평탄도를 감지합니다. 폴백으로 또는 `--deep`으로 실행됩니다.

**보이지 않는 워터마크 (LOW 신뢰도)** — 픽셀 수준의 DWT-DCT 분석으로 채널 노이즈 비대칭, 채널 간 비트 일치, 웨이블릿 에너지 패턴을 감지합니다. 메타데이터 신호가 발견되지 않으면 자동으로 폴백 실행되거나, `--deep`으로 요청 시 실행됩니다.

---

## 🎯 인식 대상

### AI 도구

| 카테고리 | 도구 |
|---------|------|
| 이미지 생성 | DALL-E, Midjourney, Stable Diffusion, Adobe Firefly, Imagen, Flux, Ideogram, Leonardo.ai, NovelAI |
| 영상 생성 | Sora, Google Veo, Runway, Pika, Kling, Vidu |
| 오디오/음악 생성 | Suno, Udio, ElevenLabs, SoundRaw, AIVA, Boomy, Mubert, Beatoven, Soundful |
| 멀티모달 | GPT-4o, GPT-4, ChatGPT, OpenAI, GPT Image |
| 플랫폼 | Bing Image Creator, Copilot Designer, Microsoft Designer, Canva AI, DreamStudio, NightCafe, Craiyon, DeepAI, Meta AI, Stability AI |
| 인터페이스 | ComfyUI, Automatic1111 (A1111), InvokeAI, Fooocus |
| 연구 | Glide, Parti, Muse, Seedream, Recraft |

### 파일 형식

| 유형 | 형식 |
|------|------|
| 이미지 | JPEG, PNG, WebP, AVIF, HEIF, TIFF, GIF, BMP |
| 영상 | MP4, MOV, AVI, WebM |
| 오디오 | MP3, M4A, WAV |
| 문서 | PDF |

---

## 💻 명령어

### `aic check [경로]`

파일의 AI 생성 신호를 분석합니다.

```bash
aic check photo.jpg                       # 단일 파일
aic check images/ -r                      # 디렉토리, 재귀적
aic check photo.jpg --json                # JSON 출력
aic check photo.jpg -q                    # 조용히 — 종료 코드만
aic check photo.jpg --min-confidence medium  # 신뢰도로 필터
aic check photo.jpg --deep                # 픽셀 수준 워터마크 분석 강제
```

### `aic info <파일>`

모든 출처 메타데이터를 출력합니다(C2PA 매니페스트, XMP 속성, EXIF 필드, MP4 아톰, ID3 태그, WAV 메타데이터, 워터마크 분석).

```bash
aic info photo.jpg
```

### 글로벌 옵션

| 옵션 | 효과 |
|------|------|
| `--json` | JSON 형식으로 출력 |
| `-q, --quiet` | 출력 억제, 종료 코드만 설정 |
| `--deep` | 모든 파일에 보이지 않는 워터마크 및 오디오 스펙트럼 분석 강제 |
| `--no-color` | 색상 출력 비활성화 |

### 종료 코드

| 코드 | 의미 |
|------|------|
| `0` | AI 신호 감지됨 |
| `1` | AI 신호 없음 |
| `2` | 오류 |

---

## ⚠️ 제한 사항

- **메타데이터 제거 = 감지 불가.** 누군가 메타데이터를 제거하면 감지할 것이 없습니다. 소셜 플랫폼은 업로드 시 이 작업을 수행합니다 — 항상 원본 파일을 분석하세요.
- **대부분의 AI 이미지에는 워터마크가 없습니다.** 감지 가능한 출처 마커가 있는 AI 이미지는 약 19%에 불과합니다(2025년 데이터).
- **독점 워터마크는 접근 불가.** SynthID, Stable Signature, VideoSeal에는 우리가 보유하지 않은 키가 필요합니다.
- **픽셀 수준 분석에는 한계가 있습니다.** 내장 DWT-DCT 워터마크 감지기는 일반적인 패턴을 포착하지만 완전한 포렌식 분류기는 아닙니다. 심층 통계적 감지에는 전용 포렌식 도구를 사용하세요.

---

## 📄 라이선스

[AGPL-3.0](LICENSE)
