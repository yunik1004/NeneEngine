//! `nene-pack` — build an `.npak` asset archive from a directory.
//!
//! # Usage
//! ```text
//! nene-pack <assets_dir> <output.npak> [--key <64-hex-chars>]
//! ```
//!
//! All files under `<assets_dir>` are packed. Entry paths inside the archive
//! are relative to `<assets_dir>` and always use forward slashes.
//!
//! The optional `--key` flag enables ChaCha20 encryption. The key must be
//! exactly 64 hex characters (= 32 bytes).
//!
//! # Examples
//! ```text
//! # Unencrypted
//! nene-pack assets/ build/assets.npak
//!
//! # Encrypted
//! nene-pack assets/ build/assets.npak --key 000102030405...1f
//! ```

use std::path::{Path, PathBuf};

use nene::pak::PakBuilder;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let (src, dst, key) = parse_args(&args).unwrap_or_else(|e| {
        eprintln!("error: {e}");
        eprintln!();
        eprintln!("usage: nene-pack <assets_dir> <output.npak> [--key <64-hex-chars>]");
        std::process::exit(1);
    });

    let mut builder = PakBuilder::new();
    let mut count = 0usize;

    for entry in walkdir(&src) {
        let rel = entry
            .strip_prefix(&src)
            .expect("entry must be under src")
            .to_string_lossy()
            .replace('\\', "/");

        let data = match std::fs::read(&entry) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("warning: skipping '{}': {e}", entry.display());
                continue;
            }
        };
        builder.add(rel, data);
        count += 1;
    }

    if let Err(e) = builder.finish_file(&dst, key.as_ref()) {
        eprintln!("error writing '{}': {e}", dst.display());
        std::process::exit(1);
    }

    let encrypted = key.is_some();
    println!(
        "packed {count} file{} → {} {}",
        if count == 1 { "" } else { "s" },
        dst.display(),
        if encrypted { "(encrypted)" } else { "" }
    );
}

// ── helpers ───────────────────────────────────────────────────────────────────

fn parse_args(args: &[String]) -> Result<(PathBuf, PathBuf, Option<[u8; 32]>), String> {
    let mut positional: Vec<&str> = Vec::new();
    let mut key: Option<[u8; 32]> = None;
    let mut i = 1;
    while i < args.len() {
        if args[i] == "--key" {
            i += 1;
            let hex = args.get(i).ok_or("--key requires a value")?;
            key = Some(parse_key(hex)?);
        } else {
            positional.push(&args[i]);
        }
        i += 1;
    }
    if positional.len() < 2 {
        return Err("expected <assets_dir> and <output.npak>".into());
    }
    Ok((
        PathBuf::from(positional[0]),
        PathBuf::from(positional[1]),
        key,
    ))
}

fn parse_key(hex: &str) -> Result<[u8; 32], String> {
    if hex.len() != 64 {
        return Err(format!("key must be 64 hex chars (got {})", hex.len()));
    }
    let mut out = [0u8; 32];
    for (i, chunk) in hex.as_bytes().chunks(2).enumerate() {
        let hi = hex_digit(chunk[0])?;
        let lo = hex_digit(chunk[1])?;
        out[i] = (hi << 4) | lo;
    }
    Ok(out)
}

fn hex_digit(b: u8) -> Result<u8, String> {
    match b {
        b'0'..=b'9' => Ok(b - b'0'),
        b'a'..=b'f' => Ok(b - b'a' + 10),
        b'A'..=b'F' => Ok(b - b'A' + 10),
        _ => Err(format!("invalid hex character '{}'", b as char)),
    }
}

fn walkdir(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    collect(root, &mut out);
    out.sort();
    out
}

fn collect(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect(&path, out);
        } else {
            out.push(path);
        }
    }
}
