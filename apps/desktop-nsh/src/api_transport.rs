//! Memory-only, instance-bound credentials and a single-flight refresh path.
use crate::model::{api_url, same_origin};
use serde_json::Value;
use std::{
    io::Read,
    sync::{Mutex, OnceLock},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

pub const USER_AGENT: &str = "MoonTVPlus App/0.5 NSH (Windows)";
pub const ACCESS_AGE_MS: u64 = 4 * 60 * 60 * 1000;
pub const REFRESH_AGE_MS: u64 = 60 * 24 * 60 * 60 * 1000;
pub const RENEWAL_THRESHOLD_MS: u64 = 10 * 60 * 1000;

#[derive(Clone, Debug, PartialEq)]
pub enum ApiError {
    Unauthorized,
    Forbidden,
    RateLimited(Option<u64>),
    Http(u16),
    Network(String),
    BadResponse(String),
    Unsupported(String),
}
impl ApiError {
    pub fn message(&self) -> String {
        match self {
            Self::Unauthorized => "会话已过期或被撤销，请重新登录（本机进度已保留）".into(),
            Self::Forbidden => "当前账号没有这项权限（会话未退出）".into(),
            Self::RateLimited(Some(s)) => format!("请求过于频繁，请等待 {s} 秒后手动重试"),
            Self::RateLimited(None) => "请求过于频繁，请稍后手动重试".into(),
            Self::Http(s) => format!("实例返回 HTTP {s}"),
            Self::Network(_) => "网络、TLS 或超时错误，请检查实例后重试".into(),
            Self::BadResponse(s) | Self::Unsupported(s) => s.clone(),
        }
    }
    pub fn is_auth(&self) -> bool {
        matches!(self, Self::Unauthorized)
    }
}
#[derive(Clone)]
pub struct Request {
    pub url: String,
    pub method: String,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}
impl Request {
    pub fn get(url: String) -> Self {
        Self {
            url,
            method: "GET".into(),
            headers: vec![],
            body: vec![],
        }
    }
}
pub struct Response {
    status: u16,
    text: String,
    retry_after: Option<u64>,
}
impl Response {
    pub fn status(&self) -> u16 {
        self.status
    }
    pub fn text(&self) -> &str {
        &self.text
    }
}
#[derive(Clone)]
struct Auth {
    base: String,
    seed: String,
    token: String,
    storage: String,
}
static AUTH: Mutex<Option<Auth>> = Mutex::new(None);
static REFRESH: OnceLock<tokio::sync::Mutex<()>> = OnceLock::new();
pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
pub fn token_metadata(token: &str) -> Option<Value> {
    serde_json::from_str(
        &percent_encoding::percent_decode_str(token)
            .decode_utf8()
            .ok()?,
    )
    .ok()
}
pub fn needs_refresh(token: &str, now: u64) -> bool {
    token_metadata(token)
        .and_then(|v| v["timestamp"].as_u64())
        .is_some_and(|ts| {
            now.saturating_add(RENEWAL_THRESHOLD_MS) >= ts.saturating_add(ACCESS_AGE_MS)
        })
}
pub fn install(base: &str, token: &str, storage: &str) {
    *AUTH.lock().unwrap() = Some(Auth {
        base: base.into(),
        seed: token.into(),
        token: token.into(),
        storage: storage.into(),
    });
}
pub fn sync_supported() -> bool {
    AUTH.lock()
        .unwrap()
        .as_ref()
        .is_some_and(|a| a.storage != "localstorage")
}
pub fn clear() {
    *AUTH.lock().unwrap() = None;
}
pub fn current_token() -> Option<String> {
    AUTH.lock().unwrap().as_ref().map(|a| a.token.clone())
}
pub fn is_current(base: &str, seed: &str) -> bool {
    AUTH.lock()
        .unwrap()
        .as_ref()
        .is_some_and(|a| a.base == base && (a.seed == seed || a.token == seed))
}
fn snapshot(base: &str, seed: &str) -> Result<Auth, ApiError> {
    AUTH.lock()
        .unwrap()
        .as_ref()
        .filter(|a| a.base == base && (a.seed == seed || a.token == seed))
        .cloned()
        .ok_or(ApiError::Unauthorized)
}

pub fn request(url: String, token: &str) -> Request {
    let mut r = Request::get(url);
    r.headers.push(("User-Agent".into(), USER_AGENT.into()));
    r.headers.push(("Accept".into(), "application/json".into()));
    if !token.is_empty() {
        r.headers
            .push(("Authorization".into(), format!("Bearer {token}")));
    }
    r
}
fn replace_auth(r: &mut Request, token: &str) {
    r.headers
        .retain(|(n, _)| !n.eq_ignore_ascii_case("authorization"));
    r.headers
        .push(("Authorization".into(), format!("Bearer {token}")));
}
fn classify(r: Response) -> Result<Response, ApiError> {
    match r.status {
        200..=299 => Ok(r),
        401 => Err(ApiError::Unauthorized),
        403 => Err(ApiError::Forbidden),
        429 => Err(ApiError::RateLimited(r.retry_after)),
        s => Err(ApiError::Http(s)),
    }
}

/// The same status mapping for callers that read a body incrementally and
/// cannot go through `classify`'s owned response.
pub fn status_error(status: u16, retry_after: Option<u64>) -> ApiError {
    match status {
        401 => ApiError::Unauthorized,
        403 => ApiError::Forbidden,
        429 => ApiError::RateLimited(retry_after),
        s => ApiError::Http(s),
    }
}

/// A proxy such as `http://127.0.0.1:7890`, normalised from whatever the
/// environment or Windows gave us. `localhost` is rewritten to `127.0.0.1`
/// because a dual-stack resolver often answers `::1` first and a Windows
/// proxy usually listens on IPv4 only.
fn normalize_proxy(raw: &str) -> Option<String> {
    let raw = raw.trim();
    if raw.is_empty() {
        return None;
    }
    let candidate = raw
        .split(';')
        .map(str::trim)
        .find_map(|part| match part.strip_prefix("http=") {
            Some(host) if !host.is_empty() => Some(host),
            _ if !part.contains('=') && !part.is_empty() => Some(part),
            _ => None,
        })?
        .replace("localhost", "127.0.0.1");
    if candidate.contains("://") {
        Some(candidate)
    } else {
        Some(format!("http://{candidate}"))
    }
}

fn proxy_from_env() -> Option<String> {
    for key in [
        "HTTPS_PROXY",
        "https_proxy",
        "HTTP_PROXY",
        "http_proxy",
        "ALL_PROXY",
        "all_proxy",
    ] {
        if let Ok(value) = std::env::var(key)
            && let Some(proxy) = normalize_proxy(&value)
        {
            return Some(proxy);
        }
    }
    None
}

/// Windows Internet Settings (the same proxy the browser and Clash/other
/// system proxies register). Read once; `reg` is asked without a console.
#[cfg(windows)]
fn proxy_from_system() -> Option<String> {
    use std::os::windows::process::CommandExt;
    let output = std::process::Command::new("reg")
        .args([
            "query",
            r"HKCU\Software\Microsoft\Windows\CurrentVersion\Internet Settings",
            "/v",
            "ProxyEnable",
        ])
        .creation_flags(0x0800_0000)
        .output()
        .ok()?;
    let enabled = String::from_utf8_lossy(&output.stdout).contains("0x1");
    if !enabled {
        return None;
    }
    let output = std::process::Command::new("reg")
        .args([
            "query",
            r"HKCU\Software\Microsoft\Windows\CurrentVersion\Internet Settings",
            "/v",
            "ProxyServer",
        ])
        .creation_flags(0x0800_0000)
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&output.stdout);
    let server = text
        .lines()
        .find(|line| line.contains("ProxyServer"))
        .and_then(|line| line.split("REG_SZ").nth(1))?;
    normalize_proxy(server)
}

