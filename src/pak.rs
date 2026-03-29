//! Binary asset archive (`.npak`) — pack multiple files into one, with optional
//! ChaCha20 stream encryption.
//!
//! # Format
//! ```text
//! [Header — 10 bytes]
//!   magic         [u8; 4] = b"NPAK"
//!   version       u8      = 1
//!   flags         u8      bit 0 = encrypted
//!   entry_count   u32le
//!
//! [Entry table — entry_count records]
//!   path_len      u16le
//!   path          [u8; path_len]  UTF-8, forward-slash separated
//!   data_offset   u64le           absolute byte offset from file start
//!   data_size     u64le
//!
//! [Data section]
//!   raw (or encrypted) bytes of each entry, back-to-back
//! ```
//!
//! When encrypted, every entry is independently encrypted with ChaCha20:
//! key = 32 bytes supplied by the caller, nonce = `[0u8;4] ++ idx_as_u64le`
//! (unique per entry, deterministic so the same key always decrypts).
//!
//! # Quick start
//! ```no_run
//! use nene::pak::{PakBuilder, PakReader};
//!
//! // Pack
//! let mut b = PakBuilder::new();
//! b.add("textures/hero.png", std::fs::read("assets/textures/hero.png").unwrap());
//! b.finish_file("build/assets.npak", None).unwrap();
//!
//! // Unpack
//! let pak = PakReader::from_file("build/assets.npak", None).unwrap();
//! let bytes = pak.read("textures/hero.png").unwrap();
//! ```

use std::collections::HashMap;
use std::io::{self, Write};
use std::path::Path;
use std::sync::OnceLock;

use chacha20::ChaCha20;
use chacha20::cipher::{KeyIvInit, StreamCipher};

// ── Embedded PAK (set by `embed_assets!` macro) ───────────────────────────────

static EMBEDDED_PAK: OnceLock<&'static [u8]> = OnceLock::new();

/// Register a compile-time-embedded PAK buffer so that [`crate::asset::Assets`]
/// can auto-mount it in `new()`.
///
/// Called automatically by the [`embed_assets!`](crate::embed_assets) macro —
/// don't call this directly.
pub fn register_embedded_pak(bytes: &'static [u8]) {
    let _ = EMBEDDED_PAK.set(bytes);
}

/// Return the embedded PAK bytes, if [`register_embedded_pak`] was called.
pub(crate) fn embedded_pak_bytes() -> Option<&'static [u8]> {
    EMBEDDED_PAK.get().copied()
}

const MAGIC: &[u8; 4] = b"NPAK";
const VERSION: u8 = 1;
const FLAG_ENCRYPTED: u8 = 0x01;

// ── PakBuilder ────────────────────────────────────────────────────────────────

/// Assembles an `.npak` archive from named byte buffers.
///
/// Use [`finish`](Self::finish) or [`finish_file`](Self::finish_file) to write
/// the result. Pass a 32-byte key to enable ChaCha20 encryption.
pub struct PakBuilder {
    entries: Vec<(String, Vec<u8>)>,
}

impl PakBuilder {
    pub fn new() -> Self {
        Self { entries: Vec::new() }
    }

    /// Add (or replace) a file entry.
    ///
    /// `path` should use forward slashes, e.g. `"textures/hero.png"`.
    pub fn add(&mut self, path: impl Into<String>, data: Vec<u8>) -> &mut Self {
        self.entries.push((path.into(), data));
        self
    }

    /// Write the archive to `out`. Pass `Some(key)` to encrypt.
    pub fn finish(&self, out: &mut impl Write, key: Option<&[u8; 32]>) -> io::Result<()> {
        let flags: u8 = if key.is_some() { FLAG_ENCRYPTED } else { 0 };

        // Header
        out.write_all(MAGIC)?;
        out.write_all(&[VERSION, flags])?;
        out.write_all(&(self.entries.len() as u32).to_le_bytes())?;

        // Table size: sum of (2 + path_bytes + 8 + 8) per entry
        // Header is 10 bytes.
        let table_bytes: u64 = self
            .entries
            .iter()
            .map(|(p, _)| 2 + p.len() as u64 + 16)
            .sum();
        let data_start: u64 = 10 + table_bytes;

        // Entry table
        let mut offset = data_start;
        for (path, data) in &self.entries {
            let path_bytes = path.as_bytes();
            out.write_all(&(path_bytes.len() as u16).to_le_bytes())?;
            out.write_all(path_bytes)?;
            out.write_all(&offset.to_le_bytes())?;
            out.write_all(&(data.len() as u64).to_le_bytes())?;
            offset += data.len() as u64;
        }

        // Data section
        for (idx, (_, data)) in self.entries.iter().enumerate() {
            if let Some(k) = key {
                out.write_all(&chacha20_xor(data, k, idx as u64))?;
            } else {
                out.write_all(data)?;
            }
        }
        Ok(())
    }

