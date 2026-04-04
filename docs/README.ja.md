> [English](README.en.md) | [简体中文](../README.md) | [Deutsch](README.de.md) | **日本語** | [한국어](README.ko.md) | [हिन्दी](README.hi.md) | [Español](README.es.md)

<div align="center">

# AICheck

**AI生成コンテンツを検出。オフライン。APIキー不要。セットアップ不要。**

[![CI](https://github.com/MatrixA/aicheck/actions/workflows/ci.yml/badge.svg)](https://github.com/MatrixA/aicheck/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/aicheck)](https://crates.io/crates/aicheck)
[![License: AGPL-3.0](https://img.shields.io/badge/license-AGPL--3.0-blue.svg)](../LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.86%2B-orange.svg)](https://www.rust-lang.org/)

</div>

*あのバズった画像——AIか本物か？*
*この動画はどのモデルで生成された？*
*この写真のメタデータは信頼できる？*

AICheckはファイルのメタデータと不可視ウォーターマークを分析してこれらの疑問に答えます。APIキー不要、ネットワーク不要、セットアップ不要。

**10種の検出方法** · **61種のAIツール** · **16種のファイル形式** · **3段階の信頼度** · **ネットワーク通信ゼロ**

![デモ](demo-en.gif)

> フル動画：[YouTube](https://youtu.be/1u-6TkHtWiA) | [Bilibili](https://www.bilibili.com/video/BV16Mc6zAE1s)

---

## ⚡ クイックスタート

```bash
cargo install aicheck
```

> [Rust 1.86+](https://rust-lang.org/tools/install/)が必要です。ソースからビルド：`cargo install --path .`

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

## 🔍 仕組み

```
                          あなたのファイル
                                 |
     +------+------+------+------+------+------+------+
     |      |      |      |      |      |      |      |
     v      v      v      v      v      v      v      v
  [C2PA] [XMP]  [EXIF] [PNG]  [MP4]  [ID3]  [WAV] [ファイル名]
  HIGH  MEDIUM  LOW    LOW   MEDIUM MEDIUM MEDIUM   LOW
     |      |      |      |      |      |      |      |
     +--+---+--+---+--+---+--+---+--+---+--+---+--+---+
        |                                          |
        v                                          v
  メタデータシグナル検出？                  シグナルなし？
        |                                          |
        v                                          v
   [ 判定 ]                [ 不可視ウォーターマーク / 音声スペクトル分析 ]
                             DWT-DCTまたはFFT分析
                             信頼度: LOW
                                       |
                                       v
                                    [ 判定 ]
```

### 検出方法

**C2PAマニフェスト（HIGH信頼度）**— 暗号署名による来歴証明。C2PAマニフェストに「DALL-Eで作成」と記載されていれば、それはメタデータが提供できる最も権威ある証拠です。`digitalSourceType`と`claim_generator`を読み取ります。画像、動画、音声（例：ElevenLabs）に対応。

**XMP/IPTCメタデータ（MEDIUM信頼度）**— 標準的な写真メタデータ：`DigitalSourceType`、`AISystemUsed`、`AIPromptInformation`、`CreatorTool`。信頼性は高いが署名なし——偽造や削除が可能。

**MP4コンテナメタデータ（MEDIUM信頼度）**— iTunes形式のアトム（`©too`、`©swr`）、AIGCラベル（中国規格、JSON `ProduceID`付き）、H.264 SEIウォーターマークマーカー（Kling、Sora、Runway、Pika、Luma、Hailuo、Pixverse、Vidu、Genmo、Haiper）を解析。非AI制作ソフトウェア（FFmpeg、Remotion、Premiereなど）も情報表示として検出。他の方法では見逃されるビデオコンテナに埋め込まれたAIシグナルを検出。

**ID3音声メタデータ（MEDIUM信頼度）**— MP3ファイルのID3v2タグを読み取り：コメントフレーム（COMM）、URLフレーム（WOAS/WOAF/WXXX）、テキストフレーム（TENC/TPUB/TXXX）。SunoなどのAI音声プラットフォームを検出（埋め込みURLや「made with suno」コメント経由）。

**WAVコンテナメタデータ（MEDIUM/LOW信頼度）**— RIFF LIST/INFOチャンク（ISFT、ICMT、IART）を解析してAIツールへの参照を検出。TTS特有の音声特性も検出：モノラル + 非標準サンプルレート（16kHz、22050Hz、24000Hz）。

**EXIFヒューリスティクス（LOW信頼度）**— `Software`タグが既知のAIツールと一致し、かつ典型的なカメラフィールド（Make、Model、GPS、焦点距離）が欠落している場合、AI生成の可能性が高い。ハッシュ形式のArtistタグも検出。

**PNGテキストチャンク（LOW信頼度）**— `tEXt`および`iTXt`チャンクのSoftware、Comment、Description、Source、Author、parameters、promptキーワードからAIツールへの参照をスキャン。

**ファイル名パターン（LOW信頼度）**— ファイル名を既知のAIツールの命名規則と照合（例：ElevenLabsのタイムスタンプ形式 `ElevenLabs_YYYY-MM-DDTHH_MM_SS_*`、Suno/SoundRawプレフィックス、ファイル名中のMidjourney/DALL-E）。

**音声スペクトル分析（LOW信頼度）**— FFTベースのWAV音声分析：硬い周波数カットオフ（ナイキスト以下にエネルギーが集中）とTTS/AI合成に典型的な異常なスペクトル平坦度を検出。フォールバックとして、または`--deep`で実行。

**不可視ウォーターマーク（LOW信頼度）**— ピクセルレベルのDWT-DCT分析で、チャンネルノイズの非対称性、チャンネル間ビット一致、ウェーブレットエネルギーパターンを検出。動画の場合は`ffmpeg`でキーフレームを自動抽出し、個別に分析。メタデータシグナルが見つからない場合に自動的にフォールバックとして実行、または`--deep`でオンデマンド実行。

---

## 🎯 認識対象

### AIツール

| カテゴリ | ツール |
|---------|--------|
| 画像生成 | DALL-E, Midjourney, Stable Diffusion, Adobe Firefly, Imagen, Flux, Ideogram, Leonardo.ai, NovelAI, Grok, Jimeng (即梦) |
| 動画生成 | Sora, Google Veo, Runway, Pika, Kling, Vidu, Luma, Hailuo (海螺), Pixverse, Genmo, Haiper |
| 音声/音楽生成 | Suno, Udio, ElevenLabs, SoundRaw, AIVA, Boomy, Mubert, Beatoven, Soundful, Hume, Fish Audio |
| マルチモーダル | GPT-4o, GPT-4, ChatGPT, OpenAI, GPT Image, Gemini |
| プラットフォーム | Bing Image Creator, Copilot Designer, Microsoft Designer, Canva AI, DreamStudio, NightCafe, Craiyon, DeepAI, Meta AI, Stability AI |
| インターフェース | ComfyUI, Automatic1111 (A1111), InvokeAI, Fooocus |
| 研究 | Glide, Parti, Muse, Seedream, Recraft |

### ファイル形式

| タイプ | 形式 |
|--------|------|
| 画像 | JPEG, PNG, WebP, AVIF, HEIF, TIFF, GIF, BMP |
| 動画 | MP4, MOV, AVI, WebM |
| 音声 | MP3, M4A, WAV |
| ドキュメント | PDF |

---

## 💻 コマンド

### `aic check [パス]`

ファイルのAI生成シグナルを分析。

```bash
aic check photo.jpg                       # 単一ファイル
aic check images/ -r                      # ディレクトリ、再帰的
aic check photo.jpg --json                # JSON出力
aic check photo.jpg -q                    # 静音——終了コードのみ
aic check photo.jpg --min-confidence medium  # 信頼度でフィルタ
aic check photo.jpg --deep                # ピクセルレベルのウォーターマーク分析を強制
```

### `aic info <ファイル>`

すべての来歴メタデータを出力（C2PAマニフェスト、XMPプロパティ、EXIFフィールド、MP4アトム、ID3タグ、WAVメタデータ、ウォーターマーク分析）。

```bash
aic info photo.jpg
```

### グローバルオプション

| オプション | 効果 |
|-----------|------|
| `--json` | JSON形式で出力 |
| `-q, --quiet` | 出力を抑制、終了コードのみ設定 |
| `--deep` | すべてのファイルに不可視ウォーターマークと音声スペクトル分析を強制 |
| `--no-color` | 色付き出力を無効化 |

### 終了コード

| コード | 意味 |
|--------|------|
| `0` | AIシグナル検出 |
| `1` | AIシグナルなし |
| `2` | エラー |

---

## ⚠️ 制限事項

- **メタデータ削除 = 検出不可。** メタデータが削除されると、検出するものがありません。SNSプラットフォームはアップロード時にこれを行います——必ず元のファイルを分析してください。
- **ほとんどのAI画像にはウォーターマークがない。** 検出可能な来歴マーカーを持つAI画像はわずか約19%です（2025年データ）。
- **プロプライエタリなウォーターマークは対象外。** SynthID、Stable Signature、VideoSealには入手できないキーが必要です。
- **ピクセルレベル分析には限界がある。** 内蔵のDWT-DCTウォーターマーク検出器は一般的なパターンを捉えますが、完全なフォレンジック分類器ではありません。詳細な統計的検出には専用のフォレンジックツールを使用してください。

---

## 📄 ライセンス

[AGPL-3.0](../LICENSE)
