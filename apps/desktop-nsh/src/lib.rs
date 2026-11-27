//! MoonTVPlus, rebuilt as a native NSH desktop app. No web view, Tauri,
//! KMP or copied client code: views and input are NSH, media is the native
//! Windows engine, and the HTTP API is new code over the existing contract.

pub mod api;
pub mod api_transport;
pub mod demo;
pub mod discover;
pub mod logbook;
pub mod media_server;
pub mod model;
pub mod probe;
pub mod relay;
pub mod search_stream;
pub mod state;
pub mod ui;

use nsh_core::prelude::*;

#[component]
pub fn App() -> impl IntoElement {
    state::bootstrap();
    state::install_toast_timer();
    state::install_login_worker();
    ui::shell::Shell::element(())
}
