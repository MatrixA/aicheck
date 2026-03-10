> [English](README.md) | **简体中文**

```
    _    ___ ____ _               _
   / \  |_ _/ ___| |__   ___  ___| | __
  / _ \  | | |   | '_ \ / _ \/ __| |/ /
 / ___ \ | | |___| | | |  __/ (__|   <
/_/   \_\___\____|_| |_|\___|\___|_|\_\
```

**读取文件自带的「身份信息」，判断它是不是 AI 生成的。**

**41 种 AI 工具** · **16 种文件格式** · **3 级置信度** · **完全离线运行**

---

## 快速开始

```bash
cargo install --path .
```

> 需要 Rust 1.86+

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

就这么简单。不需要 API key，不需要联网，不需要额外配置。

---

## 工作原理

```
                     你的文件
                        |
                        v
                +-------+-------+
                |               |
                v               v
         [ C2PA 清单 ]     [ 原始字节 ]
         置信度: HIGH          |
                               +--------+--------+
                               |                  |
                               v                  v
                        [ XMP / IPTC ]       [ EXIF 标签 ]
                        置信度: MEDIUM       置信度: LOW
                               |                  |
                               +--------+---------+
                                        |
                                        v
                                    [ 判定 ]
                              AI / 非 AI / 错误
```

- **C2PA（HIGH 置信度）**— 经过加密签名的来源证明。如果 C2PA 清单写着「由 DALL-E 生成」，这基本就是元数据里最权威的证据了。读取 `digitalSourceType` 和 `claim_generator` 字段。
- **XMP/IPTC（MEDIUM 置信度）**— 标准的照片元数据字段：`DigitalSourceType`、`AISystemUsed`、`CreatorTool`。可靠但没有签名——可以伪造或删除。
- **EXIF（LOW 置信度）**— 启发式判断：如果 `Software` 标签匹配已知 AI 工具，且缺少典型的相机字段（Make、Model、GPS），那大概率是 AI 生成的。

---

## 识别能力

### AI 工具

| 类别 | 工具 |
|------|------|
| 图像生成 | DALL-E, Midjourney, Stable Diffusion, Adobe Firefly, Imagen, Flux, Ideogram, Leonardo.ai, NovelAI |
| 视频生成 | Sora, Veo, Runway, Pika |
| 平台 | Bing Image Creator, Copilot Designer, Microsoft Designer, Canva AI, DreamStudio, NightCafe, Craiyon, DeepAI, Meta AI, Stability AI |
| 界面工具 | ComfyUI, Automatic1111 (A1111), InvokeAI, Fooocus |
| 研究项目 | Glide, Parti, Muse |

### 文件格式

| 类型 | 格式 |
|------|------|
| 图片 | JPEG, PNG, WebP, AVIF, HEIF, TIFF, GIF, BMP |
| 视频 | MP4, MOV, AVI, WebM |
| 音频 | MP3, M4A, WAV |
| 文档 | PDF |

---

## 命令

### `aic check [PATHS]`

分析文件中的 AI 生成信号。

```bash
aic check photo.jpg                       # 单个文件
aic check images/ -r                      # 目录，递归扫描
aic check photo.jpg --json                # JSON 输出
aic check photo.jpg -q                    # 静默模式——仅返回退出码
aic check photo.jpg --min-confidence medium  # 按置信度过滤
```

### `aic info <FILE>`

输出所有溯源元数据（C2PA 清单、XMP 属性、EXIF 字段）。

```bash
aic info photo.jpg
```

### 全局选项

| 选项 | 说明 |
|------|------|
| `--json` | 以 JSON 格式输出 |
| `-q, --quiet` | 不输出内容，仅设置退出码 |
| `--no-color` | 禁用彩色输出 |

### 退出码

| 退出码 | 含义 |
|--------|------|
| `0` | 检测到 AI 信号 |
| `1` | 未检测到 AI 信号 |
| `2` | 错误 |

---

## 做不到的事

- **元数据被删了就没辙。** 如果有人把元数据剥掉了，那就没有可检测的内容。社交平台上传时会自动做这件事——请分析原始文件。
- **大多数 AI 图片没有水印。** 仅约 19% 的 AI 图片携带可检测的来源标记（2025 年数据）。
- **专有水印无法识别。** SynthID、Stable Signature、VideoSeal 需要我们没有的密钥。
- **这不是像素级检测工具。** 它读的是元数据，不是像素。像素层面的统计检测请使用专业的取证工具。

---

## 许可证

MIT
