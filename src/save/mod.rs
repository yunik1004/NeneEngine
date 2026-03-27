//! Save / load system.
//!
//! Stores named slots as JSON files under a configurable directory.
//! Each slot is a flat key-value map where values are JSON-serialisable.
//!
//! # Quick start
//! ```no_run
//! use nene::save::SaveStore;
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
