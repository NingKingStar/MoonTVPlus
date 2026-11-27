//! The event-stream half of search: the instance's `/api/search/ws` is a
//! `text/event-stream` (not a WebSocket), so this module parses SSE frames
//! and turns them into search events. Frames are decoded from raw bytes:
//! a network read can split a Chinese character in three, and one event can
//! span several `data:` lines. Nothing here touches UI state; the caller
//! decides when an event is still current.

use std::{
    io::Read,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use serde_json::Value;

use crate::{
    api_transport::{self, ApiError, USER_AGENT},
    model::{Candidate, api_url, encode_query},
};

/// A fully decoded SSE frame: only the fields this product reads.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Frame {
    pub event: String,
    pub data: String,
    pub id: String,
}

/// Incremental SSE decoder. It buffers bytes and emits one frame per blank
/// line, per the event-stream grammar: `data:` lines are joined with `\n`,
/// a single leading space after the colon is dropped, comments (`:`) and
/// unknown fields are ignored.
#[derive(Debug, Default)]
pub struct Decoder {
    buffer: Vec<u8>,
}

impl Decoder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn buffered_len(&self) -> usize {
        self.buffer.len()
    }

    /// Feeds a chunk and returns every frame completed by it.
    pub fn push(&mut self, chunk: &[u8]) -> Vec<Frame> {
        self.buffer.extend_from_slice(chunk);
        let mut frames = Vec::new();
        while let Some((frame_end, consume)) = find_separator(&self.buffer) {
            let block = self.buffer[..frame_end].to_vec();
            self.buffer.drain(..consume);
            if let Some(frame) = parse_block(&block) {
                frames.push(frame);
            }
        }
        frames
    }

    /// Flushes a trailing block that arrived without its blank line (some
    /// proxies close the stream that way). Empty or field-less blocks yield
    /// nothing.
    pub fn finish(&mut self) -> Option<Frame> {
        if self.buffer.is_empty() {
            return None;
        }
        let block = std::mem::take(&mut self.buffer);
        parse_block(&block)
    }
}

/// Finds the first blank line: `\n\n` or `\r\n\r\n`. Returns the end of the
/// frame's content and how many bytes to drain.
fn find_separator(buffer: &[u8]) -> Option<(usize, usize)> {
    for index in 0..buffer.len() {
        if buffer[index] != b'\n' {
            continue;
        }
        match buffer.get(index + 1) {
            Some(b'\n') => return Some((index, index + 2)),
            Some(b'\r') if buffer.get(index + 2) == Some(&b'\n') => {
                return Some((index, index + 3));
            }
            _ => {}
        }
    }
    None
}

fn parse_block(block: &[u8]) -> Option<Frame> {
    let mut frame = Frame::default();
    let mut data: Vec<String> = Vec::new();
    for line in block.split(|byte| *byte == b'\n') {
        let line = line.strip_suffix(b"\r").unwrap_or(line);
        if line.is_empty() || line.starts_with(b":") {
            continue;
        }
        let (field, value) = match line.iter().position(|byte| *byte == b':') {
            Some(index) => {
                let mut value = &line[index + 1..];
                if value.first() == Some(&b' ') {
                    value = &value[1..];
                }
                (&line[..index], value)
            }
            None => (line, &b""[..]),
        };
        let Ok(value) = std::str::from_utf8(value) else {
            continue;
        };
        match field {
            b"data" => data.push(value.to_string()),
            b"event" => frame.event = value.to_string(),
            b"id" => frame.id = value.to_string(),
            _ => {}
        }
    }
    if data.is_empty() {
        return None;
    }
    frame.data = data.join("\n");
    Some(frame)
}

/// One incremental search event from the instance.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    Start {
        total_sources: usize,
    },
    SourceResults {
        source: String,
        source_name: String,
        results: Vec<Candidate>,
    },
    SourceError {
        source: String,
        source_name: String,
        error: String,
    },
    Complete {
        total_results: usize,
        completed_sources: usize,
    },
    Failed {
        error: ApiError,
    },
}

