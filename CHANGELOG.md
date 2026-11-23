# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Model attribution from C2PA `c2pa.created` actions: Volcengine `parameters.model_name` (Seedream / Seedance backend IDs), OpenAI `softwareAgent.version` (`gpt-image-*`), BFL series, Luma and Ideogram backend versions, Kling vendor. Cross-checks `c2pa.ai-disclosure` and `com.ideogram.generation` in the same manifest.
- `Signal` fields `model`, `scope`, `evidence_field`, `raw_value`, `evidence`; `FileReport` fields `attribution` and `evidence_level`; JSON summary counts `declared` / `heuristic_only`.
- C2PA validation summary per signal (`invalid` / `valid_untrusted` / `valid` / `trusted`); ingredient manifests are marked and do not set the file's attribution scope.
- TC260 AIGC labels in XMP (`http://www.tc260.org.cn/ns/AIGC/1.0/`, e.g. Qwen Image PNGs).
- Shared AIGC label parser (EXIF, MP4, XMP) using structured JSON; producer mappings for MiniMax, Vidu, Kling; the shared Alibaba code maps to `wan` for videos and `qwen/wan` for images.

### Fixed

- C2PA manifests are read by sniffed content type instead of file extension: Seedream JPEGs saved as `.png` were reported as "no signals".
- SEI markers are confirmed inside H.264/H.265 SEI NAL units; raw byte matches are downgraded to a low-confidence heuristic.
- Known-tool matching uses word boundaries (e.g. `udio` no longer matches "audio", `wan` no longer matches "swan"); `claim_generator_info` is matched on name/version only.
- Heuristic-only results (pixel watermark statistics, corner badges, TTS audio profile, filenames) are labelled as such and never produce a vendor attribution.

## [0.2.0] - 2026-03-13

### Added

- 10 new AI tool recognitions: Grok, Gemini, Jimeng (即梦), Luma, Hailuo (海螺), Pixverse, Genmo, Haiper, Hume, Fish Audio (51 → 61 tools)
- Video frame watermark analysis: extracts keyframes via `ffmpeg` and runs DWT-DCT analysis on video content
- Creation software detection: displays non-AI tools (FFmpeg, Remotion, Premiere, etc.) as informational metadata
- 9 new MP4 SEI watermark markers: Sora, Runway, Pika, Luma, Hailuo, Pixverse, Vidu, Genmo, Haiper
- Metadata stripping hint when no signals are detected (all 7 languages)

### Fixed

- Udio pattern matching: `udio.com` → `udio` for broader text matching

## [0.1.0] - 2025-06-01

### Added

- CLI commands: `aic check [PATHS]` and `aic info <FILE>`
- C2PA manifest detection and validation
- XMP/IPTC metadata analysis for AI provenance signals
- EXIF heuristic analysis
- PNG text chunk inspection
- MP4 container metadata parsing
- ID3 audio tag detection
- WAV container metadata analysis
- Filename pattern matching for known AI tools
- Audio spectral analysis for synthetic content
- Invisible watermark detection (DWT-DCT)
- Confidence tiers: HIGH, MEDIUM, LOW
- Support for image formats: JPEG, PNG, WebP, AVIF, HEIF, TIFF, GIF, BMP
- Support for video formats: MP4, MOV, AVI, WebM
- Support for audio formats: MP3, M4A, WAV
- Support for document formats: PDF
- JSON output mode (`--json`)
- Quiet mode (`--quiet`)
- Deep analysis mode (`--deep`)
- Internationalization support for 7 languages (en, de, es, hi, ja, ko, zh-CN)
- Rustdoc documentation with GitHub Pages deployment
