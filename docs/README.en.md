> **English** | [简体中文](../README.md) | [Deutsch](README.de.md) | [日本語](README.ja.md) | [한국어](README.ko.md) | [हिन्दी](README.hi.md) | [Español](README.es.md)

<div align="center">

# AICheck

**Detect AI-generated content. Offline. No API keys. No setup.**

[![CI](https://github.com/MatrixA/aicheck/actions/workflows/ci.yml/badge.svg)](https://github.com/MatrixA/aicheck/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/aicheck)](https://crates.io/crates/aicheck)
[![License: AGPL-3.0](https://img.shields.io/badge/license-AGPL--3.0-blue.svg)](../LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.86%2B-orange.svg)](https://www.rust-lang.org/)

</div>

*That viral image — AI or real?*
*Which model generated this video?*
*Can you trust this photo's metadata?*

AICheck answers these questions by analyzing file metadata and invisible watermarks. No API keys, no network, no setup.

**11 detection methods** · **62 AI tools** · **16 file formats** · **3 confidence tiers** · **Zero network requests**

![Demo](demo-en.gif)

> Full video: [YouTube](https://youtu.be/1u-6TkHtWiA) | [Bilibili](https://www.bilibili.com/video/BV16Mc6zAE1s)

---

## ⚡ Quick Start

```bash
cargo install aicheck
```

> Requires [Rust 1.86+](https://rust-lang.org/tools/install/). Or build from source: `cargo install --path .`

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

## 🔍 How It Works

```
                              your file
                                 |
     +------+------+------+------+------+------+------+
     |      |      |      |      |      |      |      |
     v      v      v      v      v      v      v      v
  [C2PA] [XMP]  [EXIF] [PNG]  [MP4]  [ID3]  [WAV]  [FILE]
  HIGH  MEDIUM  LOW    LOW   MEDIUM MEDIUM MEDIUM   LOW
     |      |      |      |      |      |      |      |
     +--+---+--+---+--+---+--+---+--+---+--+---+--+---+
        |                                          |
        v                                          v
  metadata signals found?                    no signals?
        |                                          |
        v                                          v
   [ Verdict ]        [ Invisible / Visible Watermark / Audio Spectral ]
                        DWT-DCT / luminance / FFT analysis
                        confidence: LOW–MEDIUM
                                       |
                                       v
                                  [ Verdict ]
```

### Detection Methods

**C2PA Manifests (HIGH confidence)** — Cryptographically signed provenance. If a C2PA manifest says "made by DALL-E," that's the most authoritative signal metadata can provide. Reads `digitalSourceType`, `claim_generator`, and `claim_generator_info`. Can infer specific AI tools from vendor identifiers in the claim generator (e.g. Google → Google AI). Works on images, videos, and audio (e.g. ElevenLabs).

**XMP/IPTC Metadata (MEDIUM confidence)** — Standard photo metadata: `DigitalSourceType`, `AISystemUsed`, `AIPromptInformation`, `CreatorTool`, `Credit` (e.g. Google AI's `photoshop:Credit`). Reliable but unsigned — can be faked or stripped.

**MP4 Container Metadata (MEDIUM confidence)** — Parses iTunes-style atoms (`©too`, `©swr`), AIGC labels (China standard with JSON `ProduceID` and `ContentProducer` enterprise ID → tool mapping, e.g. Wan videos), and H.264 SEI watermark markers (Kling, Sora, Runway, Pika, Luma, Hailuo, Pixverse, Vidu, Genmo, Haiper). Also detects non-AI creation software (FFmpeg, Remotion, Premiere, etc.) for informational display. Catches AI signals baked into video containers that other methods miss.

**ID3 Audio Metadata (MEDIUM confidence)** — Reads ID3v2 tags from MP3 files: comment frames (COMM), URL frames (WOAS/WOAF/WXXX), and text frames (TENC/TPUB/TXXX). Detects AI audio platforms like Suno (via embedded URLs and "made with suno" comments).

**WAV Container Metadata (MEDIUM/LOW confidence)** — Parses RIFF LIST/INFO chunks (ISFT, ICMT, IART) for AI tool references. Also flags TTS-typical audio characteristics: mono channel + non-standard sample rates (16kHz, 22050Hz, 24000Hz).

**EXIF Heuristics (LOW–MEDIUM confidence)** — If the `Software` tag matches a known AI tool AND typical camera fields (Make, Model, GPS, focal length) are absent, it's likely AI-generated. Also detects hash-like Artist tags. Additionally, parses AIGC JSON labels embedded in `UserComment` (e.g. Qianfan Qwen images), mapping `ContentProducer` enterprise IDs to specific tools (MEDIUM confidence).

**PNG Text Chunks (LOW confidence)** — Scans `tEXt` and `iTXt` chunks for AI tool references in Software, Comment, Description, Source, Author, parameters, and prompt keywords.

**Filename Patterns (LOW confidence)** — Matches filenames against known AI tool naming conventions (e.g. ElevenLabs timestamp format `ElevenLabs_YYYY-MM-DDTHH_MM_SS_*`, Suno/SoundRaw prefixes, Midjourney/DALL-E in filenames).

**Audio Spectral Analysis (LOW confidence)** — FFT-based analysis of WAV audio: detects hard frequency cutoffs (energy concentrated below Nyquist) and abnormal spectral flatness typical of TTS/AI synthesis. Runs as a fallback or with `--deep`.

**Invisible Watermarks (LOW confidence)** — Pixel-level DWT-DCT analysis that detects channel noise asymmetry, cross-channel bit agreement, and wavelet energy patterns. For videos, automatically extracts keyframes via `ffmpeg` and analyzes them individually. Runs automatically as a fallback when no metadata signals are found, or on demand with `--deep`.

**Visible Watermarks (MEDIUM confidence)** — Detects visible text overlays in image corner regions (e.g. Chinese AI-generated content disclosure labels). Uses luminance analysis and text-line pattern detection to identify small text badges in corners. Runs alongside invisible watermark detection, images only.

---

## 🎯 What It Recognizes

### AI Tools

| Category | Tools |
|----------|-------|
| Image generation | DALL-E, Midjourney, Stable Diffusion, Adobe Firefly, Imagen, Flux, Ideogram, Leonardo.ai, NovelAI, Grok, Jimeng (即梦), Qwen (通义万相) |
| Video generation | Sora, Google Veo, Runway, Pika, Kling, Vidu, Luma, Hailuo (海螺), Pixverse, Genmo, Haiper, Wan |
| Audio/Music generation | Suno, Udio, ElevenLabs, SoundRaw, AIVA, Boomy, Mubert, Loudly, Beatoven, Soundful, Hume, Fish Audio |
| Multimodal | GPT-4o, GPT-4, ChatGPT, OpenAI, GPT Image, Gemini, Google AI |
| Platforms | Bing Image Creator, Copilot Designer, Microsoft Designer, Canva AI, DreamStudio, NightCafe, Craiyon, DeepAI, Meta AI, Stability AI |
| Interfaces | ComfyUI, Automatic1111 (A1111), InvokeAI, Fooocus |
| Research | Glide, Parti, Muse, Seedream, Recraft |

### File Formats

| Type | Formats |
|------|---------|
| Image | JPEG, PNG, WebP, AVIF, HEIF, TIFF, GIF, BMP |
| Video | MP4, MOV, AVI, WebM |
| Audio | MP3, M4A, WAV |
| Document | PDF |

---

## 💻 Commands

### `aic check [PATHS]`

Analyze files for AI-generation signals.

```bash
aic check photo.jpg                       # single file
aic check images/ -r                      # directory, recursive
aic check photo.jpg --json                # JSON output
aic check photo.jpg -q                    # quiet — exit code only
aic check photo.jpg --min-confidence medium  # filter by confidence
aic check photo.jpg --deep                # force pixel-level watermark analysis
```

### `aic info <FILE>`

Dump all provenance metadata (C2PA manifests, XMP properties, EXIF fields, MP4 atoms, ID3 tags, WAV metadata, watermark analysis).

```bash
aic info photo.jpg
```

### Global Flags

| Flag | Effect |
|------|--------|
| `--json` | Output as JSON |
| `-q, --quiet` | Suppress output, set exit code only |
| `--no-color` | Disable colored output |
| `--lang <LANG>` | Override display language (en, zh-CN, de, ja, ko, hi, es) |

### Check Options

| Flag | Effect |
|------|--------|
| `-r, --recursive` | Recurse into directories |
| `--deep` | Force invisible watermark and audio spectral analysis on all files |
| `--min-confidence <LEVEL>` | Filter by confidence level (low, medium, high; default: low) |

### Exit Codes

| Code | Meaning |
|------|---------|
| `0` | AI signals detected |
| `1` | No AI signals |
| `2` | Error |

---

## ⚠️ Limitations

- **Stripped metadata = invisible.** If someone removes the metadata, there's nothing to detect. Social platforms do this on upload — always analyze the original file.
- **Most AI images have no watermark.** Only ~19% of AI images carry detectable provenance markers (2025 data).
- **Proprietary watermarks are out of reach.** SynthID, Stable Signature, and VideoSeal require keys we don't have.
- **Pixel-level analysis has limits.** The built-in DWT-DCT watermark detector catches common patterns but is not a full forensic classifier. For deep statistical detection, use dedicated forensic tools.

---

## 📄 License

[AGPL-3.0](../LICENSE)
