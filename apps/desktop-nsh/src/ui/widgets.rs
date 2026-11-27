//! Shared product widgets: buttons, chips, posters, cards, empty states,
//! toasts. They are native views, not a web component set.

use nsh_core::{element::Rgba, geometry::px, prelude::*};

use crate::{
    model::{Work, percent_label},
    state::{self, ToastTone},
    ui::{
        icons::{Icon, icon},
        theme::*,
    },
};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Btn {
    Primary,
    Secondary,
    Ghost,
    Danger,
}

pub fn button(
    label: impl Into<String> + 'static,
    kind: Btn,
    on_press: impl Fn() + 'static,
) -> AnyElement {
    let label = label.into();
    let (bg, fg, border) = match kind {
        Btn::Primary => (ACCENT, ON_ACCENT, ACCENT),
        Btn::Secondary => (RAISED, TEXT, BORDER),
        Btn::Ghost => (alpha(0x000000, 0.0), MUTED, alpha(0x000000, 0.0)),
        Btn::Danger => (DANGER_TINT, DANGER, alpha(0xf07b7b, 0.35)),
    };
    view()
        .h(px(36.0))
        .px(px(16.0))
        .rounded(px(RADIUS_CONTROL))
        .bg(bg)
        .border_1()
        .border_color(border)
        .flex_row()
        .items_center()
        .justify_center()
        .gap(px(7.0))
        .focusable(true)
        .role(AccessibilityRole::Button)
        .label(label.clone())
        .self_start()
        .hover(move |s| {
            s.bg(match kind {
                Btn::Primary => ACCENT_BRIGHT,
                Btn::Secondary | Btn::Ghost => HOVER,
                Btn::Danger => alpha(0xf07b7b, 0.18),
            })
        })
        .active(|s| s.opacity(0.82))
        .on_press(on_press)
        .child(
            text(label)
                .text_size(px(13.5))
                .font_weight(FontWeight::MEDIUM)
                .text_color(fg)
                .number_of_lines(1),
        )
        .into_element()
}

pub fn icon_button(
    icon_name: Icon,
    label: &'static str,
    on_press: impl Fn() + 'static,
) -> AnyElement {
    view()
        .size(px(34.0))
        .rounded(px(RADIUS_CONTROL))
        .items_center()
        .justify_center()
        .focusable(true)
        .role(AccessibilityRole::Button)
        .label(label)
        .hover(|s| s.bg(HOVER))
        .active(|s| s.opacity(0.75))
        .on_press(on_press)
        .child(icon(icon_name, 18.0, || MUTED))
        .into_element()
}

pub fn chip(label: impl Into<String> + 'static, fg: Rgba, bg: Rgba) -> AnyElement {
    let label = label.into();
    view()
        .h(px(22.0))
        .px(px(9.0))
        .rounded_full()
        .bg(bg)
        .flex_row()
        .items_center()
        .child(
            text(label)
                .text_size(px(11.0))
                .font_weight(FontWeight::MEDIUM)
                .text_color(fg)
                .number_of_lines(1),
        )
        .into_element()
}

pub fn demo_chip() -> AnyElement {
    view()
        .flex_row()
        .items_center()
        .gap(px(6.0))
        .child(view().size(px(6.0)).rounded_full().bg(ACCENT))
        .child(
            text("演示数据".to_owned())
                .text_size(px(11.0))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(ACCENT),
        )
        .into_element()
}

