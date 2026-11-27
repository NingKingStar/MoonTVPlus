//! The native player: NSH Video, a drawn control bar, the episode/source
//! panel, fullscreen, keyboard ownership and automatic next episode.

use std::time::Duration;

use nsh_core::{
    desktop::WindowMode,
    element::{KeyOptions, PressEvent, desktop::current_window},
    keymap::When,
    prelude::*,
};

use crate::{
    model::{Candidate, format_time},
    state::{self, PanelTab, PlayPhase},
    ui::{
        icons::{Icon, icon},
        theme::*,
        widgets::{Btn, button, chip, demo_chip},
    },
};

#[component]
pub fn PlayerPage() -> impl IntoElement {
    let player = state::player();
    let settings = state::settings();
    let window = current_window();
    let video_ref = element_ref();
    let track_width = signal(0.0_f32);

    let toggle_fullscreen = {
        let window = window.clone();
        move || {
            let full = state::player().get().fullscreen;
            let mut state = player.get();
            state.fullscreen = !full;
            player.set(state);
            window.set_mode(if full {
                WindowMode::Normal
            } else {
                WindowMode::FullScreen
            });
            video_ref.focus();
            crate::logbook::event(
                "player_fullscreen",
                serde_json::json!({"full": !full, "how": "F/button"}),
            );
        }
    };
    let toggle_fullscreen_esc = {
        let window = window.clone();
        move || {
            if state::player().get().fullscreen {
                window.set_mode(WindowMode::Normal);
                let mut state = player.get();
                state.fullscreen = false;
                player.set(state);
                video_ref.focus();
                crate::logbook::event(
                    "player_fullscreen",
                    serde_json::json!({"full": false, "how": "Esc"}),
                );
            }
        }
    };
    let toggle_fullscreen_f = toggle_fullscreen.clone();
    on_key_with(
        "f",
        KeyOptions {
            when: When::NotEditing,
            ..Default::default()
        },
        move |_| toggle_fullscreen_f(),
    );
    on_key("escape", {
        let toggle_fullscreen_esc = toggle_fullscreen_esc.clone();
        move |_| toggle_fullscreen_esc()
    });
    on_key_with(
        "space",
        KeyOptions {
            when: When::NotEditing,
            for_focus: true,
            ..Default::default()
        },
        move |_| {
            state::cancel_auto_next();
            state::toggle_play();
        },
    );
    for (key, delta) in [("left", -5.0_f64), ("right", 5.0_f64)] {
        on_key_with(
            key,
            KeyOptions {
                when: When::NotEditing,
                for_focus: true,
                ..Default::default()
            },
            move |_| {
                let current = state::player().get().position;
                let duration = state::player().get().duration;
                let target = (current + delta).clamp(0.0, duration.max(0.0));
                video_ref.seek(target);
                state::set_position_after_seek(target);
                crate::logbook::event(
                    "player_seek_key",
                    serde_json::json!({"delta": delta, "to": target}),
                );
            },
        );
    }
    // F11 is the runner's reserved native fullscreen key; follow the actual
    // window state so the header disappears for it too.
    effect({
        let window = window.clone();
        move || {
            if let Some(state_) = window.state() {
                let is_full = state_.mode == WindowMode::FullScreen;
                let mut state = player.get();
                if is_full {
                    if !state.fullscreen {
                        state.fullscreen = true;
                        player.set(state);
                    }
                } else if state.fullscreen {
                    let window = window.clone();
                    set_timeout(Duration::from_millis(450), move || {
                        if window.state().map(|state| state.mode) != Some(WindowMode::FullScreen) {
                            let mut state = player.get();
                            if state.fullscreen {
                                state.fullscreen = false;
                                player.set(state);
                            }
                        }
                    });
                }
            }
        }
    });
    cleanup({
        let window = window.clone();
        move || {
            if state::player().get().fullscreen {
                window.set_mode(WindowMode::Normal);
            }
            // Closing or leaving the player must not lose the last seconds
            // of progress that the old per-tick save used to write.
            state::save_progress(false);
        }
    });
    spawn(async move {
        tokio::time::sleep(Duration::from_millis(120)).await;
        video_ref.focus();
    });

    let fullscreen = computed(move || player.get().fullscreen);
    let panel_open = computed(move || player.get().panel_open);
    let source = computed(move || player.get().url());
    let video_playing = computed(move || player.get().playing);
    let video_muted = computed(move || player.get().muted);
    let _auto_tick = set_interval(Duration::from_millis(1000), move || {
        let p = state::player().get();
        let Some(remaining) = p.auto_in else {
            return;
        };
        if p.auto_token == 0 || state::route() != crate::state::Route::Player {
            return;
        }
        let mut next = p;
        if remaining == 0 {
            next.auto_in = None;
            state::player().set(next);
            state::next_episode(true);
        } else {
            next.auto_in = Some(remaining - 1);
            state::player().set(next);
        }
    });
    let trace = computed(move || player.get().trace.clone());
    let title = self_title();
    let current_episode = computed(move || {
        let p = player.get();
        p.current()
            .map(|item| item.title.clone())
            .unwrap_or_default()
    });
    let source_name = computed(move || {
        player
            .get()
            .trace
            .as_ref()
            .map(|trace| trace.source_name.clone())
            .unwrap_or_default()
    });
    let portrait_mode = player.get().portrait;
    let portrait_width = move || {
        let size = nsh_core::element::window();
        let stage_height = (size.height.0 - 56.0 - 64.0).max(160.0);
        (stage_height * 9.0 / 16.0).min((size.width.0 - 48.0).max(160.0))
    };
    let make_video = move || {
        video(source)
            .element_ref(&video_ref)
            .w_full()
            .h_full()
            .bg(STAGE)
            .paused(move || !video_playing.get())
            .muted(move || video_muted.get())
            .volume(move || settings.get().volume)
            .focusable(true)
            .on_focus(move || {
                crate::logbook::event("player_focus", serde_json::json!("video"));
            })
            .on_press(move || video_ref.focus())
            .on_load_start(move || {
                crate::logbook::event(
                    "video_load_start",
                    serde_json::json!({"url": crate::logbook::safe_url(&state::player().get().url())}),
                );
                state::observe_buffering(true);
            })
            .on_load(move |info: &VideoInfo| {
                state::observe_meta(info.duration as f64);
                crate::logbook::event(
                    "player_load",
                    serde_json::json!({
                        "duration": info.duration,
                        "reported_size": format!("{:?}", info.size),
                        "url": crate::logbook::safe_url(&state::player().get().url()),
                    }),
                );
            })
            .on_progress(move |progress: &VideoProgress| {
                state::observe_progress(progress.current_time as f64, progress.duration as f64);
                let token = state::player().get().load_token;
                maybe_resume(video_ref, token, progress);
            })
            .on_buffer(move |buffering: &bool| {
                state::observe_buffering(*buffering);
            })
            .on_error(move |message: &str| {
                state::observe_error(message);
            })
            .on_end(move || {
                if state::player().get().phase != PlayPhase::Ended {
                    state::observe_end();
                }
            })
    };
    let stage = if portrait_mode {
        view()
            .flex_1()
            .min_h(px(0.0))
            .items_center()
            .justify_center()
            .bg(rgb(0x05070c))
            .child(
                view()
                    .w(move || px(portrait_width()))
                    .h_full()
                    .flex_col()
                    .child(
                        view()
                            .flex_1()
                            .min_h(px(0.0))
                            .relative()
                            .bg(STAGE)
                            .child(make_video())
                            .child(when(
                                move || source.get().is_empty(),
                                move || {
                                    view()
                                        .absolute()
                                        .inset_0()
                                        .items_center()
                                        .justify_center()
                                        .child(
                                            text("正在准备播放…".to_owned())
                                                .text_size(px(12.0))
                                                .text_color(MUTED),
                                        )
                                },
                            ))
                            .child(overlays(video_ref)),
                    )
                    .child(control_bar(video_ref, track_width)),
            )
            .into_element()
    } else {
        view()
            .flex_1()
            .min_h(px(0.0))
            .relative()
            .bg(STAGE)
            .child(make_video())
            .child(when(
                move || source.get().is_empty(),
                move || {
                    view()
                        .absolute()
                        .inset_0()
                        .items_center()
                        .justify_center()
                        .child(
                            text("正在准备播放…".to_owned())
                                .text_size(px(12.0))
                                .text_color(MUTED),
                        )
                },
            ))
            .child(overlays(video_ref))
            .into_element()
    };

    view()
        .flex_1()
        .h_full()
        .bg(STAGE)
        .relative()
        .flex_col()
        .child(
            view()
                .h(px(56.0))
                .px(px(16.0))
                .flex_row()
                .items_center()
                .gap(px(12.0))
                .bg(rgb(0x0c0f14))
                .border_b_1()
                .border_b_color(LINE)
                .h(move || px(if fullscreen.get() { 0.0 } else { 56.0 }))
                .overflow_hidden()
                .child(button("返回".to_owned(), Btn::Ghost, move || {
                    if state::player().get().fullscreen {
                        window_clone_mode(window.clone(), WindowMode::Normal);
                    }
                    state::leave_player();
                }))
                .child(
                    view()
                        .flex_1()
                        .min_w(px(0.0))
                        .gap(px(1.0))
                        .child(
                            text(move || title.clone())
                                .text_size(px(14.0))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(TEXT)
                                .number_of_lines(1),
                        )
                        .child(
                            text(move || {
                                format!("{} · {}", current_episode.get(), source_name.get())
                            })
                            .text_size(px(10.5))
                            .text_color(MUTED)
                            .number_of_lines(1),
                        ),
                )
                .child(dynamic(move || {
                    let is_demo = trace
                        .get()
                        .as_ref()
                        .is_some_and(|trace| trace.source.starts_with("demo-"));
                    if is_demo {
                        demo_chip()
                    } else {
                        view().into_element()
                    }
                }))
                .children(portrait_mode.then(|| chip("竖屏短剧", ACCENT, DEMO_TINT)))
                .child(dynamic(move || {
                    let notice = player.get().notice;
                    if notice.is_empty() {
                        view().into_element()
                    } else {
                        view()
                            .h(px(24.0))
                            .max_w(px(260.0))
                            .px(px(9.0))
                            .rounded_full()
                            .bg(DEMO_TINT)
                            .border_1()
                            .border_color(alpha(0xe9b872, 0.32))
                            .items_center()
                            .child(
                                text(notice)
                                    .text_size(px(10.5))
                                    .text_color(ACCENT)
                                    .number_of_lines(1),
                            )
                            .into_element()
                    }
                }))
                .child(favorite_button())
                .child(control_icon(Icon::Panel, "选集与换源", move || {
                    let mut state = player.get();
                    state.panel_open = !state.panel_open;
                    player.set(state);
                }))
                .child(control_icon(
                    Icon::Fullscreen,
                    "全屏",
                    toggle_fullscreen.clone(),
                )),
        )
        .child(stage)
        .children((!portrait_mode).then(|| control_bar(video_ref, track_width)))
        .child(dynamic(move || {
            if !fullscreen.get() {
                return view().into_element();
            }
            view()
                .absolute()
                .top(px(14.0))
                .right(px(16.0))
                .px(px(9.0))
                .py(px(5.0))
                .rounded(px(6.0))
                .bg(alpha(0x000000, 0.55))
                .pointer_events(PointerEvents::None)
                .child(
                    text("Esc 退出全屏".to_owned())
                        .text_size(px(10.0))
                        .text_color(alpha(0xffffff, 0.65)),
                )
                .into_element()
        }))
        .child(panel(panel_open, player))
        .into_element()
}

