> [English](README.en.md) | **简体中文** | [Deutsch](README.de.md) | [日本語](README.ja.md) | [한국어](README.ko.md) | [हिन्दी](README.hi.md) | [Español](README.es.md)

<div align="center">

# AICheck

**检测 AI 生成的内容。离线运行。无需 API key。无需配置。**

[![CI](https://github.com/MatrixA/aicheck/actions/workflows/ci.yml/badge.svg)](https://github.com/MatrixA/aicheck/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/aicheck)](https://crates.io/crates/aicheck)
[![License: AGPL-3.0](https://img.shields.io/badge/license-AGPL--3.0-blue.svg)](../LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.86%2B-orange.svg)](https://www.rust-lang.org/)

</div>

*那张疯传的图片——是 AI 还是真的？*
*这个视频是用哪个模型生成的？*
*这张照片的元数据可信吗？*

AICheck 通过分析文件元数据和隐形水印来回答这些问题。不需要 API key，不需要联网，不需要配置。

**10 种检测方法** · **61 种 AI 工具** · **16 种文件格式** · **3 级置信度** · **完全离线运行**

![演示](demo-zh.gif)

> 完整视频：[YouTube](https://youtu.be/1u-6TkHtWiA) | [Bilibili](https://www.bilibili.com/video/BV16Mc6zAE1s)

---

## ⚡ 快速开始

```bash
cargo install aicheck
```

> 需要 [Rust 1.86+](https://rust-lang.org/tools/install/)。从源码构建：`cargo install --path .`

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

## 🔍 工作原理

```
                              你的文件
                                 |
     +------+------+------+------+------+------+------+
     |      |      |      |      |      |      |      |
     v      v      v      v      v      v      v      v
  [C2PA] [XMP]  [EXIF] [PNG]  [MP4]  [ID3]  [WAV]  [文件名]
  HIGH  MEDIUM  LOW    LOW   MEDIUM MEDIUM MEDIUM   LOW
     |      |      |      |      |      |      |      |
     +--+---+--+---+--+---+--+---+--+---+--+---+--+---+
        |                                          |
        v                                          v
  检测到元数据信号?                          没有信号?
        |                                          |
        v                                          v
   [ 判定 ]                    [ 隐形水印 / 音频频谱分析 ]
                                DWT-DCT 或 FFT 分析
                                置信度: LOW
                                       |
                                       v
                                  [ 判定 ]
```

### 检测方法

**C2PA 清单（HIGH 置信度）**— 经过加密签名的来源证明。如果 C2PA 清单写着「由 DALL-E 生成」，这就是元数据能提供的最权威证据。读取 `digitalSourceType` 和 `claim_generator` 字段。支持图片、视频和音频（如 ElevenLabs）。

**XMP/IPTC 元数据（MEDIUM 置信度）**— 标准照片元数据：`DigitalSourceType`、`AISystemUsed`、`AIPromptInformation`、`CreatorTool`。可靠但没有签名——可以伪造或删除。

**MP4 容器元数据（MEDIUM 置信度）**— 解析 iTunes 风格原子（`©too`、`©swr`）、AIGC 标签（中国标准，含 JSON `ProduceID`）和 H.264 SEI 水印标记（Kling、Sora、Runway、Pika、Luma、Hailuo、Pixverse、Vidu、Genmo、Haiper）。同时检测非 AI 创作软件（FFmpeg、Remotion、Premiere 等）作为信息展示。能捕获嵌入视频容器中的 AI 信号。

**ID3 音频元数据（MEDIUM 置信度）**— 读取 MP3 文件的 ID3v2 标签：注释帧（COMM）、URL 帧（WOAS/WOAF/WXXX）和文本帧（TENC/TPUB/TXXX）。可检测 Suno 等 AI 音频平台（通过嵌入的 URL 和「made with suno」注释）。

**WAV 容器元数据（MEDIUM/LOW 置信度）**— 解析 RIFF LIST/INFO 块（ISFT、ICMT、IART）中的 AI 工具引用。同时标记 TTS 典型音频特征：单声道 + 非标准采样率（16kHz、22050Hz、24000Hz）。

**EXIF 启发式（LOW 置信度）**— 如果 `Software` 标签匹配已知 AI 工具，且缺少典型的相机字段（Make、Model、GPS、焦距），那大概率是 AI 生成的。也能检测哈希式的 Artist 标签。

**PNG 文本块（LOW 置信度）**— 扫描 `tEXt` 和 `iTXt` 块中 Software、Comment、Description、Source、Author、parameters、prompt 等关键字里的 AI 工具引用。

**文件名模式（LOW 置信度）**— 将文件名与已知 AI 工具的命名规则匹配（如 ElevenLabs 的时间戳格式 `ElevenLabs_YYYY-MM-DDTHH_MM_SS_*`、Suno/SoundRaw 前缀、文件名中的 Midjourney/DALL-E）。

**音频频谱分析（LOW 置信度）**— 基于 FFT 的 WAV 音频分析：检测硬频率截断（能量集中在奈奎斯特频率以下）和异常的频谱平坦度，这些是 TTS/AI 合成的典型特征。作为后备方案自动运行，或通过 `--deep` 强制启用。

**隐形水印（LOW 置信度）**— 像素级 DWT-DCT 分析，检测通道噪声不对称性、跨通道比特一致性和小波能量模式。对于视频文件，自动通过 `ffmpeg` 提取关键帧并逐帧分析。当未检测到元数据信号时自动运行，也可通过 `--deep` 强制启用。

---

## 🎯 识别能力

### AI 工具

| 类别 | 工具 |
|------|------|
| 图像生成 | DALL-E, Midjourney, Stable Diffusion, Adobe Firefly, Imagen, Flux, Ideogram, Leonardo.ai, NovelAI, Grok, Jimeng (即梦) |
| 视频生成 | Sora, Google Veo, Runway, Pika, Kling, Vidu, Luma, Hailuo (海螺), Pixverse, Genmo, Haiper |
| 音频/音乐生成 | Suno, Udio, ElevenLabs, SoundRaw, AIVA, Boomy, Mubert, Beatoven, Soundful, Hume, Fish Audio |
| 多模态 | GPT-4o, GPT-4, ChatGPT, OpenAI, GPT Image, Gemini |
| 平台 | Bing Image Creator, Copilot Designer, Microsoft Designer, Canva AI, DreamStudio, NightCafe, Craiyon, DeepAI, Meta AI, Stability AI |
| 界面工具 | ComfyUI, Automatic1111 (A1111), InvokeAI, Fooocus |
| 研究项目 | Glide, Parti, Muse, Seedream, Recraft |

### 文件格式

| 类型 | 格式 |
|------|------|
| 图片 | JPEG, PNG, WebP, AVIF, HEIF, TIFF, GIF, BMP |
| 视频 | MP4, MOV, AVI, WebM |
| 音频 | MP3, M4A, WAV |
| 文档 | PDF |

---

## 💻 命令

### `aic check [PATHS]`

分析文件中的 AI 生成信号。

```bash
aic check photo.jpg                       # 单个文件
aic check images/ -r                      # 目录，递归扫描
aic check photo.jpg --json                # JSON 输出
aic check photo.jpg -q                    # 静默模式——仅返回退出码
aic check photo.jpg --min-confidence medium  # 按置信度过滤
aic check photo.jpg --deep                # 强制启用像素级水印分析
```

### `aic info <FILE>`

输出所有溯源元数据（C2PA 清单、XMP 属性、EXIF 字段、MP4 原子、ID3 标签、WAV 元数据、水印分析）。

```bash
aic info photo.jpg
```

### 全局选项

| 选项 | 说明 |
|------|------|
| `--json` | 以 JSON 格式输出 |
| `-q, --quiet` | 不输出内容，仅设置退出码 |
| `--deep` | 强制对所有文件进行隐形水印和音频频谱分析 |
| `--no-color` | 禁用彩色输出 |

### 退出码

| 退出码 | 含义 |
|--------|------|
| `0` | 检测到 AI 信号 |
| `1` | 未检测到 AI 信号 |
| `2` | 错误 |

---

## ⚠️ 局限性

- **元数据被删了就没辙。** 如果有人把元数据剥掉了，那就没有可检测的内容。社交平台上传时会自动做这件事——请分析原始文件。
- **大多数 AI 图片没有水印。** 仅约 19% 的 AI 图片携带可检测的来源标记（2025 年数据）。
- **专有水印无法识别。** SynthID、Stable Signature、VideoSeal 需要我们没有的密钥。
- **像素级分析有局限。** 内置的 DWT-DCT 水印检测器能捕获常见模式，但不是完整的取证分类器。深度统计检测请使用专业取证工具。

---

## 📄 许可证

[AGPL-3.0](../LICENSE)
