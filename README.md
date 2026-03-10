# aicheck (`aic`)

**Detect AI-generated content from provenance signals.**
**通过溯源信号检测 AI 生成内容。**

A CLI tool that reads C2PA manifests, XMP/IPTC metadata, and EXIF tags to determine if a file was created by AI.
一个命令行工具，通过读取 C2PA 清单、XMP/IPTC 元数据和 EXIF 标签来判断文件是否由 AI 生成。

## Install / 安装

```bash
cargo install --path .
```

Requires Rust 1.86+.

## Usage / 使用

```bash
# Check a file / 检测文件
aic check photo.jpg

# Check a directory / 检测目录
aic check images/ -r

# JSON output / JSON 输出
aic check photo.jpg --json

# Quiet mode (exit code only) / 静默模式（仅返回退出码）
aic check photo.jpg -q

# Dump all metadata / 输出所有元数据
aic info photo.jpg

# Filter by confidence / 按置信度过滤
aic check photo.jpg --min-confidence medium
```

### Exit Codes / 退出码

| Code | Meaning |
|------|---------|
| `0` | AI signals detected / 检测到 AI 信号 |
| `1` | No AI signals / 未检测到 AI 信号 |
| `2` | Error / 错误 |

## Detection / 检测方式

| Layer | Confidence | Source |
|-------|-----------|--------|
| **C2PA** | HIGH | `digitalSourceType`, `claim_generator` — DALL-E, Firefly, Bing, Sora, Veo |
| **XMP/IPTC** | MEDIUM | `DigitalSourceType`, `AISystemUsed`, `CreatorTool` |
| **EXIF** | LOW | Software tag + camera metadata absence heuristic / Software 标签 + 相机元数据缺失启发式 |

### Supported Formats / 支持格式

JPEG, PNG, WebP, AVIF, HEIF, TIFF, GIF, BMP, MP4, MOV, AVI, WebM, MP3, M4A, WAV, PDF

### Example Output / 输出示例

```
photo.jpg
  HIGH   C2PA: digitalSourceType = trainedAlgorithmicMedia (fully AI-generated)
  HIGH   C2PA: claim_generator matches AI tool: DALL-E 3/OpenAI [dall-e]
  MEDIUM XMP: AISystemUsed = DALL-E 3 [dall-e]
  Verdict: AI-generated (confidence: HIGH)

real_photo.jpg
  No AI-generation signals detected.
```

## Limitations / 局限性

- Only detects signals embedded in files — stripped metadata = no detection
  仅检测文件内嵌的信号 — 元数据被剥离则无法检测
- ~19% of AI images have detectable watermarks (2025)
  约 19% 的 AI 图片有可检测的水印（2025 年数据）
- Social platforms strip metadata on upload — analyze original files
  社交平台上传时会剥离元数据 — 请分析原始文件
- SynthID, Stable Signature, VideoSeal not supported (proprietary / key-dependent)
  不支持 SynthID、Stable Signature、VideoSeal（专有/需密钥）

## License

MIT
