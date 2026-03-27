use nene::settings::Settings;
use nene::{Deserialize, Serialize};

fn tmp(name: &str) -> Settings {
    let path = std::env::temp_dir().join(format!("nene_settings_{name}.json"));
    let _ = std::fs::remove_file(&path);
    Settings::new(&path)
}

// ── get / set ─────────────────────────────────────────────────────────────────

#[test]
fn set_then_get() {
    let mut s = tmp("set_get");
    s.set("volume", &0.8f32).unwrap();
    let v: f32 = s.get("volume").unwrap();
    assert!((v - 0.8).abs() < 1e-6);
}

#[test]
fn get_missing_returns_none() {
    let mut s = tmp("missing");
    let v: Option<u32> = s.get("nope");
    assert!(v.is_none());
}

#[test]
fn overwrite_value() {
    let mut s = tmp("overwrite");
    s.set("x", &1u32).unwrap();
    s.set("x", &42u32).unwrap();
    let v: u32 = s.get("x").unwrap();
    assert_eq!(v, 42);
}

// ── defaults ──────────────────────────────────────────────────────────────────

#[test]
fn register_default_returned_when_absent() {
    let mut s = tmp("default");
    s.register("volume", 1.0f32);
    let v: f32 = s.get("volume").unwrap();
    assert!((v - 1.0).abs() < 1e-6);
}

#[test]
fn set_overrides_default() {
    let mut s = tmp("override");
    s.register("volume", 1.0f32);
    s.set("volume", &0.5f32).unwrap();
    let v: f32 = s.get("volume").unwrap();
    assert!((v - 0.5).abs() < 1e-6);
}

#[test]
fn default_not_written_to_store() {
    let mut s = tmp("default_not_stored");
    s.register("volume", 1.0f32);
    assert!(!s.has("volume"));
}

// ── has / remove / reset ──────────────────────────────────────────────────────

#[test]
fn has_false_before_set() {
    let mut s = tmp("has_false");
    assert!(!s.has("x"));
}

#[test]
fn has_true_after_set() {
    let mut s = tmp("has_true");
    s.set("x", &1u32).unwrap();
    assert!(s.has("x"));
}

#[test]
fn remove_clears_key() {
    let mut s = tmp("remove");
    s.set("x", &1u32).unwrap();
    s.remove("x");
    assert!(!s.has("x"));
}

#[test]
fn reset_falls_back_to_default() {
    let mut s = tmp("reset");
    s.register("vol", 1.0f32);
    s.set("vol", &0.3f32).unwrap();
    s.reset("vol");
    let v: f32 = s.get("vol").unwrap();
    assert!((v - 1.0).abs() < 1e-6);
}

#[test]
fn reset_all_clears_stored_keys() {
    let mut s = tmp("reset_all");
    s.set("a", &1u32).unwrap();
    s.set("b", &2u32).unwrap();
    s.reset_all();
    assert!(s.keys().is_empty());
}

// ── persistence ───────────────────────────────────────────────────────────────

#[test]
fn save_creates_file() {
    let path = std::env::temp_dir().join("nene_settings_save_file.json");
    let _ = std::fs::remove_file(&path);
    let mut s = Settings::new(&path);
    s.set("x", &99u32).unwrap();
    s.save().unwrap();
    assert!(path.exists());
}

#[test]
fn reload_reads_from_disk() {
    let path = std::env::temp_dir().join("nene_settings_reload.json");
    let _ = std::fs::remove_file(&path);
    let mut s = Settings::new(&path);
    s.set("x", &7u32).unwrap();
    s.save().unwrap();
    s.reload().unwrap();
    let v: u32 = s.get("x").unwrap();
    assert_eq!(v, 7);
}

#[test]
fn exists_false_before_save() {
    let mut s = tmp("exists_false");
    s.set("x", &1u32).unwrap();
    assert!(!s.exists());
}

#[test]
fn exists_true_after_save() {
    let path = std::env::temp_dir().join("nene_settings_exists.json");
    let _ = std::fs::remove_file(&path);
    let mut s = Settings::new(&path);
    s.set("x", &1u32).unwrap();
    s.save().unwrap();
    assert!(s.exists());
}

// ── struct value ──────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, PartialEq, Debug)]
struct Resolution {
    w: u32,
    h: u32,
}

#[test]
fn save_struct_value() {
    let mut s = tmp("struct_val");
    let r = Resolution { w: 1920, h: 1080 };
    s.set("resolution", &r).unwrap();
    let loaded: Resolution = s.get("resolution").unwrap();
    assert_eq!(loaded, r);
}

#[test]
fn register_struct_default() {
    let mut s = tmp("struct_default");
    s.register("resolution", Resolution { w: 1280, h: 720 });
    let r: Resolution = s.get("resolution").unwrap();
    assert_eq!(r, Resolution { w: 1280, h: 720 });
}