#[cfg(not(windows))]
fn proxy_from_system() -> Option<String> {
    None
}

/// Proxy used for every request, resolved once per process.
fn proxy_url() -> Option<&'static str> {
    static PROXY: OnceLock<Option<String>> = OnceLock::new();
    PROXY
        .get_or_init(|| {
            let from_env = proxy_from_env();
            let source = if from_env.is_some() { "env" } else { "system" };
            let found = from_env.or_else(proxy_from_system);
            crate::logbook::event(
                "http_proxy",
                serde_json::json!({
                    "configured": found.is_some(),
                    "source": if found.is_some() { source } else { "none" },
                }),
            );
            found
        })
        .as_deref()
}

/// Loopback and LAN targets must never go through the system proxy; the
/// isolated contract tests and a self-hosted instance both rely on that.
fn is_loopback_or_private(host: &str) -> bool {
    let host = host
        .trim_start_matches('[')
        .trim_end_matches(']')
        .to_ascii_lowercase();
    if host == "localhost" || host.ends_with(".localhost") || host == "::1" {
        return true;
    }
    if let Ok(ip) = host.parse::<std::net::IpAddr>() {
        return match ip {
            std::net::IpAddr::V4(v4) => v4.is_loopback() || v4.is_private() || v4.is_link_local(),
            std::net::IpAddr::V6(v6) => v6.is_loopback(),
        };
    }
    host.ends_with(".local")
}

