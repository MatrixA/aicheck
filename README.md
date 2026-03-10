> **English** | [简体中文](README.zh-CN.md)

# AICheck

*That viral image — AI or real?*
*Which model generated this video?*
*Can you trust this photo's metadata?*

AICheck answers these questions by analyzing file metadata and invisible watermarks. No API keys, no network, no setup.

**6 detection methods** · **39 AI tools** · **16 file formats** · **3 confidence tiers** · **Zero network requests**

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

---

## How It Works

```
                       your file
                          |
          +-------+-------+-------+-------+
          |       |       |       |       |
          v       v       v       v       v
      [ C2PA ] [XMP/IPTC] [EXIF] [PNG]  [MP4]
       HIGH    MEDIUM     LOW    LOW    MEDIUM
          |       |       |       |       |
          +---+---+---+---+---+---+---+---+
              |                       |
              v                       v
    metadata signals found?     no signals?
              |                       |
              v                       v
         [ Verdict ]        [ Invisible Watermark ]
                              DWT-DCT analysis
                              confidence: LOW
                                    |
                                    v
                               [ Verdict ]
```

### Detection Methods

**C2PA Manifests (HIGH confidence)** — Cryptographically signed provenance. If a C2PA manifest says "made by DALL-E," that's the most authoritative signal metadata can provide. Reads `digitalSourceType` and `claim_generator`. Works on both images and videos.

**XMP/IPTC Metadata (MEDIUM confidence)** — Standard photo metadata: `DigitalSourceType`, `AISystemUsed`, `AIPromptInformation`, `CreatorTool`. Reliable but unsigned — can be faked or stripped.

**MP4 Container Metadata (MEDIUM confidence)** — Parses iTunes-style atoms (`©too`, `©swr`), AIGC labels (China standard with JSON `ProduceID`), and H.264 SEI watermark markers (e.g. Kling). Catches AI signals baked into video containers that other methods miss.

**EXIF Heuristics (LOW confidence)** — If the `Software` tag matches a known AI tool AND typical camera fields (Make, Model, GPS, focal length) are absent, it's likely AI-generated. Also detects hash-like Artist tags.

**PNG Text Chunks (LOW confidence)** — Scans `tEXt` and `iTXt` chunks for AI tool references in Software, Comment, Description, Source, Author, parameters, and prompt keywords.

**Invisible Watermarks (LOW confidence)** — Pixel-level DWT-DCT analysis that detects channel noise asymmetry, cross-channel bit agreement, and wavelet energy patterns. Runs automatically as a fallback when no metadata signals are found, or on demand with `--deep`.

---

## What It Recognizes

### AI Tools

| Category | Tools |
|----------|-------|
| Image generation | DALL-E, Midjourney, Stable Diffusion, Adobe Firefly, Imagen, Flux, Ideogram, Leonardo.ai, NovelAI |
| Video generation | Sora, Google Veo, Runway, Pika, Kling, Vidu |
| Multimodal | GPT-4o, GPT-4, ChatGPT, OpenAI, GPT Image |
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

## Commands

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

Dump all provenance metadata (C2PA manifests, XMP properties, EXIF fields, MP4 atoms, watermark analysis).

```bash
aic info photo.jpg
```

### Global Flags

| Flag | Effect |
|------|--------|
| `--json` | Output as JSON |
| `-q, --quiet` | Suppress output, set exit code only |
| `--deep` | Force invisible watermark analysis on all files |
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
- **Pixel-level analysis has limits.** The built-in DWT-DCT watermark detector catches common patterns but is not a full forensic classifier. For deep statistical detection, use dedicated forensic tools.

---

## License

MIT