/// Parses the `data` payload of a frame. Unknown or malformed payloads are
/// skipped rather than failing the whole stream.
pub fn parse_event(base: &str, payload: &str) -> Option<Event> {
    let value: Value = serde_json::from_str(payload).ok()?;
    match value["type"].as_str()? {
        "start" => Some(Event::Start {
            total_sources: value["totalSources"].as_u64().unwrap_or(0) as usize,
        }),
        "source_result" => Some(Event::SourceResults {
            source: string(&value, "source"),
            source_name: string(&value, "sourceName"),
            results: value["results"]
                .as_array()
                .map(|items| {
                    items
                        .iter()
                        .filter_map(|item| crate::api::candidate_from(base, item, None))
                        .collect()
                })
                .unwrap_or_default(),
        }),
        "source_error" => Some(Event::SourceError {
            source: string(&value, "source"),
            source_name: string(&value, "sourceName"),
            error: {
                let error = string(&value, "error");
                if error.is_empty() {
                    "片源搜索失败".to_string()
                } else {
                    error
                }
            },
        }),
        "complete" => Some(Event::Complete {
            total_results: value["totalResults"].as_u64().unwrap_or(0) as usize,
            completed_sources: value["completedSources"].as_u64().unwrap_or(0) as usize,
        }),
        _ => None,
    }
}

fn string(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string()
}

/// A live stream plus its cancellation flag. Dropping it asks the worker to
/// stop at its next read.
pub struct Stream {
    receiver: tokio::sync::mpsc::UnboundedReceiver<Event>,
    cancel: Arc<AtomicBool>,
}

impl Stream {
    pub async fn next(&mut self) -> Option<Event> {
        self.receiver.recv().await
    }

    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
}

impl Drop for Stream {
    fn drop(&mut self) {
        self.cancel();
    }
}

static CURRENT: Mutex<Option<Arc<AtomicBool>>> = Mutex::new(None);

/// Cancels the newest stream. Events already queued are still delivered to
/// its owner, which drops them by generation; this only frees the worker.
pub fn cancel_current() {
    if let Some(flag) = CURRENT.lock().unwrap_or_else(|e| e.into_inner()).as_ref() {
        flag.store(true, Ordering::Relaxed);
    }
}

/// Opens the stream for `query`. Auth is renewed through the same
/// single-flight path as ordinary API calls; a 401 is retried once with the
/// refreshed token, like `api_transport::send`.
pub fn open(base: &str, token: &str, query: &str) -> Result<Stream, ApiError> {
    let base = base.to_string();
    let seed = token.to_string();
    let query = query.to_string();
    if api_transport::current_token().is_none() {
        return Err(ApiError::Unauthorized);
    }
    let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
    let cancel = Arc::new(AtomicBool::new(false));
    *CURRENT.lock().unwrap_or_else(|e| e.into_inner()) = Some(cancel.clone());
    let worker_cancel = cancel.clone();
    std::thread::Builder::new()
        .name("moontv-search-stream".to_string())
        .spawn(move || worker(base, seed, query, sender, worker_cancel))
        .map_err(|_| ApiError::Network("无法启动搜索线程".to_string()))?;
    Ok(Stream { receiver, cancel })
}

fn worker(
    base: String,
    seed: String,
    query: String,
    sender: tokio::sync::mpsc::UnboundedSender<Event>,
    cancel: Arc<AtomicBool>,
) {
    let url = api_url(&base, &format!("api/search/ws?q={}", encode_query(&query)));
    let mut token = match api_transport::media_token(&base, &seed, None) {
        Ok(token) => token,
        Err(error) => {
            let _ = sender.send(Event::Failed { error });
            return;
        }
    };
    for attempt in 0..2 {
        let agent = api_transport::proxy_agent_builder(&url)
            .redirects(0)
            .timeout_connect(Duration::from_secs(10))
            .timeout_read(Duration::from_secs(15))
            .build();
        let request = agent
            .get(&url)
            .set("User-Agent", USER_AGENT)
            .set("Accept", "text/event-stream")
            .set("Authorization", &format!("Bearer {token}"));
        let response = match request.call() {
            Ok(response) => response,
            Err(ureq::Error::Status(status, response)) => {
                if status == 401 && attempt == 0 && api_transport::sync_supported() {
                    match api_transport::media_token(&base, &seed, Some(&token)) {
                        Ok(next) => {
                            token = next;
                            continue;
                        }
                        Err(error) => {
                            let _ = sender.send(Event::Failed { error });
                            return;
                        }
                    }
                }
                let error = api_transport::status_error(
                    status,
                    response
                        .header("Retry-After")
                        .and_then(|value| value.parse().ok()),
                );
                let _ = sender.send(Event::Failed { error });
                return;
            }
            Err(ureq::Error::Transport(error)) => {
                let _ = sender.send(Event::Failed {
                    error: ApiError::Network(error.to_string()),
                });
                return;
            }
        };
        read_stream(response.into_reader(), &base, &sender, &cancel);
        break;
    }
}

