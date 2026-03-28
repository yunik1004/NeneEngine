//! Save / load system.
//!
//! Stores named slots as JSON files under a configurable directory.
//! Each slot is a flat key-value map where values are JSON-serialisable.
//!
//! # Quick start
//! ```no_run
//! use nene::persist::SaveStore;
//! use serde::{Serialize, Deserialize};
//!
//! #[derive(Serialize, Deserialize)]
//! struct Progress { level: u32, score: u32 }
//!
//! let mut store = SaveStore::new("saves");
//!
//! // Write
//! store.set("slot1", "progress", &Progress { level: 3, score: 1200 }).unwrap();
//! store.flush("slot1").unwrap();
//!
//! // Read
//! let p: Progress = store.get("slot1", "progress").unwrap();
//! assert_eq!(p.level, 3);
//! ```

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;

// ── SaveSlot ──────────────────────────────────────────────────────────────────

/// An in-memory key → JSON-value map for one save slot.
#[derive(Default)]
struct SaveSlot {
    data: HashMap<String, Value>,
    dirty: bool,
}

// ── SaveStore ─────────────────────────────────────────────────────────────────

/// Manages multiple named save slots backed by JSON files on disk.
///
/// Slots are loaded lazily on first access and written only when
/// [`flush`](Self::flush) or [`flush_all`](Self::flush_all) is called.
pub struct SaveStore {
    dir: PathBuf,
    slots: HashMap<String, SaveSlot>,
}

impl SaveStore {
    /// Create a store that reads/writes files inside `dir`.
    ///
    /// The directory is created if it does not exist.
    pub fn new(dir: impl AsRef<Path>) -> Self {
        let dir = dir.as_ref().to_path_buf();
        Self {
            dir,
            slots: HashMap::new(),
        }
    }

    // ── Reading ───────────────────────────────────────────────────────────────

    /// Return `true` if the slot file exists on disk.
    pub fn exists(&self, slot: &str) -> bool {
        self.slot_path(slot).exists()
    }

    /// Read and deserialise the value stored under `key` in `slot`.
    ///
    /// Loads from disk if the slot is not yet in memory.
    /// Returns `None` if the key is absent or the slot does not exist.
    pub fn get<T: DeserializeOwned>(&mut self, slot: &str, key: &str) -> Option<T> {
        self.ensure_loaded(slot);
        let value = self.slots.get(slot)?.data.get(key)?;
        serde_json::from_value(value.clone()).ok()
    }

    /// Return `true` if `key` exists in `slot`.
    pub fn has(&mut self, slot: &str, key: &str) -> bool {
        self.ensure_loaded(slot);
        self.slots
            .get(slot)
            .is_some_and(|s| s.data.contains_key(key))
    }

    /// Return the names of all slots that have a file on disk.
    pub fn list_slots(&self) -> Vec<String> {
        let Ok(rd) = std::fs::read_dir(&self.dir) else {
            return Vec::new();
        };
        rd.filter_map(|e| {
            let e = e.ok()?;
            let name = e.file_name().into_string().ok()?;
            name.strip_suffix(".json").map(str::to_owned)
        })
        .collect()
    }

    /// Return all keys present in `slot`.
    pub fn keys(&mut self, slot: &str) -> Vec<String> {
        self.ensure_loaded(slot);
        self.slots
            .get(slot)
            .map(|s| s.data.keys().cloned().collect())
            .unwrap_or_default()
    }

    // ── Writing ───────────────────────────────────────────────────────────────

    /// Serialise and store `value` under `key` in `slot` (in memory only).
    ///
    /// Call [`flush`](Self::flush) to persist to disk.
    pub fn set<T: Serialize>(&mut self, slot: &str, key: &str, value: &T) -> Result<(), SaveError> {
        self.ensure_loaded(slot);
        let json = serde_json::to_value(value).map_err(SaveError::Serialize)?;
        let s = self.slots.entry(slot.to_owned()).or_default();
        s.data.insert(key.to_owned(), json);
        s.dirty = true;
        Ok(())
    }

    /// Remove `key` from `slot`. Does nothing if the key is absent.
    pub fn remove(&mut self, slot: &str, key: &str) {
        if let Some(s) = self.slots.get_mut(slot)
            && s.data.remove(key).is_some()
        {
            s.dirty = true;
        }
    }

    // ── Persistence ───────────────────────────────────────────────────────────

    /// Write `slot` to disk if it has unsaved changes.
    pub fn flush(&mut self, slot: &str) -> Result<(), SaveError> {
        let Some(s) = self.slots.get(slot) else {
            return Ok(());
        };
        if !s.dirty {
            return Ok(());
        }
        let json = serde_json::to_string_pretty(&s.data).map_err(SaveError::Serialize)?;
        let path = self.slot_path(slot);
        std::fs::create_dir_all(&self.dir).map_err(SaveError::Io)?;
        std::fs::write(&path, json).map_err(SaveError::Io)?;
        self.slots.get_mut(slot).unwrap().dirty = false;
        Ok(())
    }

    /// Write all slots that have unsaved changes.
    pub fn flush_all(&mut self) -> Result<(), SaveError> {
        let keys: Vec<String> = self.slots.keys().cloned().collect();
        for key in keys {
            self.flush(&key)?;
        }
        Ok(())
    }

