//! The product pages: connect, home, search, detail, favorites, history and
//! settings. The player lives in `player.rs` because it owns the video.

use nsh_core::{element::input, prelude::*};

use crate::{
    demo,
    discover::{self, AnimeTab, Item, MediaKind},
    model::{Candidate, FavoriteEntry, ProgressEntry, Work, format_time, percent_label},
    source_browse,
    state::{
        self, DiscoverState, DuanjuState, LoadPhase, Route, SearchState, SourceBrowseState,
        ToastTone,
    },
    ui::{
        icons::{Icon, icon},
        theme::*,
        widgets::{
            Btn, button, chip, demo_chip, empty_state, field_label, poster, poster_thumb,
            progress_bar, section_header, switch, work_card,
        },
    },
};

const PAGE_PAD: f32 = 26.0;

fn page_scroll(body: impl IntoElement) -> AnyElement {
    let scroll_ref = element_ref();
    set_timeout(std::time::Duration::from_millis(0), move || {
        scroll_ref.scroll_to(nsh_core::geometry::point(px(0.0), px(0.0)), false);
    });
    scroll()
        .element_ref(&scroll_ref)
        .flex_1()
        .child(
            view()
                .p(px(PAGE_PAD))
                .gap(px(24.0))
                .max_w(px(1520.0))
                .child(body),
        )
        .into_element()
}

fn page_head(
    title: impl Into<String> + 'static,
    subtitle: impl Into<String> + 'static,
    actions: Vec<AnyElement>,
) -> AnyElement {
    let title = title.into();
    let subtitle = subtitle.into();
    view()
        .flex_row()
        .items_end()
        .justify_between()
        .gap_4()
        .child(
            view().flex_1().gap(px(5.0)).child(h1(title)).child(
                text(subtitle)
                    .text_size(px(12.0))
                    .text_color(MUTED)
                    .number_of_lines(2),
            ),
        )
        .children(actions)
        .into_element()
}

fn columns(min_width: f32, max: u16) -> Memo<u16> {
    computed(move || {
        let width = (window().width.0 - SIDEBAR_W - PAGE_PAD * 2.0).max(320.0);
        ((width / min_width).floor() as u16).clamp(1, max)
    })
}

fn works_grid(works: Vec<Work>, min_width: f32, max: u16, progress: bool) -> AnyElement {
    let count = columns(min_width, max);
    let library = state::library();
    dynamic(move || {
        let progress_map = if progress { Some(library.get()) } else { None };
        view()
            .grid()
            .grid_cols(count.get())
            .gap(px(16.0))
            .children(works.iter().map(|work| {
                let percent = progress_map.as_ref().and_then(|lib| {
                    work.candidates.iter().find_map(|candidate| {
                        lib.progress_for(&candidate.key())
                            .map(|entry| entry.percent())
                    })
                });
                work_card(work, percent)
            }))
            .into_element()
    })
}

fn work_from_progress(entry: &ProgressEntry) -> Work {
    Work {
        title: entry.title.clone(),
        year: entry.year.clone(),
        poster: entry.poster.clone(),
        desc: String::new(),
        genres: Vec::new(),
        candidates: vec![Candidate {
            source: entry.source.clone(),
            id: entry.id.clone(),
            source_name: entry.source_name.clone(),
            title: entry.title.clone(),
            year: entry.year.clone(),
            poster: entry.poster.clone(),
            ..Candidate::default()
        }],
        weight: 0,
    }
}

fn work_from_favorite(entry: &FavoriteEntry) -> Work {
    entry.work()
}

// ------------------------------- connect --------------------------------

pub fn connect_page() -> AnyElement {
    let session = state::session();
    let expired = session.get().expired;
    page_scroll(
        view()
            .gap(px(26.0))
            .child(
                view()
                    .py(px(18.0))
                    .gap(px(8.0))
                    .child(
                        text("MoonTVPlus")
                            .text_size(px(11.0))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(ACCENT)
                            .letter_spacing(px(1.6)),
                    )
                    .child(
                        text("把片库带回桌面".to_owned())
                            .text_size(px(34.0))
                            .font_weight(FontWeight::BOLD)
                            .text_color(TEXT),
                    )
                    .child(
                        text("连接你的 MoonTVPlus 实例，或先用受控演示片源体验原生播放。界面、播放与记录都在这台电脑上，不依赖网页主界面。".to_owned())
                            .text_size(px(13.0))
                            .text_color(MUTED)
                            .line_height(nsh_core::LineHeight::Multiple(1.65))
                            .max_w(px(720.0)),
                    ),
            )
            .children(expired.then(|| {
                view()
                    .p(px(12.0))
                    .rounded(px(9.0))
                    .bg(DANGER_TINT)
                    .border_1()
                    .border_color(alpha(0xf07b7b, 0.3))
                    .child(
                        text("上一次会话已过期。重新登录后会回到同一实例与账号的本机记录。".to_owned())
                            .text_size(px(12.0))
                            .text_color(DANGER),
                    )
            }))
            .child(dynamic(|| {
                if window().width.0 >= 1000.0 {
                    view()
                        .flex_row()
                        .flex_wrap()
                        .gap(px(20.0))
                        .items_start()
                        .child(
                            view()
                                .flex_1()
                                .min_w(px(360.0))
                                .max_w(px(760.0))
                                .child(connect_card()),
                        )
                        .child(
                            view()
                                .w(px(320.0))
                                .flex_shrink_0()
                                .child(demo_card()),
                        )
                        .into_element()
                } else {
                    view()
                        .gap(px(20.0))
                        .items_start()
                        .child(connect_card())
                        .child(demo_card())
                        .into_element()
                }
            }))
            .child(
                text("真实 Linux / HTTPS 实例联调与多屏 DPI 尚未验证；本版不提交任何服务器变更。".to_owned())
                    .text_size(px(11.0))
                    .text_color(FAINT),
            ),
    )
}

fn connect_card() -> AnyElement {
    view()
        .w_full()
        .min_w(px(390.0))
        .p(px(20.0))
        .rounded(px(RADIUS_CARD))
        .bg(SURFACE)
        .border_1()
        .border_color(BORDER)
        .gap(px(16.0))
        .child(
            view().gap(px(4.0)).child(h2("连接实例")).child(
                text(
                    "已预填官方实例，可改成自建地址。实例会记住，密码只用于登录，不会保存。"
                        .to_owned(),
                )
                .text_size(px(12.0))
                .text_color(MUTED),
            ),
        )
        .child(connect_form())
        .into_element()
}

fn demo_card() -> AnyElement {
    view()
        .w_full()
        .min_w(px(360.0))
        .p(px(20.0))
        .rounded(px(RADIUS_CARD))
        .bg_linear_to_br()
        .gradient_from(rgb(0x1a1a17))
        .gradient_to(rgb(0x11161d))
        .border_1()
        .border_color(alpha(0xe9b872, 0.24))
        .gap(px(14.0))
        .child(
            view()
                .flex_row()
                .items_center()
                .gap(px(9.0))
                .child(
                    view()
                        .size(px(34.0))
                        .rounded(px(10.0))
                        .bg(DEMO_TINT)
                        .items_center()
                        .justify_center()
                        .child(icon(Icon::Film, 18.0, || ACCENT)),
                )
                .child(
                    view()
                        .gap(px(2.0))
                        .child(h2("演示模式"))
                        .child(demo_chip()),
                ),
        )
        .child(
            text("使用本工程新生成的受控 H264/AAC 媒体，走同一条原生播放与交互路径。演示片单、收藏和历史单独保存，不和任何实例账号混在一起。".to_owned())
                .text_size(px(12.5))
                .text_color(MUTED)
                .line_height(nsh_core::LineHeight::Multiple(1.6)),
        )
        .child(
            view()
                .gap(px(7.0))
                .child(bullet("无需账号或网络，片单固定为演示数据"))
                .child(bullet("覆盖 MP4、HLS TS/fMP4、AES-128 与范围请求"))
                .child(bullet("可以随时退出，本机记录保留在独立资料中")),
        )
        .child(button(
            "进入演示模式".to_owned(),
            Btn::Primary,
            state::enter_demo,
        ))
        .into_element()
}

fn bullet(text_value: &'static str) -> AnyElement {
    view()
        .flex_row()
        .items_center()
        .gap(px(8.0))
        .child(view().size(px(4.0)).rounded_full().bg(ACCENT))
        .child(
            text(text_value)
                .text_size(px(12.0))
                .text_color(MUTED)
                .number_of_lines(1),
        )
        .into_element()
}

fn connect_form() -> AnyElement {
    let session = state::session();
    let saved = state::connect_draft().get_untracked();
    let stored = state::settings().get();
    let initial_base = if saved.instance.is_empty() {
        if session.get().base == "demo" {
            stored.instance.clone()
        } else {
            session.get().base.clone()
        }
    } else {
        saved.instance.clone()
    };
    let instance = signal(initial_base);
    let username = signal(if !saved.username.is_empty() {
        saved.username.clone()
    } else if session.get().username == "演示" {
        stored.last_username.clone()
    } else {
        session.get().username.clone()
    });
    let password = signal(saved.password.clone());
    let login = {
        move || {
            if session.get().connecting {
                return;
            }
            let base = match crate::model::normalize_instance(&instance.get()) {
                Ok(base) => base,
                Err(why) => {
                    state::toast(why, ToastTone::Error);
                    return;
                }
            };
            let user = username.get().trim().to_owned();
            if user.is_empty() {
                state::toast("请输入用户名".to_owned(), ToastTone::Error);
                return;
            }
            if password.get().is_empty() {
                state::toast("请输入密码".to_owned(), ToastTone::Error);
                return;
            }
            let pass = password.get();
            let mut request = state::login_request().get();
            request.active = true;
            request.attempt += 1;
            request.base = base;
            request.user = user.clone();
            request.pass = pass;
            state::login_request().set(request);
            let mut current = session.get();
            current.connecting = true;
            current.generation += 1;
            session.set(current);
        }
    };
    let cancel = {
        move || {
            let mut request = state::login_request().get();
            request.active = false;
            request.attempt += 1;
            state::login_request().set(request);
            let mut current = session.get();
            current.connecting = false;
            current.generation += 1;
            session.set(current);
            state::toast("已取消连接".to_owned(), ToastTone::Info);
        }
    };
    let instance_change = move |value: &str| {
        let mut draft = state::connect_draft().get();
        draft.instance = value.to_owned();
        state::connect_draft().set(draft);
    };
    let username_change = move |value: &str| {
        let mut draft = state::connect_draft().get();
        draft.username = value.to_owned();
        state::connect_draft().set(draft);
    };
    let password_change = move |value: &str| {
        let mut draft = state::connect_draft().get();
        draft.password = value.to_owned();
        state::connect_draft().set(draft);
    };
    let connect_now = login;
    view()
        .gap(px(13.0))
        .child(field(
            "实例地址",
            instance,
            "https://tv.example.com 或 http://127.0.0.1:3000",
            false,
            instance_change,
            connect_now,
        ))
        .child(field(
            "用户名",
            username,
            "实例账号",
            false,
            username_change,
            login,
        ))
        .child(field(
            "密码",
            password,
            "仅用于本次会话，不保存",
            true,
            password_change,
            login,
        ))
        .child(
            view()
                .flex_row()
                .items_center()
                .gap(px(9.0))
                .child(when(
                    move || session.get().connecting,
                    || {
                        text("正在连接…".to_owned())
                            .text_size(px(11.5))
                            .text_color(ACCENT)
                    },
                ))
                .child(button("连接并登录".to_owned(), Btn::Primary, {
                    login
                }))
                .child(when(
                    move || session.get().connecting,
                    move || button("取消", Btn::Ghost, cancel),
                )),
        )
        .child(
            text(
                "登录 token 只保存在内存中；重启后需要重新登录，但本机记录会按实例与账号恢复。"
                    .to_owned(),
            )
            .text_size(px(10.5))
            .text_color(FAINT)
            .line_height(nsh_core::LineHeight::Multiple(1.5)),
        )
        .into_element()
}

fn field(
    label: &'static str,
    value: Signal<String>,
    placeholder: &'static str,
    masked: bool,
    on_change: impl Fn(&str) + 'static,
    submit: impl Fn() + 'static,
) -> AnyElement {
    view()
        .gap(px(5.0))
        .child(field_label(label))
        .child(
            input()
                .value(move || value.get())
                .placeholder(placeholder)
                .placeholder_color(FAINT)
                .text_color(TEXT)
                .text_size(px(13.0))
                .h(px(38.0))
                .px(px(11.0))
                .rounded(px(8.0))
                .bg(RAISED)
                .border_1()
                .border_color(BORDER)
                .focus(|s| s.border_color(alpha(0xe9b872, 0.65)))
                .masked(masked)
                .auto_correct(false)
                .on_change(move |next: &str| {
                    value.set(next.to_owned());
                    on_change(next);
                })
                .on_submit(move |_: &str| submit()),
        )
        .into_element()
}

// -------------------------------- home ----------------------------------

