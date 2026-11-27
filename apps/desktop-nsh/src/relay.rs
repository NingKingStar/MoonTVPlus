//! A scoped, session-owned loopback relay for media that the instance
//! protects with the same Bearer the API uses. It accepts only URLs on the
//! current instance's origin, only while the current session lives, and
//! rewrites HLS manifests and key/map URIs back to itself. It streams
//! binary bodies and forwards Range; it does not decode, demux, remux or
//! transcode. This is a product path, not the Node test fixture.

use std::{
    io::{BufRead, BufReader, Read, Write},
    net::{TcpListener, TcpStream},
    sync::{Mutex, OnceLock},
    thread,
    time::Duration,
};

use url::Url;

use crate::model::{encode_query, same_origin, short_hash};

#[derive(Clone)]
struct RelayConfig {
    /// Scheme, host and port of the instance (no path).
    origin: String,
    token: String,
    capability: String,
}

static PORT: OnceLock<Option<u16>> = OnceLock::new();
static CONFIG: Mutex<Option<RelayConfig>> = Mutex::new(None);
static CAP_SEQ: Mutex<u64> = Mutex::new(0);

/// Starts the loopback relay once. It serves nothing until a session is
/// configured.
pub fn start() -> Option<u16> {
    let value = PORT.get_or_init(|| {
        let listener = TcpListener::bind("127.0.0.1:0").ok()?;
        let port = listener.local_addr().ok()?.port();
        thread::Builder::new()
            .name("moontv-session-relay".to_owned())
            .spawn(move || {
                for stream in listener.incoming() {
                    let Ok(stream) = stream else { continue };
                    let _ = thread::Builder::new()
                        .name("moontv-session-relay-conn".to_owned())
                        .spawn(move || {
                            if let Err(error) = handle(stream) {
                                log::debug!("session relay connection ended: {error}");
                            }
                        });
                }
            })
            .ok()?;
        crate::logbook::event("relay_start", serde_json::json!({"port": port}));
        Some(port)
    });
    *value
}

fn port() -> Option<u16> {
    *PORT.get()?
}

/// Makes the relay serve media for this instance and token only. Called on
/// login; the previous capability dies with the old session.
fn origin_of(base: &str) -> Option<String> {
    let url = Url::parse(base).ok()?;
    let host = url.host_str()?;
    let mut origin = format!("{}://{}", url.scheme(), host);
    if let Some(port) = url.port() {
        origin.push_str(&format!(":{port}"));
    }
    Some(origin)
}

pub fn configure(base: &str, token: &str) {
    let Some(origin) = origin_of(base) else {
        return;
    };
    let mut seq = CAP_SEQ.lock().unwrap_or_else(|e| e.into_inner());
    *seq += 1;
    let capability = short_hash(&format!(
        "{}:{}:{}:{:?}",
        origin,
        short_hash(token),
        *seq,
        std::time::SystemTime::now()
    ));
    *CONFIG.lock().unwrap_or_else(|e| e.into_inner()) = Some(RelayConfig {
        origin: origin.clone(),
        token: token.to_owned(),
        capability: capability.clone(),
    });
    crate::logbook::event(
        "relay_session",
        serde_json::json!({"origin": origin, "capability": capability}),
    );
}

/// Keeps the current capability (in-flight media keeps working) and swaps in
/// the refreshed Bearer for the same instance origin.
pub fn update_token(base: &str, token: &str) {
    let Some(origin) = origin_of(base) else {
        return;
    };
    let mut config = CONFIG.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(config) = config.as_mut().filter(|c| c.origin == origin) {
        config.token = token.to_string();
        crate::logbook::event("relay_token", serde_json::json!({"origin": origin}));
    }
}

/// Ends the session's relay; requests with the old capability are refused.
pub fn clear() {
    *CONFIG.lock().unwrap_or_else(|e| e.into_inner()) = None;
}

/// A relay URL for `upstream` when it is on the current instance's origin.
pub fn url(upstream: &str) -> Option<String> {
    let port = port()?;
    let config = CONFIG.lock().unwrap_or_else(|e| e.into_inner()).clone()?;
    let parsed = Url::parse(upstream).ok()?;
    if !parsed.username().is_empty() || parsed.password().is_some() {
        return None;
    }
    if !same_origin(upstream, &config.origin) {
        return None;
    }
    Some(format!(
        "http://127.0.0.1:{port}/r/{}?u={}",
        config.capability,
        encode_query(upstream)
    ))
}

