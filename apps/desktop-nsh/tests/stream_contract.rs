//! Offline contract for the incremental search stream: a loopback stub sends
//! real SSE bytes (with a Chinese title split across writes) and then closes
//! without `complete`. The client must deliver what arrived and report the
//! end, so the caller can fall back to `/api/search` without losing results.
//!
//! No network, no instance, no database: this runs in the normal `cargo test`.

use std::{
    io::{Read, Write},
    net::TcpListener,
    thread,
    time::Duration,
};

use moontv_native::{
    api_transport,
    search_stream::{self, Event},
};

fn candidate_frame() -> String {
    let value = serde_json::json!({
        "type": "source_result",
        "source": "fixture-a",
        "sourceName": "桩",
        "results": [{
            "source": "fixture-a",
            "id": "1",
            "title": "回声档案",
            "year": "2026",
            "source_name": "桩",
            "episodes": ["a.m3u8"],
            "episodes_titles": ["第 1 集"]
        }]
    });
    format!("data: {value}\n\n")
}

#[test]
fn partial_stream_is_delivered_before_the_connection_ends() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind stub");
    let port = listener.local_addr().expect("stub address").port();
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept");
        let mut head = [0_u8; 4096];
        let _ = stream.read(&mut head);
        stream
            .write_all(
                b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nCache-Control: no-cache\r\nConnection: close\r\n\r\n",
            )
            .expect("head");
        stream
            .write_all(br#"data: {"type":"start","totalSources":2}"#)
            .expect("start");
        stream.write_all(b"\n\n").expect("blank");
        stream.flush().expect("flush");
        thread::sleep(Duration::from_millis(80));

        // Split the JSON frame in two: the cut may fall inside a Chinese
        // character, which is exactly what the decoder must survive.
        let frame = candidate_frame();
        let bytes = frame.as_bytes();
        let cut = bytes.len() / 2;
        stream.write_all(&bytes[..cut]).expect("first half");
        stream.flush().expect("flush");
        thread::sleep(Duration::from_millis(80));
        stream.write_all(&bytes[cut..]).expect("second half");
        stream.flush().expect("flush");
        thread::sleep(Duration::from_millis(80));
        // Dropping the socket closes the body without a `complete` event.
    });

    let base = format!("http://127.0.0.1:{port}/");
    api_transport::install(&base, "stub-token", "d1");
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime");
    runtime.block_on(async {
        let mut stream = search_stream::open(&base, "stub-token", "回声").expect("open stream");
        let mut events = Vec::new();
        loop {
            let next = tokio::time::timeout(Duration::from_secs(3), stream.next())
                .await
                .expect("stream must not hang");
            match next {
                Some(event) => events.push(event),
                None => break,
            }
        }

        assert!(
            matches!(events.first(), Some(Event::Start { total_sources: 2 })),
            "start must be the first event: {events:?}"
        );
        let results = events
            .iter()
            .find_map(|event| match event {
                Event::SourceResults {
                    source_name,
                    results,
                    ..
                } if source_name == "桩" => Some(results.clone()),
                _ => None,
            })
            .expect("source results");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "回声档案");
        assert_eq!(results[0].year, "2026");
        assert!(
            events
                .iter()
                .all(|event| !matches!(event, Event::Complete { .. })),
            "the stub never sends complete: {events:?}"
        );
        assert!(
            events
                .iter()
                .all(|event| !matches!(event, Event::Failed { .. })),
            "a clean EOF is not a transport failure: {events:?}"
        );
    });
    api_transport::clear();
    server.join().expect("stub thread");
}
