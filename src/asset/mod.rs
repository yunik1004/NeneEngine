//! Asset cache — load-once, share-many resource management.
//!
//! [`Assets`] keeps a path-keyed cache of every loaded resource. The first
//! call to e.g. [`Assets::texture`] decodes the file and uploads it to the
//! GPU; every subsequent call with the same `path` returns a clone of the
//! existing [`Handle`] without any I/O.
//!
//! A [`Handle<T>`] is a thin `Arc<T>` wrapper. Cloning it is cheap and the
//! underlying resource is freed only when *all* handles and the cache entry
//! are dropped.
//!
//! # Background preloading
//!
//! To avoid blocking the game loop on first access, pre-load assets on a
//! background thread with `preload_*`, then call [`Assets::poll_preloads`]
//! each frame. Once a preload completes, the next `texture/model/sound` call
//! returns instantly from cache.
//!
//! ```no_run
//! # fn demo(ctx: &mut nene::renderer::Context) {
//! use nene::asset::Assets;
//! use nene::renderer::FilterMode;
//!
//! let mut assets = Assets::new();
//!
//! // Loading screen: kick off background loads.
//! assets.preload_texture("assets/hero.png", FilterMode::Linear);
//! assets.preload_model("assets/level.glb");
//!
//! // Each frame: drain completed loads, upload textures to GPU.
//! assets.poll_preloads(ctx);
//!
//! // In-game: always a cache hit, no I/O.
//! let tex = assets.texture(ctx, "assets/hero.png", FilterMode::Linear).unwrap();
//! # }
//! ```
//!
//! # Hot reload (debug builds only)
//!
//! Call [`Assets::enable_hot_reload`] once at startup, then
//! [`Assets::poll_changes`] each frame. Changed assets are evicted from the
//! cache and reloaded automatically; the return value lists every path that
//! was reloaded so you can re-fetch new handles.
//!
//! Both methods compile to no-ops in release builds.

use std::collections::HashMap;
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver};

use crate::audio::Sound;
use crate::mesh::Model;
use crate::pak::PakReader;
use crate::renderer::{Context, FilterMode, Texture};

// ── AssetError ────────────────────────────────────────────────────────────────

/// Error returned when an asset cannot be loaded or decoded.
#[derive(Debug)]
pub enum AssetError {
    Io(std::io::Error),
    Decode(String),
}

impl std::fmt::Display for AssetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "I/O error: {e}"),
            Self::Decode(msg) => write!(f, "decode error: {msg}"),
        }
    }
}

impl std::error::Error for AssetError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            Self::Decode(_) => None,
        }
    }
}

impl From<std::io::Error> for AssetError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

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

// ── Pending loads ─────────────────────────────────────────────────────────────

struct PendingTexture {
    path: PathBuf,
    filter: FilterMode,
    rx: Receiver<Result<image::RgbaImage, AssetError>>,
}

struct PendingModel {
    path: PathBuf,
    rx: Receiver<Result<Model, AssetError>>,
}

struct PendingSound {
    path: PathBuf,
    rx: Receiver<Result<Sound, AssetError>>,
}

// ── Assets ────────────────────────────────────────────────────────────────────

/// Centralised asset cache.
///
/// See the [module documentation](self) for an overview and usage example.
///
/// Call [`set_pak`](Self::set_pak) to mount a `.npak` archive; all subsequent
/// loads will check the archive before falling back to the filesystem.
pub struct Assets {
    textures: HashMap<(PathBuf, u8), Handle<Texture>>,
    models: HashMap<PathBuf, Handle<Model>>,
    sounds: HashMap<PathBuf, Handle<Sound>>,

    pending_textures: Vec<PendingTexture>,
    pending_models: Vec<PendingModel>,
    pending_sounds: Vec<PendingSound>,