fn self_title() -> String {
    let p = state::player().get();
    p.trace
        .as_ref()
        .map(|trace| trace.title.clone())
        .unwrap_or_default()
}

fn window_clone_mode(window: nsh_core::element::desktop::WindowRef, mode: WindowMode) {
    window.set_mode(mode);
    let mut state = state::player().get();
    state.fullscreen = false;
    state::player().set(state);
}

fn control_icon(name: Icon, label: &'static str, on_press: impl Fn() + 'static) -> AnyElement {
    view()
        .size(px(32.0))
        .rounded(px(7.0))
        .items_center()
        .justify_center()
        .focusable(true)
        .role(AccessibilityRole::Button)
        .label(label)
        .hover(|s| s.bg(alpha(0xffffff, 0.08)))
        .on_press(on_press)
        .child(icon(name, 17.0, || MUTED))
        .into_element()
}

fn favorite_button() -> AnyElement {
    dynamic(move || {
        let Some(candidate) = state::trace_candidate() else {
            return view().into_element();
        };
        let saved = state::library().get().is_favorite(&candidate.key());
        view()
            .h(px(30.0))
            .px(px(9.0))
            .rounded(px(7.0))
            .flex_row()
            .items_center()
            .gap(px(5.0))
            .focusable(true)
            .role(AccessibilityRole::Button)
            .label(if saved {
                "已收藏".to_owned()
            } else {
                "收藏".to_owned()
            })
            .hover(|s| s.bg(alpha(0xffffff, 0.08)))
            .on_press(move || state::toggle_favorite(&candidate))
            .child(icon(Icon::Bookmark, 14.0, move || {
                if saved { ACCENT } else { MUTED }
            }))
            .child(
                text(if saved {
                    "已收藏".to_owned()
                } else {
                    "收藏".to_owned()
                })
                .text_size(px(10.5))
                .text_color(if saved { ACCENT } else { MUTED }),
            )
            .into_element()
    })
}

