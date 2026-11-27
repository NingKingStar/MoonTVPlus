//! A loopback static server for the generated demo media. It exists so the
//! demo runs from the executable and `media/` alone — no Node, no fixture
//! service, no web view. Range requests are served, because the native
//! media engine uses them for MP4 and single-file HLS.

use std::{
    fs::File,
    io::{BufRead, BufReader, Read, Seek, SeekFrom, Write},
    net::{TcpListener, TcpStream},
    path::{Path, PathBuf},
    sync::OnceLock,
    thread,
};

static BASE: OnceLock<Option<String>> = OnceLock::new();

/// Starts the server once; the answer is the URL prefix (`…/media`) or why
/// there is none.
pub fn start() -> Option<String> {
    BASE.get_or_init(|| {
        let root = media_root()?;
        let canonical = root.canonicalize().ok()?;
        let listener = match TcpListener::bind("127.0.0.1:0") {
            Ok(listener) => listener,
            Err(error) => {
                log::warn!("demo media server could not bind: {error}");
                return None;
            }
        };
        let port = listener.local_addr().ok()?.port();
        let root = std::sync::Arc::new(canonical);
        {
            let root = root.clone();
            thread::Builder::new()
                .name("moontv-demo-media".to_owned())
                .spawn(move || serve(listener, root))
                .ok()?;
        }
        let base = format!("http://127.0.0.1:{port}/media");
        crate::logbook::event(
            "media_server",
            serde_json::json!({"base": base, "root": root.display().to_string()}),
        );
        Some(base)
    })
    .clone()
}

/// The URL prefix, when the server is running.
pub fn base() -> Option<String> {
    BASE.get().cloned().flatten()
}

/// `base` + `/` + a path under the media directory.
pub fn url(path: &str) -> String {
    match base() {
        Some(base) => format!("{}/{}", base, path.trim_start_matches('/')),
        None => String::new(),
    }
}

fn media_root() -> Option<PathBuf> {
    if let Ok(dir) = std::env::var("MOONTV_MEDIA_DIR")
        && !dir.trim().is_empty()
    {
        let path = PathBuf::from(dir);
        if path.join("clip.mp4").is_file() || path.join("public/clip.mp4").is_file() {
            return Some(if path.join("clip.mp4").is_file() {
                path
            } else {
                path.join("public")
            });
        }
    }
    let exe_dir = std::env::current_exe().ok()?.parent()?.to_path_buf();
    let cwd = std::env::current_dir().ok();
    let candidates = std::iter::once(exe_dir.join("media")).chain(
        cwd.iter()
            .flat_map(|dir| [dir.join("media"), dir.join("public")]),
    );
    candidates
        .into_iter()
        .find(|candidate| candidate.join("clip.mp4").is_file())
}

fn serve(listener: TcpListener, root: std::sync::Arc<PathBuf>) {
    for stream in listener.incoming() {
        let Ok(stream) = stream else { continue };
        let root = root.clone();
        let _ = thread::Builder::new()
            .name("moontv-demo-media-conn".to_owned())
            .spawn(move || {
                if let Err(error) = handle(stream, &root) {
                    log::debug!("demo media connection ended: {error}");
                }
            });
    }
}

