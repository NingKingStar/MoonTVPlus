//! The app shell: navigation column, search bar and content switch.

use nsh_core::{
    element::{KeyOptions, input},
    keymap::When,
    prelude::*,
};

use crate::{
    state::{self, Route},
    ui::{
        icons::{Icon, icon},
        pages,
        player::PlayerPage,
        theme::*,
        widgets::{demo_chip, toast_overlay},
    },
};

#[component]
pub fn Shell() -> impl IntoElement {
    let search_ref = element_ref();
    on_key_with(
        "ctrl+k",
        KeyOptions {
            when: When::NotEditing,
            ..Default::default()
        },
        move |_| {
            if state::route() == Route::Player {
                state::save_progress(true);
            }
            state::open_search();
            set_timeout(std::time::Duration::from_millis(120), move || {
                search_ref.focus()
            });
        },
    );
    view()
        .flex_1()
        .child(dynamic(move || {
            view()
                .flex_1()
                .child(if state::route() == Route::Player {
                    view()
                        .flex_1()
                        .bg(STAGE)
                        .child(PlayerPage::element(()))
                        .into_element()
                } else {
                    view()
                        .flex_1()
                        .flex_row()
                        .bg(BG)
                        .child(sidebar())
                        .child(
                            view()
                                .flex_1()
                                .min_w(px(0.0))
                                .child(topbar(search_ref))
                                .child(view().flex_1().min_h(px(0.0)).child(dynamic(move || {
                                    match state::route() {
                                        Route::Connect => pages::connect_page(),
                                        Route::Home => pages::home_page(),
                                        Route::Discover => pages::discover_page(),
                                        Route::Sources => pages::source_browse_page(),
                                        Route::Duanju => pages::duanju_page(),
                                        Route::Search => pages::search_page(),
                                        Route::Detail(_) => pages::detail_page(),
                                        Route::Favorites => pages::favorites_page(),
                                        Route::History => pages::history_page(),
                                        Route::Settings => pages::settings_page(),
                                        Route::Player => PlayerPage::element(()).into_element(),
                                    }
                                }))),
                        )
                        .into_element()
                })
                .into_element()
        }))
        .child(toast_overlay())
        .into_element()
}