fn maybe_resume(video: ElementRef, token: u64, progress: &VideoProgress) {
    let player = state::player();
    let p = player.get();
    if p.load_token != token || p.resume_done {
        return;
    }
    if p.resume_at <= 3.0 {
        let mut next = p;
        next.resume_done = true;
        player.set(next);
        return;
    }
    let duration = if progress.duration > 0.0 {
        progress.duration as f64
    } else {
        p.duration
    };
    if duration <= 0.0 {
        return;
    }
    if p.resume_at >= duration - 2.0 {
        // Too close to the end to resume; replay from the start.
        let mut next = p;
        next.resume_done = true;
        player.set(next);
        return;
    }
    let current = progress.current_time as f64;
    // Seek once, after playback is demonstrably running; early seeks during
    // metadata loading are ignored by the native engine. The flag lives in
    // player state because the page can re-render on every progress tick.
    if p.resume_attempts == 0 {
        if current >= 1.5 {
            let target = p.resume_at;
            video.seek(target);
            let mut next = p;
            next.resume_attempts = 1;
            player.set(next);
            crate::logbook::event(
                "player_resume_seek",
                serde_json::json!({"at": target, "time": current}),
            );
        }
        return;
    }
    // Confirm against observed playback time, not against the intent to seek.
    if current >= p.resume_at - 0.75 && current <= p.resume_at + 2.0 {
        let mut next = p;
        next.resume_done = true;
        let resumed_at = next.resume_at;
        player.set(next);
        state::toast(
            format!("已从上次 {} 继续", format_time(resumed_at)),
            state::ToastTone::Success,
        );
        crate::logbook::event(
            "player_resume",
            serde_json::json!({"at": resumed_at, "time": current, "ok": true}),
        );
    }
}

