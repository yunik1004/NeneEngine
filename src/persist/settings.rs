use serde::Serialize;
use serde_json::Value;

use super::store::{SaveError, SaveStore};

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