fn handle(mut stream: TcpStream) -> std::io::Result<()> {
    stream.set_read_timeout(Some(Duration::from_secs(10)))?;
    stream.set_write_timeout(Some(Duration::from_secs(30)))?;
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
        return respond(&mut stream, 405, "Method Not Allowed", &[]);
    }
    let mut range = None;
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
            range = Some(value.trim().to_owned());
        }
    }
    let Ok(url) = Url::parse(&format!("http://127.0.0.1{target}")) else {
        return respond(&mut stream, 400, "Bad Request", &[]);
    };
    let segments: Vec<&str> = url.path_segments().map(|s| s.collect()).unwrap_or_default();
    let Some(capability) = segments.get(1) else {
        return respond(&mut stream, 404, "Not Found", &[]);
    };
    let Some(upstream) = url
        .query_pairs()
        .find_map(|(key, value)| (key == "u").then(|| value.into_owned()))
    else {
        return respond(&mut stream, 400, "Bad Request", &[]);
    };
    let config = CONFIG.lock().unwrap_or_else(|e| e.into_inner()).clone();
    let Some(config) = config else {
        return respond(&mut stream, 403, "No session", &[]);
    };
    if *capability != config.capability {
        return respond(&mut stream, 403, "Expired session", &[]);
    }
    let parsed = match Url::parse(&upstream) {
        Ok(url) if matches!(url.scheme(), "http" | "https") => url,
        _ => return respond(&mut stream, 400, "Bad upstream", &[]),
    };
    if parsed.username() != ""
        || parsed.password().is_some()
        || !same_origin(&upstream, &config.origin)
    {
        return respond(&mut stream, 403, "Out of scope", &[]);
    }
    let result = fetch(&config, &upstream, range.as_deref(), 0);
    match result {
        Ok(fetched) => relay_body(&mut stream, method, fetched, &config),
        Err(error) => {
            crate::logbook::event(
                "relay_error",
                serde_json::json!({"url": crate::logbook::safe_url(&upstream), "why": error}),
            );
            respond(&mut stream, 502, "Upstream failed", &[])
        }
    }
}

struct Fetched {
    status: u16,
    content_type: String,
    content_length: Option<String>,
    content_range: Option<String>,
    accept_ranges: Option<String>,
    final_url: String,
    body: Box<dyn Read + Send>,
}

fn fetch(
    config: &RelayConfig,
    upstream: &str,
    range: Option<&str>,
    depth: u8,
) -> Result<Fetched, String> {
    if depth > 2 {
        return Err("too many redirects".to_owned());
    }
    let agent = crate::api_transport::proxy_agent_builder(upstream)
        .redirects(0)
        .timeout(Duration::from_secs(20))
        .build();
    let mut request = agent
        .get(upstream)
        .set("Authorization", &format!("Bearer {}", config.token))
        .set("User-Agent", "MoonTVPlus-NSH/0.2 (Windows)")
        .set("Accept", "*/*");
    if let Some(range) = range {
        request = request.set("Range", range);
    }
    let response = match request.call() {
        Ok(response) => response,
        Err(ureq::Error::Status(_, response)) => response,
        Err(ureq::Error::Transport(error)) => return Err(error.to_string()),
    };
    let status = response.status();
    let content_type = response
        .header("Content-Type")
        .unwrap_or("application/octet-stream")
        .to_owned();
    let location = response.header("Location").map(str::to_owned);
    if (300..400).contains(&status) {
        if let Some(location) = &location {
            let target = Url::parse(upstream)
                .ok()
                .and_then(|base| base.join(location).ok())
                .map(|url| url.to_string())
                .ok_or_else(|| "bad redirect".to_owned())?;
            if same_origin(&target, &config.origin) {
                return fetch(config, &target, range, depth + 1);
            }
            return Err(format!(
                "redirect outside instance: {}",
                crate::logbook::safe_url(&target)
            ));
        }
        return Err("redirect without location".to_owned());
    }
    Ok(Fetched {
        status,
        content_type,
        content_length: response.header("Content-Length").map(str::to_owned),
        content_range: response.header("Content-Range").map(str::to_owned),
        accept_ranges: response.header("Accept-Ranges").map(str::to_owned),
        final_url: upstream.to_owned(),
        body: Box::new(response.into_reader()),
    })
}