    /// Convenience wrapper: write directly to a file at `path`.
    pub fn finish_file(
        &self,
        path: impl AsRef<Path>,
        key: Option<&[u8; 32]>,
    ) -> io::Result<()> {
        let mut f = std::fs::File::create(path)?;
        self.finish(&mut f, key)
    }
}

impl Default for PakBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// ── PakReader ─────────────────────────────────────────────────────────────────

/// Reads entries from an in-memory `.npak` archive.
///
/// The entire archive is kept in memory for zero-copy slice access.
pub struct PakReader {
    data: Vec<u8>,
    /// path → (data_offset, data_size, entry_index)
    index: HashMap<String, (u64, u64, u64)>,
    encrypted: bool,
    key: Option<[u8; 32]>,
}

impl PakReader {
    /// Parse an archive from raw bytes. Pass `key` if it was encrypted.
    pub fn from_bytes(data: Vec<u8>, key: Option<[u8; 32]>) -> io::Result<Self> {
        if data.len() < 10 {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "PAK too small"));
        }
        if &data[0..4] != MAGIC {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "bad PAK magic"));
        }
        if data[4] != VERSION {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("unsupported PAK version {}", data[4]),
            ));
        }
        let encrypted = data[5] & FLAG_ENCRYPTED != 0;
        let count = u32::from_le_bytes(data[6..10].try_into().unwrap()) as usize;

        let mut pos = 10usize;
        let mut index = HashMap::with_capacity(count);
        for idx in 0..count {
            if pos + 2 > data.len() {
                return Err(short_read());
            }
            let path_len =
                u16::from_le_bytes(data[pos..pos + 2].try_into().unwrap()) as usize;
            pos += 2;

            if pos + path_len + 16 > data.len() {
                return Err(short_read());
            }
            let path = String::from_utf8_lossy(&data[pos..pos + path_len]).into_owned();
            pos += path_len;

            let offset = u64::from_le_bytes(data[pos..pos + 8].try_into().unwrap());
            let size = u64::from_le_bytes(data[pos + 8..pos + 16].try_into().unwrap());
            pos += 16;

            index.insert(path, (offset, size, idx as u64));
        }

        Ok(Self { data, index, encrypted, key })
    }

    /// Load an archive from disk. Pass `key` if it was encrypted.
    pub fn from_file(path: impl AsRef<Path>, key: Option<[u8; 32]>) -> io::Result<Self> {
        Self::from_bytes(std::fs::read(path)?, key)
    }

    /// Return the raw (decrypted) bytes of `path`, or `None` if not found.
    pub fn read(&self, path: &str) -> Option<Vec<u8>> {
        let &(offset, size, idx) = self.index.get(path)?;
        let start = offset as usize;
        let end = start + size as usize;
        if end > self.data.len() {
            return None;
        }
        let raw = self.data[start..end].to_vec();
        if self.encrypted {
            let key = self.key.as_ref()?;
            Some(chacha20_xor(&raw, key, idx))
        } else {
            Some(raw)
        }
    }

    /// Returns `true` if the archive contains an entry at `path`.
    pub fn contains(&self, path: &str) -> bool {
        self.index.contains_key(path)
    }

    /// Iterator over all entry paths stored in the archive.
    pub fn paths(&self) -> impl Iterator<Item = &str> {
        self.index.keys().map(String::as_str)
    }

    /// Number of entries in the archive.
    pub fn len(&self) -> usize {
        self.index.len()
    }

    /// `true` if the archive contains no entries.
    pub fn is_empty(&self) -> bool {
        self.index.is_empty()
    }
}

// ── helpers ───────────────────────────────────────────────────────────────────

/// ChaCha20 keystream XOR. Same operation for encrypt and decrypt.
fn chacha20_xor(data: &[u8], key: &[u8; 32], idx: u64) -> Vec<u8> {
    let mut nonce = [0u8; 12];
    nonce[4..12].copy_from_slice(&idx.to_le_bytes());
    let mut cipher = ChaCha20::new(key.into(), &nonce.into());
    let mut buf = data.to_vec();
    cipher.apply_keystream(&mut buf);
    buf
}

fn short_read() -> io::Error {
    io::Error::new(io::ErrorKind::UnexpectedEof, "truncated PAK entry table")
}