    pak: Option<Arc<PakReader>>,

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
            pending_textures: Vec::new(),
            pending_models: Vec::new(),
            pending_sounds: Vec::new(),
            pak: None,
            #[cfg(debug_assertions)]
            hot: None,
            #[cfg(debug_assertions)]
            texture_filters: HashMap::new(),
        }
    }

    /// Mount a `.npak` archive. All subsequent loads check the archive before
    /// the filesystem. Replaces any previously mounted archive.
    pub fn set_pak(&mut self, pak: PakReader) {
        self.pak = Some(Arc::new(pak));
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
                if self.texture_filters.contains_key(&path) {
                    // Reload every filter variant that is currently cached.
                    let mut any_ok = false;
                    for &filter in &[FilterMode::Linear, FilterMode::Nearest] {
                        let key = (path.clone(), filter_key(filter));
                        if !self.textures.contains_key(&key) {
                            continue;
                        }
                        self.textures.remove(&key);
                        match self.load_texture(ctx, &path, filter) {
                            Ok(_) => any_ok = true,
                            Err(e) => {
                                eprintln!("[nene] hot reload failed for '{}': {e}", path.display())
                            }
                        }
                    }
                    if any_ok {
                        reloaded.push(path);
                    }
                } else if self.models.contains_key(&path) {
                    self.models.remove(&path);
                    match self.load_model(&path) {
                        Ok(_) => reloaded.push(path),
                        Err(e) => {
                            eprintln!("[nene] hot reload failed for '{}': {e}", path.display())
                        }
                    }
                } else if self.sounds.contains_key(&path) {
                    self.sounds.remove(&path);
                    match self.load_sound(&path) {
                        Ok(_) => reloaded.push(path),
                        Err(e) => {
                            eprintln!("[nene] hot reload failed for '{}': {e}", path.display())
                        }
                    }
                }
            }
            reloaded
        }
        #[cfg(not(debug_assertions))]
        Vec::new()
    }

    // ── Background preloading ─────────────────────────────────────────────────

    /// Begin loading a texture from disk on a background thread.
    ///
    /// Call [`poll_preloads`](Self::poll_preloads) each frame to upload
    /// completed loads to the GPU. Silently skips paths already cached or
    /// already pending.
    pub fn preload_texture(&mut self, path: impl AsRef<Path>, filter: FilterMode) {
        let path = path.as_ref().to_owned();
        if self
            .textures
            .contains_key(&(path.clone(), filter_key(filter)))
        {
            return;
        }
        if self
            .pending_textures
            .iter()
            .any(|p| p.path == path && filter_key(p.filter) == filter_key(filter))
        {
            return;
        }
        let (tx, rx) = mpsc::channel();
        let path_clone = path.clone();
        std::thread::spawn(move || {
            let result = image::open(&path_clone)
                .map(|img| img.into_rgba8())
                .map_err(|e| AssetError::Decode(e.to_string()));
            let _ = tx.send(result);
        });
        self.pending_textures
            .push(PendingTexture { path, filter, rx });
    }

    /// Begin loading a model from disk on a background thread.
    ///
    /// Call [`poll_preloads`](Self::poll_preloads) each frame to insert
    /// completed loads into the cache. Silently skips paths already cached or
    /// already pending.
    pub fn preload_model(&mut self, path: impl AsRef<Path>) {
        let path = path.as_ref().to_owned();
        if self.models.contains_key(&path) {
            return;
        }
        if self.pending_models.iter().any(|p| p.path == path) {
            return;
        }
        let (tx, rx) = mpsc::channel();
        let path_clone = path.clone();
        std::thread::spawn(move || {
            let result = Model::load(&path_clone).map_err(|e| AssetError::Decode(e.to_string()));
            let _ = tx.send(result);
        });
        self.pending_models.push(PendingModel { path, rx });
    }

    /// Begin loading a sound from disk on a background thread.
    ///
    /// Call [`poll_preloads`](Self::poll_preloads) each frame to insert
    /// completed loads into the cache. Silently skips paths already cached or
    /// already pending.
    pub fn preload_sound(&mut self, path: impl AsRef<Path>) {
        let path = path.as_ref().to_owned();
        if self.sounds.contains_key(&path) {
            return;
        }
        if self.pending_sounds.iter().any(|p| p.path == path) {
            return;
        }
        let (tx, rx) = mpsc::channel();
        let path_clone = path.clone();
        std::thread::spawn(move || {
            let result = Sound::load(&path_clone).map_err(|e| AssetError::Decode(e.to_string()));
            let _ = tx.send(result);
        });
        self.pending_sounds.push(PendingSound { path, rx });
    }

    /// Drain background loads that have finished and insert them into the cache.
    ///
    /// Textures require a GPU upload via `ctx`; models and sounds are inserted
    /// directly. Call once per frame during a loading screen or any time
    /// [`preload_*`](Self::preload_texture) is used.
    pub fn poll_preloads(&mut self, ctx: &mut Context) {
        // Textures: CPU decode is done on background thread; GPU upload here.
        let pending = std::mem::take(&mut self.pending_textures);
        for p in pending {
            match p.rx.try_recv() {
                Ok(Ok(img)) => {
                    let texture =
                        ctx.create_texture_with(img.width(), img.height(), &img, p.filter);
                    let handle = Handle::new(texture);
                    self.textures
                        .insert((p.path.clone(), filter_key(p.filter)), handle);
                    #[cfg(debug_assertions)]
                    {
                        self.texture_filters.insert(p.path.clone(), p.filter);
                        if let Some(ref mut hot) = self.hot {
                            hot.watch(&p.path);
                        }
                    }
                }
                Ok(Err(e)) => {
                    eprintln!("[nene] preload failed for '{}': {e}", p.path.display());
                }
                Err(mpsc::TryRecvError::Empty) => {
                    self.pending_textures.push(p);
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    eprintln!(
                        "[nene] preload thread for '{}' disconnected",
                        p.path.display()
                    );
                }
            }
        }

        // Models
        let pending = std::mem::take(&mut self.pending_models);
        for p in pending {
            match p.rx.try_recv() {
                Ok(Ok(model)) => {
                    let handle = Handle::new(model);
                    self.models.insert(p.path.clone(), handle);
                    #[cfg(debug_assertions)]
                    if let Some(ref mut hot) = self.hot {
                        hot.watch(&p.path);
                    }
                }
                Ok(Err(e)) => {
                    eprintln!("[nene] preload failed for '{}': {e}", p.path.display());
                }
                Err(mpsc::TryRecvError::Empty) => {
                    self.pending_models.push(p);
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    eprintln!(
                        "[nene] preload thread for '{}' disconnected",
                        p.path.display()
                    );
                }
            }
        }

        // Sounds
        let pending = std::mem::take(&mut self.pending_sounds);
        for p in pending {
            match p.rx.try_recv() {
                Ok(Ok(sound)) => {
                    let handle = Handle::new(sound);
                    self.sounds.insert(p.path.clone(), handle);
                    #[cfg(debug_assertions)]
                    if let Some(ref mut hot) = self.hot {
                        hot.watch(&p.path);
                    }
                }
                Ok(Err(e)) => {
                    eprintln!("[nene] preload failed for '{}': {e}", p.path.display());
                }
                Err(mpsc::TryRecvError::Empty) => {
                    self.pending_sounds.push(p);
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    eprintln!(
                        "[nene] preload thread for '{}' disconnected",
                        p.path.display()
                    );
                }
            }
        }
    }

    /// Number of asset loads still in progress (not yet inserted into the cache).
    pub fn pending_count(&self) -> usize {
        self.pending_textures.len() + self.pending_models.len() + self.pending_sounds.len()
    }

    /// `true` when all previously requested preloads have finished.
    pub fn is_ready(&self) -> bool {
        self.pending_count() == 0
    }

    // ── Textures ──────────────────────────────────────────────────────────────

    /// Load (or return cached) a 2-D texture from `path`.
    ///
    /// The image is decoded to RGBA8 and uploaded to the GPU on the first call.
    /// Subsequent calls with the same `path` **and** `filter` return the cached
    /// handle immediately.
    ///
    /// Returns an error if the file cannot be opened or decoded as an image.
    pub fn texture(
        &mut self,
        ctx: &mut Context,
        path: impl AsRef<Path>,
        filter: FilterMode,
    ) -> Result<Handle<Texture>, AssetError> {
        let key = (path.as_ref().to_owned(), filter_key(filter));
        if let Some(h) = self.textures.get(&key) {
            return Ok(h.clone());
        }
        self.load_texture(ctx, path.as_ref(), filter)
    }

    fn load_texture(
        &mut self,
        ctx: &mut Context,
        path: &Path,
        filter: FilterMode,
    ) -> Result<Handle<Texture>, AssetError> {
        let img = if let Some(bytes) = self.pak_read(path) {
            image::load_from_memory(&bytes)
                .map_err(|e| AssetError::Decode(e.to_string()))?
                .into_rgba8()
        } else {
            image::open(path)
                .map_err(|e| AssetError::Decode(e.to_string()))?
                .into_rgba8()
        };
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
        Ok(handle)
    }

    /// Remove the cached texture entry for `path` + `filter`.
    ///
    /// Outstanding [`Handle`]s remain valid — the resource is freed only when
    /// *all* handles are dropped.
    pub fn evict_texture(&mut self, path: impl AsRef<Path>, filter: FilterMode) {
        self.textures
            .remove(&(path.as_ref().to_owned(), filter_key(filter)));
    }

    /// Remove all cached texture entries for `path`, regardless of filter mode.
    ///
    /// Prefer this over [`evict_texture`](Self::evict_texture) when you don't
    /// know (or don't care about) the filter mode used at load time.
    pub fn evict_texture_all(&mut self, path: impl AsRef<Path>) {
        let path = path.as_ref();
        self.textures.retain(|(p, _), _| p != path);
        #[cfg(debug_assertions)]
        self.texture_filters.remove(path);
    }

    // ── Models ────────────────────────────────────────────────────────────────

    /// Load (or return cached) a [`Model`] from an OBJ or glTF file.
    ///
    /// Returns an error if the file cannot be read or parsed.
    pub fn model(&mut self, path: impl AsRef<Path>) -> Result<Handle<Model>, AssetError> {
        let key = path.as_ref().to_owned();
        if let Some(h) = self.models.get(&key) {
            return Ok(h.clone());
        }
        self.load_model(path.as_ref())
    }

    fn load_model(&mut self, path: &Path) -> Result<Handle<Model>, AssetError> {
        let model = Model::load(path).map_err(|e| AssetError::Decode(e.to_string()))?;
        let handle = Handle::new(model);
        self.models.insert(path.to_owned(), handle.clone());
        #[cfg(debug_assertions)]
        if let Some(ref mut hot) = self.hot {
            hot.watch(path);
        }
        Ok(handle)
    }

    /// Remove the cached model entry for `path`.
    pub fn evict_model(&mut self, path: impl AsRef<Path>) {
        self.models.remove(path.as_ref());
    }

    // ── Sounds ────────────────────────────────────────────────────────────────

    /// Load (or return cached) a [`Sound`] from an audio file (MP3, WAV, OGG, …).
    ///
    /// Returns an error if the file cannot be opened or decoded.
    pub fn sound(&mut self, path: impl AsRef<Path>) -> Result<Handle<Sound>, AssetError> {
        let key = path.as_ref().to_owned();
        if let Some(h) = self.sounds.get(&key) {
            return Ok(h.clone());
        }
        self.load_sound(path.as_ref())
    }

    fn load_sound(&mut self, path: &Path) -> Result<Handle<Sound>, AssetError> {
        let sound = if let Some(bytes) = self.pak_read(path) {
            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("");
            Sound::from_bytes(bytes, ext).map_err(|e| AssetError::Decode(e.to_string()))?
        } else {
            Sound::load(path).map_err(|e| AssetError::Decode(e.to_string()))?
        };
        let handle = Handle::new(sound);
        self.sounds.insert(path.to_owned(), handle.clone());
        #[cfg(debug_assertions)]
        if let Some(ref mut hot) = self.hot {
            hot.watch(path);
        }
        Ok(handle)
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

    // ── PAK helpers ───────────────────────────────────────────────────────────

    /// Look up `path` in the mounted PAK archive (if any).
    ///
    /// The lookup key is the path converted to a forward-slash string, with
    /// any leading `./` stripped.
    fn pak_read(&self, path: &Path) -> Option<Vec<u8>> {
        let pak = self.pak.as_ref()?;
        let key = path.to_string_lossy().replace('\\', "/");
        let key = key.strip_prefix("./").unwrap_or(&key);
        pak.read(key)
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
