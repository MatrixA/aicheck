/// Known AI content generation tool name patterns (case-insensitive matching).
pub const AI_TOOL_PATTERNS: &[&str] = &[
    "dall-e",
    "dall·e",
    "dalle",
    "midjourney",
    "stable diffusion",
    "stablediffusion",
    "comfyui",
    "automatic1111",
    "a1111",
    "novelai",
    "adobe firefly",
    "firefly",
    "bing image creator",
    "copilot designer",
    "microsoft designer",
    "imagen",
    "ideogram",
    "flux",
    "leonardo.ai",
    "leonardo ai",
    "runway",
    "pika",
    "sora",
    "google veo",
    "veo",
    "kling",
    "vidu",
    "meta ai",
    "canva ai",
    "stability ai",
    "invoke ai",
    "invokeai",
    "dreamstudio",
    "nightcafe",
    "craiyon",
    "glide",
    "parti",
    "muse",
    "deepai",
    "foocus",
    "fooocus",
    "gpt-4o",
    "gpt-4",
    "gpt image",
    "openai",
    "chatgpt",
    "seedream",
    "recraft",
    "elevenlabs",
    "eleven labs",
    "suno",
    "udio",
    "soundraw",
    "aiva",
    "boomy",
    "mubert",
    "loudly.com",
    "beatoven",
    "soundful",
    // New image generation tools
    "grok",
    "gemini",
    "google ai",
    "jimeng",
    "即梦",
    "dreamina",
    "qwen",
    "通义万相",
    // New video generation tools
    "wan",
    "luma",
    "hailuo",
    "海螺",
    "pixverse",
    "genmo",
    "haiper",
    // New audio generation tools
    "hume",
    "fish audio",
];

/// Check if a string contains any known AI tool pattern (case-insensitive).
/// Returns the matched tool name if found.
///
/// ASCII patterns must match on word boundaries (the neighbouring characters
/// must not be ASCII letters or digits), so short names such as "wan", "veo"
/// or "udio" do not fire inside unrelated words ("swan", "audio").
pub fn match_ai_tool(text: &str) -> Option<&'static str> {
    let lower = text.to_lowercase();
    AI_TOOL_PATTERNS
        .iter()
        .find(|&&pattern| contains_word(&lower, pattern))
        .copied()
}

fn contains_word(haystack: &str, pattern: &str) -> bool {
    if !pattern.is_ascii() {
        return haystack.contains(pattern);
    }
    let bytes = haystack.as_bytes();
    let mut from = 0;
    while let Some(rel) = haystack[from..].find(pattern) {
        let start = from + rel;
        let end = start + pattern.len();
        let before_ok = start == 0 || !bytes[start - 1].is_ascii_alphanumeric();
        let after_ok = end == bytes.len() || !bytes[end].is_ascii_alphanumeric();
        if before_ok && after_ok {
            return true;
        }
        // Advance by one character (keep UTF-8 boundaries).
        from = start + haystack[start..].chars().next().map_or(1, |c| c.len_utf8());
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_match_known_tools() {
        assert_eq!(match_ai_tool("DALL-E 3/OpenAI"), Some("dall-e"));
        assert_eq!(match_ai_tool("Adobe Firefly 2.0"), Some("adobe firefly"));
        assert_eq!(match_ai_tool("Canon EOS R5"), None);
        assert_eq!(match_ai_tool("ComfyUI v1.2"), Some("comfyui"));
        assert_eq!(match_ai_tool("Midjourney v6"), Some("midjourney"));
        assert_eq!(match_ai_tool("Dreamina by ByteDance"), Some("dreamina"));
        assert_eq!(match_ai_tool("Made with Google AI"), Some("google ai"));
        assert_eq!(match_ai_tool("Wan 2.6"), Some("wan"));
        assert_eq!(match_ai_tool("Qwen VL Image"), Some("qwen"));
        assert_eq!(match_ai_tool("Adobe_Firefly"), Some("firefly"));
        assert_eq!(match_ai_tool("doubao-seedream-4-0"), Some("seedream"));
        assert_eq!(match_ai_tool("kling-ai"), Some("kling"));
    }

    #[test]
    fn test_word_boundaries() {
        assert_eq!(match_ai_tool("gpt-4o-audio-preview"), Some("gpt-4o"));
        assert_eq!(match_ai_tool("Black swan photo"), None);
        assert_eq!(match_ai_tool("Lavf60 audio"), None);
        assert_eq!(match_ai_tool("Developer settings"), None);
        assert_eq!(match_ai_tool("Museum guide"), None);
        assert_eq!(match_ai_tool("即梦AI"), Some("即梦"));
    }
}
