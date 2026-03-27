//! Game settings — a typed key-value store with defaults, persisted to disk.
//!
//! Thin layer over [`SaveStore`](crate::save::SaveStore) that targets a single
//! `settings.json` file, lets you register default values, and exposes a
//! slot-free API.
//!
//! # Quick start
//! ```no_run
//! use nene::settings::Settings;
//!
//! let mut s = Settings::new("saves/settings.json");
//!
//! // Register defaults (only applied when the key is absent).
//! s.register("audio.master_volume", 1.0f32);
//! s.register("video.fullscreen", false);
//! s.register("video.resolution", [1920u32, 1080u32]);
//!
//! // Read (returns the default if not yet set).
//! let vol: f32 = s.get("audio.master_volume").unwrap_or(1.0);
//!
//! // Write and persist.
//! s.set("audio.master_volume", &0.75f32).unwrap();
//! s.save().unwrap();
//! ```

use std::path::Path;

use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;

use crate::save::{SaveError, SaveStore};

// ── Settings ──────────────────────────────────────────────────────────────────

/// Typed settings store backed by a single JSON file.
///
/// Call [`register`](Self::register) to declare defaults, then
/// [`get`](Self::get) / [`set`](Self::set) to read / write values.
/// Persist with [`save`](Self::save).
pub struct Settings {
    store: SaveStore,
    /// Virtual "slot" name derived from the file stem.
    slot: String,
    defaults: std::collections::HashMap<String, Value>,
}

impl Settings {
    /// Open (or create) a settings file at `path`.
    ///
    /// The file is loaded from disk immediately if it exists.
    /// The parent directory is created on the first [`save`](Self::save).
    pub fn new(path: impl AsRef<Path>) -> Self {
        let path = path.as_ref();
        let dir = path.parent().unwrap_or(Path::new("."));
        let slot = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("settings")
            .to_owned();

        let mut store = SaveStore::new(dir);
        // Pre-load so defaults can be applied.
        let _ = store.keys(&slot); // triggers lazy load

        Self {
            store,
            slot,
            defaults: std::collections::HashMap::new(),
        }
    }

    // ── Defaults ──────────────────────────────────────────────────────────────

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

    // ── Read ──────────────────────────────────────────────────────────────────

    /// Get the value for `key`, deserialised as `T`.
    ///
    /// Falls back to the registered default if the key is absent.
    /// Returns `None` if neither a stored value nor a default exists.
    pub fn get<T: DeserializeOwned>(&mut self, key: &str) -> Option<T> {
        // Try stored value first.
        if let Some(v) = self.store.get::<T>(&self.slot, key) {
            return Some(v);
        }
        // Fall back to default.
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

    // ── Write ─────────────────────────────────────────────────────────────────

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
    /// If no default is registered, the key is simply removed.
    pub fn reset(&mut self, key: &str) {
        self.store.remove(&self.slot, key);
    }

    /// Reset all keys to defaults (clears the entire settings file in memory).
    pub fn reset_all(&mut self) {
        for key in self.keys() {
            self.store.remove(&self.slot, &key);
        }
    }

    // ── Persistence ───────────────────────────────────────────────────────────

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
