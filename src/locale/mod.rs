//! Localization — look up translated strings from an in-memory map.
//!
//! [`Locale`] holds two flat `key → string` maps (active + optional fallback)
//! and knows nothing about files or formats.  Load your translations however
//! you like — [`from_json`] is provided as a convenience for the common case.
//!
//! ```
//! use nene::locale::{Locale, from_json};
//!
//! let en = from_json(r#"{"menu":{"start":"Start Game"},"hud":{"hp":"HP: {hp}"}}"#);
//! let ko = from_json(r#"{"menu":{"start":"게임 시작"},"hud":{"hp":"체력: {hp}"}}"#);
//!
//! let mut locale = Locale::new(ko).with_fallback(en);
//!
//! assert_eq!(locale.t("menu.start"), "게임 시작");
//! assert_eq!(locale.t_with("hud.hp", &[("hp", "42")]), "체력: 42");
//!
//! // Switch language at runtime.
//! locale.set(from_json(r#"{"menu":{"start":"Start Game"}}"#));
//! assert_eq!(locale.t("menu.start"), "Start Game");
//! ```

use std::borrow::Cow;
use std::collections::HashMap;

// ── Locale ────────────────────────────────────────────────────────────────────

/// Runtime localization store.
///
/// Holds an active translation map and an optional fallback map.
/// Key lookup checks the active map first, then the fallback, then returns
/// the key itself as a visible placeholder.
pub struct Locale {
    active: HashMap<String, String>,
    fallback: HashMap<String, String>,
}

impl Locale {
    /// Create a `Locale` from an active translation map with no fallback.
    pub fn new(active: HashMap<String, String>) -> Self {
        Self {
            active,
            fallback: HashMap::new(),
        }
    }

    /// Set a fallback map used when a key is absent from the active map.
    pub fn with_fallback(mut self, fallback: HashMap<String, String>) -> Self {
        self.fallback = fallback;
        self
    }

    /// Replace the active translation map (e.g. on a language switch).
    pub fn set(&mut self, active: HashMap<String, String>) {
        self.active = active;
    }

    /// Replace the fallback translation map.
    pub fn set_fallback(&mut self, fallback: HashMap<String, String>) {
        self.fallback = fallback;
    }

    /// Look up a translation by key.
    ///
    /// Returns `Cow::Borrowed` (zero-copy) when the key is found, or
    /// `Cow::Owned(key.to_owned())` as a visible placeholder when missing.
    ///
    /// Resolution order:
    /// 1. Active map
    /// 2. Fallback map
    /// 3. The key itself
    pub fn t(&self, key: &str) -> Cow<'_, str> {
        self.active
            .get(key)
            .or_else(|| self.fallback.get(key))
            .map(|s| Cow::Borrowed(s.as_str()))
            .unwrap_or_else(|| Cow::Owned(key.to_owned()))
    }

    /// Look up a translation and substitute `{placeholder}` variables.
    ///
    /// `vars` is a slice of `(placeholder_name, value)` pairs.
    ///
    /// ```no_run
    /// # use nene::locale::{Locale, from_json};
    /// # let locale = Locale::new(from_json("{}"));
    /// // "Hello, {name}! You have {n} messages."
    /// locale.t_with("inbox.greeting", &[("name", "Alice"), ("n", "3")]);
    /// // → "Hello, Alice! You have 3 messages."
    /// ```
    pub fn t_with(&self, key: &str, vars: &[(&str, &str)]) -> String {
        substitute(&self.t(key), vars)
    }
}

// ── JSON helper ───────────────────────────────────────────────────────────────

/// Parse a JSON locale string into a flat `key → value` map.
///
/// Nested objects are flattened with dot-separated keys:
/// `{ "menu": { "start": "Start" } }` → `"menu.start" → "Start"`.
///
/// Returns an empty map on parse error.
pub fn from_json(text: &str) -> HashMap<String, String> {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(text) else {
        return HashMap::new();
    };
    let mut map = HashMap::new();
    flatten(&value, String::new(), &mut map);
    map
}

// ── private helpers ───────────────────────────────────────────────────────────

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
            out.insert(prefix, other.to_string());
        }
    }
}

fn substitute(template: &str, vars: &[(&str, &str)]) -> String {
    let mut result = template.to_owned();
    for (name, value) in vars {
        result = result.replace(&format!("{{{name}}}"), value);
    }
    result
}
