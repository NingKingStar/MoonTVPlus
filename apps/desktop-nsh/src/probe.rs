use nsh_core::{desktop::WindowMode, element::input, keymap::When, prelude::*};
use std::{fs::OpenOptions, io::Write};
fn record(kind: &str, value: serde_json::Value) {
    let row = serde_json::json!({"ms":nsh_core::time::unix_ms(),"kind":kind,"value":value});
    log::info!("{row}");
    let path = std::env::var("PROBE_LOG").unwrap_or_else(|_| ".runtime/native.jsonl".into());
    if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(f, "{row}");
    }
}
fn button(label: &'static str, action: impl Fn() + 'static) -> impl IntoElement {
    view()
        .px_3()
        .py_2()
        .rounded(px(5.))
        .bg(rgb(0x304256))
        .focusable(true)
        .on_press(action)
        .child(text(label).text_size(px(15.)))
}
#[component]
pub fn App() -> impl IntoElement {
    let player = element_ref();
    let main_area = element_ref();
    let editor = element_ref();
    let src = signal(
        std::env::var("PROBE_URL")
            .unwrap_or_else(|_| "http://127.0.0.1:18765/public/clip.mp4".into()),
    );
    let draft = signal(src.get());
    let playing = signal(true);
    let full = signal(false);
    let position = signal(0_f32);
    let duration = signal(0_f32);
    let status = signal("loading".to_owned());
    let window = nsh_core::element::desktop::current_window();
    let toggle = {
        let window = window.clone();
        move || {
            full.update(|v| *v = !*v);
            window.set_mode(if full.get() {
                WindowMode::FullScreen
            } else {
                WindowMode::Normal
            });
            main_area.focus();
            record("fullscreen_request", serde_json::json!(full.get()));
        }
    };
    on_key_with(
        "f",
        KeyOptions {
            when: When::NotEditing,
            ..Default::default()
        },
        {
            let toggle = toggle.clone();
            move |_| toggle()
        },
    );
    // F11 is reserved and implemented by the native Windows runner.
    on_key("escape", {
        let window = window.clone();
        move |_| {
            if full.get() {
                full.set(false);
                window.set_mode(WindowMode::Normal);
                main_area.focus();
                record("escape", serde_json::json!(true));
            }
        }
    });
    on_key_with(
        "space",
        KeyOptions {
            when: When::NotEditing,
            for_focus: true,
            ..Default::default()
        },
        move |_| {
            playing.update(|v| *v = !*v);
            record("pause_key", serde_json::json!(!playing.get()));
        },
    );
    for (key, delta) in [("left", -5_f32), ("right", 5_f32)] {
        on_key_with(
            key,
            KeyOptions {
                when: When::NotEditing,
                ..Default::default()
            },
            move |_| {
                let t = (position.get() + delta).max(0.).min(duration.get().max(0.));
                player.seek(t as f64);
                record("seek_request", serde_json::json!(t));
            },
        );
    }
    on_key("ctrl+k", move |_| editor.focus());
    effect(move || {
        if let Some(s) = window.state() {
            full.set(s.mode == WindowMode::FullScreen);
            record("window", serde_json::json!(format!("{s:?}")));
        }
    });
    record(
        "start",
        serde_json::json!({"source":src.get(),"nsh":"0.6.1/2f51d8611677c8a6bbad937b39756e8d0cc45350","engine":"native MF; no JS/WebView"}),
    );
    // Application-command exercise only; external keyboard/focus evidence is recorded separately.
    if std::env::var("PROBE_SEQUENCE").as_deref() == Ok("1") {
        spawn(async move {
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            playing.set(false);
            record("sequence_pause", serde_json::json!(position.get()));
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            record(
                "sequence_paused_position",
                serde_json::json!(position.get()),
            );
            player.seek(12.);
            record("sequence_seek", serde_json::json!(12));
            playing.set(true);
            tokio::time::sleep(std::time::Duration::from_secs(4)).await;
            src.set("http://127.0.0.1:18765/public/second.mp4".into());
            record("sequence_switch", serde_json::json!(position.get()));
            tokio::time::sleep(std::time::Duration::from_secs(4)).await;
            src.set(String::new());
            playing.set(false);
            record("sequence_release", serde_json::json!(true));
        });
    }
    view().element_ref(&main_area).focusable(true).flex_1().bg(rgb(0x101820)).text_color(rgb(0xeeeeee))
        .on_focus(move || record("focus",serde_json::json!("main")))
        .on_press(move || main_area.focus())
        .child(when(move || !full.get(),move || view().p_3().gap_2()
            .child(text("NSH native Video | playback gate").text_size(px(24.)))
            .child(text("F / F11 / Fullscreen | Esc | Space | Left/Right 5s | Ctrl+K URL editor").text_size(px(15.)))
            .child(input().element_ref(&editor).value(draft).on_focus(move || record("focus",serde_json::json!("editor"))).on_change(move |s:&str| {draft.set(s.to_owned()); record("edit",serde_json::json!(s));})
                .h(px(38.)).bg(rgb(0x223344)).text_color(rgb(0xffffff)))
            .child(view().flex_row().gap_2()
                .child(button("Load URL",move || {src.set(draft.get());playing.set(true);main_area.focus();record("load_request",serde_json::json!(src.get()));}))
                .child(button("MP4",move || {src.set("http://127.0.0.1:18765/public/clip.mp4".into());playing.set(true);main_area.focus();}))
                .child(button("HLS TS",move || {src.set("http://127.0.0.1:18765/public/ts/index.m3u8".into());playing.set(true);main_area.focus();}))
                .child(button("HLS fMP4",move || {src.set("http://127.0.0.1:18765/public/fmp4/index.m3u8".into());playing.set(true);main_area.focus();}))
                .child(button("Release",move || {src.set(String::new());playing.set(false);record("release_request",serde_json::json!(true));})))))
        .child(when(move || !src.get().is_empty(), move || video(src).element_ref(&player).flex_1().w_full().bg(rgb(0)).focusable(true)
            .on_focus(move || record("focus",serde_json::json!("video")))
            .paused(move || !playing.get()).muted(false).volume(0.2).controls(false)
            .on_press(move || player.focus())
            .on_load_start(move || {position.set(0.);duration.set(0.);status.set("loading".into());record("load_start",serde_json::json!(src.get()));})
            .on_load(move |e:&VideoInfo| {duration.set(e.duration);status.set("loaded".into());record("load",serde_json::json!({"duration":e.duration,"size":format!("{:?}",e.size)}));})
            .on_progress(move |e:&VideoProgress| {position.set(e.current_time);duration.set(e.duration);record("progress",serde_json::json!({"time":e.current_time,"duration":e.duration,"paused_requested":!playing.get()}));})
            .on_error(move |e:&str| {status.set(e.to_owned());record("error",serde_json::json!(e));})
            .on_buffer(move |b:&bool| record("buffer",serde_json::json!(b)))
            .on_end(move || {playing.set(false);status.set("ended".into());record("ended",serde_json::json!(src.get()));})))
        .child(view().p_2().gap_2().flex_row()
            .child(button("Play / Pause",move || {playing.update(|v| *v = !*v);main_area.focus();}))
            .child(button("-5s",move || player.seek((position.get()-5.).max(0.) as f64)))
            .child(button("+5s",move || player.seek((position.get()+5.).min(duration.get()) as f64)))
            .child(button("Fullscreen",toggle))
            .child(text(move || format!("{:.2}/{:.2}s | {} | {}",position.get(),duration.get(),if playing.get() {"play"} else {"pause"},status.get())).text_size(px(15.))))
}
