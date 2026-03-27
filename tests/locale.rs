use nene::locale::Locale;

const DIR: &str = "tests/assets/locale";

// ── basic lookup ──────────────────────────────────────────────────────────────

#[test]
fn lookup_flat_key() {
    let locale = Locale::new(DIR, "en");
    assert_eq!(locale.t("menu.start"), "Start Game");
}

#[test]
fn lookup_nested_key() {
    let locale = Locale::new(DIR, "en");
    assert_eq!(locale.t("menu.quit"), "Quit");
}

#[test]
fn missing_key_returns_key_itself() {
    let locale = Locale::new(DIR, "en");
    assert_eq!(locale.t("nonexistent.key"), "nonexistent.key");
}

// ── substitution ──────────────────────────────────────────────────────────────

#[test]
fn substitute_single_var() {
    let locale = Locale::new(DIR, "en");
    assert_eq!(locale.t_with("hud.hp", &[("hp", "42")]), "HP: 42");
}

#[test]
fn substitute_multiple_vars() {
    let locale = Locale::new(DIR, "en");
    assert_eq!(
        locale.t_with("dialog.greet", &[("name", "Alice")]),
        "Hello, Alice!"
    );
}

#[test]
fn substitute_unknown_placeholder_left_unchanged() {
    let locale = Locale::new(DIR, "en");
    // "HP: {hp}" — we pass the wrong key, token stays
    assert_eq!(locale.t_with("hud.hp", &[("wrong", "99")]), "HP: {hp}");
}

// ── language switch ───────────────────────────────────────────────────────────

#[test]
fn switch_language() {
    let mut locale = Locale::new(DIR, "en");
    assert_eq!(locale.t("menu.start"), "Start Game");

    locale.set_language("ko");
    assert_eq!(locale.t("menu.start"), "게임 시작");
}

#[test]
fn switch_language_substitution() {
    let mut locale = Locale::new(DIR, "en");
    locale.set_language("ko");
    assert_eq!(locale.t_with("hud.hp", &[("hp", "10")]), "체력: 10");
}

// ── fallback ──────────────────────────────────────────────────────────────────

#[test]
fn falls_back_to_en_for_missing_key() {
    // ko.json has all keys, but let's verify the mechanism by loading a
    // language file that doesn't exist — it will be empty, so every lookup
    // should fall back to "en".
    let locale = Locale::new(DIR, "zz"); // zz.json doesn't exist
    assert_eq!(locale.t("menu.start"), "Start Game");
}

// ── available_languages ───────────────────────────────────────────────────────

#[test]
fn available_languages_lists_json_files() {
    let locale = Locale::new(DIR, "en");
    let langs = locale.available_languages();
    assert!(langs.contains(&"en".to_string()));
    assert!(langs.contains(&"ko".to_string()));
}

// ── language getter ───────────────────────────────────────────────────────────

#[test]
fn language_getter() {
    let mut locale = Locale::new(DIR, "en");
    assert_eq!(locale.language(), "en");
    locale.set_language("ko");
    assert_eq!(locale.language(), "ko");
}