fn overlays(video: ElementRef) -> AnyElement {
    let player = state::player();
    let phase = computed(move || player.get().phase);
    let error = computed(move || player.get().error.clone());
    let auto_in = computed(move || player.get().auto_in);
    let buffering = computed(move || player.get().buffering);
    let panel_open = computed(move || player.get().panel_open);
    view()
        .absolute()
        .inset_0()
        .pointer_events(PointerEvents::None)
        .child(dynamic(move || {
            if phase.get() == PlayPhase::Loading && !panel_open.get() {
                return view()
                    .absolute()
                    .left(px(16.0))
                    .bottom(px(16.0))
                    .px(px(10.0))
                    .py(px(6.0))
                    .rounded(px(6.0))
                    .bg(alpha(0x000000, 0.55))
                    .child(
                        text("正在加载原生媒体…".to_owned())
                            .text_size(px(10.5))
                            .text_color(alpha(0xffffff, 0.7)),
                    )
                    .into_element();
            }
            view().into_element()
        }))
        .child(dynamic(move || {
            if !buffering.get() || phase.get() != PlayPhase::Ready {
                return view().into_element();
            }
            view()
                .absolute()
                .left(px(16.0))
                .bottom(px(16.0))
                .px(px(10.0))
                .py(px(6.0))
                .rounded(px(6.0))
                .bg(alpha(0x000000, 0.5))
                .child(
                    text("缓冲中…".to_owned())
                        .text_size(px(10.0))
                        .text_color(alpha(0xffffff, 0.6)),
                )
                .into_element()
        }))
        .child(dynamic(move || {
            if phase.get() != PlayPhase::Failed {
                return view().into_element();
            }
            view()
                .absolute()
                .inset_0()
                .bg(alpha(0x000000, 0.78))
                .items_center()
                .justify_center()
                .gap(px(12.0))
                .child(
                    view()
                        .p(px(22.0))
                        .rounded(px(RADIUS_CARD))
                        .bg(SURFACE)
                        .border_1()
                        .border_color(alpha(0xf07b7b, 0.35))
                        .max_w(px(520.0))
                        .gap(px(11.0))
                        .items_center()
                        .pointer_events(PointerEvents::Auto)
                        .child(icon(Icon::Alert, 26.0, || DANGER))
                        .child(
                            text("播放没有完成".to_owned())
                                .text_size(px(18.0))
                                .font_weight(FontWeight::BOLD)
                                .text_color(TEXT),
                        )
                        .child(
                            text(move || error.get())
                                .text_size(px(11.5))
                                .text_color(MUTED)
                                .line_height(nsh_core::LineHeight::Multiple(1.55))
                                .text_center(),
                        )
                        .child(
                            view()
                                .flex_row()
                                .gap(px(7.0))
                                .child(button("重试".to_owned(), Btn::Primary, {
                                    let video = video;
                                    move || {
                                        state::retry();
                                        video.focus();
                                    }
                                }))
                                .child(button("换源".to_owned(), Btn::Secondary, move || {
                                    let mut state = state::player().get();
                                    state.panel_open = true;
                                    state.panel_tab = PanelTab::Sources;
                                    state::player().set(state);
                                }))
                                .child(button(
                                    "返回详情".to_owned(),
                                    Btn::Ghost,
                                    state::leave_player,
                                )),
                        ),
                )
                .into_element()
        }))
        .child(dynamic(move || {
            if phase.get() != PlayPhase::Ended {
                return view().into_element();
            }
            if let Some(seconds) = auto_in.get() {
                return view()
                    .absolute()
                    .right(px(18.0))
                    .bottom(px(18.0))
                    .p(px(16.0))
                    .rounded(px(RADIUS_CARD))
                    .bg(alpha(0x0b0d10, 0.94))
                    .border_1()
                    .border_color(alpha(0xe9b872, 0.4))
                    .gap(px(8.0))
                    .pointer_events(PointerEvents::Auto)
                    .child(
                        text("即将播放下一集".to_owned())
                            .text_size(px(13.0))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(TEXT),
                    )
                    .child(
                        text(format!("{seconds} 秒后自动继续"))
                            .text_size(px(11.0))
                            .text_color(MUTED),
                    )
                    .child(
                        view()
                            .flex_row()
                            .gap(px(6.0))
                            .child(button("立即播放".to_owned(), Btn::Primary, || {
                                state::next_episode(false)
                            }))
                            .child(button(
                                "取消".to_owned(),
                                Btn::Ghost,
                                state::cancel_auto_next,
                            )),
                    )
                    .into_element();
            }
            let can_next = state::player().get().can_next();
            view()
                .absolute()
                .inset_0()
                .bg(alpha(0x000000, 0.4))
                .items_center()
                .justify_center()
                .pointer_events(PointerEvents::Auto)
                .child(
                    view()
                        .p(px(20.0))
                        .rounded(px(RADIUS_CARD))
                        .bg(SURFACE)
                        .border_1()
                        .border_color(BORDER)
                        .gap(px(9.0))
                        .items_center()
                        .child(
                            text("本集结束".to_owned())
                                .text_size(px(17.0))
                                .font_weight(FontWeight::BOLD)
                                .text_color(TEXT),
                        )
                        .child(
                            view()
                                .flex_row()
                                .gap(px(6.0))
                                .child(if can_next {
                                    button("下一集".to_owned(), Btn::Primary, || {
                                        state::next_episode(false)
                                    })
                                } else {
                                    button("已是最后一集".to_owned(), Btn::Secondary, || {})
                                })
                                .child(button(
                                    "重播".to_owned(),
                                    Btn::Secondary,
                                    state::toggle_play,
                                ))
                                .child(button("选集".to_owned(), Btn::Ghost, move || {
                                    let mut state = state::player().get();
                                    state.panel_open = true;
                                    state.panel_tab = PanelTab::Episodes;
                                    state::player().set(state);
                                })),
                        ),
                )
                .into_element()
        }))
        .into_element()
}

