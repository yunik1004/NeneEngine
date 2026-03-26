//! Asset cache — load-once, share-many resource management.
//!
//! [`Assets`] keeps a path-keyed cache of every loaded resource. The first
//! call to e.g. [`Assets::texture`] decodes the file and uploads it to the
//! GPU; every subsequent call with the same path returns a clone of the
//! existing [`Handle`] without any I/O.
//!
//! A [`Handle<T>`] is a thin `Arc<T>` wrapper. Cloning it is cheap and the
//! underlying resource is freed only when *all* handles and the cache entry
//! are dropped.
//!
//! # Example
//! ```no_run
//! use nene::asset::Assets;
//! use nene::renderer::FilterMode;
//! # fn demo(ctx: &mut nene::renderer::Context) {
//! let mut assets = Assets::new();
//!
//! // First call: loads from disk and uploads to GPU.
//! let tex = assets.texture(ctx, "assets/hero.png", FilterMode::Linear);
//!
//! // Second call: returns a clone of the cached handle — zero I/O.
//! let same = assets.texture(ctx, "assets/hero.png", FilterMode::Linear);
//! assert!(tex == same);
//! # }
//! ```

use std::collections::HashMap;
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::audio::Sound;
use crate::mesh::Model;
use crate::renderer::{Context, FilterMode, Texture};

// ── Handle ────────────────────────────────────────────────────────────────────

/// A reference-counted handle to a cached asset.
///
/// - `Clone` is O(1) — just an `Arc` increment.
/// - `Deref` gives transparent access to the inner type.
/// - Two handles are `==` when they point to the *same allocation* (identity,
///   not structural equality).
pub struct Handle<T>(Arc<T>);

impl<T> Handle<T> {
    fn new(value: T) -> Self {
        Self(Arc::new(value))
    }
}

impl<T> Clone for Handle<T> {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}

impl<T> Deref for Handle<T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.0
    }
}

/// Two handles are equal iff they point to the same allocation.
impl<T> PartialEq for Handle<T> {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}
impl<T> Eq for Handle<T> {}

impl<T: std::fmt::Debug> std::fmt::Debug for Handle<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Handle({:?})", &*self.0)
    }
}

// ── Assets ────────────────────────────────────────────────────────────────────

/// Centralised asset cache.
///
/// See the [module documentation](self) for an overview and usage example.
pub struct Assets {
    textures: HashMap<(PathBuf, u8), Handle<Texture>>,
    models: HashMap<PathBuf, Handle<Model>>,
    sounds: HashMap<PathBuf, Handle<Sound>>,
}

impl Assets {
    /// Create an empty cache.
    pub fn new() -> Self {
        Self {
            textures: HashMap::new(),
            models: HashMap::new(),
            sounds: HashMap::new(),
        }
    }

    // ── Textures ──────────────────────────────────────────────────────────────

    /// Load (or return cached) a 2-D texture from `path`.
    ///
    /// The image is decoded to RGBA8 and uploaded to the GPU on the first call.
    /// Subsequent calls with the same `path` **and** `filter` return the cached
    /// handle immediately.
    ///
    /// # Panics
    /// Panics if the file cannot be opened or decoded as an image.
    pub fn texture(
        &mut self,
        ctx: &mut Context,
        path: impl AsRef<Path>,
        filter: FilterMode,
    ) -> Handle<Texture> {
        let key = (path.as_ref().to_owned(), filter_key(filter));
        if let Some(h) = self.textures.get(&key) {
            return h.clone();
        }
        let img = image::open(path.as_ref())
            .unwrap_or_else(|e| {
                panic!("failed to load texture '{}': {e}", path.as_ref().display())
            })
            .into_rgba8();
        let texture = ctx.create_texture_with(img.width(), img.height(), &img, filter);
        let handle = Handle::new(texture);
        self.textures.insert(key, handle.clone());
        handle
    }

    /// Remove the cached texture entry for `path` + `filter`.
    ///
    /// Outstanding [`Handle`]s remain valid — the resource is freed only when
    /// *all* handles are dropped.
    pub fn evict_texture(&mut self, path: impl AsRef<Path>, filter: FilterMode) {
        self.textures
            .remove(&(path.as_ref().to_owned(), filter_key(filter)));
    }

    // ── Models ────────────────────────────────────────────────────────────────

    /// Load (or return cached) a [`Model`] from an OBJ file.
    ///
    /// # Panics
    /// Panics if the file cannot be read or parsed.
    pub fn model(&mut self, path: impl AsRef<Path>) -> Handle<Model> {
        let key = path.as_ref().to_owned();
        if let Some(h) = self.models.get(&key) {
            return h.clone();
        }
        let model = Model::load(&key);
        let handle = Handle::new(model);
        self.models.insert(key, handle.clone());
        handle
    }

    /// Remove the cached model entry for `path`.
    pub fn evict_model(&mut self, path: impl AsRef<Path>) {
        self.models.remove(path.as_ref());
    }

    // ── Sounds ────────────────────────────────────────────────────────────────

    /// Load (or return cached) a [`Sound`] from an audio file (MP3, WAV, OGG, …).
    ///
    /// # Panics
    /// Panics if the file cannot be opened or decoded.
    pub fn sound(&mut self, path: impl AsRef<Path>) -> Handle<Sound> {
        let key = path.as_ref().to_owned();
        if let Some(h) = self.sounds.get(&key) {
            return h.clone();
        }
        let sound = Sound::load(&key);
        let handle = Handle::new(sound);
        self.sounds.insert(key, handle.clone());
        handle
    }

    /// Remove the cached sound entry for `path`.
    pub fn evict_sound(&mut self, path: impl AsRef<Path>) {
        self.sounds.remove(path.as_ref());
    }

    // ── Cache management ──────────────────────────────────────────────────────

    /// Drop all cache entries.
    ///
    /// Outstanding [`Handle`]s stay valid until they are themselves dropped.
    pub fn clear(&mut self) {
        self.textures.clear();
        self.models.clear();
        self.sounds.clear();
    }

    /// Total number of cached entries across all asset types.
    pub fn len(&self) -> usize {
        self.textures.len() + self.models.len() + self.sounds.len()
    }

    /// `true` if no assets are cached.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl Default for Assets {
    fn default() -> Self {
        Self::new()
    }
}

// ── helpers ───────────────────────────────────────────────────────────────────

fn filter_key(f: FilterMode) -> u8 {
    match f {
        FilterMode::Linear => 0,
        FilterMode::Nearest => 1,
    }
}
