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
    "jimeng",
    "即梦",
    // New video generation tools
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
pub fn match_ai_tool(text: &str) -> Option<&'static str> {
    let lower = text.to_lowercase();
    AI_TOOL_PATTERNS
        .iter()
        .find(|&&pattern| lower.contains(pattern))
        .copied()
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
    }
}
