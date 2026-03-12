> [English](../README.md) | [简体中文](README.zh-CN.md) | [Deutsch](README.de.md) | [日本語](README.ja.md) | [한국어](README.ko.md) | **हिन्दी** | [Español](README.es.md)

<div align="center">

# AICheck

**AI-जनित सामग्री का पता लगाएं। ऑफ़लाइन। API कुंजी अनावश्यक। सेटअप अनावश्यक।**

[![CI](https://github.com/MatrixA/aicheck/actions/workflows/ci.yml/badge.svg)](https://github.com/MatrixA/aicheck/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/aicheck)](https://crates.io/crates/aicheck)
[![License: AGPL-3.0](https://img.shields.io/badge/license-AGPL--3.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.86%2B-orange.svg)](https://www.rust-lang.org/)

</div>

*वह वायरल इमेज — AI है या असली?*
*यह वीडियो किस मॉडल ने बनाया?*
*क्या इस फ़ोटो के मेटाडेटा पर भरोसा किया जा सकता है?*

AICheck फ़ाइल मेटाडेटा और अदृश्य वॉटरमार्क का विश्लेषण करके इन सवालों का जवाब देता है। API कुंजी नहीं, नेटवर्क नहीं, सेटअप नहीं।

**10 पहचान विधियाँ** · **61 AI उपकरण** · **16 फ़ाइल प्रारूप** · **3 विश्वसनीयता स्तर** · **शून्य नेटवर्क अनुरोध**

---

## ⚡ त्वरित शुरुआत

```bash
cargo install aicheck
```

> [Rust 1.86+](https://rust-lang.org/tools/install/) आवश्यक। सोर्स से बिल्ड करें: `cargo install --path .`

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

## 🔍 यह कैसे काम करता है

```
                             आपकी फ़ाइल
                                 |
     +------+------+------+------+------+------+------+
     |      |      |      |      |      |      |      |
     v      v      v      v      v      v      v      v
  [C2PA] [XMP]  [EXIF] [PNG]  [MP4]  [ID3]  [WAV] [फ़ाइल नाम]
  HIGH  MEDIUM  LOW    LOW   MEDIUM MEDIUM MEDIUM   LOW
     |      |      |      |      |      |      |      |
     +--+---+--+---+--+---+--+---+--+---+--+---+--+---+
        |                                          |
        v                                          v
  मेटाडेटा सिग्नल मिले?                    कोई सिग्नल नहीं?
        |                                          |
        v                                          v
   [ निर्णय ]              [ अदृश्य वॉटरमार्क / ऑडियो स्पेक्ट्रल विश्लेषण ]
                             DWT-DCT या FFT विश्लेषण
                             विश्वसनीयता: LOW
                                       |
                                       v
                                   [ निर्णय ]
```

### पहचान विधियाँ

**C2PA मैनिफ़ेस्ट (HIGH विश्वसनीयता)** — क्रिप्टोग्राफ़िक रूप से हस्ताक्षरित उत्पत्ति प्रमाण। यदि C2PA मैनिफ़ेस्ट कहता है "DALL-E द्वारा बनाया गया", तो यह मेटाडेटा द्वारा दिया जा सकने वाला सबसे प्रामाणिक संकेत है। `digitalSourceType` और `claim_generator` पढ़ता है। इमेज, वीडियो और ऑडियो (जैसे ElevenLabs) पर काम करता है।

**XMP/IPTC मेटाडेटा (MEDIUM विश्वसनीयता)** — मानक फ़ोटो मेटाडेटा: `DigitalSourceType`, `AISystemUsed`, `AIPromptInformation`, `CreatorTool`। विश्वसनीय लेकिन बिना हस्ताक्षर — नकली बनाया या हटाया जा सकता है।

**MP4 कंटेनर मेटाडेटा (MEDIUM विश्वसनीयता)** — iTunes-शैली के एटम (`©too`, `©swr`), AIGC लेबल (चीनी मानक, JSON `ProduceID` सहित), और H.264 SEI वॉटरमार्क मार्कर (Kling, Sora, Runway, Pika, Luma, Hailuo, Pixverse, Vidu, Genmo, Haiper) का विश्लेषण करता है। गैर-AI निर्माण सॉफ़्टवेयर (FFmpeg, Remotion, Premiere आदि) को भी सूचनात्मक प्रदर्शन के लिए पहचानता है। वीडियो कंटेनरों में एम्बेडेड AI सिग्नल पकड़ता है जो अन्य विधियाँ चूक जाती हैं।

**ID3 ऑडियो मेटाडेटा (MEDIUM विश्वसनीयता)** — MP3 फ़ाइलों से ID3v2 टैग पढ़ता है: कमेंट फ़्रेम (COMM), URL फ़्रेम (WOAS/WOAF/WXXX), और टेक्स्ट फ़्रेम (TENC/TPUB/TXXX)। Suno जैसे AI ऑडियो प्लेटफ़ॉर्म का पता लगाता है (एम्बेडेड URL और "made with suno" कमेंट के माध्यम से)।

**WAV कंटेनर मेटाडेटा (MEDIUM/LOW विश्वसनीयता)** — AI उपकरण संदर्भों के लिए RIFF LIST/INFO चंक (ISFT, ICMT, IART) का विश्लेषण करता है। TTS-विशिष्ट ऑडियो विशेषताओं को भी चिह्नित करता है: मोनो चैनल + गैर-मानक सैंपल दर (16kHz, 22050Hz, 24000Hz)।

**EXIF ह्यूरिस्टिक्स (LOW विश्वसनीयता)** — यदि `Software` टैग किसी ज्ञात AI उपकरण से मेल खाता है और विशिष्ट कैमरा फ़ील्ड (Make, Model, GPS, फ़ोकल लंबाई) अनुपस्थित हैं, तो यह संभवतः AI-जनित है। हैश-जैसे Artist टैग का भी पता लगाता है।

**PNG टेक्स्ट चंक (LOW विश्वसनीयता)** — Software, Comment, Description, Source, Author, parameters, और prompt कीवर्ड में AI उपकरण संदर्भों के लिए `tEXt` और `iTXt` चंक को स्कैन करता है।

**फ़ाइल नाम पैटर्न (LOW विश्वसनीयता)** — फ़ाइल नामों को ज्ञात AI उपकरण नामकरण परंपराओं से मिलाता है (जैसे ElevenLabs टाइमस्टैंप प्रारूप `ElevenLabs_YYYY-MM-DDTHH_MM_SS_*`, Suno/SoundRaw उपसर्ग, फ़ाइल नामों में Midjourney/DALL-E)।

**ऑडियो स्पेक्ट्रल विश्लेषण (LOW विश्वसनीयता)** — WAV ऑडियो का FFT-आधारित विश्लेषण: हार्ड फ़्रीक्वेंसी कटऑफ़ (Nyquist से नीचे ऊर्जा केंद्रित) और TTS/AI संश्लेषण की विशिष्ट असामान्य स्पेक्ट्रल समतलता का पता लगाता है। फ़ॉलबैक के रूप में या `--deep` के साथ चलता है।

**अदृश्य वॉटरमार्क (LOW विश्वसनीयता)** — पिक्सेल-स्तरीय DWT-DCT विश्लेषण जो चैनल नॉइज़ असमानता, क्रॉस-चैनल बिट सहमति, और वेवलेट ऊर्जा पैटर्न का पता लगाता है। वीडियो के लिए, `ffmpeg` के माध्यम से स्वचालित रूप से कीफ़्रेम निकालकर व्यक्तिगत रूप से विश्लेषण करता है। जब कोई मेटाडेटा सिग्नल नहीं मिलता तो स्वचालित रूप से फ़ॉलबैक के रूप में चलता है, या `--deep` के साथ माँग पर चलता है।

---

## 🎯 क्या पहचानता है

### AI उपकरण

| श्रेणी | उपकरण |
|--------|--------|
| इमेज जनरेशन | DALL-E, Midjourney, Stable Diffusion, Adobe Firefly, Imagen, Flux, Ideogram, Leonardo.ai, NovelAI, Grok, Jimeng (即梦) |
| वीडियो जनरेशन | Sora, Google Veo, Runway, Pika, Kling, Vidu, Luma, Hailuo (海螺), Pixverse, Genmo, Haiper |
| ऑडियो/म्यूज़िक जनरेशन | Suno, Udio, ElevenLabs, SoundRaw, AIVA, Boomy, Mubert, Beatoven, Soundful, Hume, Fish Audio |
| मल्टीमोडल | GPT-4o, GPT-4, ChatGPT, OpenAI, GPT Image, Gemini |
| प्लेटफ़ॉर्म | Bing Image Creator, Copilot Designer, Microsoft Designer, Canva AI, DreamStudio, NightCafe, Craiyon, DeepAI, Meta AI, Stability AI |
| इंटरफ़ेस | ComfyUI, Automatic1111 (A1111), InvokeAI, Fooocus |
| रिसर्च | Glide, Parti, Muse, Seedream, Recraft |

### फ़ाइल प्रारूप

| प्रकार | प्रारूप |
|--------|---------|
| इमेज | JPEG, PNG, WebP, AVIF, HEIF, TIFF, GIF, BMP |
| वीडियो | MP4, MOV, AVI, WebM |
| ऑडियो | MP3, M4A, WAV |
| दस्तावेज़ | PDF |

---

## 💻 कमांड

### `aic check [पथ]`

फ़ाइलों में AI-जनरेशन सिग्नल का विश्लेषण करें।

```bash
aic check photo.jpg                       # एकल फ़ाइल
aic check images/ -r                      # डायरेक्टरी, रिकर्सिव
aic check photo.jpg --json                # JSON आउटपुट
aic check photo.jpg -q                    # शांत — केवल एग्ज़िट कोड
aic check photo.jpg --min-confidence medium  # विश्वसनीयता के अनुसार फ़िल्टर
aic check photo.jpg --deep                # पिक्सेल-स्तरीय वॉटरमार्क विश्लेषण को बाध्य करें
```

### `aic info <फ़ाइल>`

सभी उत्पत्ति मेटाडेटा दिखाएं (C2PA मैनिफ़ेस्ट, XMP प्रॉपर्टी, EXIF फ़ील्ड, MP4 एटम, ID3 टैग, WAV मेटाडेटा, वॉटरमार्क विश्लेषण)।

```bash
aic info photo.jpg
```

### ग्लोबल विकल्प

| विकल्प | प्रभाव |
|--------|--------|
| `--json` | JSON प्रारूप में आउटपुट |
| `-q, --quiet` | आउटपुट दबाएं, केवल एग्ज़िट कोड सेट करें |
| `--deep` | सभी फ़ाइलों पर अदृश्य वॉटरमार्क और ऑडियो स्पेक्ट्रल विश्लेषण बाध्य करें |
| `--no-color` | रंगीन आउटपुट अक्षम करें |

### एग्ज़िट कोड

| कोड | अर्थ |
|------|------|
| `0` | AI सिग्नल पाए गए |
| `1` | कोई AI सिग्नल नहीं |
| `2` | त्रुटि |

---

## ⚠️ सीमाएं

- **मेटाडेटा हटाया गया = अदृश्य।** यदि किसी ने मेटाडेटा हटा दिया है, तो पता लगाने के लिए कुछ नहीं है। सोशल प्लेटफ़ॉर्म अपलोड करते समय ऐसा करते हैं — हमेशा मूल फ़ाइल का विश्लेषण करें।
- **अधिकांश AI इमेज में वॉटरमार्क नहीं होता।** केवल ~19% AI इमेज में पता लगाने योग्य उत्पत्ति मार्कर होते हैं (2025 डेटा)।
- **मालिकाना वॉटरमार्क पहुँच से बाहर हैं।** SynthID, Stable Signature, और VideoSeal को ऐसी कुंजियों की आवश्यकता होती है जो हमारे पास नहीं हैं।
- **पिक्सेल-स्तरीय विश्लेषण की सीमाएं हैं।** बिल्ट-इन DWT-DCT वॉटरमार्क डिटेक्टर सामान्य पैटर्न पकड़ता है लेकिन पूर्ण फ़ोरेंसिक क्लासिफ़ायर नहीं है। गहन सांख्यिकीय पहचान के लिए समर्पित फ़ोरेंसिक उपकरणों का उपयोग करें।

---

## 📄 लाइसेंस

[AGPL-3.0](LICENSE)