fn control_bar(video: ElementRef, track_width: Signal<f32>) -> AnyElement {
    let player = state::player();
    let playing = computed(move || player.get().playing);
    let position = computed(move || player.get().position);
    let duration = computed(move || player.get().duration);
    let progress = computed(move || player.get().progress());
    let muted = computed(move || player.get().muted);
    view()
        .h(px(64.0))
        .px(px(16.0))
        .bg(rgb(0x0b0d10))
        .border_t_1()
        .border_t_color(LINE)
        .flex_col()
        .justify_center()
        .gap(px(4.0))
        .child(
            view()
                .h(px(16.0))
                .w_full()
                .items_center()
                .on_layout(move |bounds: &Bounds<Points>| track_width.set(bounds.size.width.0))
                .on_press(move |event: &PressEvent| {
                    let state = player.get();
                    if state.duration <= 0.0 {
                        return;
                    }
                    let ratio = (event.position.x.0 / track_width.get().max(1.0)).clamp(0.0, 1.0);
                    let target = state.duration * ratio as f64;
                    video.seek(target);
                    state::set_position_after_seek(target);
                    crate::logbook::event(
                        "player_seek",
                        serde_json::json!({"to": target, "ratio": ratio}),
                    );
                })
                .child(
                    view()
                        .w_full()
                        .h(px(4.0))
                        .rounded_full()
                        .bg(alpha(0xffffff, 0.16))
                        .overflow_hidden()
                        .child(
                            view()
                                .h_full()
                                .w(move || percentage(progress.get()))
                                .rounded_full()
                                .bg(ACCENT),
                        ),
                ),
        )
        .child(
            view()
                .flex_row()
                .items_center()
                .gap(px(8.0))
                .child(
                    view()
                        .size(px(38.0))
                        .rounded_full()
                        .bg(ACCENT)
                        .items_center()
                        .justify_center()
                        .focusable(true)
                        .role(AccessibilityRole::Button)
                        .label("播放 / 暂停".to_owned())
                        .on_press(move || {
                            state::cancel_auto_next();
                            state::toggle_play();
                        })
                        .child(dynamic(move || {
                            icon(
                                if playing.get() {
                                    Icon::Pause
                                } else {
                                    Icon::Play
                                },
                                17.0,
                                || ON_ACCENT,
                            )
                        })),
                )
                .child(control_icon(Icon::SkipBack, "上一集", move || {
                    state::cancel_auto_next();
                    state::prev_episode();
                }))
                .child(control_icon(Icon::SkipForward, "下一集", move || {
                    state::cancel_auto_next();
                    state::next_episode(false);
                }))
                .child(button("-5s", Btn::Ghost, move || {
                    let state = player.get();
                    let target = (state.position - 5.0).max(0.0);
                    video.seek(target);
                    state::set_position_after_seek(target);
                }))
                .child(button("+5s", Btn::Ghost, move || {
                    let state = player.get();
                    let target = (state.position + 5.0).min(state.duration.max(0.0));
                    video.seek(target);
                    state::set_position_after_seek(target);
                }))
                .child(
                    text(move || {
                        format!(
                            "{} / {}",
                            format_time(position.get()),
                            format_time(duration.get())
                        )
                    })
                    .text_size(px(11.0))
                    .text_color(MUTED)
                    .tabular_nums(),
                )
                .child(view().flex_1())
                .child(dynamic(move || {
                    control_icon(
                        if muted.get() {
                            Icon::Mute
                        } else {
                            Icon::Volume
                        },
                        "静音",
                        move || {
                            let mut state = player.get();
                            state.muted = !state.muted;
                            player.set(state);
                        },
                    )
                }))
                .child(control_icon(Icon::Panel, "选集与换源", move || {
                    let mut state = player.get();
                    state.panel_open = !state.panel_open;
                    player.set(state);
                }))
                .child(control_icon(Icon::Fullscreen, "全屏", {
                    let window = current_window();
                    move || {
                        let full = state::player().get().fullscreen;
                        window.set_mode(if full {
                            WindowMode::Normal
                        } else {
                            WindowMode::FullScreen
                        });
                        let mut state = state::player().get();
                        state.fullscreen = !full;
                        state::player().set(state);
                        crate::logbook::event(
                            "player_fullscreen",
                            serde_json::json!({"full": !full, "how": "button"}),
                        );
                    }
                })),
        )
        .into_element()
}

