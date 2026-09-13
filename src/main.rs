
mod sdk;
mod swift;

use axum::{
    extract::State,
    http::StatusCode,
    response::Html,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;

const DATA_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/data");
const WEB_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/web");
const PORT: u16 = 8787;

#[derive(Clone)]
struct AppState {
    combined_json: Arc<serde_json::Value>,
    quran_path: PathBuf,
}

/// A bare io::Error can't auto-convert to our `(StatusCode, String)` error
/// tuple (orphan rule), so this one-liner is the conversion we use at every
/// `std::fs::*` site instead of a `?` that won't compile.
fn io_err(e: std::io::Error) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
}

#[derive(Deserialize)]
struct DirReq {
    folder: String,
}

#[derive(Deserialize)]
struct ActionReq {
    folder: String,
    #[serde(default)]
    action: String,
}

#[derive(Serialize)]
struct ScanResult {
    found: bool,
    path: String,
    exists: bool,
    empty: bool,
    wired: bool,
    size: u64,
}

#[derive(Serialize)]
struct CreateResult {
    ok: bool,
    message: String,
    path: String,
    created: bool,
    existed: bool,
}

#[derive(Serialize)]
struct ImportResult {
    ok: bool,
    message: String,
    path: String,
    copied: bool,
    size: u64,
}

#[derive(Serialize)]
struct WireResult {
    ok: bool,
    message: String,
    path: String,
    state: String,
    first_line: String,
}

async fn scan_folder(
    State(_st): State<AppState>,
    Json(req): Json<DirReq>,
) -> Result<Json<ScanResult>, (StatusCode, String)> {
    let base = PathBuf::from(&req.folder);
    if !base.exists() {
        return Err((StatusCode::NOT_FOUND, format!("folder {} does not exist", req.folder)));
    }
    match find_file(&base, "hifzsdk.swift") {
        Some(p) => {
            let raw = std::fs::read_to_string(&p).unwrap_or_default();
            let empty = raw.trim().is_empty();
            let wired = raw.lines().next().map(|l| l.trim() == swift::WIRE_MARKER).unwrap_or(false);
            Ok(Json(ScanResult {
                found: true,
                path: p.to_string_lossy().into_owned(),
                exists: true,
                empty,
                wired,
                size: std::fs::metadata(&p).map(|m| m.len()).unwrap_or(0),
            }))
        }
        None => Ok(Json(ScanResult {
            found: false,
            path: String::new(),
            exists: false,
            empty: false,
            wired: false,
            size: 0,
        })),
    }
}

/// Phase 1 - CREATE an EMPTY hifzsdk.swift shell. No code yet, just the file.
async fn create_folder(
    State(_st): State<AppState>,
    Json(req): Json<DirReq>,
) -> Result<Json<CreateResult>, (StatusCode, String)> {
    let base = PathBuf::from(&req.folder);
    if !base.exists() {
        return Err((StatusCode::NOT_FOUND, format!("folder {} does not exist", req.folder)));
    }

    let target = find_file(&base, "hifzsdk.swift").unwrap_or_else(|| base.join("hifzsdk.swift"));
    if target.exists() {
        return Ok(Json(CreateResult {
            ok: true,
            message: "hifzsdk.swift is already there. Leaving it untouched.".into(),
            path: target.to_string_lossy().into_owned(),
            created: false,
            existed: true,
        }));
    }

    std::fs::write(&target, "") .map_err(io_err)?;
    Ok(Json(CreateResult {
        ok: true,
        message: "Created an empty hifzsdk.swift shell.".into(),
        path: target.to_string_lossy().into_owned(),
        created: true,
        existed: false,
    }))
}

/// Phase 2 - IMPORT the modified quran.json (whole mushaf + sabqi/manzil plan
/// baked in) next to the swift file, so the app bundles data, not the mushaf.
async fn import_folder(
    State(st): State<AppState>,
    Json(req): Json<DirReq>,
) -> Result<Json<ImportResult>, (StatusCode, String)> {
    let base = PathBuf::from(&req.folder);
    if !base.exists() {
        return Err((StatusCode::NOT_FOUND, format!("folder {} does not exist", req.folder)));
    }

    let target = find_file(&base, "hifzsdk.swift").unwrap_or_else(|| base.join("hifzsdk.swift"));
    let parent = target.parent().unwrap_or(&base);

    std::fs::create_dir_all(parent) .map_err(io_err)?;
    let out = parent.join("quran.json");
    std::fs::copy(&st.quran_path, &out)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("quran copy failed: {e}")))?;
    let size = std::fs::metadata(&out).map(|m| m.len()).unwrap_or(0);

    Ok(Json(ImportResult {
        ok: true,
        message: "Imported the new quran.json (with the sabqi -> manzil plan) into your project.".into(),
        path: out.to_string_lossy().into_owned(),
        copied: true,
        size,
    }))
}

