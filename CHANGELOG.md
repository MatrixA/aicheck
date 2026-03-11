# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
