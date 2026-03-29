//! Build-script helper for `nene` — packs an `assets/` directory into a
//! `.npak` binary archive at compile time.
//!
//! # Usage
//!
//! Add to your game's `Cargo.toml`:
//! ```toml
//! [build-dependencies]
//! nene-build = { path = "..." }
//! ```
//!
//! Create `build.rs` (one line):
//! ```rust,no_run
//! nene_build::pack_assets("assets");
//! ```
//!
//! Add to your `main()` (one line):
//! ```rust,ignore
//! fn main() {
//!     nene::embed_assets!();
//!     nene::run::<MyGame>();
//! }
//! ```
//!
//! That's it. Cargo will re-pack the archive whenever any file under
//! `assets/` changes, and `Assets::new()` will automatically mount it.

use std::io::Write;
use std::path::{Path, PathBuf};

/// Pack all files under `assets_dir` into `$OUT_DIR/assets.npak`.
///
/// Emits the Cargo directives needed for incremental rebuilds and makes the
/// archive available to [`nene::embed_assets!`] via `NENE_ASSETS_PAK`.
///
/// Call this from your `build.rs`:
/// ```rust,no_run
/// nene_build::pack_assets("assets");
/// ```
pub fn pack_assets(assets_dir: impl AsRef<Path>) {
    pack_assets_with_key(assets_dir, None);
}

/// Like [`pack_assets`] but encrypts every entry with a 32-byte ChaCha20 key.
///
/// The same key must be supplied to `PakReader::from_bytes` / `from_file` at
/// runtime.
pub fn pack_assets_encrypted(assets_dir: impl AsRef<Path>, key: [u8; 32]) {
    pack_assets_with_key(assets_dir, Some(key));
}

// ── internals ─────────────────────────────────────────────────────────────────

fn pack_assets_with_key(assets_dir: impl AsRef<Path>, key: Option<[u8; 32]>) {
    let dir = assets_dir.as_ref();
    if !dir.exists() {
        return;
    }

    // Tell Cargo to rerun when any asset changes.
    println!("cargo:rerun-if-changed={}", dir.display());
    let mut entries: Vec<(String, Vec<u8>)> = Vec::new();
    collect(dir, dir, &mut entries);
    entries.sort_by(|a, b| a.0.cmp(&b.0));
    for (path, _) in &entries {
        println!("cargo:rerun-if-changed={}/{}", dir.display(), path);
    }

    let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR not set");
    let pak_path = PathBuf::from(&out_dir).join("assets.npak");

    let mut f = std::fs::File::create(&pak_path)
        .unwrap_or_else(|e| panic!("nene-build: failed to create {}: {e}", pak_path.display()));

    write_pak(&mut f, &entries, key.as_ref())
        .unwrap_or_else(|e| panic!("nene-build: failed to write PAK: {e}"));

    // Expose to nene::embed_assets!
    println!("cargo:rustc-env=NENE_ASSETS_PAK={}", pak_path.display());
    println!("cargo:rustc-cfg=nene_has_pak");
}

fn collect(root: &Path, dir: &Path, out: &mut Vec<(String, Vec<u8>)>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect(root, &path, out);
        } else {
            let rel = path
                .strip_prefix(root)
                .expect("path must be under root")
                .to_string_lossy()
                .replace('\\', "/");
            match std::fs::read(&path) {
                Ok(data) => out.push((rel, data)),
                Err(e) => eprintln!("nene-build: skipping {}: {e}", path.display()),
            }
        }
    }
}

// ── NPAK writer (mirrors src/pak.rs — no dep on the main crate) ───────────────

const MAGIC: &[u8; 4] = b"NPAK";
const FLAG_ENCRYPTED: u8 = 0x01;

fn write_pak(
    out: &mut impl Write,
    entries: &[(String, Vec<u8>)],
    key: Option<&[u8; 32]>,
) -> std::io::Result<()> {
    let flags: u8 = if key.is_some() { FLAG_ENCRYPTED } else { 0 };

    out.write_all(MAGIC)?;
    out.write_all(&[1u8, flags])?; // version = 1
    out.write_all(&(entries.len() as u32).to_le_bytes())?;

    let table_bytes: u64 = entries.iter().map(|(p, _)| 2 + p.len() as u64 + 16).sum();
    let data_start: u64 = 10 + table_bytes;

    let mut offset = data_start;
    for (path, data) in entries {
        let pb = path.as_bytes();
        out.write_all(&(pb.len() as u16).to_le_bytes())?;
        out.write_all(pb)?;
        out.write_all(&offset.to_le_bytes())?;
        out.write_all(&(data.len() as u64).to_le_bytes())?;
        offset += data.len() as u64;
    }

    for (idx, (_, data)) in entries.iter().enumerate() {
        if let Some(k) = key {
            out.write_all(&chacha20_xor(data, k, idx as u64))?;
        } else {
            out.write_all(data)?;
        }
    }
    Ok(())
}

fn chacha20_xor(data: &[u8], key: &[u8; 32], idx: u64) -> Vec<u8> {
    use chacha20::ChaCha20;
    use chacha20::cipher::{KeyIvInit, StreamCipher};
    let mut nonce = [0u8; 12];
    nonce[4..12].copy_from_slice(&idx.to_le_bytes());
    let mut cipher = ChaCha20::new(key.into(), &nonce.into());
    let mut buf = data.to_vec();
    cipher.apply_keystream(&mut buf);
    buf
}