/// A title card stands in for a poster: typography and geometry, not a
/// fabricated photograph. A real poster URL, when the API has one, covers it.
pub fn poster(work: &Work, compact: bool) -> AnyElement {
    let (top, bottom, ink) = poster_colors(&work.title);
    let title_size = 18.0;
    let image_src = work.poster.clone();
    let failed = signal(false);
    let has_image = !image_src.is_empty();
    view()
        .w_full()
        .aspect_ratio(2.0 / 3.0)
        .rounded(px(RADIUS_CARD))
        .overflow_hidden()
        .bg_linear_to_b()
        .gradient_from(top)
        .gradient_to(bottom)
        .border_1()
        .border_color(alpha(0xffffff, 0.06))
        .child(
            // A large, quiet geometric mark: a film ring, off the corner.
            view()
                .absolute()
                .right(px(-34.0))
                .top(px(-26.0))
                .size(px(150.0))
                .rounded_full()
                .border_1()
                .border_color(alpha(0xffffff, 0.07)),
        )
        .child(
            view()
                .absolute()
                .right(px(24.0))
                .top(px(46.0))
                .size(px(64.0))
                .rounded_full()
                .bg(alpha(0x000000, 0.16)),
        )
        .child(
            view()
                .absolute()
                .left(px(14.0))
                .right(px(14.0))
                .top(px(13.0))
                .flex_row()
                .items_center()
                .justify_between()
                .child(
                    text(if work.year.is_empty() {
                        "片单".to_owned()
                    } else {
                        work.year.clone()
                    })
                    .text_size(px(10.5))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(alpha(0xffffff, 0.55))
                    .letter_spacing(px(1.2)),
                )
                .child(
                    text("MOONTV")
                        .text_size(px(9.0))
                        .font_weight(FontWeight::BOLD)
                        .text_color(alpha(0xffffff, 0.35))
                        .letter_spacing(px(1.4)),
                ),
        )
        .child(
            view()
                .absolute()
                .left(px(0.0))
                .right(px(0.0))
                .bottom(px(0.0))
                .h(px(if compact { 110.0 } else { 150.0 }))
                .bg_linear_to_b()
                .gradient_from(alpha(0x000000, 0.0))
                .gradient_to(alpha(0x000000, 0.72)),
        )
        .child(
            view()
                .absolute()
                .left(px(14.0))
                .right(px(14.0))
                .bottom(px(16.0))
                .gap(px(5.0))
                .child(
                    text(
                        work.genres
                            .first()
                            .cloned()
                            .unwrap_or_else(|| "原生播放".to_owned()),
                    )
                    .text_size(px(9.5))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(ink)
                    .number_of_lines(1)
                    .letter_spacing(px(0.6)),
                )
                .child(
                    text(work.title.clone())
                        .text_size(px(title_size))
                        .font_weight(FontWeight::BOLD)
                        .text_color(alpha(0xffffff, 0.96))
                        .line_height(nsh_core::LineHeight::Multiple(1.12))
                        .number_of_lines(2),
                ),
        )
        .children(has_image.then(|| {
            when(
                move || !failed.get(),
                move || {
                    image(image_src.clone())
                        .size_full()
                        .object_fit(ResizeMode::Cover)
                        .on_error(move |_: &str| failed.set(true))
                },
            )
        }))
        .into_element()
}