fn sidebar() -> AnyElement {
    view()
        .w(px(SIDEBAR_W))
        .h_full()
        .bg(SIDEBAR)
        .border_r_1()
        .border_r_color(LINE)
        .flex_col()
        .child(brand())
        .child(
            view()
                .px(px(14.0))
                .pt(px(14.0))
                .gap(px(3.0))
                .child(
                    text("浏览".to_owned())
                        .text_size(px(10.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(FAINT)
                        .letter_spacing(px(1.2))
                        .px(px(10.0))
                        .pb(px(6.0)),
                )
                .child(nav_row("home", "首页", Icon::Home))
                .child(nav_row("discover", "发现", Icon::Grid))
                .child(nav_row("sources", "单源", Icon::Server))
                .child(nav_row("duanju", "短剧", Icon::Film))
                .child(nav_row("favorites", "收藏", Icon::Bookmark))
                .child(nav_row("history", "历史", Icon::History)),
        )
        .child(
            view()
                .px(px(14.0))
                .pt(px(18.0))
                .gap(px(3.0))
                .child(
                    text("系统".to_owned())
                        .text_size(px(10.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(FAINT)
                        .letter_spacing(px(1.2))
                        .px(px(10.0))
                        .pb(px(6.0)),
                )
                .child(nav_row("settings", "设置", Icon::Settings)),
        )
        .child(view().flex_1())
        .child(session_card())
        .into_element()
}

fn brand() -> AnyElement {
    view()
        .flex_row()
        .items_center()
        .gap(px(10.0))
        .px(px(18.0))
        .pt(px(22.0))
        .pb(px(22.0))
        .child(
            view()
                .size(px(34.0))
                .rounded(px(10.0))
                .bg_linear_to_br()
                .gradient_from(rgb(0xf0c583))
                .gradient_to(rgb(0xb77b35))
                .items_center()
                .justify_center()
                .child(icon(Icon::Film, 19.0, || rgb(0x1b1307))),
        )
        .child(
            view()
                .gap(px(1.0))
                .child(
                    text("MoonTVPlus")
                        .text_size(px(15.0))
                        .font_weight(FontWeight::BOLD)
                        .text_color(TEXT)
                        .letter_spacing(px(0.2)),
                )
                .child(
                    text(format!("NSH 原生 · {}", env!("CARGO_PKG_VERSION")))
                        .text_size(px(10.5))
                        .text_color(FAINT)
                        .letter_spacing(px(0.4)),
                ),
        )
        .into_element()
}

fn nav_row(key: &'static str, label: &'static str, icon_name: Icon) -> AnyElement {
    let active = move || state::route().nav_key() == Some(key);
    view()
        .h(px(38.0))
        .px(px(10.0))
        .rounded(px(9.0))
        .flex_row()
        .relative()
        .items_center()
        .gap(px(10.0))
        .focusable(true)
        .role(AccessibilityRole::Button)
        .label(label)
        .accessibility_state(move || AccessibilityState {
            selected: active(),
            ..AccessibilityState::default()
        })
        .style_when(active, |s| s.bg(RAISED))
        .hover(|s| s.bg(HOVER))
        .active(|s| s.opacity(0.8))
        .on_press(move || match key {
            "discover" => state::open_discover(),
            "sources" => state::open_sources(),
            "duanju" => state::open_duanju(),
            _ => state::go(route_for_key(key)),
        })
        .child(icon(
            icon_name,
            17.0,
            move || if active() { ACCENT } else { MUTED },
        ))
        .child(
            text(label)
                .text_size(px(13.5))
                .font_weight(FontWeight::MEDIUM)
                .style_when(active, |s| s.font_weight(FontWeight::SEMIBOLD))
                .text_color(move || if active() { TEXT } else { MUTED }),
        )
        .children(if key == "favorites" {
            Some(counter(move || state::library().get().favorites.len()))
        } else if key == "history" {
            Some(counter(move || state::library().get().progress.len()))
        } else {
            None
        })
        .child(
            view()
                .absolute()
                .left_0()
                .top(px(10.0))
                .w(px(2.5))
                .h(px(18.0))
                .rounded(px(1.5))
                .bg(ACCENT)
                .opacity(move || if active() { 1.0_f32 } else { 0.0_f32 }),
        )
        .into_element()
}

fn route_for_key(key: &str) -> Route {
    match key {
        "home" => Route::Home,
        "discover" => Route::Discover,
        "sources" => Route::Sources,
        "duanju" => Route::Duanju,
        "favorites" => Route::Favorites,
        "history" => Route::History,
        _ => Route::Settings,
    }
}

fn counter(read: impl Fn() -> usize + 'static) -> AnyElement {
    view()
        .flex_1()
        .items_end()
        .child(
            text(move || {
                let count = read();
                if count == 0 {
                    String::new()
                } else {
                    count.to_string()
                }
            })
            .text_size(px(10.5))
            .text_color(FAINT),
        )
        .into_element()
}

fn session_card() -> AnyElement {
    let session = state::session();
    view()
        .m(px(12.0))
        .p(px(12.0))
        .rounded(px(11.0))
        .bg(RAISED)
        .border_1()
        .border_color(BORDER)
        .gap(px(6.0))
        .focusable(true)
        .role(AccessibilityRole::Button)
        .label("连接与实例设置".to_owned())
        .hover(|s| s.bg(HOVER))
        .on_press(move || state::go(Route::Settings))
        .child(
            view()
                .flex_row()
                .items_center()
                .gap(px(7.0))
                .child(status_dot())
                .child(
                    text(move || session.get().instance_label())
                        .text_size(px(11.5))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(TEXT)
                        .number_of_lines(1),
                ),
        )
        .child(
            text(move || session.get().status_text())
                .text_size(px(10.5))
                .text_color(MUTED)
                .line_height(nsh_core::LineHeight::Multiple(1.4))
                .number_of_lines(2),
        )
        .child(when(move || session.get().demo(), demo_chip))
        .into_element()
}

fn status_dot() -> AnyElement {
    let session = state::session();
    view()
        .size(px(7.0))
        .rounded_full()
        .bg(move || {
            let state = session.get();
            if state.demo() {
                ACCENT
            } else if state.expired {
                DANGER
            } else if state.connected() {
                SUCCESS
            } else {
                FAINT
            }
        })
        .into_element()
}

fn topbar(search_ref: ElementRef) -> AnyElement {
    effect(move || {
        if state::ui().get().search_focus {
            search_ref.focus();
            state::take_search_focus();
        }
    });
    let route_label = computed(|| state::route().label());
    view()
        .h(px(TOPBAR_H))
        .px(px(26.0))
        .flex_row()
        .items_center()
        .gap(px(18.0))
        .border_b_1()
        .border_b_color(LINE)
        .child(
            view()
                .flex_1()
                .min_w(px(120.0))
                .gap(px(1.0))
                .child(
                    text(move || route_label.get())
                        .text_size(px(15.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(TEXT),
                )
                .child(
                    text(move || state::session().get().account_label())
                        .text_size(px(10.5))
                        .text_color(FAINT),
                ),
        )
        .child(search_bar(search_ref))
        .child(status_button())
        .into_element()
}

fn search_bar(search_ref: ElementRef) -> AnyElement {
    let search = state::search_state();
    view()
        .w(px(390.0))
        .flex_shrink(1.0)
        .min_w(px(160.0))
        .h(px(36.0))
        .px(px(10.0))
        .flex_row()
        .items_center()
        .gap(px(7.0))
        .rounded(px(9.0))
        .bg(SURFACE)
        .border_1()
        .border_color(BORDER)
        .focus_within(|s| s.border_color(alpha(0xe9b872, 0.55)))
        .child(icon(Icon::Search, 15.0, || FAINT))
        .child(
            input()
                .element_ref(&search_ref)
                .flex_1()
                .h(px(32.0))
                .bg(alpha(0x000000, 0.0))
                .text_color(TEXT)
                .placeholder("搜索片名、类型或片源  ·  Ctrl+K".to_owned())
                .placeholder_color(FAINT)
                .text_size(px(12.5))
                .value(move || search.get().draft)
                .on_change(move |value: &str| state::set_draft(value))
                .on_submit(move |_: &str| state::run_search()),
        )
        .child(when(
            move || !search.get().draft.is_empty(),
            move || {
                view()
                    .size(px(20.0))
                    .rounded_full()
                    .items_center()
                    .justify_center()
                    .hover(|s| s.bg(HOVER))
                    .on_press(move || {
                        state::set_draft("");
                    })
                    .child(icon(Icon::Close, 12.0, || MUTED))
            },
        ))
        .into_element()
}

fn status_button() -> AnyElement {
    let session = state::session();
    view()
        .h(px(30.0))
        .px(px(10.0))
        .rounded_full()
        .bg(SURFACE)
        .border_1()
        .border_color(BORDER)
        .flex_row()
        .items_center()
        .gap(px(7.0))
        .focusable(true)
        .role(AccessibilityRole::Button)
        .label("连接状态".to_owned())
        .hover(|s| s.bg(HOVER))
        .on_press(move || state::go(Route::Settings))
        .child(view().size(px(6.0)).rounded_full().bg(move || {
            let state = session.get();
            if state.demo() {
                ACCENT
            } else if state.connected() {
                SUCCESS
            } else if state.expired {
                DANGER
            } else {
                FAINT
            }
        }))
        .child(
            text(move || {
                let state = session.get();
                if state.demo() {
                    "演示".to_owned()
                } else if state.expired {
                    "会话过期".to_owned()
                } else if state.connected() {
                    "已连接".to_owned()
                } else {
                    "未连接".to_owned()
                }
            })
            .text_size(px(10.5))
            .font_weight(FontWeight::MEDIUM)
            .text_color(MUTED),
        )
        .into_element()
}