fn panel(open: Memo<bool>, player: Signal<state::PlayerState>) -> AnyElement {
    let queue = computed(move || {
        let state = player.get();
        (state.load_token, state.index, state.queue.clone())
    });
    let work = computed(move || player.get().work.clone());
    let tab = computed(move || player.get().panel_tab);
    dynamic(move || {
        if !open.get() {
            return view().into_element();
        }
        let (_, current, items) = queue.get();
        let work = work.get();
        let tab = tab.get();
        view()
            .absolute()
            .top(px(0.0))
            .right(px(0.0))
            .bottom(px(64.0))
            .w(move || px((window().width.0 * 0.34).clamp(300.0, 380.0)))
            .bg(SURFACE)
            .border_l_1()
            .border_l_color(BORDER)
            .flex_col()
            .z_index(20)
            .child(
                view()
                    .h(px(50.0))
                    .px(px(14.0))
                    .flex_row()
                    .items_center()
                    .gap(px(5.0))
                    .border_b_1()
                    .border_b_color(LINE)
                    .child(tab_button("剧集", tab == PanelTab::Episodes, move || {
                        let mut state = player.get();
                        state.panel_tab = PanelTab::Episodes;
                        player.set(state);
                    }))
                    .child(tab_button("片源", tab == PanelTab::Sources, move || {
                        let mut state = player.get();
                        state.panel_tab = PanelTab::Sources;
                        player.set(state);
                    }))
                    .child(view().flex_1())
                    .child(
                        view()
                            .size(px(28.0))
                            .rounded(px(6.0))
                            .items_center()
                            .justify_center()
                            .focusable(true)
                            .role(AccessibilityRole::Button)
                            .label("关闭面板".to_owned())
                            .hover(|s| s.bg(HOVER))
                            .on_press(move || {
                                let mut state = player.get();
                                state.panel_open = false;
                                player.set(state);
                            })
                            .child(icon(Icon::Close, 14.0, || MUTED)),
                    ),
            )
            .child(
                scroll()
                    .flex_1()
                    .child(view().p(px(12.0)).gap(px(5.0)).children(
                        if tab == PanelTab::Episodes {
                            items
                                .iter()
                                .enumerate()
                                .map(|(i, item)| episode_row(item, i == current, i))
                                .collect::<Vec<_>>()
                        } else {
                            work.map(|work| {
                                work.candidates
                                    .iter()
                                    .map(|candidate| source_row(candidate, &player))
                                    .collect::<Vec<_>>()
                            })
                            .unwrap_or_default()
                        },
                    )),
            )
            .into_element()
    })
}