pub fn home_page() -> AnyElement {
    let session = state::session();
    let library = state::library();
    let stored = state::settings();
    let demo_mode = session.get().demo();
    let connected = session.get().connected();
    if !demo_mode && !connected {
        return page_scroll(not_connected_home());
    }
    let recent = dynamic(move || {
        let entries = library.get().continue_watching();
        let favorites = library.get().favorites.clone();
        let works = if demo_mode { demo::works() } else { Vec::new() };
        view()
            .gap(px(24.0))
            .child(continue_section(
                entries,
                !library.get().progress.is_empty(),
            ))
            .children((!favorites.is_empty()).then(|| {
                view()
                    .gap(px(12.0))
                    .child(section_header(
                        "本机片单".to_owned(),
                        Some("收藏先写本机；连接实例后尝试同步，状态逐条标注".to_owned()),
                        Some(button("查看全部", Btn::Ghost, || {
                            state::go(Route::Favorites)
                        })),
                    ))
                    .child(works_grid(
                        favorites.iter().take(6).map(work_from_favorite).collect(),
                        220.0,
                        6,
                        false,
                    ))
            }))
            .children(if demo_mode {
                Some(
                    view()
                        .gap(px(12.0))
                        .child(section_header(
                            "演示片单".to_owned(),
                            Some("受控媒体；每一张卡都来自本机测试片源".to_owned()),
                            Some(demo_chip()),
                        ))
                        .child(works_grid(works, 220.0, 6, false)),
                )
            } else {
                Some(
                    view()
                        .p(px(18.0))
                        .rounded(px(RADIUS_CARD))
                        .bg(SURFACE)
                        .border_1()
                        .border_color(BORDER)
                        .flex_row()
                        .items_center()
                        .gap_3()
                        .child(icon(Icon::Search, 20.0, || ACCENT))
                        .child(
                            view()
                                .flex_1()
                                .gap(px(2.0))
                                .child(
                                    text("发现".to_owned())
                                        .text_size(px(14.0))
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .text_color(TEXT),
                                )
                                .child(
                                    text(
                                        "豆瓣热门、最新、高分与 Bangumi 每日放送；也可用搜索聚合所有已启用片源。"
                                            .to_owned(),
                                    )
                                    .text_size(px(11.5))
                                    .text_color(MUTED),
                                ),
                        )
                        .child(button(
                            "去看看".to_owned(),
                            Btn::Secondary,
                            state::open_discover,
                        )),
                )
            })
            .into_element()
    });
    page_scroll(
        view()
            .gap(px(22.0))
            .child(page_head(
                format!("欢迎回来，{}", session.get().account_label()),
                if demo_mode {
                    "演示资料 · 受控片源 · 与账号实例完全隔离".to_owned()
                } else {
                    "本机进度与收藏；账号会话只在本次运行有效".to_owned()
                },
                vec![
                    button("搜索片库".to_owned(), Btn::Primary, state::open_search),
                    button("发现".to_owned(), Btn::Secondary, state::open_discover),
                    button("设置".to_owned(), Btn::Ghost, || {
                        state::go(Route::Settings)
                    }),
                ],
            ))
            .child(recent)
            .child(
                text(move || {
                    let size = window();
                    format!(
                        "窗口 {:.0} × {:.0} pt · 自动下一集：{}",
                        size.width.0,
                        size.height.0,
                        if stored.get().auto_next { "开" } else { "关" }
                    )
                })
                .text_size(px(10.0))
                .text_color(FAINT),
            ),
    )
}

fn not_connected_home() -> AnyElement {
    let session = state::session();
    view()
        .gap(px(22.0))
        .child(
            view()
                .p(px(26.0))
                .rounded(px(RADIUS_CARD))
                .bg_linear_to_br()
                .gradient_from(rgb(0x171b21))
                .gradient_to(rgb(0x0e1218))
                .border_1()
                .border_color(BORDER)
                .gap(px(14.0))
                .child(
                    text("先连接，再观看".to_owned())
                        .text_size(px(26.0))
                        .font_weight(FontWeight::BOLD)
                        .text_color(TEXT),
                )
                .child(
                    text("你的实例还在原来的服务器上。这个原生桌面端只负责连接、浏览和播放；不会修改服务端、CI 或部署。".to_owned())
                        .text_size(px(13.0))
                        .text_color(MUTED)
                        .line_height(nsh_core::LineHeight::Multiple(1.65))
                        .max_w(px(640.0)),
                )
                .child(
                    text(move || {
                        let state = session.get();
                        if state.expired {
                            "上次会话已过期，请重新登录。".to_owned()
                        } else if !state.base.is_empty() {
                            "已记住实例地址；登录后继续使用同一账号的本机记录。".to_owned()
                        } else {
                            "还没有保存实例地址。".to_owned()
                        }
                    })
                    .text_size(px(12.0))
                    .text_color(ACCENT),
                )
                .child(
                    view()
                        .flex_row()
                        .gap(px(9.0))
                        .child(button("连接实例".to_owned(), Btn::Primary, || state::go(Route::Connect)))
                        .child(button("进入演示".to_owned(), Btn::Secondary, state::enter_demo)),
                ),
        )
        .child(
            view()
                .gap(px(10.0))
                .child(section_header(
                    "本机已有记录".to_owned(),
                    Some("记录属于上次登录的账号；登录后即可继续".to_owned()),
                    None,
                ))
                .child(poster_placeholder_row()),
        )
        .into_element()
}

fn poster_placeholder_row() -> AnyElement {
    let library = state::library();
    let entries = library.get();
    let works: Vec<Work> = entries
        .continue_watching()
        .iter()
        .take(5)
        .map(work_from_progress)
        .collect();
    if works.is_empty() {
        return empty_state(
            Icon::History,
            "还没有本机记录".to_owned(),
            "登录后播放任意一集，首页会从这里继续。".to_owned(),
            None,
        );
    }
    dynamic(move || {
        view()
            .grid()
            .grid_cols(5)
            .gap(px(14.0))
            .children(works.iter().map(|work| work_card(work, None)))
            .into_element()
    })
}

fn continue_section(entries: Vec<ProgressEntry>, has_history: bool) -> AnyElement {
    if entries.is_empty() {
        return view()
            .gap(px(12.0))
            .child(section_header(
                "继续观看".to_owned(),
                Some("播放一集后，这里会出现本机进度".to_owned()),
                None,
            ))
            .child(empty_state(
                Icon::Play,
                if has_history {
                    "没有未看完的内容".to_owned()
                } else {
                    "还没有观看记录".to_owned()
                },
                if has_history {
                    "本机历史里可以重播已完成的剧集；继续观看只保留未看完的进度。".to_owned()
                } else {
                    "从搜索选一部作品开始；暂停、切集和退出时都会写入本机进度。".to_owned()
                },
                Some(button(
                    "去搜索".to_owned(),
                    Btn::Primary,
                    state::open_search,
                )),
            ))
            .into_element();
    }
    let hero = entries[0].clone();
    let rest: Vec<ProgressEntry> = entries.iter().skip(1).take(5).cloned().collect();
    view()
        .gap(px(16.0))
        .child(section_header(
            "继续观看".to_owned(),
            Some("从上次停下的位置继续；源地址会重新解析".to_owned()),
            Some(button("全部历史", Btn::Ghost, || {
                state::go(Route::History)
            })),
        ))
        .child(hero_card(&hero))
        .children((!rest.is_empty()).then(|| {
            let count = columns(190.0, 6);
            dynamic(move || {
                view()
                    .grid()
                    .grid_cols(count.get())
                    .gap(px(14.0))
                    .children(
                        rest.iter().map(|entry| {
                            work_card(&work_from_progress(entry), Some(entry.percent()))
                        }),
                    )
                    .into_element()
            })
        }))
        .into_element()
}

fn hero_card(entry: &ProgressEntry) -> AnyElement {
    let work = work_from_progress(entry);
    let resume = entry.clone();
    let open = entry.clone();
    view()
        .p(px(16.0))
        .rounded(px(RADIUS_CARD))
        .bg_linear_to_r()
        .gradient_from(rgb(0x171c24))
        .gradient_to(rgb(0x10141a))
        .border_1()
        .border_color(BORDER)
        .flex_row()
        .gap(px(18.0))
        .items_center()
        .child(
            view()
                .w(px(150.0))
                .flex_shrink_0()
                .child(poster(&work, false)),
        )
        .child(
            view()
                .flex_1()
                .gap(px(9.0))
                .child(
                    text(entry.title.clone())
                        .text_size(px(26.0))
                        .font_weight(FontWeight::BOLD)
                        .text_color(TEXT)
                        .number_of_lines(2),
                )
                .child(
                    text(format!(
                        "{} · {} · {}",
                        if entry.year.is_empty() {
                            "未知年份"
                        } else {
                            &entry.year
                        },
                        entry.source_name,
                        entry.episode_title
                    ))
                    .text_size(px(12.0))
                    .text_color(MUTED)
                    .number_of_lines(1),
                )
                .child(
                    text(format!(
                        "上次看到 {} · 已完成 {}",
                        format_time(entry.position),
                        percent_label(entry.percent())
                    ))
                    .text_size(px(11.5))
                    .text_color(ACCENT),
                )
                .child(progress_bar(entry.percent(), 5.0))
                .child(
                    view()
                        .flex_row()
                        .gap(px(8.0))
                        .pt(px(2.0))
                        .child(button("继续观看".to_owned(), Btn::Primary, move || {
                            state::resume_progress(&resume)
                        }))
                        .child(button("查看详情".to_owned(), Btn::Ghost, move || {
                            state::open_detail(work_from_progress(&open), Some(open.source.clone()))
                        })),
                ),
        )
        .child(
            view()
                .w(px(150.0))
                .flex_shrink_0()
                .items_end()
                .gap(px(2.0))
                .child(
                    text(percent_label(entry.percent()))
                        .text_size(px(38.0))
                        .font_weight(FontWeight::BOLD)
                        .text_color(ACCENT),
                )
                .child(
                    text("已观看".to_owned())
                        .text_size(px(10.0))
                        .text_color(FAINT),
                ),
        )
        .into_element()
}

// ----------------------------- source browsing --------------------------

fn browse_work(item: &source_browse::Item) -> Work {
    Work {
        title: item.title.clone(),
        year: item.year.clone(),
        poster: item.poster.clone(),
        desc: item.desc.clone(),
        genres: item
            .type_name
            .split(['/', ',', '，', '、'])
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_owned)
            .take(2)
            .collect(),
        candidates: vec![Candidate {
            source: item.source.clone(),
            id: item.id.clone(),
            source_name: item.source_name.clone(),
            title: item.title.clone(),
            year: item.year.clone(),
            poster: item.poster.clone(),
            episodes: item.episodes.clone(),
            episode_titles: item.episode_titles.clone(),
            ..Candidate::default()
        }],
        weight: 0,
    }
}

fn browse_card(item: source_browse::Item, duanju: bool) -> AnyElement {
    let work = browse_work(&item);
    let meta = if item.year.is_empty() {
        item.source_name.clone()
    } else {
        format!("{} · {}", item.year, item.source_name)
    };
    let episodes = item.episodes.len();
    let open = item.clone();
    view()
        .gap(px(7.0))
        .rounded(px(RADIUS_CARD))
        .focusable(true)
        .role(AccessibilityRole::Button)
        .label(format!("{}，{}", item.title, meta))
        .hover(|s| s.opacity(0.94))
        .active(|s| s.opacity(0.82))
        .on_press(move || {
            if duanju {
                state::open_duanju_item(&open);
            } else {
                state::open_source_item(&open);
            }
        })
        .child(poster(&work, true))
        .child(
            view()
                .px(px(2.0))
                .gap(px(3.0))
                .child(
                    text(item.title.clone())
                        .text_size(px(13.5))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(TEXT)
                        .number_of_lines(1),
                )
                .child(
                    view()
                        .flex_row()
                        .items_center()
                        .gap(px(6.0))
                        .child(
                            text(meta)
                                .text_size(px(11.0))
                                .text_color(MUTED)
                                .number_of_lines(1),
                        )
                        .children((episodes > 0).then(|| {
                            text(format!("{episodes} 集"))
                                .text_size(px(10.0))
                                .text_color(FAINT)
                        }))
                        .children(duanju.then(|| chip("竖屏", ACCENT, DEMO_TINT))),
                )
                .children((!item.remarks.is_empty()).then(|| {
                    text(item.remarks.clone())
                        .text_size(px(10.0))
                        .text_color(FAINT)
                        .number_of_lines(1)
                })),
        )
        .into_element()
}

fn browse_grid(items: Vec<source_browse::Item>, duanju: bool) -> AnyElement {
    let count = columns(180.0, 8);
    dynamic(move || {
        view()
            .grid()
            .grid_cols(count.get())
            .gap(px(14.0))
            .children(items.iter().cloned().map(|item| browse_card(item, duanju)))
            .into_element()
    })
}

fn site_chips(
    sites: Vec<(String, String)>,
    active: String,
    on_select: impl Fn(String) + Clone + 'static,
) -> AnyElement {
    view()
        .flex_row()
        .flex_wrap()
        .gap(px(6.0))
        .children(sites.into_iter().map(|(key, name)| {
            let selected = key == active;
            let action = on_select.clone();
            let target = key.clone();
            view()
                .h(px(30.0))
                .px(px(11.0))
                .rounded_full()
                .bg(if selected { DEMO_TINT } else { RAISED })
                .border_1()
                .border_color(if selected {
                    alpha(0xe9b872, 0.38)
                } else {
                    BORDER
                })
                .items_center()
                .justify_center()
                .focusable(true)
                .role(AccessibilityRole::Button)
                .label(name.clone())
                .accessibility_state(AccessibilityState {
                    selected,
                    ..AccessibilityState::default()
                })
                .hover(|s| s.bg(HOVER))
                .on_press(move || action(target.clone()))
                .child(
                    text(name)
                        .text_size(px(11.5))
                        .font_weight(if selected {
                            FontWeight::SEMIBOLD
                        } else {
                            FontWeight::MEDIUM
                        })
                        .text_color(if selected { ACCENT } else { MUTED })
                        .number_of_lines(1),
                )
                .into_element()
        }))
        .into_element()
}

