> [English](README.en.md) | [简体中文](../README.md) | [Deutsch](README.de.md) | [日本語](README.ja.md) | [한국어](README.ko.md) | [हिन्दी](README.hi.md) | **Español**

<div align="center">

# AICheck

**Detecta contenido generado por IA. Sin conexión. Sin API keys. Sin configuración.**

[![CI](https://github.com/MatrixA/aicheck/actions/workflows/ci.yml/badge.svg)](https://github.com/MatrixA/aicheck/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/aicheck)](https://crates.io/crates/aicheck)
[![License: AGPL-3.0](https://img.shields.io/badge/license-AGPL--3.0-blue.svg)](../LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.86%2B-orange.svg)](https://www.rust-lang.org/)

</div>

*Esa imagen viral — ¿IA o real?*
*¿Qué modelo generó este video?*
*¿Puedes confiar en los metadatos de esta foto?*

AICheck responde estas preguntas analizando los metadatos de archivos y marcas de agua invisibles. Sin API keys, sin red, sin configuración.

**10 métodos de detección** · **76 herramientas de IA** · **16 formatos de archivo** · **3 niveles de confianza** · **Cero peticiones de red**

![Demo](demo-en.gif)

> Video completo: [YouTube](https://youtu.be/1u-6TkHtWiA) | [Bilibili](https://www.bilibili.com/video/BV16Mc6zAE1s)

---

## ⚡ Inicio Rápido

```bash
cargo install aicheck
```

> Requiere [Rust 1.86+](https://rust-lang.org/tools/install/). O compilar desde el código fuente: `cargo install --path .`

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

## 🔍 Cómo Funciona

```
                            tu archivo
                                 |
     +------+------+------+------+------+------+------+
     |      |      |      |      |      |      |      |
     v      v      v      v      v      v      v      v
  [C2PA] [XMP]  [EXIF] [PNG]  [MP4]  [ID3]  [WAV] [NOMBRE]
  HIGH  MEDIUM  LOW    LOW   MEDIUM MEDIUM MEDIUM   LOW
     |      |      |      |      |      |      |      |
     +--+---+--+---+--+---+--+---+--+---+--+---+--+---+
        |                                          |
        v                                          v
  ¿señales de metadatos?                   ¿sin señales?
        |                                          |
        v                                          v
   [ Veredicto ]          [ Marca de agua invisible / Análisis espectral ]
                            Análisis DWT-DCT o FFT
                            confianza: LOW
                                       |
                                       v
                                 [ Veredicto ]
```

### Métodos de Detección

**Manifiestos C2PA (confianza HIGH)** — Procedencia con firma criptográfica. Si un manifiesto C2PA dice "hecho por DALL-E", esa es la señal más autoritativa que los metadatos pueden proporcionar. Lee `digitalSourceType` y `claim_generator`. Funciona con imágenes, videos y audio (ej. ElevenLabs).

**Metadatos XMP/IPTC (confianza MEDIUM)** — Metadatos fotográficos estándar: `DigitalSourceType`, `AISystemUsed`, `AIPromptInformation`, `CreatorTool`. Fiables pero sin firma — pueden ser falsificados o eliminados.

**Metadatos de contenedor MP4 (confianza MEDIUM)** — Analiza átomos estilo iTunes (`©too`, `©swr`), etiquetas AIGC (estándar chino con JSON `ProduceID`) y marcadores de marca de agua SEI H.264 (Kling, Sora, Runway, Pika, Luma, Hailuo, Pixverse, Vidu, Genmo, Haiper). También detecta software de creación no-IA (FFmpeg, Remotion, Premiere, etc.) para visualización informativa. Captura señales de IA integradas en contenedores de video que otros métodos no detectan.

**Metadatos de audio ID3 (confianza MEDIUM)** — Lee etiquetas ID3v2 de archivos MP3: marcos de comentario (COMM), marcos de URL (WOAS/WOAF/WXXX) y marcos de texto (TENC/TPUB/TXXX). Detecta plataformas de audio IA como Suno (mediante URLs incrustadas y comentarios "made with suno").

**Metadatos de contenedor WAV (confianza MEDIUM/LOW)** — Analiza bloques RIFF LIST/INFO (ISFT, ICMT, IART) en busca de referencias a herramientas de IA. También marca características de audio típicas de TTS: canal mono + frecuencias de muestreo no estándar (16kHz, 22050Hz, 24000Hz).

**Heurísticas EXIF (confianza LOW)** — Si la etiqueta `Software` coincide con una herramienta de IA conocida Y faltan campos típicos de cámara (Make, Model, GPS, distancia focal), probablemente es generado por IA. También detecta etiquetas Artist con formato hash.

**Bloques de texto PNG (confianza LOW)** — Escanea bloques `tEXt` e `iTXt` en busca de referencias a herramientas de IA en las palabras clave Software, Comment, Description, Source, Author, parameters y prompt.

**Patrones de nombre de archivo (confianza LOW)** — Compara nombres de archivo con convenciones de nomenclatura de herramientas de IA conocidas (ej. formato de marca temporal de ElevenLabs `ElevenLabs_YYYY-MM-DDTHH_MM_SS_*`, prefijos Suno/SoundRaw, Midjourney/DALL-E en nombres de archivo).

**Análisis espectral de audio (confianza LOW)** — Análisis basado en FFT de audio WAV: detecta cortes de frecuencia abruptos (energía concentrada por debajo de Nyquist) y planitud espectral anormal típica de síntesis TTS/IA. Se ejecuta como respaldo o con `--deep`.

**Marcas de agua invisibles (confianza LOW)** — Análisis DWT-DCT a nivel de píxel que detecta asimetría de ruido entre canales, concordancia de bits entre canales y patrones de energía wavelet. Para videos, extrae automáticamente fotogramas clave mediante `ffmpeg` y los analiza individualmente. Se ejecuta automáticamente como respaldo cuando no se encuentran señales de metadatos, o bajo demanda con `--deep`.

---

## 🎯 Qué Reconoce

### Herramientas de IA

| Categoría | Herramientas |
|-----------|-------------|
| Generación de imágenes | DALL-E, Midjourney, Stable Diffusion, Adobe Firefly, Imagen, Flux, Ideogram, Leonardo.ai, NovelAI, Grok, Jimeng (即梦) |
| Generación de video | Sora, Google Veo, Runway, Pika, Kling, Vidu, Luma, Hailuo (海螺), Pixverse, Genmo, Haiper |
| Generación de audio/música | Suno, Udio, ElevenLabs, SoundRaw, AIVA, Boomy, Mubert, Beatoven, Soundful, Hume, Fish Audio |
| Multimodal | GPT-4o, GPT-4, ChatGPT, OpenAI, GPT Image, Gemini |
| Plataformas | Bing Image Creator, Copilot Designer, Microsoft Designer, Canva AI, DreamStudio, NightCafe, Craiyon, DeepAI, Meta AI, Stability AI |
| Interfaces | ComfyUI, Automatic1111 (A1111), InvokeAI, Fooocus |
| Investigación | Glide, Parti, Muse, Seedream, Recraft |

### Formatos de Archivo

| Tipo | Formatos |
|------|----------|
| Imagen | JPEG, PNG, WebP, AVIF, HEIF, TIFF, GIF, BMP |
| Video | MP4, MOV, AVI, WebM |
| Audio | MP3, M4A, WAV |
| Documento | PDF |

---

## 💻 Comandos

### `aic check [RUTAS]`

Analiza archivos en busca de señales de generación por IA.

```bash
aic check photo.jpg                       # archivo individual
aic check images/ -r                      # directorio, recursivo
aic check photo.jpg --json                # salida JSON
aic check photo.jpg -q                    # silencioso — solo código de salida
aic check photo.jpg --min-confidence medium  # filtrar por confianza
aic check photo.jpg --deep                # forzar análisis de marca de agua a nivel de píxel
```

### `aic info <ARCHIVO>`

Muestra todos los metadatos de procedencia (manifiestos C2PA, propiedades XMP, campos EXIF, átomos MP4, etiquetas ID3, metadatos WAV, análisis de marca de agua).

```bash
aic info photo.jpg
```

### Opciones Globales

| Opción | Efecto |
|--------|--------|
| `--json` | Salida en formato JSON |
| `-q, --quiet` | Suprimir salida, solo establecer código de salida |
| `--deep` | Forzar análisis de marca de agua invisible y espectral de audio en todos los archivos |
| `--no-color` | Desactivar salida con colores |

### Códigos de Salida

| Código | Significado |
|--------|-------------|
| `0` | Señales de IA detectadas |
| `1` | Sin señales de IA |
| `2` | Error |

---

## ⚠️ Limitaciones

- **Sin metadatos = invisible.** Si alguien elimina los metadatos, no hay nada que detectar. Las plataformas sociales hacen esto al subir archivos — analiza siempre el archivo original.
- **La mayoría de imágenes IA no tienen marca de agua.** Solo ~19% de las imágenes IA llevan marcadores de procedencia detectables (datos de 2025).
- **Las marcas de agua propietarias están fuera de alcance.** SynthID, Stable Signature y VideoSeal requieren claves que no tenemos.
- **El análisis a nivel de píxel tiene límites.** El detector de marca de agua DWT-DCT integrado captura patrones comunes pero no es un clasificador forense completo. Para detección estadística profunda, usa herramientas forenses dedicadas.

---

## 📄 Licencia

[AGPL-3.0](../LICENSE)
