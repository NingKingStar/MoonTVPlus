//! Line icons drawn in a 20 × 20 box with the tint as stroke. Canvas
//! drawing keeps them crisp at any DPI and avoids shipping image sets.

use std::f32::consts::{PI, TAU};

use nsh_core::{
    canvas::{LineCap, LineJoin},
    element::{Painter, Rgba},
    geometry::{bounds, point, px, size},
    prelude::*,
};

/// The glyphs the product uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Icon {
    Home,
    Grid,
    Bookmark,
    History,
    Settings,
    Search,
    ChevronLeft,
    ChevronRight,
    ChevronDown,
    Play,
    Pause,
    SkipBack,
    SkipForward,
    Volume,
    Mute,
    Panel,
    Fullscreen,
    Refresh,
    Plus,
    Check,
    Close,
    Trash,
    Link,
    Film,
    Server,
    Alert,
    Info,
    Replay,
}

fn line(p: Painter, points: &[f32]) -> Painter {
    let mut p = p.begin_path().move_to(points[0], points[1]);
    for xy in points[2..].chunks(2) {
        p = p.line_to(xy[0], xy[1]);
    }
    p.stroke()
}

fn rect(p: Painter, x: f32, y: f32, w: f32, h: f32, r: f32) -> Painter {
    p.begin_path()
        .round_rect(bounds(point(px(x), px(y)), size(px(w), px(h))), px(r))
        .stroke()
}

fn circle(p: Painter, x: f32, y: f32, r: f32, fill: bool) -> Painter {
    let p = p.begin_path().arc(x, y, r, 0.0, TAU);
    if fill { p.fill() } else { p.stroke() }
}

