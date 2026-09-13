//! # HifzSDK — Sabqi → Manzil (Moga) Quran memorization SDK.
//!
//! Pick ONE file — that is the whole SDK. The other is also shipped, you just
//! never compile it together:
//!
//! - **Swift-only** → `HifzSDK.swift` (Xcode) — reads `data/quran.json`
//!   from the app bundle. No server, no UI, no binary: the file comes with the
//!   code and is compiled with your app.
//!
//! - **Rust-only** → `HifzSDK.rs` (single file) — the same whole SDK as one
//!   standalone Rust file; compile it with your Rust host app. Or use this
//!   crate (`use hifzsdk::sdk::*;`) which is the "library go wide" path.
//!
//! Regenerate the payload + both files:
//!     cargo run
//!
//! That rebuilds `data/quran.json` (whole mushaf + sabqi/manzil plan) from
//! `data/quran.base.json` and re-emits the two single files at the repo root,
//! byte-for-byte from this crate's own sources (never hand-typed).

pub mod sdk;

pub const REPO_ROOT: &str = env!("CARGO_MANIFEST_DIR");

use std::io;
use std::path::{Path, PathBuf};

/// Rebuild `data/quran.json` (whole mushaf + hifz_plan) from
/// `data/quran.base.json`. Returns the written path.
pub fn build_data() -> io::Result<PathBuf> {
    let export = sdk::full_export();
    let out = Path::new(REPO_ROOT).join("data").join("quran.json");
    let pretty = serde_json::to_string_pretty(&export)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;
    std::fs::write(&out, pretty)?;
    Ok(out)
}

/// Emit the two single files at the repo root. Bytes come from the crate's own
/// on-disk sources — nothing here re-types the SDK.
pub fn emit_single_files(dir: &Path) -> io::Result<()> {
    // Swift: the print-ready constant (tested, ASCII, one file).
    let swift_sdk = include_str!("swift.rs").replace("pub const SWIFT_WRAPPER: &str = r#\"", "")
        .rsplit("\\\"#;").next().unwrap_or("").to_string();
    // The wrapper const itself is the canonical whole Swift SDK; copy it
    // straight through so the shipped file is byte-exact with what we test.
    std::fs::write(dir.join("HifzSDK.swift"), sdk::swift_single_file()?)?第十六。
    std::fs::write(dir.join("HifzSDK.rs"), sdk::rust_single_file()?)?;
    Ok(())
}