pub fn work_card(work: &Work, progress: Option<f32>) -> AnyElement {
    let work_for_press = work.clone();
    let sources = work.candidates.len();
    let source_label = work
        .candidates
        .first()
        .map(|c| c.source_name.clone())
        .unwrap_or_default();
    let meta = if work.year.is_empty() {
        format!("{sources} 个片源")
    } else if sources > 1 {
        format!("{} · {} 个片源", work.year, sources)
    } else {
        format!("{} · {}", work.year, source_label)
    };
    view()
        .gap(px(8.0))
        .rounded(px(RADIUS_CARD))
        .focusable(true)
        .role(AccessibilityRole::Button)
        .label(work.title.clone())
        .hover(|s| s.opacity(0.94))
        .active(|s| s.opacity(0.82))
        .on_press(move || state::open_detail(work_for_press.clone(), None))
        .child(poster(work, true))
        .child(
            view()
                .px(px(2.0))
                .gap(px(3.0))
                .child(
                    text(work.title.clone())
                        .text_size(px(14.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(TEXT)
                        .number_of_lines(1),
                )
                .child(
                    text(meta)
                        .text_size(px(11.5))
                        .text_color(MUTED)
                        .number_of_lines(1),
                )
                .children(progress.map(|percent| {
                    view()
                        .pt(px(3.0))
                        .gap(px(3.0))
                        .child(progress_bar(percent, 3.0))
                        .child(
                            text(percent_label(percent))
                                .text_size(px(10.0))
                                .text_color(FAINT),
                        )
                })),
        )
        .into_element()
}

pub fn progress_bar(percent: f32, height: f32) -> AnyElement {
    view()
        .w_full()
        .h(px(height))
        .rounded_full()
        .bg(alpha(0xffffff, 0.10))
        .overflow_hidden()
        .child(
            view()
                .h_full()
                .w(percentage(percent.clamp(0.0, 1.0)))
                .self_start()
                .flex_none()
                .rounded_full()
                .bg(ACCENT),
        )
        .into_element()
}

/// A small monogram tile for dense rows (history), where the full title
/// card would be unreadable.
pub fn poster_thumb(work: &Work, width: f32) -> AnyElement {
    let (top, bottom, _) = poster_colors(&work.title);
    let initial = work.title.chars().next().unwrap_or('片').to_string();
    view()
        .w(px(width))
        .aspect_ratio(2.0 / 3.0)
        .rounded(px(8.0))
        .overflow_hidden()
        .bg_linear_to_b()
        .gradient_from(top)
        .gradient_to(bottom)
        .items_center()
        .justify_center()
        .child(
            text(initial)
                .text_size(px(width * 0.40))
                .font_weight(FontWeight::BOLD)
                .text_color(alpha(0xffffff, 0.82)),
        )
        .into_element()
}

pub fn section_header(
    title: impl Into<String>,
    subtitle: Option<String>,
    action: Option<AnyElement>,
) -> AnyElement {
    let title = title.into();
    view()
        .flex_row()
        .items_end()
        .justify_between()
        .gap_3()
        .child(
            view()
                .flex_1()
                .gap(px(3.0))
                .child(h2(title))
                .children(subtitle.map(|subtitle| {
                    text(subtitle)
                        .text_size(px(12.0))
                        .text_color(MUTED)
                        .number_of_lines(1)
                })),
        )
        .children(action)
        .into_element()
}

pub fn empty_state(
    icon_name: Icon,
    title: impl Into<String>,
    body: impl Into<String>,
    action: Option<AnyElement>,
) -> AnyElement {
    let title = title.into();
    let body = body.into();
    view()
        .w_full()
        .py(px(30.0))
        .px(px(24.0))
        .items_center()
        .gap(px(10.0))
        .child(
            view()
                .size(px(52.0))
                .rounded(px(15.0))
                .bg(RAISED)
                .border_1()
                .border_color(BORDER)
                .items_center()
                .justify_center()
                .child(icon(icon_name, 22.0, || ACCENT)),
        )
        .child(
            text(title)
                .text_size(px(17.0))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(TEXT)
                .text_center(),
        )
        .child(
            text(body)
                .text_size(px(12.5))
                .text_color(MUTED)
                .line_height(nsh_core::LineHeight::Multiple(1.55))
                .text_center()
                .max_w(px(420.0)),
        )
        .children(action)
        .into_element()
}

pub fn field_label(label: impl Into<String>) -> AnyElement {
    let label = label.into();
    text(label)
        .text_size(px(11.5))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(MUTED)
        .into_element()
}

pub fn switch(on: bool, on_change: impl Fn(bool) + 'static) -> AnyElement {
    let next = !on;
    view()
        .w(px(44.0))
        .h(px(24.0))
        .p(px(3.0))
        .rounded_full()
        .bg(if on { ACCENT } else { RAISED })
        .border_1()
        .border_color(if on { ACCENT } else { BORDER })
        .flex_row()
        .items_center()
        .justify_start()
        .when(on, |s| s.justify_end())
        .focusable(true)
        .role(AccessibilityRole::Switch)
        .label(if on {
            "已开启".to_owned()
        } else {
            "已关闭".to_owned()
        })
        .on_press(move || on_change(next))
        .child(
            view()
                .size(px(16.0))
                .rounded_full()
                .bg(if on { ON_ACCENT } else { FAINT }),
        )
        .into_element()
}

pub fn divider() -> AnyElement {
    view().w_full().h(px(1.0)).bg(LINE).into_element()
}

pub fn toast_overlay() -> AnyElement {
    dynamic(move || {
        let Some(toast) = state::ui().get().toast else {
            return view().into_element();
        };
        let (fg, bg, border) = match toast.tone {
            ToastTone::Info => (INFO, alpha(0x86b4ff, 0.14), alpha(0x86b4ff, 0.3)),
            ToastTone::Success => (SUCCESS, SUCCESS_TINT, alpha(0x62d6a3, 0.32)),
            ToastTone::Error => (DANGER, DANGER_TINT, alpha(0xf07b7b, 0.32)),
            ToastTone::Demo => (ACCENT, DEMO_TINT, alpha(0xe9b872, 0.32)),
        };
        view()
            .absolute()
            .bottom(px(22.0))
            .right(px(22.0))
            .max_w(px(380.0))
            .px(px(14.0))
            .py(px(11.0))
            .rounded(px(10.0))
            .bg(bg)
            .border_1()
            .border_color(border)
            .z_index(60)
            .focusable(true)
            .role(AccessibilityRole::Button)
            .label(toast.text.clone())
            .on_press(state::clear_toast)
            .child(
                text(toast.text.clone())
                    .text_size(px(12.5))
                    .text_color(fg)
                    .line_height(nsh_core::LineHeight::Multiple(1.45))
                    .number_of_lines(3),
            )
            .into_element()
    })
}

pub fn time_text(seconds: f64) -> String {
    crate::model::format_time(seconds)
}