    /// Delete a slot's file from disk and remove it from memory.
    pub fn delete(&mut self, slot: &str) -> Result<(), SaveError> {
        self.slots.remove(slot);
        let path = self.slot_path(slot);
        if path.exists() {
            std::fs::remove_file(&path).map_err(SaveError::Io)?;
        }
        Ok(())
    }

    /// Reload a slot from disk, discarding any in-memory changes.
    pub fn reload(&mut self, slot: &str) -> Result<(), SaveError> {
        self.slots.remove(slot);
        self.ensure_loaded(slot);
        Ok(())
    }

    // ── Internal ──────────────────────────────────────────────────────────────

    fn slot_path(&self, slot: &str) -> PathBuf {
        self.dir.join(format!("{slot}.json"))
    }

    /// Load slot from disk into memory if not already present.
    fn ensure_loaded(&mut self, slot: &str) {
        if self.slots.contains_key(slot) {
            return;
        }
        let path = self.slot_path(slot);
        let data: HashMap<String, Value> = if path.exists() {
            std::fs::read_to_string(&path)
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default()
        } else {
            HashMap::new()
        };
        self.slots
            .insert(slot.to_owned(), SaveSlot { data, dirty: false });
    }
}

// ── SaveError ─────────────────────────────────────────────────────────────────

/// Error type returned by save operations.
#[derive(Debug)]
pub enum SaveError {
    Io(std::io::Error),
    Serialize(serde_json::Error),
}

impl std::fmt::Display for SaveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SaveError::Io(e) => write!(f, "save I/O error: {e}"),
            SaveError::Serialize(e) => write!(f, "save serialise error: {e}"),
        }
    }
}

impl std::error::Error for SaveError {}

// ── Settings ──────────────────────────────────────────────────────────────────

/// Typed settings store backed by a single JSON file.
///
/// Call [`register`](Self::register) to declare defaults, then
/// [`get`](Self::get) / [`set`](Self::set) to read / write values.
/// Persist with [`save`](Self::save).
///
/// # Quick start
/// ```no_run
/// use nene::persist::Settings;
///
/// let mut s = Settings::new("saves/settings.json");
///
/// s.register("audio.master_volume", 1.0f32);
/// s.register("video.fullscreen", false);
///
/// let vol: f32 = s.get("audio.master_volume").unwrap_or(1.0);
///
/// s.set("audio.master_volume", &0.75f32).unwrap();
/// s.save().unwrap();
/// ```
pub struct Settings {
    store: SaveStore,
    slot: String,
    defaults: std::collections::HashMap<String, Value>,
}

impl Settings {
    /// Open (or create) a settings file at `path`.
    ///
    /// The file is loaded from disk immediately if it exists.
    /// The parent directory is created on the first [`save`](Self::save).
    pub fn new(path: impl AsRef<std::path::Path>) -> Self {
        let path = path.as_ref();
        let dir = path.parent().unwrap_or(std::path::Path::new("."));
        let slot = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("settings")
            .to_owned();

        let mut store = SaveStore::new(dir);
        let _ = store.keys(&slot);

        Self {
            store,
            slot,
            defaults: std::collections::HashMap::new(),
        }
    }

    /// Register a default value for `key`.
    ///
    /// The default is returned by [`get`](Self::get) when the key has never
    /// been set, but it is **not** written to disk until [`set`](Self::set)
    /// is called explicitly.
    pub fn register<T: Serialize>(&mut self, key: &str, default: T) {
        if let Ok(v) = serde_json::to_value(&default) {
            self.defaults.insert(key.to_owned(), v);
        }
    }

    /// Get the value for `key`, deserialised as `T`.
    ///
    /// Falls back to the registered default if the key is absent.
    pub fn get<T: serde::de::DeserializeOwned>(&mut self, key: &str) -> Option<T> {
        if let Some(v) = self.store.get::<T>(&self.slot, key) {
            return Some(v);
        }
        self.defaults
            .get(key)
            .and_then(|v| serde_json::from_value(v.clone()).ok())
    }

    /// Returns `true` if `key` has been explicitly set (ignoring defaults).
    pub fn has(&mut self, key: &str) -> bool {
        self.store.has(&self.slot, key)
    }

    /// All keys that have been explicitly set (does not include default-only keys).
    pub fn keys(&mut self) -> Vec<String> {
        self.store.keys(&self.slot)
    }

    /// Serialise and store `value` for `key` (in memory only).
    ///
    /// Call [`save`](Self::save) to write to disk.
    pub fn set<T: Serialize>(&mut self, key: &str, value: &T) -> Result<(), SaveError> {
        self.store.set(&self.slot, key, value)
    }

    /// Remove `key`. Does nothing if absent.
    pub fn remove(&mut self, key: &str) {
        self.store.remove(&self.slot, key);
    }

    /// Reset `key` to its registered default (removes it from the store).
    pub fn reset(&mut self, key: &str) {
        self.store.remove(&self.slot, key);
    }

    /// Reset all keys to defaults (clears the entire settings file in memory).
    pub fn reset_all(&mut self) {
        for key in self.keys() {
            self.store.remove(&self.slot, &key);
        }
    }

    /// Write in-memory changes to disk.
    pub fn save(&mut self) -> Result<(), SaveError> {
        self.store.flush(&self.slot)
    }

    /// Returns `true` if the settings file exists on disk.
    pub fn exists(&self) -> bool {
        self.store.exists(&self.slot)
    }

    /// Discard in-memory changes and reload from disk.
    pub fn reload(&mut self) -> Result<(), SaveError> {
        self.store.reload(&self.slot)
    }
}
