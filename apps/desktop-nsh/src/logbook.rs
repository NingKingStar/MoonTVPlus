//! An append-only local log of product events, for verification. It never
//! receives passwords, tokens or raw query strings.

use std::{fs::OpenOptions, io::Write, sync::Mutex};

use serde_json::Value;

static LOCK: Mutex<()> = Mutex::new(());

/// Where the product log lives when `MOONTV_LOG` is not set: the same app
/// data directory as the settings, so it survives restarts regardless of the
/// working directory the launcher picks.
fn default_log_path() -> std::path::PathBuf {
    let dir = std::env::var("MOONTV_DATA_DIR")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .map(std::path::PathBuf::from)
        .or_else(|| {
            std::env::var("APPDATA")
                .ok()
                .map(|base| std::path::Path::new(&base).join("MoonTVPlus").join("nsh"))
        })
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    dir.join("product.jsonl")
}

pub fn event(kind: &str, value: Value) {
    let row = serde_json::json!({
        "ms": nsh_core::time::unix_ms(),
        "kind": kind,
        "value": value,
    });
    log::info!("{row}");
    let path = std::env::var("MOONTV_LOG")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| default_log_path());
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _guard = LOCK.lock();
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(file, "{row}");
    }
}

/// A URL safe to log: scheme, host, port and path; the query is dropped
/// because media URLs can carry signatures.
pub fn safe_url(raw: &str) -> String {
    match url::Url::parse(raw) {
        Ok(url) => {
            let mut out = format!(
                "{}://{}{}",
                url.scheme(),
                url.host_str().unwrap_or(""),
                url.path()
            );
            if let Some(port) = url.port() {
                out = format!(
                    "{}://{}:{}{}",
                    url.scheme(),
                    url.host_str().unwrap_or(""),
                    port,
                    url.path()
                );
            }
            out
        }
        Err(_) => raw.split('?').next().unwrap_or(raw).to_owned(),
    }
}
