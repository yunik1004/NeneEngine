use nene::locale::{Locale, from_json};

fn en() -> std::collections::HashMap<String, String> {
    from_json(include_str!("assets/locale/en.json"))
}
fn ko() -> std::collections::HashMap<String, String> {
    from_json(include_str!("assets/locale/ko.json"))
}

// ── basic lookup ──────────────────────────────────────────────────────────────

#[test]
fn lookup_flat_key() {
    assert_eq!(Locale::new(en()).t("menu.start"), "Start Game");
}

#[test]
fn lookup_nested_key() {
    assert_eq!(Locale::new(en()).t("menu.quit"), "Quit");
}

#[test]
fn missing_key_returns_key_itself() {
    assert_eq!(Locale::new(en()).t("nonexistent.key"), "nonexistent.key");
}

// ── substitution ──────────────────────────────────────────────────────────────

#[test]
fn substitute_single_var() {
    assert_eq!(
        Locale::new(en()).t_with("hud.hp", &[("hp", "42")]),
        "HP: 42"
    );
}

#[test]
fn substitute_multiple_vars() {
    assert_eq!(
        Locale::new(en()).t_with("dialog.greet", &[("name", "Alice")]),
        "Hello, Alice!"
    );
}

#[test]
fn substitute_unknown_placeholder_left_unchanged() {
    assert_eq!(
        Locale::new(en()).t_with("hud.hp", &[("wrong", "99")]),
        "HP: {hp}"
    );
}

// ── language switch ───────────────────────────────────────────────────────────

#[test]
fn switch_language() {
    let mut locale = Locale::new(en());
    assert_eq!(locale.t("menu.start"), "Start Game");

    locale.set(ko());
    assert_eq!(locale.t("menu.start"), "게임 시작");
}

#[test]
fn switch_language_substitution() {
    let locale = Locale::new(ko());
    assert_eq!(locale.t_with("hud.hp", &[("hp", "10")]), "체력: 10");
}

// ── fallback ──────────────────────────────────────────────────────────────────

#[test]
fn falls_back_for_missing_key() {
    let locale = Locale::new(std::collections::HashMap::new()).with_fallback(en());
    assert_eq!(locale.t("menu.start"), "Start Game");
}

#[test]
fn no_fallback_returns_key() {
    let locale = Locale::new(std::collections::HashMap::new());
    assert_eq!(locale.t("menu.start"), "menu.start");
}