fn handle(mut stream: TcpStream, root: &Path) -> std::io::Result<()> {
    stream.set_read_timeout(Some(std::time::Duration::from_secs(10)))?;
    stream.set_write_timeout(Some(std::time::Duration::from_secs(30)))?;
    let peer = stream.try_clone()?;
    let mut reader = BufReader::new(peer);
    let mut request_line = String::new();
    if reader.read_line(&mut request_line)? == 0 {
        return Ok(());
    }
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or("");
    let target = parts.next().unwrap_or("");
    if method != "GET" && method != "HEAD" {
        return respond_empty(&mut stream, 405, "Method Not Allowed");
    }
    let mut range_header = None;
    for _ in 0..64 {
        let mut line = String::new();
        if reader.read_line(&mut line)? == 0 {
            break;
        }
        if line == "\r\n" || line == "\n" {
            break;
        }
        if let Some((name, value)) = line.split_once(':')
            && name.eq_ignore_ascii_case("range")
        {
            range_header = Some(value.trim().to_owned());
        }
    }
    let path = target.split('?').next().unwrap_or(target);
    let Some(relative) = path.strip_prefix("/media/") else {
        return respond_empty(&mut stream, 404, "Not Found");
    };
    let Some(relative) = decode_path(relative) else {
        return respond_empty(&mut stream, 400, "Bad Request");
    };
    if relative
        .split('/')
        .any(|segment| segment.is_empty() || segment == "." || segment == "..")
    {
        return respond_empty(&mut stream, 400, "Bad Request");
    }
    let allowed = matches!(
        Path::new(&relative).extension().and_then(|e| e.to_str()),
        Some("mp4" | "m3u8" | "ts" | "m4s" | "bin" | "key" | "jpg" | "png" | "webp")
    );
    if !allowed {
        return respond_empty(&mut stream, 404, "Not Found");
    }
    let full = root.join(&relative);
    let Ok(canonical) = full.canonicalize() else {
        log::info!("demo media 404: {relative}");
        return respond_empty(&mut stream, 404, "Not Found");
    };
    if !canonical.starts_with(root) || !canonical.is_file() {
        return respond_empty(&mut stream, 403, "Forbidden");
    }
    let mut file = File::open(&canonical)?;
    let length = file.metadata()?.len();
    let mime = mime_for(&canonical);
    let (status, start, end) = match range_header.as_deref().and_then(|h| parse_range(h, length)) {
        Some((start, end)) => (206, start, end),
        None => (200, 0, length.saturating_sub(1)),
    };
    if length == 0 {
        return respond_empty(&mut stream, 200, "OK");
    }
    let count = end.saturating_sub(start) + 1;
    let mut head = format!(
        "HTTP/1.1 {status} {}\r\nContent-Type: {mime}\r\nContent-Length: {count}\r\nAccept-Ranges: bytes\r\nCache-Control: no-cache\r\nAccess-Control-Allow-Origin: *\r\nConnection: close\r\n",
        if status == 206 {
            "Partial Content"
        } else {
            "OK"
        }
    );
    if status == 206 {
        head.push_str(&format!("Content-Range: bytes {start}-{end}/{length}\r\n"));
    }
    head.push_str("\r\n");
    stream.write_all(head.as_bytes())?;
    if method == "HEAD" {
        return Ok(());
    }
    file.seek(SeekFrom::Start(start))?;
    let mut remaining = count;
    let mut buffer = vec![0_u8; 64 * 1024];
    while remaining > 0 {
        let want = remaining.min(buffer.len() as u64) as usize;
        let read = file.read(&mut buffer[..want])?;
        if read == 0 {
            break;
        }
        stream.write_all(&buffer[..read])?;
        remaining -= read as u64;
    }
    Ok(())
}

fn respond_empty(stream: &mut TcpStream, status: u16, reason: &str) -> std::io::Result<()> {
    let body = format!("{status} {reason}");
    let response = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(response.as_bytes())
}

fn parse_range(value: &str, length: u64) -> Option<(u64, u64)> {
    let spec = value.strip_prefix("bytes=")?.split(',').next()?.trim();
    if length == 0 {
        return None;
    }
    let (start, end) = spec.split_once('-')?;
    if start.is_empty() {
        let suffix: u64 = end.parse().ok()?;
        if suffix == 0 {
            return None;
        }
        let start = length.saturating_sub(suffix);
        return Some((start, length - 1));
    }
    let start: u64 = start.parse().ok()?;
    if start >= length {
        return None;
    }
    let end: u64 = if end.is_empty() {
        length - 1
    } else {
        end.parse::<u64>().ok()?.min(length - 1)
    };
    if end < start {
        return None;
    }
    Some((start, end))
}

fn decode_path(value: &str) -> Option<String> {
    let bytes = value.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            if i + 2 >= bytes.len() {
                return None;
            }
            let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).ok()?;
            out.push(u8::from_str_radix(hex, 16).ok()?);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}

fn mime_for(path: &Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()) {
        Some("mp4") => "video/mp4",
        Some("m3u8") => "application/vnd.apple.mpegurl",
        Some("ts") => "video/mp2t",
        Some("m4s") => "video/iso.segment",
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("png") => "image/png",
        Some("webp") => "image/webp",
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ranges_are_bounded() {
        assert_eq!(parse_range("bytes=0-99", 1000), Some((0, 99)));
        assert_eq!(parse_range("bytes=900-", 1000), Some((900, 999)));
        assert_eq!(parse_range("bytes=-100", 1000), Some((900, 999)));
        assert_eq!(parse_range("bytes=1000-", 1000), None);
        assert_eq!(parse_range("bytes=5-1", 1000), None);
        assert_eq!(parse_range("items=0-1", 1000), None);
    }

    #[test]
    fn paths_are_decoded_and_traversal_is_refused() {
        assert_eq!(decode_path("a%20b.mp4").as_deref(), Some("a b.mp4"));
        assert!(decode_path("%zz").is_none());
    }
}
