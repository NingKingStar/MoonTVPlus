#![windows_subsystem = "windows"]

use std::path::PathBuf;

/// Where settings, the local library and caches live.
///
/// Without an explicit directory the runner has no app bundle to take a name
/// from, so storage would stay in memory and every restart would lose the
/// local favorites/history. `MOONTV_DATA_DIR` keeps side-by-side test runs
/// apart; otherwise this is the user's roaming app data.
fn data_dir() -> Option<PathBuf> {
    if let Some(dir) = std::env::var_os("MOONTV_DATA_DIR").filter(|dir| !dir.is_empty()) {
        return Some(PathBuf::from(dir));
    }
    std::env::var_os("APPDATA")
        .or_else(|| std::env::var_os("LOCALAPPDATA"))
        .map(|base| PathBuf::from(base).join("MoonTVPlus").join("nsh"))
}

fn main() -> impl std::process::Termination {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    moontv_native::media_server::start();
    moontv_native::relay::start();
    nsh_platform_windows::run_app(
        nsh_platform_windows::AppSource::Rust(
            nsh_platform_windows::Root::of::<moontv_native::App>(),
        ),
        nsh_platform_windows::AppOptions {
            title: "MoonTVPlus".to_owned(),
            app_id: "org.moontvplus.desktop-nsh.p0".to_owned(),
            size: (1440, 900),
            appearance: nsh_core::platform::Appearance::Dark,
            data_dir: data_dir(),
            ..Default::default()
        },
    )
}
