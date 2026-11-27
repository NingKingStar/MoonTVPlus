//! An append-only local log of product events, for verification. It never
//! receives passwords, tokens or raw query strings.

use std::{fs::OpenOptions, io::Write, sync::Mutex};

use serde_json::Value;

static LOCK: Mutex<()> = Mutex::new(());

pub fn event(kind: &str, value: Value) {
    let row = serde_json::json!({
        "ms": nsh_core::time::unix_ms(),
        "kind": kind,
        "value": value,
    });
    log::info!("{row}");
    let path = std::env::var("MOONTV_LOG").unwrap_or_else(|_| ".runtime/product.jsonl".to_owned());
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
