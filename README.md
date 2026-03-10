> **English** | [简体中文](README.zh-CN.md)

```
    _    ___ ____ _               _
   / \  |_ _/ ___| |__   ___  ___| | __
  / _ \  | | |   | '_ \ / _ \/ __| |/ /
 / ___ \ | | |___| | | |  __/ (__|   <
/_/   \_\___\____|_| |_|\___|\___|_|\_\
```

**Detect AI-generated content by reading what the file already knows about itself.**

**41 AI tools** · **16 file formats** · **3 confidence tiers** · **Zero network requests**

---

## Quick Start

```bash
cargo install --path .
```

> Requires Rust 1.86+

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

That's it. No API keys, no network, no setup.

---

## How It Works

```
                     your file
                        |
                        v
                +-------+-------+
                |               |
                v               v
         [ C2PA manifest ]  [ Raw bytes ]
         confidence: HIGH      |
                               +--------+--------+
                               |                  |
                               v                  v
                        [ XMP / IPTC ]       [ EXIF tags ]
                        conf: MEDIUM         conf: LOW
                               |                  |
                               +--------+---------+
                                        |
                                        v
                                   [ Verdict ]
                              AI / Not AI / Error
```

- **C2PA (HIGH confidence)** — Cryptographically signed provenance. If a C2PA manifest says "made by DALL-E," that's about as authoritative as metadata gets. Reads `digitalSourceType` and `claim_generator`.
- **XMP/IPTC (MEDIUM confidence)** — Standard photo metadata: `DigitalSourceType`, `AISystemUsed`, `CreatorTool`. Reliable but unsigned — can be faked or stripped.
- **EXIF (LOW confidence)** — Heuristic: if the `Software` tag matches an AI tool AND typical camera fields (Make, Model, GPS) are absent, it's probably AI-generated.

---

## What It Recognizes

### AI Tools

| Category | Tools |
|----------|-------|
| Image generation | DALL-E, Midjourney, Stable Diffusion, Adobe Firefly, Imagen, Flux, Ideogram, Leonardo.ai, NovelAI |
| Video generation | Sora, Veo, Runway, Pika |
| Platforms | Bing Image Creator, Copilot Designer, Microsoft Designer, Canva AI, DreamStudio, NightCafe, Craiyon, DeepAI, Meta AI, Stability AI |
| Interfaces | ComfyUI, Automatic1111 (A1111), InvokeAI, Fooocus |
| Research | Glide, Parti, Muse |

### File Formats

| Type | Formats |
|------|---------|
| Image | JPEG, PNG, WebP, AVIF, HEIF, TIFF, GIF, BMP |
| Video | MP4, MOV, AVI, WebM |
| Audio | MP3, M4A, WAV |
| Document | PDF |

---

## Commands

### `aic check [PATHS]`

Analyze files for AI-generation signals.

```bash
aic check photo.jpg                       # single file
aic check images/ -r                      # directory, recursive
aic check photo.jpg --json                # JSON output
aic check photo.jpg -q                    # quiet — exit code only
aic check photo.jpg --min-confidence medium  # filter by confidence
```

### `aic info <FILE>`

Dump all provenance metadata (C2PA manifests, XMP properties, EXIF fields).

```bash
aic info photo.jpg
```

### Global Flags

| Flag | Effect |
|------|--------|
| `--json` | Output as JSON |
| `-q, --quiet` | Suppress output, set exit code only |
| `--no-color` | Disable colored output |

### Exit Codes

| Code | Meaning |
|------|---------|
| `0` | AI signals detected |
| `1` | No AI signals |
| `2` | Error |

---

## What It Can't Do

- **Stripped metadata = invisible.** If someone removes the metadata, there's nothing to detect. Social platforms do this on upload — always analyze the original file.
- **Most AI images have no watermark.** Only ~19% of AI images carry detectable provenance markers (2025 data).
- **Proprietary watermarks are out of reach.** SynthID, Stable Signature, and VideoSeal require keys we don't have.
- **This is not a pixel-level detector.** It reads metadata, not pixels. For statistical detection, use dedicated forensic tools.

---

## License

MIT