/// Proxy for one target URL, or `None` for loopback/LAN hosts.
fn proxy_for(url: &str) -> Option<&'static str> {
    let host = url::Url::parse(url).ok()?.host_str()?.to_string();
    if is_loopback_or_private(&host) {
        None
    } else {
        proxy_url()
    }
}

fn agent(seconds: u64, proxy: Option<&str>) -> ureq::Agent {
    let mut builder = ureq::AgentBuilder::new()
        .redirects(0)
        .timeout(Duration::from_secs(seconds));
    if let Some(proxy) = proxy.and_then(|p| ureq::Proxy::new(p).ok()) {
        builder = builder.proxy(proxy);
    }
    builder.build()
}

/// Builder that resolves the same proxy as ordinary API calls. Streaming
/// paths (SSE search, media relay) must use this too, otherwise they silently
/// bypass the proxy and fail on networks where direct access is blocked.
pub fn proxy_agent_builder(url: &str) -> ureq::AgentBuilder {
    let mut builder = ureq::AgentBuilder::new();
    if let Some(proxy) = proxy_for(url).and_then(|p| ureq::Proxy::new(p).ok()) {
        builder = builder.proxy(proxy);
    }
    builder
}

/// `ureq::Error` is large; box it so the closure stays cheap to match on.
fn send_with_body(request: ureq::Request, body: &[u8]) -> Result<ureq::Response, Box<ureq::Error>> {
    let result = if body.is_empty() {
        request.call()
    } else {
        request.send_bytes(body)
    };
    result.map_err(Box::new)
}
pub async fn raw(r: Request, seconds: u64) -> Result<Response, ApiError> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    std::thread::spawn(move || {
        let result = (|| {
            let proxy = proxy_for(&r.url);
            let mut q = agent(seconds, proxy).request(&r.method, &r.url);
            for (n, v) in &r.headers {
                q = q.set(n, v);
            }
            let resp = match send_with_body(q, &r.body) {
                Ok(v) => v,
                Err(first) => match *first {
                    ureq::Error::Status(_, v) => v,
                    // A stale proxy is common on Windows; retry the same
                    // request directly before reporting a network failure.
                    first if proxy.is_some() => {
                        let mut direct = agent(seconds, None).request(&r.method, &r.url);
                        for (n, v) in &r.headers {
                            direct = direct.set(n, v);
                        }
                        match send_with_body(direct, &r.body) {
                            Ok(v) => v,
                            Err(second) => match *second {
                                ureq::Error::Status(_, v) => v,
                                second => {
                                    return Err(ApiError::Network(format!(
                                        "request failed via proxy ({first}) and directly ({second})"
                                    )));
                                }
                            },
                        }
                    }
                    other => {
                        return Err(ApiError::Network(format!("request failed: {other}")));
                    }
                },
            };
            let status = resp.status();
            let retry_after = resp.header("Retry-After").and_then(|s| s.parse().ok());
            let mut bytes = Vec::new();
            resp.into_reader()
                .take(8 * 1024 * 1024 + 1)
                .read_to_end(&mut bytes)
                .map_err(|_| ApiError::Network("response incomplete".into()))?;
            if bytes.len() > 8 * 1024 * 1024 {
                return Err(ApiError::BadResponse("响应超过 8 MiB 限制".into()));
            }
            Ok(Response {
                status,
                text: String::from_utf8(bytes)
                    .map_err(|_| ApiError::BadResponse("响应编码不是 UTF-8".into()))?,
                retry_after,
            })
        })();
        let _ = tx.send(result);
    });
    rx.await
        .map_err(|_| ApiError::Network("worker stopped".into()))?
}
async fn renew(base: &str, seed: &str, failed_token: Option<&str>) -> Result<String, ApiError> {
    let _guard = REFRESH
        .get_or_init(|| tokio::sync::Mutex::new(()))
        .lock()
        .await;
    let a = snapshot(base, seed)?;
    if a.storage == "localstorage" {
        return Ok(a.token);
    }
    if let Some(failed) = failed_token {
        if a.token != failed {
            return Ok(a.token);
        }
    } else if !needs_refresh(&a.token, now_ms()) {
        return Ok(a.token);
    }
    if token_metadata(&a.token)
        .and_then(|v| v["refreshExpires"].as_u64())
        .is_none_or(|t| now_ms() >= t)
    {
        return Err(ApiError::Unauthorized);
    }
    let mut req = request(api_url(base, "api/auth/refresh"), &a.token);
    req.method = "POST".into();
    let response = classify(raw(req, 20).await?)?;
    let v: Value = serde_json::from_str(response.text())
        .map_err(|_| ApiError::BadResponse("刷新响应不是 JSON".into()))?;
    let token = v["token"]
        .as_str()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| ApiError::BadResponse("刷新没有返回 token".into()))?
        .to_owned();
    let mut auth = AUTH.lock().unwrap();
    let current = auth
        .as_mut()
        .filter(|s| s.base == base && s.seed == a.seed)
        .ok_or(ApiError::Unauthorized)?;
    current.token = token.clone();
    drop(auth);
    crate::relay::update_token(base, &token);
    Ok(token)
}
pub async fn send(mut r: Request, seconds: u64) -> Result<Response, ApiError> {
    let bearer = r
        .headers
        .iter()
        .find(|(n, _)| n.eq_ignore_ascii_case("authorization"))
        .map(|(_, v)| v.trim_start_matches("Bearer ").to_owned());
    let Some(seed) = bearer else {
        return classify(raw(r, seconds).await?);
    };
    let a = AUTH
        .lock()
        .unwrap()
        .clone()
        .filter(|a| a.seed == seed || a.token == seed)
        .ok_or(ApiError::Unauthorized)?;
    if !same_origin(&r.url, &a.base) || !r.url.starts_with(&a.base) {
        return Err(ApiError::Forbidden);
    }
    let token = renew(&a.base, &seed, None).await?;
    replace_auth(&mut r, &token);
    let response = raw(r.clone(), seconds).await?;
    if !is_current(&a.base, &seed) {
        return Err(ApiError::Unauthorized);
    }
    if response.status == 401 && a.storage != "localstorage" {
        let token = renew(&a.base, &seed, Some(&token)).await?;
        replace_auth(&mut r, &token);
        return classify(raw(r, seconds).await?);
    }
    classify(response)
}
/// Off-UI relay workers share the same single-flight gate as API calls.
pub fn media_token(base: &str, seed: &str, failed: Option<&str>) -> Result<String, ApiError> {
    let _ = snapshot(base, seed)?;
    tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .map_err(|_| ApiError::Network("runtime".into()))?
        .block_on(renew(base, seed, failed))
}
pub fn take_logout() -> Option<Request> {
    let a = AUTH.lock().unwrap().take()?;
    let mut r = request(api_url(&a.base, "api/logout"), &a.token);
    r.method = "POST".into();
    Some(r)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_expiry_threshold() {
        let token = crate::model::encode_query(
            &serde_json::json!({"timestamp":1000,"refreshExpires":REFRESH_AGE_MS}).to_string(),
        );
        assert!(!needs_refresh(
            &token,
            1000 + ACCESS_AGE_MS - RENEWAL_THRESHOLD_MS - 1
        ));
        assert!(needs_refresh(
            &token,
            1000 + ACCESS_AGE_MS - RENEWAL_THRESHOLD_MS
        ));
    }
    #[test]
    fn status_meanings() {
        assert!(!ApiError::Forbidden.is_auth());
        assert!(ApiError::Unauthorized.is_auth());
        assert!(ApiError::RateLimited(Some(90)).message().contains("90"));
    }
}