fn tab_button(label: &'static str, active: bool, on_press: impl Fn() + 'static) -> AnyElement {
    view()
        .h(px(30.0))
        .px(px(10.0))
        .rounded(px(6.0))
        .items_center()
        .justify_center()
        .bg(if active { RAISED } else { alpha(0x000000, 0.0) })
        .focusable(true)
        .role(AccessibilityRole::Tab)
        .label(label)
        .accessibility_state(AccessibilityState {
            selected: active,
            ..AccessibilityState::default()
        })
        .hover(|s| s.bg(HOVER))
        .on_press(on_press)
        .child(
            text(label)
                .text_size(px(11.5))
                .font_weight(if active {
                    FontWeight::SEMIBOLD
                } else {
                    FontWeight::MEDIUM
                })
                .text_color(if active { TEXT } else { MUTED }),
        )
        .into_element()
}

fn episode_row(item: &state::QueueItem, active: bool, index: usize) -> AnyElement {
    view()
        .h(px(40.0))
        .px(px(9.0))
        .rounded(px(7.0))
        .bg(if active { RAISED } else { alpha(0x000000, 0.0) })
        .flex_row()
        .items_center()
        .gap(px(8.0))
        .focusable(true)
        .role(AccessibilityRole::Button)
        .label(item.title.clone())
        .accessibility_state(AccessibilityState {
            selected: active,
            ..AccessibilityState::default()
        })
        .hover(|s| s.bg(HOVER))
        .on_press(move || {
            state::cancel_auto_next();
            state::play_index(index, true);
        })
        .child(
            text(format!("{:02}", item.index + 1))
                .text_size(px(10.0))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(if active { ACCENT } else { FAINT }),
        )
        .child(
            text(item.title.clone())
                .flex_1()
                .text_size(px(11.0))
                .text_color(if active { TEXT } else { MUTED })
                .number_of_lines(1),
        )
        .children(active.then(|| view().size(px(4.0)).rounded_full().bg(ACCENT)))
        .into_element()
}