/// Phase 3 - WIRE. action="connect" plugs the wire in; action="disconnect"
/// empties hifzsdk.swift of all code but KEEPS the file itself.
async fn wire_folder(
    State(_st): State<AppState>,
    Json(req): Json<ActionReq>,
) -> Result<Json<WireResult>, (StatusCode, String)> {
    let base = PathBuf::from(&req.folder);
    if !base.exists() {
        return Err((StatusCode::NOT_FOUND, format!("folder {} does not exist", req.folder)));
    }

    let target = find_file(&base, "hifzsdk.swift").unwrap_or_else(|| base.join("hifzsdk.swift"));

    if req.action == "disconnect" {
        // Unplug: wipe all code but keep the (now empty) file on disk.
        if target.exists() {
            std::fs::write(&target, "") .map_err(io_err)?;
        } else {
            std::fs::write(&target, "") .map_err(io_err)?;
        }
        return Ok(Json(WireResult {
            ok: true,
            message: "Disconnected. hifzsdk.swift is emptied but kept on disk.".into(),
            path: target.to_string_lossy().into_owned(),
            state: "disconnected".into(),
            first_line: String::new(),
        }));
    }

    // connect: empty file gets the full wrapper; anything else gets spliced.
    let raw = if target.exists() {
        std::fs::read_to_string(&target) .map_err(io_err)?
    } else {
        String::new()
    };
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        std::fs::write(&target, swift::SWIFT_WRAPPER) .map_err(io_err)?;
        return Ok(Json(WireResult {
            ok: true,
            message: "Connected. Full HifzSDK Swift wrapper written.".into(),
            path: target.to_string_lossy().into_owned(),
            state: "connected".into(),
            first_line: swift::WIRE_MARKER.to_string(),
        }));
    }
    if trimmed.starts_with(swift::WIRE_MARKER) {
        return Ok(Json(WireResult {
            ok: true,
            message: "Already connected.".into(),
            path: target.to_string_lossy().into_owned(),
            state: "already connected".into(),
            first_line: swift::WIRE_MARKER.to_string(),
        }));
    }
    let mut lines: Vec<&str> = raw.lines().collect();
    lines.insert(0, swift::WIRE_MARKER);
    lines.insert(1, "");
    std::fs::write(&target, lines.join("\n")) .map_err(io_err)?;
    Ok(Json(WireResult {
        ok: true,
        message: "Connected. HifzSDK spliced onto line 1; your code left intact.".into(),
        path: target.to_string_lossy().into_owned(),
        state: "connected".into(),
        first_line: swift::WIRE_MARKER.to_string(),
    }))
}

/// Recursively search `base` for `file`.
fn find_file(base: &Path, file: &str) -> Option<PathBuf> {
    let mut stack: Vec<PathBuf> = vec![base.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else { continue };
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() {
                let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if name.starts_with('.') || name == "DerivedData" || name == "build" {
                    continue;
                }
                stack.push(p);
            } else if p.file_name().and_then(|n| n.to_str()) == Some(file) {
                return Some(p);
            }
        }
    }
    None
}

fn copy_quran_into(from: &Path, dir: &Path) -> std::io::Result<bool> {
    let target_dir = dir;
    std::fs::create_dir_all(target_dir)?;
    let out = target_dir.join("quran.json");
    std::fs::copy(from, &out)?;
    Ok(true)
}

#[tokio::main]
async fn main() {
    if std::env::args().any(|a| a == "--build-data") {
        let plan = sdk::full_export();
        let out = std::path::Path::new(DATA_DIR).join("quran.json");
        std::fs::write(&out, serde_json::to_string_pretty(&plan).unwrap()).unwrap();
        println!("[HifzSDK] wrote {} - quran.json now carries the Sabqi -> Manzil plan.", out.display());
        return;
    }

    let combined = sdk::full_export();
    let quran_path = std::path::Path::new(DATA_DIR).join("quran.json");
    let st = AppState {
        combined_json: Arc::new(combined),
        quran_path,
    };

    let app = Router::new()
        .route("/", get(serve_index))
        .route("/api/quran", get(serve_quran))
        .route("/api/scan", post(scan_folder))
        .route("/api/create", post(create_folder))
        .route("/api/import", post(import_folder))
        .route("/api/wire", post(wire_folder))
        .with_state(st);

    let listener = tokio::net::TcpListener::bind(("127.0.0.1", PORT)).await.unwrap();
    let addr = format!("http://127.0.0.1:{}", PORT);
    println!("[HifzSDK] wiring studio running at {}", addr);

    let _ = std::process::Command::new("sh")
        .arg("-c")
        .arg(format!("(xdg-open {} || sensible-browser {} || gnome-open {}) >/dev/null 2>&1 &", addr, addr, addr))
        .spawn();

    axum::serve(listener, app).await.unwrap();
}

async fn serve_index() -> Result<Html<String>, (StatusCode, String)> {
    std::fs::read_to_string(std::path::Path::new(WEB_DIR).join("index.html"))
        .map(Html)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

async fn serve_quran(State(st): State<AppState>) -> Json<serde_json::Value> {
    Json((*st.combined_json).clone())
}
