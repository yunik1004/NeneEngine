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
//! # Hot reload (debug builds only)
//!
//! Call [`Assets::enable_hot_reload`] once at startup, then
//! [`Assets::poll_changes`] each frame. Changed assets are evicted from the
//! cache and reloaded automatically; the return value lists every path that
//! was reloaded so you can re-fetch new handles.
//!
//! Both methods compile to no-ops in release builds.
//!
//! # Example
//! ```no_run
//! use nene::asset::Assets;
//! use nene::renderer::FilterMode;
//! # fn demo(ctx: &mut nene::renderer::Context) {
//! let mut assets = Assets::new();
//! assets.enable_hot_reload();
//!
//! // First call: loads from disk and uploads to GPU.
//! let mut tex = assets.texture(ctx, "assets/hero.png", FilterMode::Linear);
//!
//! // Each frame:
//! let changed = assets.poll_changes(ctx);
//! if changed.iter().any(|p| p.ends_with("hero.png")) {
//!     tex = assets.texture(ctx, "assets/hero.png", FilterMode::Linear);
//! }
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

// ── Hot reloader (debug only) ─────────────────────────────────────────────────

#[cfg(debug_assertions)]
struct HotReloader {
    watcher: notify::RecommendedWatcher,
    rx: std::sync::mpsc::Receiver<PathBuf>,
}

#[cfg(debug_assertions)]
impl HotReloader {
    fn new() -> Self {
        let (tx, rx) = std::sync::mpsc::channel::<PathBuf>();
        let watcher = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
            if let Ok(ev) = res {
                use notify::EventKind::*;
                if matches!(ev.kind, Modify(_) | Create(_)) {
                    for path in ev.paths {
                        let _ = tx.send(path);
                    }
                }
            }
        })
        .expect("failed to create file watcher");
        Self { watcher, rx }
    }

    fn watch(&mut self, path: &Path) {
        use notify::Watcher as _;
        let _ = self
            .watcher
            .watch(path, notify::RecursiveMode::NonRecursive);
    }

    fn drain(&self) -> Vec<PathBuf> {
        self.rx.try_iter().collect()
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
    #[cfg(debug_assertions)]
    hot: Option<HotReloader>,
    /// Tracks filter mode per path so poll_changes can reload correctly.
    #[cfg(debug_assertions)]
    texture_filters: HashMap<PathBuf, FilterMode>,
}

impl Assets {
    /// Create an empty cache.
    pub fn new() -> Self {
        Self {
            textures: HashMap::new(),
            models: HashMap::new(),
            sounds: HashMap::new(),
            #[cfg(debug_assertions)]
            hot: None,
            #[cfg(debug_assertions)]
            texture_filters: HashMap::new(),
        }
    }

    // ── Hot reload ────────────────────────────────────────────────────────────

    /// Start watching loaded assets for file-system changes.
    ///
    /// Must be called before loading any assets you want watched.
    /// No-op in release builds.
    pub fn enable_hot_reload(&mut self) {
        #[cfg(debug_assertions)]
        {
            self.hot = Some(HotReloader::new());
        }
    }

    /// Reload any assets whose source files changed since the last call.
    ///
    /// Evicts stale cache entries and reloads them immediately. Returns the
    /// paths of every reloaded asset so you can re-fetch updated [`Handle`]s.
    ///
    /// Always returns an empty `Vec` in release builds.
    pub fn poll_changes(&mut self, ctx: &mut Context) -> Vec<PathBuf> {
        #[cfg(debug_assertions)]
        {
            let changed: Vec<PathBuf> = match &self.hot {
                Some(hot) => hot.drain(),
                None => return Vec::new(),
            };

            let mut reloaded = Vec::new();
            for path in changed {
                if let Some(filter) = self.texture_filters.get(&path).copied() {
                    self.textures.remove(&(path.clone(), filter_key(filter)));
                    self.load_texture(ctx, &path, filter);
                    reloaded.push(path);
                } else if self.models.contains_key(&path) {
                    self.models.remove(&path);
                    self.load_model(&path);
                    reloaded.push(path);
                } else if self.sounds.contains_key(&path) {
                    self.sounds.remove(&path);
                    self.load_sound(&path);
                    reloaded.push(path);
                }
            }
            reloaded
        }
        #[cfg(not(debug_assertions))]
        Vec::new()
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
        self.load_texture(ctx, path.as_ref(), filter)
    }

    fn load_texture(
        &mut self,
        ctx: &mut Context,
        path: &Path,
        filter: FilterMode,
    ) -> Handle<Texture> {
        let img = image::open(path)
            .unwrap_or_else(|e| panic!("failed to load texture '{}': {e}", path.display()))
            .into_rgba8();
        let texture = ctx.create_texture_with(img.width(), img.height(), &img, filter);
        let handle = Handle::new(texture);
        self.textures
            .insert((path.to_owned(), filter_key(filter)), handle.clone());
        #[cfg(debug_assertions)]
        {
            self.texture_filters.insert(path.to_owned(), filter);
            if let Some(ref mut hot) = self.hot {
                hot.watch(path);
            }
        }
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
        self.load_model(path.as_ref())
    }

    fn load_model(&mut self, path: &Path) -> Handle<Model> {
        let model = Model::load(path).expect("failed to load model");
        let handle = Handle::new(model);
        self.models.insert(path.to_owned(), handle.clone());
        #[cfg(debug_assertions)]
        if let Some(ref mut hot) = self.hot {
            hot.watch(path);
        }
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
        self.load_sound(path.as_ref())
    }

    fn load_sound(&mut self, path: &Path) -> Handle<Sound> {
        let sound = Sound::load(path);
        let handle = Handle::new(sound);
        self.sounds.insert(path.to_owned(), handle.clone());
        #[cfg(debug_assertions)]
        if let Some(ref mut hot) = self.hot {
            hot.watch(path);
        }
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