fn category_chips(
    categories: Vec<source_browse::Category>,
    parent: String,
    category: String,
    on_parent: impl Fn(String) + Clone + 'static,
    on_category: impl Fn(String) + Clone + 'static,
) -> AnyElement {
    let hierarchical = source_browse::is_hierarchical(&categories);
    let parents = if hierarchical {
        source_browse::parents(&categories)
    } else {
        Vec::new()
    };
    let multi_parent = parents.len() > 1;
    let flat = if !hierarchical {
        categories.clone()
    } else if parents.len() == 1 {
        let mut list = parents.clone();
        list.extend(source_browse::children(&categories, &parents[0].id));
        list
    } else {
        Vec::new()
    };
    let children = if multi_parent {
        source_browse::children(&categories, &parent)
    } else {
        Vec::new()
    };
    let parent_name = categories
        .iter()
        .find(|item| item.id == parent)
        .map(|item| item.name.clone())
        .unwrap_or_default();
    let category_name = categories
        .iter()
        .find(|item| item.id == category)
        .map(|item| item.name.clone())
        .unwrap_or_default();
    let breadcrumb = if parent_name.is_empty() {
        category_name.clone()
    } else if category_name.is_empty() || category_name == parent_name {
        parent_name.clone()
    } else {
        format!("{parent_name} / {category_name}")
    };
    view()
        .gap(px(7.0))
        .children((!breadcrumb.is_empty()).then(|| {
            text(breadcrumb)
                .text_size(px(11.0))
                .text_color(FAINT)
                .number_of_lines(1)
        }))
        .children((!parents.is_empty()).then(|| {
            view()
                .flex_row()
                .flex_wrap()
                .gap(px(5.0))
                .children(parents.into_iter().map(|item| {
                    let active = item.id == parent;
                    let action = on_parent.clone();
                    let target = item.id.clone();
                    category_chip(item.name, active, move || action(target.clone()))
                }))
        }))
        .children((!children.is_empty()).then(|| {
            view()
                .flex_row()
                .flex_wrap()
                .gap(px(5.0))
                .children(children.into_iter().map(|item| {
                    let active = item.id == category;
                    let action = on_category.clone();
                    let target = item.id.clone();
                    category_chip(item.name, active, move || action(target.clone()))
                }))
        }))
        .children((!flat.is_empty()).then(|| {
            view()
                .flex_row()
                .flex_wrap()
                .gap(px(5.0))
                .children(flat.into_iter().map(|item| {
                    let active = item.id == category;
                    let action = on_category.clone();
                    let target = item.id.clone();
                    category_chip(item.name, active, move || action(target.clone()))
                }))
        }))
        .into_element()
}

fn category_chip(label: String, active: bool, on_press: impl Fn() + 'static) -> AnyElement {
    view()
        .h(px(27.0))
        .px(px(9.0))
        .rounded_full()
        .bg(if active { RAISED } else { alpha(0x000000, 0.0) })
        .border_1()
        .border_color(if active {
            alpha(0xe9b872, 0.42)
        } else {
            BORDER
        })
        .items_center()
        .justify_center()
        .focusable(true)
        .role(AccessibilityRole::Button)
        .label(label.clone())
        .accessibility_state(AccessibilityState {
            selected: active,
            ..AccessibilityState::default()
        })
        .hover(|s| s.bg(HOVER))
        .on_press(on_press)
        .child(
            text(label)
                .text_size(px(11.0))
                .font_weight(if active {
                    FontWeight::SEMIBOLD
                } else {
                    FontWeight::MEDIUM
                })
                .text_color(if active { TEXT } else { MUTED })
                .number_of_lines(1),
        )
        .into_element()
}

fn pagination_footer(
    page: usize,
    page_count: usize,
    total: usize,
    loading: bool,
    on_more: impl Fn() + 'static,
    on_retry: impl Fn() + 'static,
) -> AnyElement {
    view()
        .flex_row()
        .flex_wrap()
        .items_center()
        .justify_between()
        .gap_3()
        .child(
            text(format!(
                "第 {} / {} 页 · {} 条{}",
                page.max(1),
                page_count.max(page),
                total,
                if page < page_count {
                    ""
                } else {
                    " · 已到末页"
                }
            ))
            .text_size(px(10.5))
            .text_color(FAINT),
        )
        .child(
            view()
                .flex_row()
                .items_center()
                .gap(px(8.0))
                .children(loading.then(|| {
                    text("正在加载下一页…")
                        .text_size(px(11.0))
                        .text_color(MUTED)
                }))
                .children(
                    (page < page_count && !loading)
                        .then(|| button("加载更多", Btn::Secondary, on_more)),
                )
                .child(button("刷新列表", Btn::Ghost, on_retry)),
        )
        .into_element()
}

pub fn source_browse_page() -> AnyElement {
    let browse = state::source_browse_state();
    let session = state::session();
    let demo_mode = session.get().demo();
    let connected = session.get().connected();
    let input = signal(state::source_browse_state().get().search_input);
    page_scroll(
        view()
            .gap(px(14.0))
            .child(page_head(
                "单源浏览",
                "按片源分类浏览；切源会清空旧分类与分页，源内搜索只查当前源".to_string(),
                vec![
                    button("刷新源列表", Btn::Ghost, state::source_refresh_sites),
                    demo_badge_if(demo_mode),
                ],
            ))
            .children((!connected && !demo_mode).then(|| {
                empty_state(
                    Icon::Server,
                    "需要连接实例",
                    "单源浏览使用实例已授权的片源；演示模式没有额外片源。",
                    Some(button("去连接", Btn::Primary, || {
                        state::go(Route::Connect)
                    })),
                )
            }))
            .children((connected && !demo_mode).then(move || {
                view()
                    .gap(px(12.0))
                    .child(dynamic(move || source_browse_panel(&browse)))
                    .child(source_search_row(input))
                    .child(dynamic(move || source_search_status(&browse, input)))
                    .child(dynamic(move || source_list_body(&browse)))
                    .into_element()
            }))
            .children(demo_mode.then(|| {
                empty_state(
                    Icon::Server,
                    "演示模式没有单源浏览",
                    "演示片单是受控的虚构作品；单源分类需要实例的已授权采集源。",
                    Some(button("回到搜索", Btn::Primary, state::open_search)),
                )
            }))
            .into_element(),
    )
}

