use std::collections::HashMap;
use std::sync::OnceLock;

const EN_US: &str = include_str!("../lang/en_us.json");
const DE_DE: &str = include_str!("../lang/de_de.json");

struct Messages {
    map: HashMap<String, String>,
}

impl Messages {
    fn parse(raw: &str) -> Self {
        let map = foundation::serialization::JSONSerialization::parse_flat_string_map(raw)
            .expect("built-in language file is invalid");
        Self { map }
    }

    fn get(&self, key: &str) -> Option<&str> {
        self.map.get(key).map(String::as_str)
    }
}

static MESSAGES: OnceLock<Messages> = OnceLock::new();

pub fn current_locale() -> &'static str {
    let lang = std::env::var("LC_ALL")
        .or_else(|_| std::env::var("LC_MESSAGES"))
        .or_else(|_| std::env::var("LANG"))
        .unwrap_or_default();

    if lang.to_lowercase().starts_with("de") {
        "de_de"
    } else {
        "en_us"
    }
}

fn messages() -> &'static Messages {
    MESSAGES.get_or_init(|| {
        let raw = match current_locale() {
            "de_de" => DE_DE,
            _ => EN_US,
        };
        Messages::parse(raw)
    })
}

pub fn t(key: &str) -> String {
    match messages().get(key) {
        Some(msg) => msg.to_string(),
        None => key.to_string(),
    }
}

pub fn t_fmt(key: &str, arg: &str) -> String {
    t(key).replace("{}", arg)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_files_parse() {
        let en = Messages::parse(EN_US);
        let de = Messages::parse(DE_DE);
        assert!(!en.get("not_available").unwrap_or_default().is_empty());
        assert_ne!(en.get("not_available"), de.get("not_available"));
    }

    #[test]
    fn format_replaces_placeholder() {
        assert_eq!(
            t_fmt("provider_failed", "boom"),
            t("provider_failed").replace("{}", "boom")
        );
    }

    #[test]
    fn locale_detection() {
        std::env::set_var("LANG", "de_DE.UTF-8");
        assert_eq!(current_locale(), "de_de");
        std::env::set_var("LANG", "en_US.UTF-8");
        assert_eq!(current_locale(), "en_us");
        std::env::remove_var("LANG");
    }
}