fn relay_body(
    stream: &mut TcpStream,
    method: &str,
    mut fetched: Fetched,
    config: &RelayConfig,
) -> std::io::Result<()> {
    let is_manifest = fetched
        .content_type
        .to_ascii_lowercase()
        .contains("mpegurl");
    let mut headers = vec![
        (
            "Accept-Ranges".to_owned(),
            fetched
                .accept_ranges
                .clone()
                .unwrap_or_else(|| "bytes".to_owned()),
        ),
        ("Cache-Control".to_owned(), "no-cache".to_owned()),
        ("Access-Control-Allow-Origin".to_owned(), "*".to_owned()),
    ];
    if is_manifest {
        let mut bytes = Vec::new();
        fetched.body.read_to_end(&mut bytes)?;
        let text = String::from_utf8_lossy(&bytes);
        let rewritten = rewrite_manifest(
            &text,
            &fetched.final_url,
            &config.capability,
            &config.origin,
        );
        let body = rewritten.into_bytes();
        headers.push((
            "Content-Type".to_owned(),
            "application/vnd.apple.mpegurl".to_owned(),
        ));
        headers.push(("Content-Length".to_owned(), body.len().to_string()));
        write_head(stream, fetched.status, &headers)?;
        if method != "HEAD" && fetched.status < 400 {
            stream.write_all(&body)?;
        }
        return Ok(());
    }
    headers.push(("Content-Type".to_owned(), fetched.content_type.clone()));
    if let Some(length) = &fetched.content_length {
        headers.push(("Content-Length".to_owned(), length.clone()));
    }
    if let Some(range) = &fetched.content_range {
        headers.push(("Content-Range".to_owned(), range.clone()));
    }
    write_head(stream, fetched.status, &headers)?;
    if method == "HEAD" {
        return Ok(());
    }
    if fetched.status >= 400 {
        return Ok(());
    }
    let mut buffer = vec![0_u8; 64 * 1024];
    loop {
        let read = fetched.body.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        stream.write_all(&buffer[..read])?;
    }
    Ok(())
}

fn rewrite_manifest(text: &str, base: &str, capability: &str, origin: &str) -> String {
    let mut out = String::with_capacity(text.len() + 256);
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            out.push_str(line);
            out.push('\n');
            continue;
        }
        if trimmed.starts_with('#') {
            out.push_str(&rewrite_tag_uris(trimmed, base, capability, origin));
            out.push('\n');
        } else {
            match relay_for(trimmed, base, capability, origin) {
                Some(url) => out.push_str(&url),
                None => out.push_str(trimmed),
            }
            out.push('\n');
        }
    }
    out
}

fn rewrite_tag_uris(line: &str, base: &str, capability: &str, origin: &str) -> String {
    let mut out = String::with_capacity(line.len() + 128);
    let mut rest = line;
    while let Some(index) = rest.find("URI=\"") {
        out.push_str(&rest[..index + 5]);
        let after = &rest[index + 5..];
        if let Some(end) = after.find('"') {
            let value = &after[..end];
            match relay_for(value, base, capability, origin) {
                Some(url) => out.push_str(&url),
                None => out.push_str(value),
            }
            out.push('"');
            rest = &after[end + 1..];
        } else {
            out.push_str(after);
            return out;
        }
    }
    out.push_str(rest);
    out
}

/// Resolves a manifest URI. Same-origin URIs go through the relay with the
/// session's Bearer; other origins stay absolute and are loaded by the
/// player directly, so the instance token is never sent to a third party.
fn relay_for(value: &str, base: &str, capability: &str, origin: &str) -> Option<String> {
    let absolute = if value.starts_with("//") {
        format!("{}:{value}", Url::parse(base).ok()?.scheme())
    } else if value.starts_with("http://") || value.starts_with("https://") {
        value.to_owned()
    } else {
        Url::parse(base).ok()?.join(value).ok()?.to_string()
    };
    let parsed = Url::parse(&absolute).ok()?;
    if parsed.scheme() != "http" && parsed.scheme() != "https" {
        return None;
    }
    if same_origin(&absolute, origin) {
        let port = port()?;
        Some(format!(
            "http://127.0.0.1:{port}/r/{capability}?u={}",
            encode_query(&absolute)
        ))
    } else {
        Some(absolute)
    }
}

fn write_head(
    stream: &mut TcpStream,
    status: u16,
    headers: &[(String, String)],
) -> std::io::Result<()> {
    let reason = match status {
        200 => "OK",
        206 => "Partial Content",
        301 => "Moved Permanently",
        302 => "Found",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        416 => "Range Not Satisfiable",
        _ => "Upstream",
    };
    let mut head = format!("HTTP/1.1 {status} {reason}\r\nConnection: close\r\n");
    for (name, value) in headers {
        head.push_str(&format!("{name}: {value}\r\n"));
    }
    head.push_str("\r\n");
    stream.write_all(head.as_bytes())
}

fn respond(
    stream: &mut TcpStream,
    status: u16,
    reason: &str,
    _headers: &[(&str, String)],
) -> std::io::Result<()> {
    let body = reason.as_bytes();
    let head = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream.write_all(head.as_bytes())?;
    stream.write_all(body)
}