fn source_browse_panel(browse: &Signal<SourceBrowseState>) -> AnyElement {
    let state_now = browse.get();
    view()
        .p(px(12.0))
        .rounded(px(RADIUS_CARD))
        .bg(SURFACE)
        .border_1()
        .border_color(BORDER)
        .gap(px(9.0))
        .child(
            view()
                .flex_row()
                .items_center()
                .gap(px(8.0))
                .child(
                    text("片源")
                        .text_size(px(11.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(FAINT),
                )
                .children((state_now.sites_phase == LoadPhase::Loading).then(|| {
                    text("正在读取可用片源…")
                        .text_size(px(11.0))
                        .text_color(MUTED)
                }))
                .children((!state_now.sites_error.is_empty()).then(|| {
                    text(state_now.sites_error.clone())
                        .text_size(px(11.0))
                        .text_color(DANGER)
                }))
                .children((!state_now.sites.is_empty()).then(|| {
                    text(format!("{} 个可用源", state_now.sites.len()))
                        .text_size(px(10.5))
                        .text_color(FAINT)
                })),
        )
        .children((!state_now.sites.is_empty()).then(|| {
            site_chips(
                state_now
                    .sites
                    .iter()
                    .map(|site| (site.key.clone(), site.name.clone()))
                    .collect(),
                state_now.source.clone(),
                state::source_select,
            )
        }))
        .children((!state_now.categories.is_empty()).then(|| {
            category_chips(
                state_now.categories.clone(),
                state_now.parent.clone(),
                state_now.category.clone(),
                state::source_set_parent,
                state::source_set_category,
            )
        }))
        .children((state_now.sites_phase == LoadPhase::Error).then(|| {
            view()
                .flex_row()
                .items_center()
                .gap(px(8.0))
                .child(button("重试", Btn::Secondary, state::source_retry))
                .child(button(
                    "刷新源列表",
                    Btn::Ghost,
                    state::source_refresh_sites,
                ))
        }))
        .into_element()
}

fn source_search_row(draft: Signal<String>) -> AnyElement {
    let submit = {
        move || {
            state::source_search_input(draft.get());
            state::source_run_search();
        }
    };
    view()
        .flex_row()
        .flex_wrap()
        .items_center()
        .gap(px(7.0))
        .child(
            input()
                .value(move || draft.get())
                .placeholder("在这个源内搜索片名…")
                .placeholder_color(FAINT)
                .text_color(TEXT)
                .text_size(px(12.0))
                .h(px(34.0))
                .flex_1()
                .min_w(px(200.0))
                .px(px(10.0))
                .rounded(px(8.0))
                .bg(RAISED)
                .border_1()
                .border_color(BORDER)
                .focus(|s| s.border_color(alpha(0xe9b872, 0.55)))
                .on_change(move |value: &str| draft.set(value.to_owned()))
                .on_submit(move |_: &str| submit()),
        )
        .child(button("源内搜索", Btn::Secondary, submit))
        .into_element()
}

fn source_search_status(browse: &Signal<SourceBrowseState>, input: Signal<String>) -> AnyElement {
    let state_now = browse.get();
    if !state_now.searching {
        return view().into_element();
    }
    view()
        .flex_row()
        .flex_wrap()
        .items_center()
        .gap(px(8.0))
        .child(
            text(format!("只搜索当前源：{}", state_now.keyword))
                .text_size(px(11.0))
                .text_color(MUTED),
        )
        .child(chip("源内搜索", ACCENT, DEMO_TINT))
        .child(button("返回浏览", Btn::Ghost, move || {
            input.set(String::new());
            state::source_back_browse();
        }))
        .into_element()
}

fn source_list_body(browse: &Signal<SourceBrowseState>) -> AnyElement {
    let state_now = browse.get();
    if state_now.sites_phase == LoadPhase::Ready && state_now.sites.is_empty() {
        return empty_state(
            Icon::Server,
            "当前账号没有可用片源",
            "管理员没有为这个账号启用采集源，或源全部被禁用；刷新源列表后重试。",
            Some(button(
                "刷新源列表",
                Btn::Secondary,
                state::source_refresh_sites,
            )),
        );
    }
    if state_now.phase == LoadPhase::Error {
        return view()
            .gap(px(10.0))
            .child(empty_state(
                Icon::Alert,
                "片源浏览没有完成",
                state_now.error.clone(),
                Some(button("重试", Btn::Primary, state::source_retry)),
            ))
            .into_element();
    }
    if (state_now.phase == LoadPhase::Idle || state_now.phase == LoadPhase::Loading)
        && state_now.items.is_empty()
    {
        return view()
            .grid()
            .grid_cols(5)
            .gap(px(14.0))
            .children((0..10).map(|_| skeleton_card()))
            .into_element();
    }
    if state_now.categories.is_empty() {
        return empty_state(
            Icon::Server,
            "这个源没有可用分类",
            if state_now.notice.is_empty() {
                "源站没有返回分类列表；它可能暂时不可用，换一个源或稍后重试。".to_string()
            } else {
                state_now.notice.clone()
            },
            Some(button("重试", Btn::Secondary, state::source_retry)),
        );
    }
    if state_now.items.is_empty() {
        return empty_state(
            Icon::Search,
            if state_now.searching {
                "这个源内没有找到匹配内容"
            } else {
                "这个分类没有条目"
            },
            if state_now.notice.is_empty() {
                "这个源可能暂时不可用；换一个分类或源后重试。".to_string()
            } else {
                state_now.notice.clone()
            },
            Some(button("重试", Btn::Secondary, state::source_retry)),
        );
    }
    view()
        .gap(px(12.0))
        .children(
            (!state_now.notice.is_empty()).then(|| notice_banner_text(&state_now.notice, false)),
        )
        .child(browse_grid(state_now.items.clone(), false))
        .child(pagination_footer(
            state_now.page,
            state_now.page_count,
            state_now.total,
            state_now.loading_more,
            state::source_more,
            || {
                state::source_retry();
            },
        ))
        .into_element()
}

// ------------------------------ short drama -----------------------------

pub fn duanju_page() -> AnyElement {
    let duanju = state::duanju_state();
    let session = state::session();
    let connected = session.get().connected();
    let demo_mode = session.get().demo();
    page_scroll(
        view()
            .gap(px(14.0))
            .child(page_head(
                "短剧",
                "实例已授权的短剧采集源；分类来自源站，竖屏连播从详情进入".to_string(),
                vec![
                    button("刷新源列表", Btn::Ghost, state::duanju_refresh_sites),
                    demo_badge_if(demo_mode),
                ],
            ))
            .children((!connected && !demo_mode).then(|| {
                empty_state(
                    Icon::Play,
                    "需要连接实例",
                    "短剧列表来自实例配置的采集源；演示模式没有短剧片源。",
                    Some(button("去连接", Btn::Primary, || {
                        state::go(Route::Connect)
                    })),
                )
            }))
            .children((connected && !demo_mode).then(move || {
                view()
                    .gap(px(12.0))
                    .child(dynamic(move || duanju_body(&duanju)))
                    .into_element()
            }))
            .children(demo_mode.then(|| {
                empty_state(
                    Icon::Film,
                    "演示模式没有短剧片源",
                    "短剧列表与竖屏连播需要实例里已配置的短剧采集源。",
                    Some(button("回到搜索", Btn::Primary, state::open_search)),
                )
            }))
            .into_element(),
    )
}

fn duanju_body(duanju: &Signal<DuanjuState>) -> AnyElement {
    let state_now = duanju.get();
    view()
        .gap(px(12.0))
        .child(
            view()
                .p(px(12.0))
                .rounded(px(RADIUS_CARD))
                .bg(SURFACE)
                .border_1()
                .border_color(BORDER)
                .gap(px(9.0))
                .child(
                    view()
                        .flex_row()
                        .items_center()
                        .gap(px(8.0))
                        .child(
                            text("短剧源")
                                .text_size(px(11.0))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(FAINT),
                        )
                        .children((state_now.sites_phase == LoadPhase::Loading).then(|| {
                            text("正在筛选包含短剧分类的源…")
                                .text_size(px(11.0))
                                .text_color(MUTED)
                        }))
                        .children((!state_now.sites_error.is_empty()).then(|| {
                            text(state_now.sites_error.clone())
                                .text_size(px(11.0))
                                .text_color(DANGER)
                        })),
                )
                .children((!state_now.sites.is_empty()).then(|| {
                    site_chips(
                        state_now
                            .sites
                            .iter()
                            .map(|site| (site.key.clone(), site.name.clone()))
                            .collect(),
                        state_now.source.clone(),
                        state::duanju_select,
                    )
                }))
                .children((!state_now.categories.is_empty()).then(|| {
                    category_chips(
                        state_now.categories.clone(),
                        state_now.parent.clone(),
                        state_now.category.clone(),
                        state::duanju_set_parent,
                        state::duanju_set_category,
                    )
                }))
                .children(state_now.sites.is_empty().then(|| {
                    text("当前配置没有包含短剧分类的采集源；添加短剧源后刷新。")
                        .text_size(px(11.0))
                        .text_color(MUTED)
                })),
        )
        .child(duanju_recommend_section(&state_now))
        .child(duanju_list_body(duanju))
        .into_element()
}

fn duanju_recommend_section(state_now: &DuanjuState) -> AnyElement {
    if state_now.recommends_phase == LoadPhase::Loading {
        return view()
            .gap(px(8.0))
            .child(section_header(
                "热播短剧",
                Some("正在读取推荐…".to_string()),
                None,
            ))
            .child(
                view()
                    .h(px(120.0))
                    .w_full()
                    .rounded(px(RADIUS_CARD))
                    .bg(RAISED),
            )
            .into_element();
    }
    if state_now.recommends_phase == LoadPhase::Error {
        return view()
            .p(px(11.0))
            .rounded(px(9.0))
            .bg(DANGER_TINT)
            .border_1()
            .border_color(alpha(0xf07b7b, 0.3))
            .flex_row()
            .items_center()
            .gap_3()
            .child(
                text(format!(
                    "热播短剧没有加载成功：{}",
                    state_now.recommends_error
                ))
                .flex_1()
                .text_size(px(11.0))
                .text_color(DANGER)
                .number_of_lines(2),
            )
            .child(button("重试", Btn::Ghost, state::duanju_retry_recommends))
            .into_element();
    }
    if state_now.recommends.is_empty() {
        return view().into_element();
    }
    view()
        .gap(px(10.0))
        .child(section_header(
            "热播短剧",
            Some("来自第一个可用短剧源；失败不会伪装成空推荐".to_string()),
            None,
        ))
        .child(browse_grid(
            state_now.recommends.iter().take(6).cloned().collect(),
            true,
        ))
        .into_element()
}

fn duanju_list_body(duanju: &Signal<DuanjuState>) -> AnyElement {
    let state_now = duanju.get();
    if state_now.sites_phase == LoadPhase::Ready && state_now.sites.is_empty() {
        return empty_state(
            Icon::Film,
            "当前没有短剧采集源",
            "实例里没有声明短剧分类的源；添加短剧源后刷新源列表。",
            Some(button(
                "刷新源列表",
                Btn::Secondary,
                state::duanju_refresh_sites,
            )),
        );
    }
    if state_now.phase == LoadPhase::Error {
        return view()
            .gap(px(10.0))
            .child(empty_state(
                Icon::Alert,
                "短剧列表没有完成",
                state_now.error.clone(),
                Some(button("重试", Btn::Primary, state::duanju_retry)),
            ))
            .into_element();
    }
    if (state_now.phase == LoadPhase::Idle || state_now.phase == LoadPhase::Loading)
        && state_now.items.is_empty()
    {
        return view()
            .grid()
            .grid_cols(5)
            .gap(px(14.0))
            .children((0..10).map(|_| skeleton_card()))
            .into_element();
    }
    if state_now.items.is_empty() {
        return empty_state(
            Icon::Film,
            "这个分类还没有短剧",
            if state_now.notice.is_empty() {
                "换一个分类或源；短剧源可能暂时不可用。".to_string()
            } else {
                state_now.notice.clone()
            },
            Some(button("重试", Btn::Secondary, state::duanju_retry)),
        );
    }
    view()
        .gap(px(12.0))
        .children(
            (!state_now.notice.is_empty()).then(|| notice_banner_text(&state_now.notice, false)),
        )
        .child(browse_grid(state_now.items.clone(), true))
        .child(pagination_footer(
            state_now.page,
            state_now.page_count,
            state_now.total,
            state_now.loading_more,
            state::duanju_more,
            || {
                state::duanju_retry();
            },
        ))
        .into_element()
}

// -------------------------------- search --------------------------------

pub fn search_page() -> AnyElement {
    let search = state::search_state();
    let session = state::session();
    let demo_mode = session.get().demo();
    let content = dynamic(move || {
        let state_ = search.get();
        match state_.phase {
            LoadPhase::Idle => search_idle(demo_mode),
            LoadPhase::Loading => search_loading(&state_),
            LoadPhase::Error => view()
                .gap(px(14.0))
                .child(empty_state(
                    Icon::Alert,
                    "搜索没有完成",
                    if state_.error.is_empty() {
                        "请检查连接后重试。".to_string()
                    } else {
                        state_.error.clone()
                    },
                    Some(button("重试", Btn::Primary, state::run_search)),
                ))
                .into_element(),
            LoadPhase::Ready => {
                if state_.works.is_empty() {
                    view()
                        .gap(px(14.0))
                        .child(empty_state(
                            Icon::Search,
                            "没有找到匹配的内容",
                            "换一个片名、类型或片源名称。聚合只在标题与年份完全一致时合并卡片，不会把同名作品强行并成一条。",
                            Some(button("再试一次", Btn::Secondary, state::open_search)),
                        ))
                        .into_element()
                } else {
                    search_results(&state_, false)
                }
            }
        }
    });
    page_scroll(
        view()
            .gap(px(16.0))
            .child(page_head(
                "搜索",
                if demo_mode {
                    "演示片单在本机即时筛选；真实实例会聚合所有已启用片源".to_string()
                } else {
                    "首个片源到达即显示；断流时自动降级到普通搜索".to_string()
                },
                vec![
                    when(
                        move || search.get().phase == LoadPhase::Loading,
                        || button("取消", Btn::Ghost, state::cancel_search),
                    )
                    .into_element(),
                    demo_badge_if(demo_mode),
                ],
            ))
            .child(content)
            .into_element(),
    )
}

fn demo_badge_if(demo_mode: bool) -> AnyElement {
    if demo_mode {
        demo_chip()
    } else {
        view().into_element()
    }
}

fn search_results(state_: &SearchState, demo_mode: bool) -> AnyElement {
    let sources: usize = state_.works.iter().map(|work| work.candidates.len()).sum();
    view()
        .gap(px(14.0))
        .children(progress_strip(state_))
        .children((!state_.notice.is_empty()).then(|| notice_banner(state_)))
        .child(
            view()
                .flex_row()
                .items_center()
                .gap(px(9.0))
                .child(
                    text(format!("{} 部作品", state_.works.len()))
                        .text_size(px(12.5))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(TEXT),
                )
                .child(
                    text(format!("{sources} 个片源结果"))
                        .text_size(px(11.5))
                        .text_color(MUTED),
                )
                .children(
                    (!state_.query.is_empty())
                        .then(|| chip(format!("“{}”", state_.query), ACCENT, DEMO_TINT)),
                )
                .children(demo_mode.then(demo_chip)),
        )
        .child(works_grid(state_.works.clone(), 210.0, 7, true))
        .into_element()
}

fn search_loading(state_: &SearchState) -> AnyElement {
    if state_.works.is_empty() {
        return view()
            .gap(px(12.0))
            .child(
                view()
                    .flex_row()
                    .items_center()
                    .gap(px(9.0))
                    .child(text("正在聚合片源…").text_size(px(13.0)).text_color(MUTED))
                    .child(button("取消", Btn::Ghost, state::cancel_search)),
            )
            .children(progress_strip(state_))
            .children((!state_.notice.is_empty()).then(|| notice_banner(state_)))
            .child(
                view()
                    .grid()
                    .grid_cols(5)
                    .gap(px(16.0))
                    .children((0..10).map(|_| skeleton_card())),
            )
            .into_element();
    }
    view()
        .gap(px(14.0))
        .child(
            view()
                .flex_row()
                .items_center()
                .gap(px(9.0))
                .child(
                    text("正在等待其余片源…")
                        .text_size(px(13.0))
                        .text_color(MUTED),
                )
                .child(button("取消", Btn::Ghost, state::cancel_search)),
        )
        .children(progress_strip(state_))
        .children((!state_.notice.is_empty()).then(|| notice_banner(state_)))
        .child(works_grid(state_.works.clone(), 210.0, 7, true))
        .into_element()
}

/// Per-source arrival: how many of the instance's sources have answered and
/// which ones failed. Results already on screen stay above it.
fn progress_strip(state_: &SearchState) -> Option<AnyElement> {
    if state_.total_sources == 0 && state_.sources.is_empty() {
        return None;
    }
    let total = state_.total_sources.max(state_.completed_sources);
    let label = if total == 0 {
        format!("已完成 {} 个片源", state_.completed_sources)
    } else {
        format!(
            "已完成 {} / {} 个片源",
            state_.completed_sources.min(total),
            total
        )
    };
    Some(
        view()
            .p(px(11.0))
            .rounded(px(9.0))
            .bg(SURFACE)
            .border_1()
            .border_color(BORDER)
            .gap(px(8.0))
            .child(
                view()
                    .flex_row()
                    .flex_wrap()
                    .items_center()
                    .gap(px(8.0))
                    .child(
                        text(label)
                            .text_size(px(11.5))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(TEXT),
                    )
                    .child(
                        text(format!("已到 {} 条", state_.results.len()))
                            .text_size(px(11.0))
                            .text_color(MUTED),
                    )
                    .children(
                        (state_.phase == LoadPhase::Loading)
                            .then(|| text("流式接收中").text_size(px(10.5)).text_color(ACCENT)),
                    ),
            )
            .children((!state_.sources.is_empty()).then(|| {
                view()
                    .flex_row()
                    .flex_wrap()
                    .gap(px(6.0))
                    .children(state_.sources.iter().map(|source| {
                        let (fg, bg) = if source.error.is_some() {
                            (DANGER, DANGER_TINT)
                        } else if source.count == 0 {
                            (MUTED, RAISED)
                        } else {
                            (SUCCESS, SUCCESS_TINT)
                        };
                        let label = if let Some(error) = &source.error {
                            format!("{} · {}", source.name, error)
                        } else if source.count == 0 {
                            format!("{} · 0", source.name)
                        } else {
                            format!("{} · {}", source.name, source.count)
                        };
                        chip(label, fg, bg)
                    }))
            }))
            .into_element(),
    )
}

fn notice_banner(state_: &SearchState) -> AnyElement {
    let (fg, bg, border) = if state_.partial {
        (DANGER, DANGER_TINT, alpha(0xf07b7b, 0.3))
    } else {
        (ACCENT, DEMO_TINT, alpha(0xe9b872, 0.3))
    };
    view()
        .p(px(11.0))
        .rounded(px(9.0))
        .bg(bg)
        .border_1()
        .border_color(border)
        .flex_row()
        .items_center()
        .gap_3()
        .child(
            text(state_.notice.clone())
                .flex_1()
                .text_size(px(11.5))
                .text_color(fg)
                .line_height(nsh_core::LineHeight::Multiple(1.5))
                .number_of_lines(3),
        )
        .child(button("重试", Btn::Ghost, state::run_search))
        .into_element()
}

fn search_idle(demo_mode: bool) -> AnyElement {
    let search = state::search_state();
    let works = if demo_mode { demo::works() } else { Vec::new() };
    view()
        .gap(px(16.0))
        .children((!search.get().suggestions.is_empty()).then(suggestion_card))
        .children((!search.get().history.is_empty()).then(history_card))
        .child(
            view()
                .p(px(20.0))
                .rounded(px(RADIUS_CARD))
                .bg(SURFACE)
                .border_1()
                .border_color(BORDER)
                .gap(px(12.0))
                .child(
                    text(if demo_mode {
                        "试试搜“演示”".to_string()
                    } else {
                        "从片名开始".to_string()
                    })
                    .text_size(px(17.0))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(TEXT),
                )
                .child(
                    text(if demo_mode {
                        "演示片单包含五部虚构作品，覆盖 MP4、HLS、AES-128 与范围请求。搜索建议与历史在演示中只保存在本机。".to_string()
                    } else {
                        "输入片名后按 Enter。首个片源到达就显示；每源进度、失败与超时会单独标注，断流时补一次普通搜索。".to_string()
                    })
                    .text_size(px(12.0))
                    .text_color(MUTED)
                    .line_height(nsh_core::LineHeight::Multiple(1.6)),
                )
                .children(demo_mode.then(|| {
                    view()
                        .flex_row()
                        .flex_wrap()
                        .gap(px(8.0))
                        .children(
                            ["回声", "暮色", "远山", "蓝色", "短测试"]
                                .into_iter()
                                .map(|value| search_chip(value.to_string(), true)),
                        )
                }))
                .children((!demo_mode).then(|| {
                    text("搜索历史会自动同步到实例；删除单条或清空会同时更新本机与实例。")
                        .text_size(px(10.5))
                        .text_color(FAINT)
                })),
        )
        .children((!works.is_empty()).then(|| {
            view()
                .gap(px(12.0))
                .child(section_header(
                    "演示片单",
                    Some("点击卡片查看片源与剧集".to_string()),
                    Some(demo_chip()),
                ))
                .child(works_grid(works, 210.0, 7, true))
        }))
        .into_element()
}

fn suggestion_card() -> AnyElement {
    let search = state::search_state();
    view()
        .p(px(14.0))
        .rounded(px(RADIUS_CARD))
        .bg(SURFACE)
        .border_1()
        .border_color(BORDER)
        .gap(px(9.0))
        .child(
            view()
                .flex_row()
                .items_center()
                .justify_between()
                .child(
                    text("联想词")
                        .text_size(px(11.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(FAINT)
                        .letter_spacing(px(0.8)),
                )
                .child(
                    text("来自首个可用片源")
                        .text_size(px(10.0))
                        .text_color(FAINT),
                ),
        )
        .child(
            view().flex_row().flex_wrap().gap(px(8.0)).children(
                search
                    .get()
                    .suggestions
                    .iter()
                    .map(|value| search_chip(value.clone(), false)),
            ),
        )
        .into_element()
}

fn history_card() -> AnyElement {
    let search = state::search_state();
    let confirm = signal(false);
    let clear = move || {
        if confirm.get() {
            state::clear_search_history();
            confirm.set(false);
        } else {
            confirm.set(true);
            state::toast(
                "再次点击“清空”会删除全部搜索历史；实例端也会同步清空。",
                ToastTone::Info,
            );
        }
    };
    view()
        .p(px(14.0))
        .rounded(px(RADIUS_CARD))
        .bg(SURFACE)
        .border_1()
        .border_color(BORDER)
        .gap(px(9.0))
        .child(
            view()
                .flex_row()
                .items_center()
                .justify_between()
                .child(
                    text("搜索历史")
                        .text_size(px(11.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(FAINT)
                        .letter_spacing(px(0.8)),
                )
                .child(button("清空", Btn::Ghost, clear)),
        )
        .child(
            view().flex_row().flex_wrap().gap(px(8.0)).children(
                search
                    .get()
                    .history
                    .iter()
                    .map(|keyword| history_chip(keyword.clone())),
            ),
        )
        .into_element()
}

fn search_chip(label: String, accent: bool) -> AnyElement {
    let action = label.clone();
    view()
        .h(px(30.0))
        .px(px(12.0))
        .rounded_full()
        .bg(if accent { DEMO_TINT } else { RAISED })
        .border_1()
        .border_color(if accent { alpha(0xe9b872, 0.3) } else { BORDER })
        .items_center()
        .justify_center()
        .focusable(true)
        .role(AccessibilityRole::Button)
        .label(format!("搜索：{label}"))
        .hover(|s| s.bg(HOVER))
        .on_press(move || state::search_for(action.clone()))
        .child(
            text(label)
                .text_size(px(11.5))
                .text_color(if accent { ACCENT } else { MUTED })
                .number_of_lines(1),
        )
        .into_element()
}

fn history_chip(keyword: String) -> AnyElement {
    let search_for = keyword.clone();
    let remove = keyword.clone();
    view()
        .h(px(30.0))
        .pl(px(11.0))
        .pr(px(4.0))
        .rounded_full()
        .bg(RAISED)
        .border_1()
        .border_color(BORDER)
        .flex_row()
        .items_center()
        .gap(px(3.0))
        .child(
            view()
                .h_full()
                .items_center()
                .focusable(true)
                .role(AccessibilityRole::Button)
                .label(format!("搜索：{keyword}"))
                .on_press(move || state::search_for(search_for.clone()))
                .child(
                    text(keyword)
                        .text_size(px(11.5))
                        .text_color(MUTED)
                        .number_of_lines(1),
                ),
        )
        .child(
            view()
                .size(px(21.0))
                .rounded_full()
                .items_center()
                .justify_center()
                .focusable(true)
                .role(AccessibilityRole::Button)
                .label(format!("删除搜索历史：{remove}"))
                .hover(|s| s.bg(HOVER))
                .on_press(move || state::remove_search_history(&remove))
                .child(icon(Icon::Close, 10.0, || FAINT)),
        )
        .into_element()
}

fn skeleton_card() -> AnyElement {
    view()
        .gap(px(8.0))
        .child(
            view()
                .w_full()
                .aspect_ratio(2.0 / 3.0)
                .rounded(px(RADIUS_CARD))
                .bg(RAISED)
                .border_1()
                .border_color(LINE),
        )
        .child(view().w(px(120.0)).h(px(10.0)).rounded(px(5.0)).bg(RAISED))
        .child(view().w(px(80.0)).h(px(8.0)).rounded(px(4.0)).bg(LINE))
        .into_element()
}

// -------------------------------- discover ------------------------------

pub fn discover_page() -> AnyElement {
    let session = state::session();
    let demo_mode = session.get().demo();
    page_scroll(
        view()
            .gap(px(16.0))
            .child(page_head(
                "发现",
                if demo_mode {
                    "演示样片：本机示例数据，不请求豆瓣或 Bangumi".to_string()
                } else {
                    "豆瓣热门/最新/高分/冷门与 Bangumi 每日放送，经实例代理读取".to_string()
                },
                vec![
                    button("刷新", Btn::Ghost, state::discover_retry),
                    demo_badge_if(demo_mode),
                ],
            ))
            .child(dynamic(kind_tabs))
            .child(dynamic(selector_bar))
            .child(dynamic(move || discover_body(demo_mode)))
            .into_element(),
    )
}

fn kind_tabs() -> AnyElement {
    let discover = state::discover_state();
    view()
        .flex_row()
        .flex_wrap()
        .gap(px(6.0))
        .children(MediaKind::all().into_iter().map(|kind| {
            let active = discover.get().kind == kind;
            view()
                .h(px(32.0))
                .px(px(14.0))
                .rounded_full()
                .bg(if active { RAISED } else { alpha(0x000000, 0.0) })
                .border_1()
                .border_color(if active {
                    alpha(0xe9b872, 0.4)
                } else {
                    alpha(0x000000, 0.0)
                })
                .items_center()
                .justify_center()
                .focusable(true)
                .role(AccessibilityRole::Tab)
                .label(kind.label())
                .accessibility_state(AccessibilityState {
                    selected: active,
                    ..AccessibilityState::default()
                })
                .hover(|s| s.bg(HOVER))
                .on_press(move || state::discover_set_kind(kind))
                .child(
                    text(kind.label())
                        .text_size(px(12.0))
                        .font_weight(if active {
                            FontWeight::SEMIBOLD
                        } else {
                            FontWeight::MEDIUM
                        })
                        .text_color(if active { TEXT } else { MUTED }),
                )
                .into_element()
        }))
        .into_element()
}

fn selector_bar() -> AnyElement {
    let discover = state::discover_state();
    let kind = discover.get().kind;
    view()
        .p(px(14.0))
        .rounded(px(RADIUS_CARD))
        .bg(SURFACE)
        .border_1()
        .border_color(BORDER)
        .gap(px(10.0))
        .child(match kind {
            MediaKind::Movie => movie_selector(),
            MediaKind::Tv => tv_selector(false),
            MediaKind::Show => tv_selector(true),
            MediaKind::Anime => anime_selector(),
        })
        .children(match kind {
            MediaKind::Movie if discover.get().movie_primary != "全部" => None,
            MediaKind::Tv | MediaKind::Show if discover.get().tv_primary != "全部" => None,
            MediaKind::Anime if discover.get().anime == AnimeTab::Daily => None,
            _ => Some(filter_bar()),
        })
        .into_element()
}

fn movie_selector() -> AnyElement {
    let discover = state::discover_state();
    let current = discover.get();
    let primary = current.movie_primary.clone();
    let region = current.movie_region.clone();
    view()
        .gap(px(9.0))
        .child(view().flex_row().flex_wrap().gap(px(6.0)).children(
            discover::movie_primaries().into_iter().map(|value| {
                let active = primary == value;
                selector_chip(value, active, move || {
                    state::discover_set_movie_primary(value)
                })
            }),
        ))
        .children((primary != "全部").then(|| {
            view()
                .flex_row()
                .flex_wrap()
                .items_center()
                .gap(px(6.0))
                .child(overline("地区"))
                .children(discover::movie_regions().into_iter().map(|value| {
                    let active = region == value;
                    selector_chip(value, active, move || {
                        state::discover_set_movie_region(value)
                    })
                }))
        }))
        .into_element()
}

fn tv_selector(show: bool) -> AnyElement {
    let discover = state::discover_state();
    let state_now = discover.get();
    let primary = if show {
        state_now.show_primary.clone()
    } else {
        state_now.tv_primary.clone()
    };
    let secondary = if show {
        state_now.show_secondary.clone()
    } else {
        state_now.tv_secondary.clone()
    };
    let secondaries = if show {
        discover::show_secondaries()
    } else {
        discover::tv_secondaries()
    };
    view()
        .gap(px(9.0))
        .child(view().flex_row().flex_wrap().gap(px(6.0)).children(
            ["全部", "最近热门"].into_iter().map(|value| {
                let active = primary == value;
                selector_chip(value, active, move || state::discover_set_tv_primary(value))
            }),
        ))
        .children((primary != "全部").then(|| {
            view()
                .flex_row()
                .flex_wrap()
                .items_center()
                .gap(px(6.0))
                .child(overline(if show { "范围" } else { "地区" }))
                .children(secondaries.into_iter().map(|(label, value)| {
                    let active = secondary == value;
                    selector_chip(label, active, move || state::discover_set_secondary(value))
                }))
        }))
        .into_element()
}

fn anime_selector() -> AnyElement {
    let discover = state::discover_state();
    let tab = discover.get().anime;
    view()
        .gap(px(9.0))
        .child(
            view().flex_row().flex_wrap().gap(px(6.0)).children(
                [AnimeTab::Daily, AnimeTab::Tv, AnimeTab::Movie]
                    .into_iter()
                    .map(|value| {
                        let active = tab == value;
                        selector_chip(value.label(), active, move || {
                            state::discover_set_anime(value)
                        })
                    }),
            ),
        )
        .into_element()
}

fn selector_chip(label: &'static str, active: bool, on_press: impl Fn() + 'static) -> AnyElement {
    view()
        .h(px(29.0))
        .px(px(11.0))
        .rounded_full()
        .bg(if active { DEMO_TINT } else { RAISED })
        .border_1()
        .border_color(if active {
            alpha(0xe9b872, 0.38)
        } else {
            BORDER
        })
        .items_center()
        .justify_center()
        .focusable(true)
        .role(AccessibilityRole::Button)
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
                .text_color(if active { ACCENT } else { MUTED }),
        )
        .into_element()
}

fn filter_bar() -> AnyElement {
    let discover = state::discover_state();
    let expanded = signal(String::new());
    dynamic(move || filter_bar_body(&discover.get(), expanded))
}

fn filter_bar_body(state_now: &DiscoverState, expanded: Signal<String>) -> AnyElement {
    let rows =
        discover::filter_specs(state_now.kind, state_now.anime)
            .into_iter()
            .map(|spec| {
                let key = spec.key;
                let title = spec.title;
                let current = discover::filter_label(
                    &discover::filter_specs(state_now.kind, state_now.anime),
                    &state_now.filters,
                    key,
                );
                let active = state_now.filters.get(key) != if key == "sort" { "T" } else { "all" };
                let chip_text = if current == title {
                    title.to_string()
                } else {
                    format!("{title} · {current}")
                };
                let options = spec.options;
                let open = expanded.get() == key;
                let toggle = expanded;
                view()
                    .gap(px(6.0))
                    .child(
                        view()
                            .flex_row()
                            .flex_wrap()
                            .items_center()
                            .gap(px(6.0))
                            .child(
                                view()
                                    .h(px(26.0))
                                    .px(px(10.0))
                                    .rounded_full()
                                    .bg(if active { DEMO_TINT } else { HOVER })
                                    .border_1()
                                    .border_color(if active {
                                        alpha(0xe9b872, 0.35)
                                    } else {
                                        BORDER
                                    })
                                    .flex_row()
                                    .items_center()
                                    .gap(px(5.0))
                                    .focusable(true)
                                    .role(AccessibilityRole::Button)
                                    .label(format!("{title}：{current}"))
                                    .on_press(move || {
                                        if toggle.get() == key {
                                            toggle.set(String::new());
                                        } else {
                                            toggle.set(key.to_string());
                                        }
                                    })
                                    .child(
                                        text(chip_text)
                                            .text_size(px(11.0))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(if active { ACCENT } else { MUTED }),
                                    )
                                    .child(icon(Icon::ChevronDown, 10.0, || FAINT)),
                            )
                            .children((!state_now.filters.is_default() && key == "sort").then(
                                || button("重置筛选", Btn::Ghost, state::discover_reset_filters),
                            )),
                    )
                    .children(open.then(|| {
                        view().flex_row().flex_wrap().gap(px(5.0)).children(
                            options.into_iter().map(|(label, value)| {
                                let selected = state_now.filters.get(key) == value;
                                let label_text = label.clone();
                                view()
                                    .h(px(26.0))
                                    .px(px(9.0))
                                    .rounded(px(7.0))
                                    .bg(if selected { DEMO_TINT } else { RAISED })
                                    .border_1()
                                    .border_color(if selected {
                                        alpha(0xe9b872, 0.35)
                                    } else {
                                        alpha(0x000000, 0.0)
                                    })
                                    .items_center()
                                    .justify_center()
                                    .focusable(true)
                                    .role(AccessibilityRole::Button)
                                    .label(label_text.clone())
                                    .accessibility_state(AccessibilityState {
                                        selected,
                                        ..AccessibilityState::default()
                                    })
                                    .hover(|s| s.bg(HOVER))
                                    .on_press(move || {
                                        state::discover_set_filter(key, &value);
                                        toggle.set(String::new());
                                    })
                                    .child(
                                        text(label_text)
                                            .text_size(px(11.0))
                                            .text_color(if selected { ACCENT } else { MUTED }),
                                    )
                                    .into_element()
                            }),
                        )
                    }))
                    .into_element()
            });
    view()
        .gap(px(8.0))
        .child(
            view()
                .flex_row()
                .items_center()
                .gap(px(8.0))
                .child(overline("筛选"))
                .children((!state_now.filters.is_default()).then(|| {
                    text("条件会在重启后恢复")
                        .text_size(px(10.0))
                        .text_color(FAINT)
                })),
        )
        .children(rows)
        .into_element()
}

fn discover_body(demo_mode: bool) -> AnyElement {
    let discover = state::discover_state();
    let state_now = discover.get();
    if state_now.kind == MediaKind::Anime && state_now.anime == AnimeTab::Daily {
        return daily_body(demo_mode, &state_now);
    }
    if state_now.phase == LoadPhase::Error && state_now.items.is_empty() {
        return empty_state(
            Icon::Alert,
            "发现页没有加载成功",
            if state_now.error.is_empty() {
                "豆瓣或实例暂时不可达。".to_string()
            } else {
                state_now.error.clone()
            },
            Some(button("重试", Btn::Primary, state::discover_retry)),
        );
    }
    if state_now.phase == LoadPhase::Loading && state_now.items.is_empty() {
        return view()
            .grid()
            .grid_cols(6)
            .gap(px(14.0))
            .children((0..12).map(|_| skeleton_card()))
            .into_element();
    }
    if state_now.items.is_empty() {
        return empty_state(
            Icon::Search,
            "这组条件下没有条目",
            "换一个地区、年代或类型；筛选条件已经记住。",
            Some(button(
                "重置筛选",
                Btn::Secondary,
                state::discover_reset_filters,
            )),
        );
    }
    view()
        .gap(px(12.0))
        .children(
            (!state_now.notice.is_empty()).then(|| notice_banner_text(&state_now.notice, false)),
        )
        .child(discover_grid(state_now.items.clone()))
        .child(
            view()
                .flex_row()
                .items_center()
                .justify_between()
                .child(
                    text(format!(
                        "第 {} 页 · {} 条{}",
                        state_now.page + 1,
                        state_now.items.len(),
                        if demo_mode { " · 演示样片" } else { "" }
                    ))
                    .text_size(px(10.5))
                    .text_color(FAINT),
                )
                .child(
                    view()
                        .flex_row()
                        .items_center()
                        .gap(px(8.0))
                        .children(state_now.loading_more.then(|| {
                            text("正在加载下一页…")
                                .text_size(px(11.0))
                                .text_color(MUTED)
                        }))
                        .children(
                            state_now
                                .has_more
                                .then(|| button("加载更多", Btn::Secondary, state::discover_more)),
                        )
                        .children(
                            (!state_now.has_more && !demo_mode)
                                .then(|| text("已到末页").text_size(px(10.5)).text_color(FAINT)),
                        ),
                ),
        )
        .into_element()
}

fn discover_grid(items: Vec<Item>) -> AnyElement {
    let count = columns(180.0, 8);
    dynamic(move || {
        view()
            .grid()
            .grid_cols(count.get())
            .gap(px(14.0))
            .children(items.iter().map(discover_card))
            .into_element()
    })
}

fn discover_card(item: &Item) -> AnyElement {
    let work = Work {
        title: item.title.clone(),
        year: item.year.clone(),
        poster: item.poster.clone(),
        desc: String::new(),
        genres: if item.kind.is_empty() {
            Vec::new()
        } else {
            vec![item.kind.clone()]
        },
        candidates: Vec::new(),
        weight: 0,
    };
    let open = item.clone();
    let meta = match (item.rate.is_empty(), item.year.is_empty()) {
        (false, false) => format!("{} · {}", item.rate, item.year),
        (false, true) => item.rate.clone(),
        (true, false) => item.year.clone(),
        (true, true) => "暂无评分".to_string(),
    };
    view()
        .gap(px(7.0))
        .rounded(px(RADIUS_CARD))
        .focusable(true)
        .role(AccessibilityRole::Button)
        .label(format!("{}，{}，去搜索片源", item.title, meta))
        .hover(|s| s.opacity(0.94))
        .active(|s| s.opacity(0.82))
        .on_press(move || state::discover_open_item(&open))
        .child(poster(&work, true))
        .child(
            view()
                .px(px(2.0))
                .gap(px(2.0))
                .child(
                    text(item.title.clone())
                        .text_size(px(13.5))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(TEXT)
                        .number_of_lines(1),
                )
                .child(
                    view()
                        .flex_row()
                        .items_center()
                        .gap(px(6.0))
                        .child(
                            text(meta)
                                .text_size(px(11.0))
                                .text_color(if item.rate.is_empty() { FAINT } else { ACCENT }),
                        )
                        .child(text("去搜索").text_size(px(10.0)).text_color(FAINT)),
                ),
        )
        .into_element()
}

fn daily_body(demo_mode: bool, state_now: &DiscoverState) -> AnyElement {
    if state_now.phase == LoadPhase::Error && state_now.days.is_empty() {
        return empty_state(
            Icon::Alert,
            "每日放送没有加载成功",
            if state_now.error.is_empty() {
                "Bangumi 日历暂时不可达。".to_string()
            } else {
                state_now.error.clone()
            },
            Some(button("重试", Btn::Primary, state::discover_retry)),
        );
    }
    if state_now.phase == LoadPhase::Loading && state_now.days.is_empty() {
        return view()
            .grid()
            .grid_cols(6)
            .gap(px(14.0))
            .children((0..12).map(|_| skeleton_card()))
            .into_element();
    }
    let selected = state_now
        .days
        .iter()
        .find(|day| day.en == state_now.weekday);
    let items = selected.map(|day| day.items.clone()).unwrap_or_default();
    view()
        .gap(px(12.0))
        .children(
            (!state_now.notice.is_empty()).then(|| notice_banner_text(&state_now.notice, false)),
        )
        .child(view().flex_row().flex_wrap().gap(px(5.0)).children(
            discover::weekday_order().into_iter().map(|en| {
                let active = state_now.weekday == en;
                let count = state_now
                    .days
                    .iter()
                    .find(|day| day.en == en)
                    .map(|day| day.items.len())
                    .unwrap_or(0);
                view()
                    .h(px(30.0))
                    .px(px(10.0))
                    .rounded_full()
                    .bg(if active { RAISED } else { alpha(0x000000, 0.0) })
                    .border_1()
                    .border_color(if active {
                        alpha(0xe9b872, 0.4)
                    } else {
                        alpha(0x000000, 0.0)
                    })
                    .flex_row()
                    .items_center()
                    .gap(px(4.0))
                    .focusable(true)
                    .role(AccessibilityRole::Tab)
                    .label(format!("{}，{} 部", discover::weekday_cn(en), count))
                    .accessibility_state(AccessibilityState {
                        selected: active,
                        ..AccessibilityState::default()
                    })
                    .hover(|s| s.bg(HOVER))
                    .on_press(move || state::discover_set_weekday(en))
                    .child(
                        text(discover::weekday_cn(en))
                            .text_size(px(11.5))
                            .font_weight(if active {
                                FontWeight::SEMIBOLD
                            } else {
                                FontWeight::MEDIUM
                            })
                            .text_color(if active { TEXT } else { MUTED }),
                    )
                    .children(
                        (count > 0 || demo_mode)
                            .then(|| text(count.to_string()).text_size(px(9.5)).text_color(FAINT)),
                    )
                    .into_element()
            }),
        ))
        .child(
            text(format!(
                "{} · {} 部放送{}",
                discover::weekday_cn(&state_now.weekday),
                items.len(),
                if demo_mode {
                    " · 演示样片"
                } else {
                    " · 来自 Bangumi 日历"
                }
            ))
            .text_size(px(10.5))
            .text_color(FAINT),
        )
        .children(if items.is_empty() {
            Some(empty_state(
                Icon::History,
                "这一天没有放送条目",
                "切换星期；空的一天不会伪造番名或时间。",
                None,
            ))
        } else {
            Some(discover_grid(items))
        })
        .into_element()
}

fn notice_banner_text(text_value: &str, danger: bool) -> AnyElement {
    let (fg, bg, border) = if danger {
        (DANGER, DANGER_TINT, alpha(0xf07b7b, 0.3))
    } else {
        (ACCENT, DEMO_TINT, alpha(0xe9b872, 0.3))
    };
    view()
        .p(px(11.0))
        .rounded(px(9.0))
        .bg(bg)
        .border_1()
        .border_color(border)
        .child(
            text(text_value.to_string())
                .text_size(px(11.0))
                .text_color(fg)
                .line_height(nsh_core::LineHeight::Multiple(1.5))
                .number_of_lines(2),
        )
        .into_element()
}

// -------------------------------- detail --------------------------------

pub fn detail_page() -> AnyElement {
    dynamic(detail_body)
}

fn detail_body() -> AnyElement {
    let detail = state::detail_state();
    let session = state::session();
    let work = detail.get().work.clone();
    let Some(work) = work else {
        return page_scroll(empty_state(
            Icon::Film,
            "没有选中的作品".to_owned(),
            "回到搜索重新选择。".to_owned(),
            Some(button("返回".to_owned(), Btn::Secondary, state::back)),
        ));
    };
    let source = detail.get().source.clone();
    let candidate = detail
        .get()
        .detail
        .clone()
        .or_else(|| work.candidates.iter().find(|c| c.source == source).cloned());
    let Some(candidate) = candidate else {
        return page_scroll(empty_state(
            Icon::Alert,
            "这个片源没有详情".to_owned(),
            "回到搜索换一个来源。".to_owned(),
            Some(button("返回".to_owned(), Btn::Secondary, state::back)),
        ));
    };
    let key = candidate.key();
    let duanju_mode = detail.get().duanju || source_browse::looks_like_duanju(&candidate.kind);
    let is_fav = state::library().get().is_favorite(&key);
    let progress = state::library()
        .get()
        .progress_for(&key)
        .filter(|p| p.episode_index < candidate.episode_count().max(1) && !p.completed())
        .cloned();
    let selected = detail
        .get()
        .selected_episode
        .min(candidate.episode_count().saturating_sub(1));
    let episode_count = candidate.episode_count();
    let detail_error = if detail.get().phase == LoadPhase::Error {
        Some(detail.get().error.clone())
    } else {
        None
    };
    let loading = detail.get().phase == LoadPhase::Loading;
    let demo_mode = session.get().demo();
    page_scroll(
        view()
            .gap(px(18.0))
            .child(
                view()
                    .flex_row()
                    .items_center()
                    .gap(px(9.0))
                    .child(button("返回".to_owned(), Btn::Ghost, state::back))
                    .child(
                        text(work.title.clone())
                            .text_size(px(12.0))
                            .text_color(FAINT)
                            .number_of_lines(1),
                    )
                    .children(demo_mode.then(demo_chip)),
            )
            .child(
                view()
                    .flex_row()
                    .flex_wrap()
                    .gap(px(24.0))
                    .items_start()
                    .child(
                        view()
                            .w(px(210.0))
                            .flex_shrink_0()
                            .child(poster(&work, false)),
                    )
                    .child(
                        view()
                            .flex_1()
                            .min_w(px(360.0))
                            .gap(px(12.0))
                            .child(
                                text(work.title.clone())
                                    .text_size(px(34.0))
                                    .font_weight(FontWeight::BOLD)
                                    .text_color(TEXT)
                                    .line_height(nsh_core::LineHeight::Multiple(1.15)),
                            )
                            .child(
                                view()
                                    .flex_row()
                                    .flex_wrap()
                                    .items_center()
                                    .gap(px(8.0))
                                    .children((!work.year.is_empty()).then(|| chip(
                                        work.year.clone(),
                                        MUTED,
                                        RAISED,
                                    )))
                                    .children(work.genres.iter().take(3).map(|genre| {
                                        chip(genre.clone(), MUTED, RAISED)
                                    }))
                                    .child(chip(
                                        format!("{} 个片源", work.candidates.len()),
                                        MUTED,
                                        RAISED,
                                    )),
                            )
                            .child(
                                text(if candidate.desc.is_empty() {
                                    "这个实例没有返回简介。".to_owned()
                                } else {
                                    candidate.desc.clone()
                                })
                                .text_size(px(12.5))
                                .text_color(MUTED)
                                .line_height(nsh_core::LineHeight::Multiple(1.7))
                                .max_w(px(760.0))
                                .number_of_lines(6),
                            )
                            .children(loading.then(|| {
                                text("正在刷新详情…".to_owned())
                                    .text_size(px(11.0))
                                    .text_color(FAINT)
                            }))
                            .children(detail_error.map(|why| {
                                view()
                                    .p(px(10.0))
                                    .rounded(px(8.0))
                                    .bg(DANGER_TINT)
                                    .border_1()
                                    .border_color(alpha(0xf07b7b, 0.3))
                                    .flex_row()
                                    .items_center()
                                    .gap_3()
                                    .child(
                                        text(why)
                                            .flex_1()
                                            .text_size(px(11.5))
                                            .text_color(DANGER)
                                            .number_of_lines(2),
                                    )
                                    .child(button("重试", Btn::Ghost, state::fetch_detail))
                            }))
                            .children(progress.clone().map(|entry| {
                                view()
                                    .flex_row()
                                    .items_center()
                                    .gap(px(8.0))
                                    .child(
                                        text(format!(
                                            "上次看到 {} · {}",
                                            entry.episode_title,
                                            format_time(entry.position)
                                        ))
                                        .text_size(px(11.5))
                                        .text_color(ACCENT),
                                    )
                                    .child(button("继续这一集", Btn::Ghost, move || {
                                        state::resume_progress(&entry)
                                    }))
                            }))
                            .child(
                                view()
                                    .flex_row()
                                    .items_center()
                                    .gap(px(9.0))
                                    .child(button(
                                        if episode_count == 0 {
                                            "没有可用剧集".to_owned()
                                        } else {
                                            "播放本集".to_owned()
                                        },
                                        Btn::Primary,
                                        move || {
                                            if episode_count > 0 {
                                                state::play_selected(selected);
                                            }
                                        },
                                    ))
                                    .children(duanju_mode.then(|| {
                                        let candidate = candidate.clone();
                                        button("竖屏连播", Btn::Secondary, move || {
                                            if episode_count > 0 {
                                                state::start_playback_portrait(
                                                    &candidate,
                                                    selected,
                                                );
                                            }
                                        })
                                    }))
                                    .child(button(
                                        if is_fav { "已收藏".to_owned() } else { "收藏".to_owned() },
                                        Btn::Secondary,
                                        {
                                            let candidate = candidate.clone();
                                            move || state::toggle_favorite(&candidate)
                                        },
                                    )),
                            ),
                    ),
            )
            .child(divider_line())
            .child(
                view()
                    .gap(px(12.0))
                    .child(section_header(
                        "选集".to_owned(),
                        Some(format!(
                            "当前源：{} · 共 {} 集{}",
                            candidate.source_name,
                            episode_count,
                            if candidate.remarks.is_empty() { String::new() } else { format!(" · {}", candidate.remarks) }
                        )),
                        None,
                    ))
                    .child(source_tabs(&work, &candidate))
                    .child(episode_grid(&candidate, selected, progress.as_ref()))
                    .children((episode_count == 0).then(|| {
                        empty_state(
                            Icon::Alert,
                            "这个片源没有剧集地址".to_owned(),
                            if demo_mode {
                                "演示媒体可能缺失；请确认项目 media 目录存在，或设置 MOONTV_MEDIA_DIR。".to_owned()
                            } else {
                                "实例返回的详情没有播放地址。可以换一个片源，或稍后重试。".to_owned()
                            },
                            Some(button("换一个片源".to_owned(), Btn::Secondary, {
                                let work = work.clone();
                                let current = candidate.source.clone();
                                move || {
                                    if let Some(next) = work.candidates.iter().find(|c| c.source != current) {
                                        state::select_source(next.source.clone());
                                    }
                                }
                            })),
                        )
                    })),
            ),
    )
}

fn divider_line() -> AnyElement {
    view().w_full().h(px(1.0)).bg(LINE).into_element()
}

fn source_tabs(work: &Work, current: &Candidate) -> AnyElement {
    view()
        .gap(px(6.0))
        .child(
            view()
                .flex_row()
                .flex_wrap()
                .gap(px(7.0))
                .children(work.candidates.iter().map(|candidate| {
                    let active = candidate.source == current.source;
                    let source = candidate.source.clone();
                    view()
                        .h(px(32.0))
                        .px(px(11.0))
                        .rounded(px(8.0))
                        .bg(if active { RAISED } else { alpha(0x000000, 0.0) })
                        .border_1()
                        .border_color(if active {
                            alpha(0xe9b872, 0.45)
                        } else {
                            BORDER
                        })
                        .flex_row()
                        .items_center()
                        .gap(px(6.0))
                        .focusable(true)
                        .role(AccessibilityRole::Button)
                        .label(candidate.source_name.clone())
                        .accessibility_state(AccessibilityState {
                            selected: active,
                            ..AccessibilityState::default()
                        })
                        .hover(|s| s.bg(HOVER))
                        .on_press(move || state::select_source(source.clone()))
                        .child(
                            text(format!(
                                "{}{}",
                                candidate.source_name,
                                if active { " · 当前" } else { "" }
                            ))
                            .text_size(px(11.5))
                            .font_weight(if active {
                                FontWeight::SEMIBOLD
                            } else {
                                FontWeight::MEDIUM
                            })
                            .text_color(if active {
                                TEXT
                            } else {
                                MUTED
                            }),
                        )
                        .child(
                            text(format!("{} 集", candidate.episode_count()))
                                .text_size(px(10.0))
                                .text_color(FAINT),
                        )
                        .into_element()
                })),
        )
        .into_element()
}

fn episode_grid(
    candidate: &Candidate,
    selected: usize,
    progress: Option<&ProgressEntry>,
) -> AnyElement {
    if candidate.episodes.is_empty() {
        return view().into_element();
    }
    let count = columns(165.0, 6);
    let candidate = candidate.clone();
    let progress = progress.cloned();
    dynamic(move || {
        view()
            .grid()
            .grid_cols(count.get())
            .gap(px(7.0))
            .children((0..candidate.episodes.len()).map(|index| {
                let active = index == selected;
                let last = progress
                    .as_ref()
                    .is_some_and(|entry| entry.episode_index == index);
                let candidate = candidate.clone();
                let candidate_for_press = candidate.clone();
                view()
                    .h(px(42.0))
                    .px(px(9.0))
                    .rounded(px(8.0))
                    .bg(if active { RAISED } else { SURFACE })
                    .border_1()
                    .border_color(if active { alpha(0xe9b872, 0.5) } else { LINE })
                    .flex_row()
                    .items_center()
                    .gap(px(7.0))
                    .focusable(true)
                    .role(AccessibilityRole::Button)
                    .label(candidate.episode_title(index))
                    .hover(|s| s.bg(HOVER))
                    .on_press(move || state::start_playback(&candidate_for_press, index))
                    .child(
                        text(format!("{:02}", index + 1))
                            .text_size(px(10.0))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(if active { ACCENT } else { FAINT }),
                    )
                    .child(
                        text(candidate.episode_title(index))
                            .flex_1()
                            .text_size(px(11.0))
                            .text_color(if active { TEXT } else { MUTED })
                            .number_of_lines(1),
                    )
                    .children(last.then(|| view().size(px(4.0)).rounded_full().bg(ACCENT)))
                    .into_element()
            }))
            .into_element()
    })
}

// ------------------------------ favorites -------------------------------

pub fn favorites_page() -> AnyElement {
    let session = state::session();
    let demo_mode = session.get().demo();
    let connected = session.get().connected();
    let library = state::library();
    let count = columns(220.0, 6);
    let list = dynamic(move || {
        let lib = library.get();
        if lib.favorites.is_empty() {
            return empty_state(
                Icon::Bookmark,
                "本机片单还是空的".to_owned(),
                if demo_mode {
                    "演示模式只保存本机片单；去搜索演示片单，打开详情后点收藏。".to_owned()
                } else {
                    "在搜索或详情页收藏；真实实例同步状态会逐条标注。".to_owned()
                },
                Some(button(
                    "去搜索".to_owned(),
                    Btn::Primary,
                    state::open_search,
                )),
            );
        }
        let total = computed(move || library.get().favorites.len());
        view()
            .gap(px(14.0))
            .child(
                view()
                    .flex_row()
                    .items_center()
                    .gap(px(8.0))
                    .child(
                        text(move || format!("{} 部作品", total.get()))
                            .text_size(px(12.0))
                            .text_color(MUTED),
                    )
                    .children((!demo_mode && connected).then(|| {
                        text("按每张卡的标注区分已同步与仅本机".to_owned())
                            .text_size(px(10.5))
                            .text_color(FAINT)
                    })),
            )
            .child(view().grid().grid_cols(count.get()).gap(px(16.0)).children(
                lib.favorites.iter().map(|entry| {
                    let work = work_from_favorite(entry);
                    let key = entry.key.clone();
                    let synced = entry.synced;
                    view()
                        .gap(px(7.0))
                        .child(work_card(&work, None))
                        .child(
                            view()
                                .flex_row()
                                .items_center()
                                .justify_between()
                                .gap_2()
                                .child(if demo_mode {
                                    chip("本机 · 演示".to_owned(), ACCENT, DEMO_TINT)
                                } else {
                                    match synced {
                                        Some(true) => {
                                            chip("已同步".to_owned(), SUCCESS, SUCCESS_TINT)
                                        }
                                        Some(false) => {
                                            chip("仅本机".to_owned(), DANGER, DANGER_TINT)
                                        }
                                        None => chip("本机".to_owned(), MUTED, RAISED),
                                    }
                                })
                                .child(button("移除".to_owned(), Btn::Ghost, move || {
                                    state::remove_favorite(&key)
                                })),
                        )
                        .into_element()
                }),
            ))
            .into_element()
    });
    page_scroll(
        view()
            .gap(px(18.0))
            .child(page_head(
                "收藏".to_owned(),
                if demo_mode {
                    "演示资料的本机片单；不会与账号数据混在一起".to_owned()
                } else {
                    "本机片单；连接实例时尝试同步，失败会明确标注".to_owned()
                },
                if connected && !demo_mode {
                    vec![button(
                        "从实例刷新".to_owned(),
                        Btn::Secondary,
                        state::refresh_favorites_from_instance,
                    )]
                } else {
                    vec![demo_badge_if(demo_mode)]
                },
            ))
            .child(list)
            .into_element(),
    )
}

// ------------------------------- history --------------------------------

pub fn history_page() -> AnyElement {
    let library = state::library();
    let session = state::session();
    let confirming = signal(false);
    let demo_mode = session.get().demo();
    let list = dynamic(move || {
        let lib = library.get();
        if lib.progress.is_empty() {
            return empty_state(
                Icon::History,
                "还没有观看历史".to_owned(),
                "播放、暂停、切集和退出时会写入本机进度；这里按最近一次观看排序。".to_owned(),
                Some(button(
                    "去搜索".to_owned(),
                    Btn::Primary,
                    state::open_search,
                )),
            );
        }
        view()
            .gap(px(10.0))
            .children(
                lib.progress
                    .iter()
                    .map(|entry| history_row(entry, demo_mode)),
            )
            .into_element()
    });
    let clear = {
        move || {
            if confirming.get() {
                state::clear_history();
                confirming.set(false);
            } else {
                confirming.set(true);
                state::toast(
                    "再次点击“确认清除”会删除当前资料的历史；收藏不受影响。".to_owned(),
                    ToastTone::Info,
                );
            }
        }
    };
    let clear_label = "清除历史".to_owned();
    page_scroll(
        view()
            .gap(px(16.0))
            .child(page_head(
                "历史".to_owned(),
                if demo_mode {
                    "演示资料的本机记录；账号资料与演示资料分开保存".to_owned()
                } else {
                    "本机优先；连接实例后进度会同步，失败的行标“仅本机”".to_owned()
                },
                vec![button(clear_label, Btn::Ghost, clear)],
            ))
            .child(list)
            .into_element(),
    )
}

fn history_row(entry: &ProgressEntry, demo_mode: bool) -> AnyElement {
    let work = work_from_progress(entry);
    let resume = entry.clone();
    let open = entry.clone();
    view()
        .p(px(12.0))
        .rounded(px(RADIUS_CARD))
        .bg(SURFACE)
        .border_1()
        .border_color(BORDER)
        .flex_row()
        .items_center()
        .gap(px(14.0))
        .child(
            view()
                .w(px(54.0))
                .flex_shrink_0()
                .child(poster_thumb(&work, 54.0)),
        )
        .child(
            view()
                .flex_1()
                .min_w(px(0.0))
                .gap(px(4.0))
                .child(
                    view()
                        .flex_row()
                        .items_center()
                        .gap(px(7.0))
                        .child(
                            text(entry.title.clone())
                                .text_size(px(14.5))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(TEXT)
                                .number_of_lines(1),
                        )
                        .child(
                            text(if demo_mode {
                                "演示".to_owned()
                            } else {
                                entry.source_name.clone()
                            })
                            .text_size(px(10.0))
                            .text_color(FAINT)
                            .number_of_lines(1),
                        )
                        .child(if demo_mode {
                            chip("本机 · 演示".to_owned(), ACCENT, DEMO_TINT)
                        } else {
                            match entry.synced {
                                Some(true) => chip("已同步".to_owned(), SUCCESS, SUCCESS_TINT),
                                Some(false) => chip("仅本机".to_owned(), DANGER, DANGER_TINT),
                                None => chip("本机".to_owned(), MUTED, RAISED),
                            }
                        }),
                )
                .child(
                    text(format!(
                        "{} · {} / {} · {}",
                        entry.episode_title,
                        format_time(entry.position),
                        if entry.duration > 0.0 {
                            format_time(entry.duration)
                        } else {
                            "--:--".to_owned()
                        },
                        relative_time(entry.updated_at)
                    ))
                    .text_size(px(11.0))
                    .text_color(MUTED)
                    .number_of_lines(1),
                )
                .child(progress_bar(entry.percent(), 3.0)),
        )
        .child(
            view()
                .flex_row()
                .gap(px(6.0))
                .child(button(
                    if entry.completed() {
                        "重播".to_owned()
                    } else {
                        "继续".to_owned()
                    },
                    Btn::Secondary,
                    move || state::resume_progress(&resume),
                ))
                .child(button("详情".to_owned(), Btn::Ghost, move || {
                    state::open_detail(work_from_progress(&open), Some(open.source.clone()))
                })),
        )
        .into_element()
}

fn relative_time(ms: u64) -> String {
    let now = nsh_core::time::unix_ms() as u64;
    let delta = now.saturating_sub(ms) / 1000;
    if delta < 90 {
        "刚刚".to_owned()
    } else if delta < 3600 {
        format!("{} 分钟前", delta / 60)
    } else if delta < 86400 {
        format!("{} 小时前", delta / 3600)
    } else if delta < 86400 * 30 {
        format!("{} 天前", delta / 86400)
    } else {
        format!("{} 个月前", delta / (86400 * 30))
    }
}

// ------------------------------- settings -------------------------------

pub fn settings_page() -> AnyElement {
    dynamic(settings_body)
}

fn settings_body() -> AnyElement {
    let session = state::session();
    let library = state::library();
    let demo_mode = session.get().demo();
    page_scroll(
        view()
            .gap(px(18.0))
            .child(page_head(
                "设置".to_owned(),
                "实例、播放与本地数据。密码只用于本次登录，不写入设置。".to_owned(),
                vec![],
            ))
            .child(
                view()
                    .flex_col()
                    .gap(px(18.0))
                    .items_start()
                    .child(view().w_full().gap(px(18.0)).child(connection_section()))
                    .child(view().w_full().gap(px(18.0)).child(playback_section())),
            )
            .child(
                view()
                    .flex_col()
                    .gap(px(18.0))
                    .items_start()
                    .child(view().w_full().gap(px(18.0)).child(data_section()))
                    .child(view().w_full().gap(px(18.0)).child(about_section())),
            )
            .child(
                text(format!(
                    "当前资料 {} · 收藏 {} · 历史 {} · 本机共 {} 个资料",
                    session.get().profile,
                    library.get().favorites.len(),
                    library.get().progress.len(),
                    state::profile_count()
                ))
                .text_size(px(10.0))
                .text_color(FAINT),
            )
            .children((demo_mode).then(|| {
                view().p(px(10.0)).rounded(px(8.0)).bg(DEMO_TINT).child(
                    text("演示模式正在使用本机独立资料；“退出演示”不会删除片单与历史。".to_owned())
                        .text_size(px(11.0))
                        .text_color(ACCENT),
                )
            }))
            .into_element(),
    )
}

fn panel(title: &'static str, subtitle: &'static str, body: impl IntoElement) -> AnyElement {
    view()
        .p(px(18.0))
        .rounded(px(RADIUS_CARD))
        .bg(SURFACE)
        .border_1()
        .border_color(BORDER)
        .gap(px(13.0))
        .child(
            view().gap(px(3.0)).child(h2(title)).child(
                text(subtitle)
                    .text_size(px(11.5))
                    .text_color(MUTED)
                    .line_height(nsh_core::LineHeight::Multiple(1.5)),
            ),
        )
        .child(body)
        .into_element()
}

fn connection_section() -> AnyElement {
    let session = state::session();
    let demo_mode = session.get().demo();
    let connected = session.get().connected();
    let expired = session.get().expired;
    panel(
        "实例与账号",
        "连接现有 Linux / API 实例；不修改服务器，不写入凭据。",
        view()
            .gap(px(12.0))
            .child(
                view()
                    .p(px(11.0))
                    .rounded(px(8.0))
                    .bg(if demo_mode { DEMO_TINT } else { RAISED })
                    .border_1()
                    .border_color(if demo_mode { alpha(0xe9b872, 0.3) } else { BORDER })
                    .gap(px(4.0))
                    .child(
                        text(move || {
                            let state = session.get();
                            if state.demo() {
                                "演示模式：无需实例，数据仅在本机".to_string()
                            } else if state.expired {
                                "会话已过期：请重新登录".to_string()
                            } else if state.connected() {
                                format!("已连接 {} · {}", state.base.trim_end_matches('/'), state.account_label())
                            } else if state.base.is_empty() {
                                "尚未保存实例".to_string()
                            } else {
                                format!("已记住 {}；需要登录", state.base.trim_end_matches('/'))
                            }
                        })
                        .text_size(px(12.0))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(move || {
                            if demo_mode { ACCENT } else if expired { DANGER } else { TEXT }
                        })
                        .number_of_lines(2),
                    )
                    .children((!demo_mode && !connected).then(connect_form)),
            )
            .child(
                view()
                    .flex_row()
                    .flex_wrap()
                    .gap(px(8.0))
                    .child(button(
                        if demo_mode { "退出演示".to_owned() } else { "进入演示模式".to_owned() },
                        Btn::Secondary,
                        move || {
                            if session.get().demo() {
                                state::leave_session();
                            } else {
                                state::enter_demo();
                            }
                        },
                    ))
                    .children((!demo_mode && (!session.get().base.is_empty() || connected)).then(|| {
                        button("断开连接".to_owned(), Btn::Ghost, state::leave_session)
                    })),
            )
            .children((!demo_mode && !connected).then(|| {
                text("实例地址会持久化；会话 token 不持久化，重启后请重新登录。服务端登录协议按只读上游基线核对，真实实例尚未联调。".to_owned())
                    .text_size(px(10.5))
                    .text_color(FAINT)
                    .line_height(nsh_core::LineHeight::Multiple(1.5))
            })),
    )
}

fn playback_section() -> AnyElement {
    let stored = state::settings();
    panel(
        "播放",
        "原生 NSH Video；F / F11 全屏，Esc 退出，空格与左右键在非输入焦点下控制。",
        view()
            .gap(px(12.0))
            .child(
                view()
                    .flex_row()
                    .items_center()
                    .justify_between()
                    .gap_3()
                    .child(
                        view()
                            .flex_1()
                            .gap(px(2.0))
                            .child(
                                text("自动播放下一集".to_owned())
                                    .text_size(px(13.0))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(TEXT),
                            )
                            .child(
                                text("一集结束后 5 秒倒计时，可取消；只在当前会话触发一次".to_owned())
                                    .text_size(px(10.5))
                                    .text_color(MUTED)
                                    .number_of_lines(2),
                            ),
                    )
                    .child(dynamic(move || {
                        switch(stored.get().auto_next, move |on| {
                            state::update_settings(|settings| settings.auto_next = on);
                            state::toast(
                                if on { "已开启自动下一集" } else { "已关闭自动下一集" },
                                ToastTone::Info,
                            );
                        })
                    })),
            )
            .child(divider_line())
            .child(
                view()
                    .flex_row()
                    .flex_wrap()
                    .gap(px(12.0))
                    .child(seconds_field(
                        "片头秒数",
                        stored.get().intro_seconds,
                        "0 = 关闭；片头区间内显示「跳过片头」",
                        move |value| state::update_settings(|settings| settings.intro_seconds = value),
                    ))
                    .child(seconds_field(
                        "片尾秒数",
                        stored.get().outro_seconds,
                        "0 = 关闭；临近片尾显示「下一集」",
                        move |value| state::update_settings(|settings| settings.outro_seconds = value),
                    )),
            )
            .child(
                view()
                    .flex_row()
                    .items_center()
                    .justify_between()
                    .gap_3()
                    .child(
                        view()
                            .flex_1()
                            .gap(px(2.0))
                            .child(
                                text("自动跳过片头".to_owned())
                                    .text_size(px(13.0))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(TEXT),
                            )
                            .child(
                                text("开启后进入片头区间直接 seek；关闭时保留手动按钮".to_owned())
                                    .text_size(px(10.5))
                                    .text_color(MUTED)
                                    .number_of_lines(2),
                            ),
                    )
                    .child(dynamic(move || {
                        switch(stored.get().auto_skip_intro, move |on| {
                            state::update_settings(|settings| settings.auto_skip_intro = on);
                            state::toast(
                                if on { "已开启自动跳过片头" } else { "已关闭自动跳过片头" },
                                ToastTone::Info,
                            );
                        })
                    })),
            )
            .child(divider_line())
            .child(
                view()
                    .gap(px(5.0))
                    .child(field_label("快捷键"))
                    .child(
                        text("Ctrl+K 搜索 · F 播放区全屏 · F11 主区全屏 · Esc 退出全屏 · Space 暂停 · ←/→ 5 秒 · [ / ] 倍速".to_owned())
                            .text_size(px(11.0))
                            .text_color(MUTED)
                            .line_height(nsh_core::LineHeight::Multiple(1.6)),
                    )
                    .child(
                        text("输入框聚焦时，F、空格、方向键和 [ / ] 只编辑文字；这是原生焦点隔离。".to_owned())
                            .text_size(px(10.5))
                            .text_color(FAINT)
                            .line_height(nsh_core::LineHeight::Multiple(1.5)),
                    ),
            ),
    )
}

fn seconds_field(
    label: &'static str,
    initial: f64,
    hint: &'static str,
    set: impl Fn(f64) + 'static,
) -> AnyElement {
    let draft = signal(format!("{:.0}", initial.max(0.0)));
    let set = std::rc::Rc::new(set);
    let commit = {
        let set = set.clone();
        std::rc::Rc::new(move || {
            let value = draft
                .get()
                .trim()
                .parse::<f64>()
                .unwrap_or(0.0)
                .clamp(0.0, 3600.0);
            set(value);
            draft.set(format!("{value:.0}"));
        })
    };
    view()
        .flex_1()
        .min_w(px(180.0))
        .gap(px(5.0))
        .child(field_label(label))
        .child(
            input()
                .value(move || draft.get())
                .placeholder("0")
                .placeholder_color(FAINT)
                .text_color(TEXT)
                .text_size(px(13.0))
                .h(px(36.0))
                .px(px(10.0))
                .rounded(px(8.0))
                .bg(RAISED)
                .border_1()
                .border_color(BORDER)
                .focus(|s| s.border_color(alpha(0xe9b872, 0.65)))
                .auto_correct(false)
                .on_change(move |next: &str| draft.set(next.to_string()))
                .on_submit({
                    let commit = commit.clone();
                    move |_: &str| commit()
                })
                .on_blur({
                    let commit = commit.clone();
                    move |_: &()| commit()
                }),
        )
        .child(
            text(hint)
                .text_size(px(10.0))
                .text_color(FAINT)
                .line_height(nsh_core::LineHeight::Multiple(1.4)),
        )
        .into_element()
}

fn data_section() -> AnyElement {
    let library = state::library();
    let stored = state::settings();
    panel(
        "本机数据",
        "收藏、历史与进度按实例和账号分资料保存；演示资料单独一份。",
        view()
            .gap(px(11.0))
            .child(
                view()
                    .flex_row()
                    .gap(px(9.0))
                    .child(stat_card("收藏", move || state::library().get().favorites.len().to_string()))
                    .child(stat_card("历史", move || state::library().get().progress.len().to_string()))
                    .child(stat_card("自动下一集", move || if stored.get().auto_next { "开".to_owned() } else { "关".to_owned() })),
            )
            .child(
                view()
                    .flex_row()
                    .flex_wrap()
                    .gap(px(8.0))
                    .child(button("清除本机历史", Btn::Ghost, {
                        move || {
                            let has = !library.get().progress.is_empty();
                            if has {
                                state::clear_history();
                            } else {
                                state::toast("当前资料没有历史可清除".to_owned(), ToastTone::Info);
                            }
                        }
                    }))
                    .child(button("清除全部本机资料".to_owned(), Btn::Danger, state::clear_all_local)),
            )
            .child(
                text("收藏与观看记录在已连接实例且存储类型支持同步时写入服务端；失败的行标记“仅本机”，不会冒充云同步。连不上的记录仍留在本机，重登后按时间合并。".to_owned())
                    .text_size(px(10.5))
                    .text_color(FAINT)
                    .line_height(nsh_core::LineHeight::Multiple(1.55)),
            ),
    )
}

fn stat_card(label: &'static str, value: impl Fn() -> String + 'static) -> AnyElement {
    view()
        .flex_1()
        .min_w(px(90.0))
        .p(px(10.0))
        .rounded(px(8.0))
        .bg(RAISED)
        .border_1()
        .border_color(BORDER)
        .gap(px(2.0))
        .child(
            text(value)
                .text_size(px(20.0))
                .font_weight(FontWeight::BOLD)
                .text_color(TEXT),
        )
        .child(text(label).text_size(px(10.0)).text_color(MUTED))
        .into_element()
}

fn about_section() -> AnyElement {
    panel(
        "关于",
        "全新 NSH 原生客户端；不加载网页主界面、Tauri、KMP 或视频 WebView。",
        view()
            .gap(px(7.0))
            .child(info_row("版本", env!("CARGO_PKG_VERSION")))
            .child(info_row("运行时", "NSH 0.6.1 · Rust 1.100.0 · Windows MSVC"))
            .child(info_row("媒体", "Media Foundation + Direct2D · 无 QuickJS/WebView"))
            .child(info_row("验证基线", "Happier-X/MoonTVPlus 1834a818… 只读接口参考"))
            .child(divider_line())
            .child(
                text("未测：真实 Linux/HTTPS 实例、多屏与 100/200% DPI、硬件解码、人工听感与精确音画同步、Turnstile 实例、干净机与签名发布。收藏与播放记录的本机↔实例闭环已在隔离实例上验过，真实部署实例未验。".to_owned())
                    .text_size(px(10.5))
                    .text_color(MUTED)
                    .line_height(nsh_core::LineHeight::Multiple(1.6)),
            )
            .child(
                text("验证入口仍保留：target/debug/nsh-native-video-probe.exe；受控媒体与证据在项目 .runtime 与 docs/validation.md。".to_owned())
                    .text_size(px(10.5))
                    .text_color(FAINT)
                    .line_height(nsh_core::LineHeight::Multiple(1.55)),
            ),
    )
}

fn info_row(label: &'static str, value: &'static str) -> AnyElement {
    view()
        .flex_row()
        .items_center()
        .justify_between()
        .gap_3()
        .child(text(label).text_size(px(11.5)).text_color(MUTED))
        .child(
            text(value)
                .text_size(px(11.5))
                .text_color(TEXT)
                .number_of_lines(2),
        )
        .into_element()
}
