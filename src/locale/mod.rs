//! Localization — load language files and look up translated strings.
//!
//! Language files are plain JSON objects stored under a common directory,
//! one file per language:
//!
//! ```text
//! assets/locale/en.json
//! assets/locale/ko.json
//! assets/locale/ja.json
//! ```
//!
//! Each file maps dot-separated keys to translated strings.  Values may
//! contain `{name}`-style placeholders that are filled in at call time.
//!
//! ```json
//! {
//!   "menu.start":  "Start Game",
//!   "hud.hp":      "HP: {hp}",
//!   "dialog.greet": "Hello, {name}!"
//! }
//! ```
//!
//! # Quick start
//! ```no_run
//! use nene::locale::Locale;
//!
//! let mut locale = Locale::new("assets/locale", "en");
//!
//! assert_eq!(locale.t("menu.start"), "Start Game");
//! assert_eq!(locale.t_with("hud.hp", &[("hp", "42")]), "HP: 42");
//!
//! // Switch language at runtime (e.g. from the settings screen).
//! locale.set_language("ko");
//! assert_eq!(locale.t("menu.start"), "게임 시작");
//! ```

use std::borrow::Cow;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

// ── Locale ────────────────────────────────────────────────────────────────────

/// Runtime localization store.
///
/// Holds the currently active language's translations and, optionally, a
/// fallback language (default: `"en"`) used when a key is missing from the
/// active language.
pub struct Locale {
    dir: PathBuf,
    language: String,
    fallback: String,
    /// Active language strings.
    active: HashMap<String, String>,
    /// Fallback language strings (empty when active == fallback).
    fallback_map: HashMap<String, String>,
}

impl Locale {
    /// Create a new `Locale`, loading `language` from `dir`.
    ///
    /// Falls back to `"en"` when a key is absent.  If `language == "en"` the
    /// fallback map is not loaded separately.
    ///
    /// Missing files are silently treated as empty maps.
    pub fn new(dir: impl AsRef<Path>, language: impl Into<String>) -> Self {
        let dir = dir.as_ref().to_path_buf();
        let language = language.into();
        let fallback = "en".to_string();

        let active = load_file(&dir, &language);
        let fallback_map = if language != fallback {
            load_file(&dir, &fallback)
        } else {
            HashMap::new()
        };

        Self {
            dir,
            language,
            fallback,
            active,
            fallback_map,
        }
    }

    /// Override the fallback language (default: `"en"`).
    pub fn with_fallback(mut self, fallback: impl Into<String>) -> Self {
        self.fallback = fallback.into();
        self.reload_fallback();
        self
    }

    /// Switch to a different language, reloading translations from disk.
    pub fn set_language(&mut self, language: impl Into<String>) {
        self.language = language.into();
        self.active = load_file(&self.dir, &self.language);
        self.reload_fallback();
    }

    /// Currently active language code (e.g. `"ko"`).
    pub fn language(&self) -> &str {
        &self.language
    }

    /// Look up a translation by key.
    ///
    /// Returns `Cow::Borrowed` (zero-copy) when the key is found, or
    /// `Cow::Owned(key.to_owned())` as a visible placeholder when missing.
    ///
    /// Resolution order:
    /// 1. Active language map
    /// 2. Fallback language map
    /// 3. The key itself
    pub fn t(&self, key: &str) -> Cow<'_, str> {
        self.active
            .get(key)
            .or_else(|| self.fallback_map.get(key))
            .map(|s| Cow::Borrowed(s.as_str()))
            .unwrap_or_else(|| Cow::Owned(key.to_owned()))
    }

    /// Look up a translation and substitute `{placeholder}` variables.
    ///
    /// `vars` is a slice of `(placeholder_name, value)` pairs.  Any
    /// `{name}` token in the translated string is replaced with the
    /// corresponding value.  Unknown placeholders are left unchanged.
    ///
    /// ```no_run
    /// # use nene::locale::Locale;
    /// # let locale = Locale::new("assets/locale", "en");
    /// // "Hello, {name}! You have {n} messages."
    /// locale.t_with("inbox.greeting", &[("name", "Alice"), ("n", "3")]);
    /// // → "Hello, Alice! You have 3 messages."
    /// ```
    pub fn t_with(&self, key: &str, vars: &[(&str, &str)]) -> String {
        let template = self.t(key);
        substitute(&template, vars)
    }

    /// Returns all language codes found in the locale directory.
    ///
    /// Each `.json` file whose stem is a valid language code is included.
    /// The list is sorted alphabetically.
    pub fn available_languages(&self) -> Vec<String> {
        let Ok(rd) = std::fs::read_dir(&self.dir) else {
            return Vec::new();
        };
        let mut langs: Vec<String> = rd
            .filter_map(|e| {
                let e = e.ok()?;
                let p = e.path();
                if p.extension()?.to_str()? == "json" {
                    Some(p.file_stem()?.to_str()?.to_owned())
                } else {
                    None
                }
            })
            .collect();
        langs.sort();
        langs
    }

    // ── private ───────────────────────────────────────────────────────────────

    fn reload_fallback(&mut self) {
        self.fallback_map = if self.language != self.fallback {
            load_file(&self.dir, &self.fallback)
        } else {
            HashMap::new()
        };
    }
}

// ── helpers ───────────────────────────────────────────────────────────────────

/// Load `<dir>/<lang>.json` into a flat string map.  Returns an empty map on
/// any I/O or parse error.
fn load_file(dir: &Path, lang: &str) -> HashMap<String, String> {
    let path = dir.join(format!("{lang}.json"));
    let Ok(text) = std::fs::read_to_string(&path) else {
        return HashMap::new();
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) else {
        return HashMap::new();
    };
    let mut map = HashMap::new();
    flatten(&value, String::new(), &mut map);
    map
}

/// Recursively flatten a JSON object into dot-separated keys.
///
/// ```json
/// { "menu": { "start": "Start" } }
/// ```
/// becomes `"menu.start" → "Start"`.
fn flatten(value: &serde_json::Value, prefix: String, out: &mut HashMap<String, String>) {
    match value {
        serde_json::Value::Object(obj) => {
            for (k, v) in obj {
                let key = if prefix.is_empty() {
                    k.clone()
                } else {
                    format!("{prefix}.{k}")
                };
                flatten(v, key, out);
            }
        }
        serde_json::Value::String(s) => {
            out.insert(prefix, s.clone());
        }
        other => {
            // Numbers / bools / arrays: stringify so they are usable.
            out.insert(prefix, other.to_string());
        }
    }
}

/// Replace `{key}` tokens in `template` with values from `vars`.
fn substitute(template: &str, vars: &[(&str, &str)]) -> String {
    let mut result = template.to_owned();
    for (name, value) in vars {
        let token = format!("{{{name}}}");
        result = result.replace(&token, value);
    }
    result
}
