/// Internationalization helpers.
///
/// Provides locale detection, initialization, and message rendering with
/// named parameter substitution.

/// Supported locale codes.
const SUPPORTED_LOCALES: &[&str] = &["en", "zh-CN", "de", "ja", "ko", "hi", "es"];

/// Map a raw locale string (e.g. "zh_CN.UTF-8", "de_DE") to a supported locale code.
fn normalize_locale(raw: &str) -> &'static str {
    let lower = raw.to_lowercase();

    // Try exact match first
    for &loc in SUPPORTED_LOCALES {
        if lower == loc.to_lowercase() {
            return loc;
        }
    }

    // Try prefix match (e.g. "zh_cn.utf-8" -> "zh-CN", "de_de.utf-8" -> "de")
    let normalized = lower.replace('_', "-").replace(".utf-8", "");
    for &loc in SUPPORTED_LOCALES {
        if normalized.starts_with(&loc.to_lowercase()) {
            return loc;
        }
    }

    // Try language-only match (e.g. "zh" -> "zh-CN")
    let lang = normalized.split('-').next().unwrap_or("");
    match lang {
        "zh" => "zh-CN",
        "de" => "de",
        "ja" => "ja",
        "ko" => "ko",
        "hi" => "hi",
        "es" => "es",
        _ => "en",
    }
}

/// Initialize the locale from --lang flag or system locale.
pub fn init_locale(lang_override: Option<&str>) {
    let locale = match lang_override {
        Some(lang) => normalize_locale(lang),
        None => {
            let sys = sys_locale::get_locale().unwrap_or_else(|| "en".to_string());
            normalize_locale(&sys)
        }
    };
    rust_i18n::set_locale(locale);
}

/// Render a translated message with named parameter substitution.
///
/// Parameters in the translation string use the `%{name}` syntax.
pub fn t(key: &str, params: &[(&str, &str)]) -> String {
    let mut msg = rust_i18n::t!(key).to_string();
    for &(name, value) in params {
        msg = msg.replace(&format!("%{{{}}}", name), value);
    }
    msg
}

/// Render a translated message in English (for JSON output / stored descriptions).
pub fn t_en(key: &str, params: &[(&str, &str)]) -> String {
    let mut msg = rust_i18n::t!(key, locale = "en").to_string();
    for &(name, value) in params {
        msg = msg.replace(&format!("%{{{}}}", name), value);
    }
    msg
}