impl Icon {
    fn draw(self, p: Painter) -> Painter {
        match self {
            Icon::Home => {
                let p = line(p, &[3.5, 9.0, 10.0, 3.5, 16.5, 9.0]);
                let p = line(p, &[5.5, 8.5, 5.5, 16.5, 14.5, 16.5, 14.5, 8.5]);
                line(p, &[8.5, 16.5, 8.5, 12.0, 11.5, 12.0, 11.5, 16.5])
            }
            Icon::Grid => {
                let p = rect(p, 3.5, 3.5, 5.5, 5.5, 1.2);
                let p = rect(p, 11.0, 3.5, 5.5, 5.5, 1.2);
                let p = rect(p, 3.5, 11.0, 5.5, 5.5, 1.2);
                rect(p, 11.0, 11.0, 5.5, 5.5, 1.2)
            }
            Icon::Bookmark => {
                let p = line(
                    p,
                    &[
                        5.5, 3.5, 14.5, 3.5, 14.5, 16.5, 10.0, 13.0, 5.5, 16.5, 5.5, 3.5,
                    ],
                );
                line(p, &[8.0, 7.0, 12.0, 7.0])
            }
            Icon::History => {
                let p = line(
                    p,
                    &[
                        3.6, 10.0, 6.4, 6.4, 10.0, 5.2, 13.6, 6.4, 16.4, 10.0, 15.6, 14.2, 12.5,
                        16.2, 8.4, 15.9, 5.6, 13.2, 5.2, 9.7,
                    ],
                );
                let p = line(p, &[3.6, 5.6, 3.6, 10.0, 8.0, 10.0]);
                line(p, &[10.0, 8.0, 10.0, 10.4, 12.2, 11.6])
            }
            Icon::Settings => {
                let p = line(p, &[3.0, 5.5, 17.0, 5.5]);
                let p = circle(p, 7.0, 5.5, 2.0, false);
                let p = line(p, &[3.0, 14.5, 17.0, 14.5]);
                let p = circle(p, 13.0, 14.5, 2.0, false);
                let p = line(p, &[3.0, 10.0, 17.0, 10.0]);
                circle(p, 9.5, 10.0, 2.0, false)
            }
            Icon::Search => {
                let p = circle(p, 8.8, 8.8, 4.6, false);
                line(p, &[12.2, 12.2, 16.5, 16.5])
            }
            Icon::ChevronLeft => line(p, &[12.0, 4.5, 6.5, 10.0, 12.0, 15.5]),
            Icon::ChevronRight => line(p, &[8.0, 4.5, 13.5, 10.0, 8.0, 15.5]),
            Icon::ChevronDown => line(p, &[4.5, 8.0, 10.0, 13.5, 15.5, 8.0]),
            Icon::Play => p
                .begin_path()
                .move_to(6.5, 4.0)
                .line_to(16.0, 10.0)
                .line_to(6.5, 16.0)
                .close_path()
                .fill(),
            Icon::Pause => {
                let p = p
                    .begin_path()
                    .round_rect(
                        bounds(point(px(6.0), px(4.0)), size(px(3.0), px(12.0))),
                        px(1.0),
                    )
                    .fill();
                p.begin_path()
                    .round_rect(
                        bounds(point(px(11.0), px(4.0)), size(px(3.0), px(12.0))),
                        px(1.0),
                    )
                    .fill()
            }
            Icon::SkipBack => {
                let p = p
                    .begin_path()
                    .move_to(15.5, 5.0)
                    .line_to(8.0, 10.0)
                    .line_to(15.5, 15.0)
                    .close_path()
                    .fill();
                line(p, &[4.5, 5.0, 4.5, 15.0])
            }
            Icon::SkipForward => {
                let p = p
                    .begin_path()
                    .move_to(4.5, 5.0)
                    .line_to(12.0, 10.0)
                    .line_to(4.5, 15.0)
                    .close_path()
                    .fill();
                line(p, &[15.5, 5.0, 15.5, 15.0])
            }
            Icon::Volume => {
                let p = p
                    .begin_path()
                    .move_to(3.0, 7.5)
                    .line_to(6.0, 7.5)
                    .line_to(9.5, 4.5)
                    .line_to(9.5, 15.5)
                    .line_to(6.0, 12.5)
                    .line_to(3.0, 12.5)
                    .close_path()
                    .fill();
                let p = p.begin_path().arc(9.5, 10.0, 4.0, -0.9, 0.9).stroke();
                p.begin_path().arc(9.5, 10.0, 7.0, -0.9, 0.9).stroke()
            }
            Icon::Mute => {
                let p = p
                    .begin_path()
                    .move_to(3.0, 7.5)
                    .line_to(6.0, 7.5)
                    .line_to(9.5, 4.5)
                    .line_to(9.5, 15.5)
                    .line_to(6.0, 12.5)
                    .line_to(3.0, 12.5)
                    .close_path()
                    .fill();
                let p = line(p, &[12.5, 8.0, 16.5, 12.0]);
                line(p, &[16.5, 8.0, 12.5, 12.0])
            }
            Icon::Panel => {
                let p = rect(p, 3.0, 4.5, 14.0, 11.0, 2.0);
                line(p, &[12.0, 4.5, 12.0, 15.5])
            }
            Icon::Fullscreen => {
                let p = line(p, &[3.5, 8.0, 3.5, 3.5, 8.0, 3.5]);
                let p = line(p, &[12.0, 3.5, 16.5, 3.5, 16.5, 8.0]);
                let p = line(p, &[16.5, 12.0, 16.5, 16.5, 12.0, 16.5]);
                line(p, &[8.0, 16.5, 3.5, 16.5, 3.5, 12.0])
            }
            Icon::Refresh => {
                let p = p
                    .begin_path()
                    .arc(10.0, 10.0, 6.2, -PI * 0.15, PI * 1.25)
                    .stroke();
                line(p, &[15.2, 3.8, 16.4, 7.6, 12.6, 8.2])
            }
            Icon::Plus => {
                let p = line(p, &[10.0, 4.0, 10.0, 16.0]);
                line(p, &[4.0, 10.0, 16.0, 10.0])
            }
            Icon::Check => line(p, &[4.5, 10.5, 8.4, 14.0, 15.5, 6.2]),
            Icon::Close => {
                let p = line(p, &[5.0, 5.0, 15.0, 15.0]);
                line(p, &[15.0, 5.0, 5.0, 15.0])
            }
            Icon::Trash => {
                let p = line(p, &[4.0, 6.0, 16.0, 6.0]);
                let p = line(p, &[8.0, 3.5, 12.0, 3.5]);
                let p = line(p, &[5.5, 6.0, 6.5, 16.5, 13.5, 16.5, 14.5, 6.0]);
                line(p, &[8.5, 9.0, 8.9, 13.5])
            }
            Icon::Link => {
                let p = line(p, &[8.2, 11.8, 11.8, 8.2]);
                let p = line(p, &[7.4, 6.2, 5.4, 8.2, 5.4, 11.0, 7.8, 13.4]);
                line(p, &[12.6, 13.8, 14.6, 11.8, 14.6, 9.0, 12.2, 6.6])
            }
            Icon::Film => {
                let p = rect(p, 3.0, 4.0, 14.0, 12.0, 2.0);
                let p = line(p, &[7.0, 4.0, 7.0, 16.0]);
                let p = line(p, &[13.0, 4.0, 13.0, 16.0]);
                let p = line(p, &[3.0, 8.0, 7.0, 8.0]);
                let p = line(p, &[3.0, 12.0, 7.0, 12.0]);
                let p = line(p, &[13.0, 8.0, 17.0, 8.0]);
                line(p, &[13.0, 12.0, 17.0, 12.0])
            }
            Icon::Server => {
                let p = rect(p, 3.0, 4.0, 14.0, 5.0, 1.5);
                let p = rect(p, 3.0, 11.0, 14.0, 5.0, 1.5);
                let p = circle(p, 6.0, 6.5, 0.8, true);
                circle(p, 6.0, 13.5, 0.8, true)
            }
            Icon::Alert => {
                let p = line(p, &[10.0, 3.5, 17.0, 16.0, 3.0, 16.0, 10.0, 3.5]);
                let p = line(p, &[10.0, 7.5, 10.0, 11.5]);
                circle(p, 10.0, 14.0, 0.8, true)
            }
            Icon::Info => {
                let p = circle(p, 10.0, 10.0, 6.8, false);
                let p = line(p, &[10.0, 9.0, 10.0, 13.5]);
                circle(p, 10.0, 6.4, 0.8, true)
            }
            Icon::Replay => {
                let p = p
                    .begin_path()
                    .arc(10.0, 10.0, 6.0, PI * 1.1, PI * 2.2)
                    .stroke();
                line(p, &[4.0, 7.0, 3.4, 11.0, 7.4, 11.4])
            }
        }
    }
}

/// A glyph at `size` points, tinted by `tint` (read reactively).
pub fn icon(name: Icon, size: f32, tint: impl Fn() -> Rgba + 'static) -> AnyElement {
    view()
        .size(px(size))
        .pointer_events(PointerEvents::None)
        .child(
            canvas(move || {
                let p = Painter::new()
                    .scale(size / 20.0, size / 20.0)
                    .stroke_color(color(tint()))
                    .fill_color(color(tint()))
                    .line_width(1.55)
                    .line_cap(LineCap::Round)
                    .line_join(LineJoin::Round);
                name.draw(p)
            })
            .size_full(),
        )
        .into_element()
}
