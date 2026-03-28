use nene::persist::SaveStore;
use nene::{Deserialize, Serialize};
use std::path::PathBuf;

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Each test gets its own temp directory to avoid interference.
fn tmp_store(name: &str) -> (SaveStore, PathBuf) {
    let dir = std::env::temp_dir().join(format!("nene_save_test_{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    (SaveStore::new(&dir), dir)
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Player {
    name: String,
    level: u32,
    score: f64,
}

// ── Basic get / set ───────────────────────────────────────────────────────────

#[test]
fn set_then_get_primitive() {
    let (mut store, _dir) = tmp_store("prim");
    store.set("s1", "hp", &100u32).unwrap();
    let hp: u32 = store.get("s1", "hp").unwrap();
    assert_eq!(hp, 100);
}

#[test]
fn set_then_get_struct() {
    let (mut store, _dir) = tmp_store("struct");
    let p = Player {
        name: "Alice".into(),
        level: 5,
        score: 3.14,
    };
    store.set("s1", "player", &p).unwrap();
    let loaded: Player = store.get("s1", "player").unwrap();
    assert_eq!(loaded, p);
}

#[test]
fn get_missing_key_returns_none() {
    let (mut store, _dir) = tmp_store("missing_key");
    let v: Option<u32> = store.get("s1", "nope");
    assert!(v.is_none());
}

#[test]
fn get_missing_slot_returns_none() {
    let (mut store, _dir) = tmp_store("missing_slot");
    let v: Option<u32> = store.get("ghost", "x");
    assert!(v.is_none());
}

#[test]
fn overwrite_key() {
    let (mut store, _dir) = tmp_store("overwrite");
    store.set("s1", "x", &1u32).unwrap();
    store.set("s1", "x", &99u32).unwrap();
    let v: u32 = store.get("s1", "x").unwrap();
    assert_eq!(v, 99);
}

// ── has / remove ─────────────────────────────────────────────────────────────

#[test]
fn has_returns_true_after_set() {
    let (mut store, _dir) = tmp_store("has");
    store.set("s1", "k", &42u32).unwrap();
    assert!(store.has("s1", "k"));
}

#[test]
fn has_returns_false_for_missing() {
    let (mut store, _dir) = tmp_store("has_missing");
    assert!(!store.has("s1", "nope"));
}

#[test]
fn remove_key() {
    let (mut store, _dir) = tmp_store("remove");
    store.set("s1", "k", &1u32).unwrap();
    store.remove("s1", "k");
    assert!(!store.has("s1", "k"));
}

// ── flush / reload ────────────────────────────────────────────────────────────

#[test]
fn flush_creates_file() {
    let (mut store, dir) = tmp_store("flush");
    store.set("s1", "x", &7u32).unwrap();
    store.flush("s1").unwrap();
    assert!(dir.join("s1.json").exists());
}

#[test]
fn reload_reads_from_disk() {
    let (mut store, _dir) = tmp_store("reload");
    store.set("s1", "x", &42u32).unwrap();
    store.flush("s1").unwrap();
    store.reload("s1").unwrap();
    let v: u32 = store.get("s1", "x").unwrap();
    assert_eq!(v, 42);
}

#[test]
fn flush_not_called_means_no_file() {
    let (mut store, dir) = tmp_store("no_flush");
    store.set("s1", "x", &1u32).unwrap();
    // No flush → file must not exist
    assert!(!dir.join("s1.json").exists());
}

#[test]
fn flush_idempotent_when_clean() {
    let (mut store, _dir) = tmp_store("idempotent");
    store.set("s1", "x", &1u32).unwrap();
    store.flush("s1").unwrap();
    store.flush("s1").unwrap(); // second flush: no error
}

// ── delete ────────────────────────────────────────────────────────────────────

#[test]
fn delete_removes_file() {
    let (mut store, dir) = tmp_store("delete");
    store.set("s1", "x", &1u32).unwrap();
    store.flush("s1").unwrap();
    assert!(dir.join("s1.json").exists());
    store.delete("s1").unwrap();
    assert!(!dir.join("s1.json").exists());
}

#[test]
fn delete_nonexistent_slot_ok() {
    let (mut store, _dir) = tmp_store("del_noop");
    store.delete("ghost").unwrap(); // should not panic or error
}

// ── list_slots / keys ─────────────────────────────────────────────────────────

#[test]
fn list_slots_empty_dir() {
    let (store, _dir) = tmp_store("list_empty");
    assert!(store.list_slots().is_empty());
}

#[test]
fn list_slots_after_flush() {
    let (mut store, _dir) = tmp_store("list");
    store.set("slot_a", "x", &1u32).unwrap();
    store.set("slot_b", "x", &2u32).unwrap();
    store.flush_all().unwrap();
    let mut slots = store.list_slots();
    slots.sort();
    assert_eq!(slots, ["slot_a", "slot_b"]);
}

#[test]
fn keys_returns_all_keys() {
    let (mut store, _dir) = tmp_store("keys");
    store.set("s1", "a", &1u32).unwrap();
    store.set("s1", "b", &2u32).unwrap();
    let mut keys = store.keys("s1");
    keys.sort();
    assert_eq!(keys, ["a", "b"]);
}

// ── exists ────────────────────────────────────────────────────────────────────

#[test]
fn exists_false_before_flush() {
    let (mut store, _dir) = tmp_store("exists_false");
    store.set("s1", "x", &1u32).unwrap();
    assert!(!store.exists("s1")); // not on disk yet
}

#[test]
fn exists_true_after_flush() {
    let (mut store, _dir) = tmp_store("exists_true");
    store.set("s1", "x", &1u32).unwrap();
    store.flush("s1").unwrap();
    assert!(store.exists("s1"));
}

// ── multiple slots ────────────────────────────────────────────────────────────

#[test]
fn multiple_slots_independent() {
    let (mut store, _dir) = tmp_store("multi");
    store.set("a", "x", &1u32).unwrap();
    store.set("b", "x", &99u32).unwrap();
    let a: u32 = store.get("a", "x").unwrap();
    let b: u32 = store.get("b", "x").unwrap();
    assert_eq!(a, 1);
    assert_eq!(b, 99);
}

// ── string / vec / bool values ────────────────────────────────────────────────

#[test]
fn save_string() {
    let (mut store, _dir) = tmp_store("str");
    store.set("s1", "name", &"hello").unwrap();
    let v: String = store.get("s1", "name").unwrap();
    assert_eq!(v, "hello");
}

#[test]
fn save_vec() {
    let (mut store, _dir) = tmp_store("vec");
    store.set("s1", "items", &vec![1u32, 2, 3]).unwrap();
    let v: Vec<u32> = store.get("s1", "items").unwrap();
    assert_eq!(v, [1, 2, 3]);
}

#[test]
fn save_bool() {
    let (mut store, _dir) = tmp_store("bool");
    store.set("s1", "flag", &true).unwrap();
    let v: bool = store.get("s1", "flag").unwrap();
    assert!(v);
}