fn read_stream(
    mut reader: Box<dyn Read + Send>,
    base: &str,
    sender: &tokio::sync::mpsc::UnboundedSender<Event>,
    cancel: &AtomicBool,
) {
    let mut decoder = Decoder::new();
    let mut buffer = [0_u8; 8192];
    loop {
        if cancel.load(Ordering::Relaxed) {
            return;
        }
        match reader.read(&mut buffer) {
            Ok(0) => {
                if let Some(frame) = decoder.finish()
                    && let Some(event) = parse_event(base, &frame.data)
                {
                    let _ = sender.send(event);
                }
                return;
            }
            Ok(read) => {
                for frame in decoder.push(&buffer[..read]) {
                    let Some(event) = parse_event(base, &frame.data) else {
                        continue;
                    };
                    let complete = matches!(event, Event::Complete { .. });
                    if sender.send(event).is_err() {
                        return;
                    }
                    if complete {
                        return;
                    }
                }
            }
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::Interrupted | std::io::ErrorKind::WouldBlock
                ) =>
            {
                continue;
            }
            Err(error) => {
                let _ = sender.send(Event::Failed {
                    error: ApiError::Network(error.to_string()),
                });
                return;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frames_survive_utf8_split_across_chunks() {
        let text = "data: {\"type\":\"start\",\"totalSources\":2,\"name\":\"回声档案\"}\n\n";
        let bytes = text.as_bytes();
        let mut decoder = Decoder::new();
        let mut frames = Vec::new();
        // Split in the middle of every multibyte character.
        for chunk in bytes.chunks(7) {
            frames.extend(decoder.push(chunk));
        }
        assert_eq!(frames.len(), 1, "one frame from many chunks");
        assert!(frames[0].data.contains("回声档案"));
        assert_eq!(
            parse_event("https://tv.example/", &frames[0].data),
            Some(Event::Start { total_sources: 2 })
        );
    }

    #[test]
    fn multiline_data_is_joined_with_newlines() {
        let mut decoder = Decoder::new();
        let frames = decoder
            .push(b"event: message\ndata: {\"type\":\"complete\",\ndata: \"totalResults\":1}\n\n");
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0].event, "message");
        assert_eq!(
            frames[0].data,
            "{\"type\":\"complete\",\n\"totalResults\":1}"
        );
        assert_eq!(
            parse_event("x", &frames[0].data),
            Some(Event::Complete {
                total_results: 1,
                completed_sources: 0
            })
        );
    }

    #[test]
    fn crlf_and_comments_are_accepted() {
        let mut decoder = Decoder::new();
        let frames = decoder.push(b": keep-alive\r\nid: 7\r\ndata:{} \r\n\r\n");
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0].id, "7");
        assert_eq!(frames[0].data, "{} ");
    }

    #[test]
    fn finish_flushes_a_block_without_a_blank_line() {
        let mut decoder = Decoder::new();
        assert!(
            decoder
                .push(b"data: {\"type\":\"start\",\"totalSources\":3}")
                .is_empty()
        );
        let frame = decoder.finish().expect("trailing block");
        assert_eq!(
            parse_event("x", &frame.data),
            Some(Event::Start { total_sources: 3 })
        );
    }

    #[test]
    fn empty_and_comment_only_blocks_yield_nothing() {
        let mut decoder = Decoder::new();
        assert!(decoder.push(b": ping\n\n\n\n").is_empty());
        assert_eq!(decoder.buffered_len(), 0);
    }

    #[test]
    fn unknown_payloads_are_skipped_not_fatal() {
        assert_eq!(parse_event("x", "not json"), None);
        assert_eq!(parse_event("x", "{\"type\":\"future_event\"}"), None);
    }

    #[test]
    fn source_error_has_a_fallback_message() {
        let event = parse_event(
            "x",
            "{\"type\":\"source_error\",\"source\":\"a\",\"sourceName\":\"A\"}",
        );
        assert!(matches!(
            event,
            Some(Event::SourceError { error, .. }) if !error.is_empty()
        ));
    }
}
