> [English](../README.md) | [简体中文](README.zh-CN.md) | **Deutsch** | [日本語](README.ja.md) | [한국어](README.ko.md) | [हिन्दी](README.hi.md) | [Español](README.es.md)

<div align="center">

# AICheck

**KI-generierte Inhalte erkennen. Offline. Keine API-Keys. Keine Einrichtung.**

[![CI](https://github.com/MatrixA/aicheck/actions/workflows/ci.yml/badge.svg)](https://github.com/MatrixA/aicheck/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/aicheck)](https://crates.io/crates/aicheck)
[![License: AGPL-3.0](https://img.shields.io/badge/license-AGPL--3.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.86%2B-orange.svg)](https://www.rust-lang.org/)

</div>

*Dieses virale Bild — KI oder echt?*
*Welches Modell hat dieses Video generiert?*
*Kann man den Metadaten dieses Fotos vertrauen?*

AICheck beantwortet diese Fragen durch Analyse von Datei-Metadaten und unsichtbaren Wasserzeichen. Keine API-Keys, kein Netzwerk, keine Einrichtung.

**10 Erkennungsmethoden** · **51 KI-Tools** · **16 Dateiformate** · **3 Konfidenzstufen** · **Null Netzwerkanfragen**

---

## ⚡ Schnellstart

```bash
cargo install aicheck
```

> Erfordert Rust 1.86+. Oder aus dem Quellcode bauen: `cargo install --path .`

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

## 🔍 Funktionsweise

```
                            Deine Datei
                                 |
     +------+------+------+------+------+------+------+
     |      |      |      |      |      |      |      |
     v      v      v      v      v      v      v      v
  [C2PA] [XMP]  [EXIF] [PNG]  [MP4]  [ID3]  [WAV] [DATEI]
  HIGH  MEDIUM  LOW    LOW   MEDIUM MEDIUM MEDIUM   LOW
     |      |      |      |      |      |      |      |
     +--+---+--+---+--+---+--+---+--+---+--+---+--+---+
        |                                          |
        v                                          v
  Metadaten-Signale gefunden?              Keine Signale?
        |                                          |
        v                                          v
   [ Ergebnis ]         [ Unsichtbares Wasserzeichen / Audio-Spektral ]
                          DWT-DCT- oder FFT-Analyse
                          Konfidenz: LOW
                                       |
                                       v
                                  [ Ergebnis ]
```

### Erkennungsmethoden

**C2PA-Manifeste (HIGH Konfidenz)** — Kryptografisch signierte Herkunftsnachweise. Wenn ein C2PA-Manifest sagt „hergestellt von DALL-E", ist das das autoritativste Signal, das Metadaten liefern können. Liest `digitalSourceType` und `claim_generator`. Funktioniert mit Bildern, Videos und Audio (z.B. ElevenLabs).

**XMP/IPTC-Metadaten (MEDIUM Konfidenz)** — Standard-Foto-Metadaten: `DigitalSourceType`, `AISystemUsed`, `AIPromptInformation`, `CreatorTool`. Zuverlässig, aber nicht signiert — kann gefälscht oder entfernt werden.

**MP4-Container-Metadaten (MEDIUM Konfidenz)** — Analysiert iTunes-Stil-Atome (`©too`, `©swr`), AIGC-Labels (chinesischer Standard mit JSON `ProduceID`) und H.264-SEI-Wasserzeichenmarker (z.B. Kling). Erfasst KI-Signale, die in Videocontainern eingebettet sind und von anderen Methoden übersehen werden.

**ID3-Audio-Metadaten (MEDIUM Konfidenz)** — Liest ID3v2-Tags aus MP3-Dateien: Kommentarframes (COMM), URL-Frames (WOAS/WOAF/WXXX) und Textframes (TENC/TPUB/TXXX). Erkennt KI-Audioplattformen wie Suno (über eingebettete URLs und „made with suno"-Kommentare).

**WAV-Container-Metadaten (MEDIUM/LOW Konfidenz)** — Analysiert RIFF-LIST/INFO-Blöcke (ISFT, ICMT, IART) auf Verweise zu KI-Tools. Kennzeichnet außerdem TTS-typische Audioeigenschaften: Mono-Kanal + nicht-standardmäßige Abtastraten (16kHz, 22050Hz, 24000Hz).

**EXIF-Heuristiken (LOW Konfidenz)** — Wenn das `Software`-Tag mit einem bekannten KI-Tool übereinstimmt UND typische Kamerafelder (Make, Model, GPS, Brennweite) fehlen, ist es wahrscheinlich KI-generiert. Erkennt auch hash-artige Artist-Tags.

**PNG-Textblöcke (LOW Konfidenz)** — Durchsucht `tEXt`- und `iTXt`-Blöcke nach KI-Tool-Verweisen in den Schlüsselwörtern Software, Comment, Description, Source, Author, parameters und prompt.

**Dateinamenmuster (LOW Konfidenz)** — Gleicht Dateinamen mit bekannten Benennungskonventionen von KI-Tools ab (z.B. ElevenLabs-Zeitstempelformat `ElevenLabs_YYYY-MM-DDTHH_MM_SS_*`, Suno/SoundRaw-Präfixe, Midjourney/DALL-E in Dateinamen).

**Audio-Spektralanalyse (LOW Konfidenz)** — FFT-basierte Analyse von WAV-Audio: erkennt harte Frequenzabschneide (Energie konzentriert unterhalb von Nyquist) und abnormale spektrale Flachheit, typisch für TTS/KI-Synthese. Läuft als Fallback oder mit `--deep`.

**Unsichtbare Wasserzeichen (LOW Konfidenz)** — Pixelbasierte DWT-DCT-Analyse, die Kanalrauschen-Asymmetrie, kanalübergreifende Bit-Übereinstimmung und Wavelet-Energiemuster erkennt. Läuft automatisch als Fallback, wenn keine Metadaten-Signale gefunden werden, oder auf Anforderung mit `--deep`.

---

## 🎯 Erkannte Inhalte

### KI-Tools

| Kategorie | Tools |
|-----------|-------|
| Bildgenerierung | DALL-E, Midjourney, Stable Diffusion, Adobe Firefly, Imagen, Flux, Ideogram, Leonardo.ai, NovelAI |
| Videogenerierung | Sora, Google Veo, Runway, Pika, Kling, Vidu |
| Audio-/Musikgenerierung | Suno, Udio, ElevenLabs, SoundRaw, AIVA, Boomy, Mubert, Beatoven, Soundful |
| Multimodal | GPT-4o, GPT-4, ChatGPT, OpenAI, GPT Image |
| Plattformen | Bing Image Creator, Copilot Designer, Microsoft Designer, Canva AI, DreamStudio, NightCafe, Craiyon, DeepAI, Meta AI, Stability AI |
| Oberflächen | ComfyUI, Automatic1111 (A1111), InvokeAI, Fooocus |
| Forschung | Glide, Parti, Muse, Seedream, Recraft |

### Dateiformate

| Typ | Formate |
|-----|---------|
| Bild | JPEG, PNG, WebP, AVIF, HEIF, TIFF, GIF, BMP |
| Video | MP4, MOV, AVI, WebM |
| Audio | MP3, M4A, WAV |
| Dokument | PDF |

---

## 💻 Befehle

### `aic check [PFADE]`

Dateien auf KI-Generierungssignale analysieren.

```bash
aic check photo.jpg                       # einzelne Datei
aic check images/ -r                      # Verzeichnis, rekursiv
aic check photo.jpg --json                # JSON-Ausgabe
aic check photo.jpg -q                    # leise — nur Exit-Code
aic check photo.jpg --min-confidence medium  # nach Konfidenz filtern
aic check photo.jpg --deep                # Pixelbasierte Wasserzeichenanalyse erzwingen
```

### `aic info <DATEI>`

Alle Herkunfts-Metadaten ausgeben (C2PA-Manifeste, XMP-Eigenschaften, EXIF-Felder, MP4-Atome, ID3-Tags, WAV-Metadaten, Wasserzeichenanalyse).

```bash
aic info photo.jpg
```

### Globale Optionen

| Option | Wirkung |
|--------|---------|
| `--json` | Ausgabe als JSON |
| `-q, --quiet` | Ausgabe unterdrücken, nur Exit-Code setzen |
| `--deep` | Unsichtbare Wasserzeichen- und Audio-Spektralanalyse für alle Dateien erzwingen |
| `--no-color` | Farbige Ausgabe deaktivieren |

### Exit-Codes

| Code | Bedeutung |
|------|-----------|
| `0` | KI-Signale erkannt |
| `1` | Keine KI-Signale |
| `2` | Fehler |

---

## ⚠️ Einschränkungen

- **Entfernte Metadaten = unsichtbar.** Wenn jemand die Metadaten entfernt, gibt es nichts zu erkennen. Soziale Plattformen tun dies beim Upload — analysiere immer die Originaldatei.
- **Die meisten KI-Bilder haben kein Wasserzeichen.** Nur ~19% der KI-Bilder tragen erkennbare Herkunftsmarker (Daten von 2025).
- **Proprietäre Wasserzeichen sind unerreichbar.** SynthID, Stable Signature und VideoSeal benötigen Schlüssel, die wir nicht haben.
- **Pixelbasierte Analyse hat Grenzen.** Der eingebaute DWT-DCT-Wasserzeichendetektor erkennt häufige Muster, ist aber kein vollständiger forensischer Klassifikator. Für tiefgehende statistische Erkennung verwende spezialisierte forensische Tools.

---

## 📄 Lizenz

[AGPL-3.0](LICENSE)