fn source_row(candidate: &Candidate, player: &Signal<state::PlayerState>) -> AnyElement {
    let active = player
        .get_untracked()
        .trace
        .as_ref()
        .is_some_and(|trace| trace.source == candidate.source);
    let source = candidate.source.clone();
    view()
        .p(px(9.0))
        .rounded(px(8.0))
        .bg(if active { RAISED } else { alpha(0x000000, 0.0) })
        .border_1()
        .border_color(if active {
            alpha(0xe9b872, 0.45)
        } else {
            alpha(0x000000, 0.0)
        })
        .gap(px(3.0))
        .focusable(true)
        .role(AccessibilityRole::Button)
        .label(candidate.source_name.clone())
        .hover(|s| s.bg(HOVER))
        .on_press(move || {
            state::cancel_auto_next();
            state::switch_player_source(source.clone());
        })
        .child(
            view()
                .flex_row()
                .items_center()
                .gap(px(6.0))
                .child(
                    text(candidate.source_name.clone())
                        .text_size(px(11.5))
                        .font_weight(if active {
                            FontWeight::SEMIBOLD
                        } else {
                            FontWeight::MEDIUM
                        })
                        .text_color(if active { TEXT } else { MUTED })
                        .number_of_lines(1),
                )
                .children(active.then(|| chip("当前".to_owned(), ACCENT, DEMO_TINT))),
        )
        .child(
            text(format!(
                "{} 集{}",
                candidate.episode_count(),
                if candidate.remarks.is_empty() {
                    String::new()
                } else {
                    format!(" · {}", candidate.remarks)
                }
            ))
            .text_size(px(10.0))
            .text_color(FAINT),
        )
        .into_element()
}
