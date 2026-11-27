//! Application state: session, navigation, search, detail, player and the
//! persisted library. Signals are the single source of truth; pages read
//! them and handlers update them.

use std::time::Duration;

use nsh_core::{prelude::*, storage::Storage};

use crate::{
    api, demo,
    discover::{self, AnimeTab, Day, Filters, Item, MediaKind},
    logbook,
    model::{
        Candidate, FavoriteEntry, Library, ProgressEntry, Settings, Work, format_time,
        load_library, load_search_history, load_settings, profile_for, save_library,
        save_search_history, save_settings, short_hash,
    },
    search_stream,
    source_browse::{self, Category, DuanjuSite, Site},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Mode {
    #[default]
    None,
    Demo,
    Real,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Session {
    pub mode: Mode,
    /// Normalized instance base with a trailing slash; `demo` for the demo.
    pub base: String,
    pub username: String,
    /// The opaque session token. Never persisted, never logged.
    pub token: String,
    pub expired: bool,
    pub connecting: bool,
    pub profile: String,
    /// Bumped when the session changes, so late answers can be dropped.
    pub generation: u64,
}

impl Session {
    pub fn demo(&self) -> bool {
        self.mode == Mode::Demo
    }

    pub fn connected(&self) -> bool {
        self.mode == Mode::Real && !self.token.is_empty() && !self.expired
    }

    pub fn show_connect(&self) -> bool {
        self.mode == Mode::None || (self.mode == Mode::Real && self.token.is_empty())
    }

    pub fn instance_label(&self) -> String {
        if self.demo() {
            "本机演示（隔离数据）".to_owned()
        } else if self.base.is_empty() {
            "未连接实例".to_owned()
        } else {
            crate::model::instance_host(&self.base)
        }
    }

    pub fn account_label(&self) -> String {
        if self.demo() {
            return "演示模式".to_owned();
        }
        if self.username.is_empty() {
            "未登录".to_owned()
        } else {
            self.username.clone()
        }
    }

    pub fn status_text(&self) -> String {
        if self.demo() {
            "演示模式 · 数据仅保存在本机".to_owned()
        } else if self.expired {
            "会话过期 · 请重新登录".to_owned()
        } else if self.connecting {
            "正在连接…".to_owned()
        } else if self.connected() {
            format!("已连接 · {}", self.base.trim_end_matches('/'))
        } else if self.base.is_empty() {
            "还没有连接实例".to_owned()
        } else {
            "需要登录".to_owned()
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Route {
    Connect,
    Home,
    Discover,
    Sources,
    Duanju,
    Search,
    Detail(Work),
    Player,
    Favorites,
    History,
    Settings,
}

impl Route {
    pub fn section(&self) -> &'static str {
        match self {
            Route::Connect => "connect",
            Route::Home => "home",
            Route::Discover => "discover",
            Route::Sources => "sources",
            Route::Duanju => "duanju",
            Route::Search => "search",
            Route::Detail(_) => "detail",
            Route::Player => "player",
            Route::Favorites => "favorites",
            Route::History => "history",
            Route::Settings => "settings",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Route::Connect => "连接",
            Route::Home => "首页",
            Route::Discover => "发现",
            Route::Sources => "单源",
            Route::Duanju => "短剧",
            Route::Search => "搜索",
            Route::Detail(_) => "详情",
            Route::Player => "播放",
            Route::Favorites => "收藏",
            Route::History => "历史",
            Route::Settings => "设置",
        }
    }

    pub fn nav_key(&self) -> Option<&'static str> {
        match self {
            Route::Home => Some("home"),
            Route::Discover => Some("discover"),
            Route::Sources => Some("sources"),
            Route::Duanju => Some("duanju"),
            Route::Favorites => Some("favorites"),
            Route::History => Some("history"),
            Route::Settings => Some("settings"),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Nav {
    pub route: Route,
    pub stack: Vec<Route>,
}

impl Default for Nav {
    fn default() -> Self {
        Self {
            route: Route::Connect,
            stack: Vec::new(),
        }
    }
}

impl Nav {
    pub fn go(&mut self, route: Route) {
        if self.route == route {
            return;
        }
        if matches!(route, Route::Player) {
            // The player is entered from detail; keep the trail.
        }
        self.stack.push(self.route.clone());
        if self.stack.len() > 24 {
            self.stack.remove(0);
        }
        self.route = route;
    }

    pub fn replace(&mut self, route: Route) {
        self.route = route;
    }

    pub fn back(&mut self) -> bool {
        if let Some(route) = self.stack.pop() {
            self.route = route;
            true
        } else {
            false
        }
    }

    pub fn home(&mut self) {
        self.stack.clear();
        self.route = Route::Home;
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum LoadPhase {
    #[default]
    Idle,
    Loading,
    Ready,
    Error,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SearchState {
    pub draft: String,
    pub query: String,
    pub phase: LoadPhase,
    pub works: Vec<Work>,
    pub error: String,
    pub generation: u64,
    pub has_run: bool,
    /// Every candidate the current query has produced so far, before
    /// grouping. Incremental results and the JSON fallback merge into this.
    pub results: Vec<Candidate>,
    /// Sources that reported while streaming, in arrival order.
    pub sources: Vec<SourceLog>,
    pub total_sources: usize,
    pub completed_sources: usize,
    /// The stream ended early; results may be incomplete (fallback adds the
    /// rest and clears this).
    pub partial: bool,
    pub notice: String,
    pub history: Vec<String>,
    pub suggestions: Vec<String>,
    pub suggestion_generation: u64,
    pub history_loaded: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SourceLog {
    pub source: String,
    pub name: String,
    pub count: usize,
    pub error: Option<String>,
}

impl Default for SearchState {
    fn default() -> Self {
        Self {
            draft: String::new(),
            query: String::new(),
            phase: LoadPhase::Idle,
            works: Vec::new(),
            error: String::new(),
            generation: 0,
            has_run: false,
            results: Vec::new(),
            sources: Vec::new(),
            total_sources: 0,
            completed_sources: 0,
            partial: false,
            notice: String::new(),
            history: Vec::new(),
            suggestions: Vec::new(),
            suggestion_generation: 0,
            history_loaded: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct DiscoverState {
    pub kind: MediaKind,
    pub movie_primary: String,
    pub movie_region: String,
    pub tv_primary: String,
    pub tv_secondary: String,
    pub show_primary: String,
    pub show_secondary: String,
    pub anime: AnimeTab,
    pub filters: Filters,
    pub weekday: String,
    pub days: Vec<Day>,
    pub items: Vec<Item>,
    pub phase: LoadPhase,
    pub error: String,
    pub page: usize,
    pub has_more: bool,
    pub loading_more: bool,
    pub generation: u64,
    pub notice: String,
}

impl Default for DiscoverState {
    fn default() -> Self {
        Self {
            kind: MediaKind::Movie,
            movie_primary: "全部".into(),
            movie_region: "全部".into(),
            tv_primary: "全部".into(),
            tv_secondary: "tv".into(),
            show_primary: "全部".into(),
            show_secondary: "show".into(),
            anime: AnimeTab::Daily,
            filters: Filters::reset(),
            weekday: discover::today().to_string(),
            days: Vec::new(),
            items: Vec::new(),
            phase: LoadPhase::Idle,
            error: String::new(),
            page: 0,
            has_more: false,
            loading_more: false,
            generation: 0,
            notice: String::new(),
        }
    }
}

/// Per-source browsing: sources, the CMS category tree, a page of results,
/// and an optional source-scoped search.
#[derive(Clone, Debug, PartialEq)]
pub struct SourceBrowseState {
    pub sites: Vec<Site>,
    pub sites_phase: LoadPhase,
    pub sites_error: String,
    pub sites_generation: u64,
    pub source: String,
    pub categories: Vec<Category>,
    pub parent: String,
    pub category: String,
    pub categories_loaded: bool,
    pub items: Vec<source_browse::Item>,
    pub phase: LoadPhase,
    pub error: String,
    pub notice: String,
    pub page: usize,
    pub page_count: usize,
    pub total: usize,
    pub loading_more: bool,
    pub generation: u64,
    pub list_generation: u64,
    pub searching: bool,
    pub keyword: String,
    pub search_input: String,
}

impl Default for SourceBrowseState {
    fn default() -> Self {
        Self {
            sites: Vec::new(),
            sites_phase: LoadPhase::Idle,
            sites_error: String::new(),
            sites_generation: 0,
            source: String::new(),
            categories: Vec::new(),
            parent: String::new(),
            category: String::new(),
            categories_loaded: false,
            items: Vec::new(),
            phase: LoadPhase::Idle,
            error: String::new(),
            notice: String::new(),
            page: 0,
            page_count: 0,
            total: 0,
            loading_more: false,
            generation: 0,
            list_generation: 0,
            searching: false,
            keyword: String::new(),
            search_input: String::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct DuanjuState {
    pub sites: Vec<DuanjuSite>,
    pub sites_phase: LoadPhase,
    pub sites_error: String,
    pub sites_generation: u64,
    pub source: String,
    pub categories: Vec<Category>,
    pub parent: String,
    pub category: String,
    pub items: Vec<source_browse::Item>,
    pub recommends: Vec<source_browse::Item>,
    pub recommends_phase: LoadPhase,
    pub recommends_error: String,
    pub phase: LoadPhase,
    pub error: String,
    pub notice: String,
    pub page: usize,
    pub page_count: usize,
    pub total: usize,
    pub loading_more: bool,
    pub generation: u64,
    pub list_generation: u64,
}

impl Default for DuanjuState {
    fn default() -> Self {
        Self {
            sites: Vec::new(),
            sites_phase: LoadPhase::Idle,
            sites_error: String::new(),
            sites_generation: 0,
            source: String::new(),
            categories: Vec::new(),
            parent: String::new(),
            category: String::new(),
            items: Vec::new(),
            recommends: Vec::new(),
            recommends_phase: LoadPhase::Idle,
            recommends_error: String::new(),
            phase: LoadPhase::Idle,
            error: String::new(),
            notice: String::new(),
            page: 0,
            page_count: 0,
            total: 0,
            loading_more: false,
            generation: 0,
            list_generation: 0,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct DetailState {
    pub work: Option<Work>,
    pub source: String,
    pub detail: Option<Candidate>,
    pub phase: LoadPhase,
    pub error: String,
    pub generation: u64,
    pub selected_episode: usize,
    /// Entered from the short-drama page: show the portrait autoplay action.
    pub duanju: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PlayTrace {
    pub source: String,
    pub id: String,
    pub source_name: String,
    pub title: String,
    pub year: String,
    pub poster: String,
    pub work_key: String,
    pub total_episodes: usize,
    pub proxy_mode: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct QueueItem {
    pub index: usize,
    pub title: String,
    pub url: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum PlayPhase {
    #[default]
    Idle,
    Loading,
    Ready,
    Failed,
    Ended,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PlayerState {
    pub trace: Option<PlayTrace>,
    pub work: Option<Work>,
    pub queue: Vec<QueueItem>,
    pub index: usize,
    pub load_token: u64,
    pub playing: bool,
    pub position: f64,
    pub duration: f64,
    pub buffering: bool,
    pub phase: PlayPhase,
    pub error: String,
    pub panel_open: bool,
    pub panel_tab: PanelTab,
    pub resume_at: f64,
    pub resume_attempts: u8,
    pub resume_done: bool,
    pub auto_in: Option<u32>,
    pub auto_token: u64,
    pub fullscreen: bool,
    /// Short-drama portrait stage: a centered 9:16 column with its own
    /// controls, entered from the short-drama page or a portrait resume.
    pub portrait: bool,
    pub muted: bool,
    pub notice: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum PanelTab {
    #[default]
    Episodes,
    Sources,
}

impl Default for PlayerState {
    fn default() -> Self {
        Self {
            trace: None,
            work: None,
            queue: Vec::new(),
            index: 0,
            load_token: 0,
            playing: false,
            position: 0.0,
            duration: 0.0,
            buffering: false,
            phase: PlayPhase::Idle,
            error: String::new(),
            panel_open: false,
            panel_tab: PanelTab::Episodes,
            resume_at: 0.0,
            resume_attempts: 0,
            resume_done: false,
            auto_in: None,
            auto_token: 0,
            fullscreen: false,
            portrait: false,
            muted: false,
            notice: String::new(),
        }
    }
}

impl PlayerState {
    pub fn current(&self) -> Option<&QueueItem> {
        self.queue.get(self.index)
    }

    pub fn url(&self) -> String {
        self.current()
            .map(|item| item.url.clone())
            .unwrap_or_default()
    }

    pub fn current_title(&self) -> String {
        self.current()
            .map(|item| item.title.clone())
            .unwrap_or_default()
    }

    pub fn can_next(&self) -> bool {
        self.index + 1 < self.queue.len()
    }

    pub fn can_prev(&self) -> bool {
        self.index > 0
    }

    pub fn progress(&self) -> f32 {
        if self.duration <= 0.0 {
            return 0.0;
        }
        (self.position / self.duration).clamp(0.0, 1.0) as f32
    }

    pub fn percent_text(&self) -> String {
        format!(
            "{} / {}",
            format_time(self.position),
            if self.duration > 0.0 {
                format_time(self.duration)
            } else {
                "--:--".to_owned()
            }
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToastTone {
    Info,
    Success,
    Error,
    Demo,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Toast {
    pub id: u64,
    pub text: String,
    pub tone: ToastTone,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct UiState {
    pub toast: Option<Toast>,
    pub toast_seq: u64,
    pub search_focus: bool,
    pub show_key_help: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ConnectDraft {
    pub instance: String,
    pub username: String,
    pub password: String,
}

impl Default for ConnectDraft {
    fn default() -> Self {
        let stored = load_settings();
        Self {
            instance: stored.instance,
            username: stored.last_username,
            password: String::new(),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct LoginRequest {
    pub active: bool,
    pub attempt: u64,
    pub base: String,
    pub user: String,
    pub pass: String,
}

#[derive(Debug, Default, PartialEq, Clone)]
struct Booted(bool);

pub fn session() -> Signal<Session> {
    global::<Session>()
}

pub fn nav() -> Signal<Nav> {
    global::<Nav>()
}

pub fn settings() -> Signal<Settings> {
    global::<Settings>()
}

pub fn library() -> Signal<Library> {
    global::<Library>()
}

pub fn search_state() -> Signal<SearchState> {
    global::<SearchState>()
}

pub fn detail_state() -> Signal<DetailState> {
    global::<DetailState>()
}

pub fn discover_state() -> Signal<DiscoverState> {
    global::<DiscoverState>()
}

pub fn source_browse_state() -> Signal<SourceBrowseState> {
    global::<SourceBrowseState>()
}

pub fn duanju_state() -> Signal<DuanjuState> {
    global::<DuanjuState>()
}

pub fn player() -> Signal<PlayerState> {
    global::<PlayerState>()
}

pub fn ui() -> Signal<UiState> {
    global::<UiState>()
}

pub fn connect_draft() -> Signal<ConnectDraft> {
    global::<ConnectDraft>()
}

/// Runs a task under no owner, so a page that is made again (or a shell
/// that re-renders) does not cancel a request whose result is global state.
/// The task holds only signals that outlive it and ends by itself.
pub fn spawn_global(future: impl std::future::Future<Output = ()> + 'static) {
    Owner::detached().run(|| {
        spawn(future);
    });
}

pub fn login_request() -> Signal<LoginRequest> {
    global::<LoginRequest>()
}

/// Performs login requests under the app's owner, so a form that is made
/// again while the request is in flight does not cancel it.
pub fn install_login_worker() {
    let running = stored(0_u64);
    effect(move || {
        let request = login_request().get();
        if !request.active || running.get() == request.attempt {
            return;
        }
        running.replace(request.attempt);
        let attempt = request.attempt;
        let base = request.base.clone();
        let user = request.user.clone();
        let pass = request.pass.clone();
        spawn_global(async move {
            let result = api::login(&base, &user, &pass).await;
            if login_request().get().attempt != attempt {
                return;
            }
            let mut request = login_request().get();
            request.active = false;
            request.pass.clear();
            login_request().set(request);
            let mut state = session().get();
            state.connecting = false;
            session().set(state);
            match result {
                Ok(login) => begin_real_session(base, login.username, login.token),
                Err(why) => {
                    if why.is_auth() {
                        let mut state = session().get();
                        state.expired = true;
                        session().set(state);
                    }
                    toast(why.message(), ToastTone::Error);
                    logbook::event(
                        "login_failed",
                        serde_json::json!({"kind": if why.is_auth() { "auth" } else { "other" }}),
                    );
                }
            }
        });
    });
}

pub fn route() -> Route {
    nav().get().route
}

/// Loads persisted settings and library once and chooses the first screen.
pub fn bootstrap() {
    if global::<Booted>().get().0 {
        return;
    }
    global::<Booted>().set(Booted(true));
    let stored = load_settings();
    settings().set(stored.clone());
    let profile = if stored.demo_active {
        "demo".to_owned()
    } else {
        profile_for(&stored.instance, &stored.last_username)
    };
    library().set(load_library(&profile));
    let mut state = Session {
        profile,
        base: stored.instance.clone(),
        username: stored.last_username.clone(),
        ..Session::default()
    };
    if stored.demo_active {
        state.mode = Mode::Demo;
        state.base = "demo".to_owned();
        state.username = "演示".to_owned();
        state.profile = "demo".to_owned();
        nav().set(Nav {
            route: Route::Home,
            stack: Vec::new(),
        });
    } else if !stored.instance.is_empty() {
        state.mode = Mode::None;
        nav().set(Nav {
            route: Route::Connect,
            stack: Vec::new(),
        });
    } else {
        // First launch: offer the built-in instance instead of an empty box.
        state.base = crate::model::DEFAULT_INSTANCE.to_owned();
        nav().set(Nav {
            route: Route::Connect,
            stack: Vec::new(),
        });
    }
    session().set(state);
    if !stored.demo_active && !stored.instance.is_empty() {
        restore_session();
    }
    logbook::event(
        "boot",
        serde_json::json!({
            "mode": if stored.demo_active { "demo" } else { "real" },
            "instance": logbook::safe_url(&stored.instance),
            "has_saved_instance": !stored.instance.is_empty(),
        }),
    );
    reset_search_history(&session().get().profile);
}

/// Resumes the DPAPI-sealed session from the last run before asking for a
/// password again. A revoked or expired refresh window clears it.
fn restore_session() {
    let Some(saved) = crate::session_store::load() else {
        return;
    };
    if saved.base.is_empty() || saved.token.is_empty() {
        return;
    }
    let mut state = session().get();
    state.connecting = true;
    session().set(state);
    spawn_global(async move {
        let storage = api::server_config(&saved.base)
            .await
            .ok()
            .and_then(|value| value["StorageType"].as_str().map(str::to_owned));
        let Some(storage) = storage else {
            let mut state = session().get();
            state.connecting = false;
            session().set(state);
            toast(
                "无法联系上次的实例，请检查网络后登录".to_owned(),
                ToastTone::Error,
            );
            return;
        };
        crate::api_transport::install(&saved.base, &saved.token, &storage);
        // A cheap authenticated call both validates the token and lets the
        // transport refresh it when only the access window has passed.
        let usable = match api::favorites(&saved.base, &saved.token).await {
            Ok(_) => true,
            Err(error) if error.is_auth() => false,
            Err(_) => true,
        };
        if usable {
            begin_real_session(saved.base, saved.username, saved.token);
        } else {
            crate::session_store::clear();
            let mut state = session().get();
            state.connecting = false;
            session().set(state);
            logbook::event("session_restore_expired", serde_json::json!({}));
            toast("上次登录已失效，请重新输入密码".to_owned(), ToastTone::Info);
        }
    });
}

pub fn save_current_settings() {
    let value = settings().get();
    save_settings(&value);
}

pub fn toast(text: impl Into<String>, tone: ToastTone) {
    let mut state = ui().get();
    state.toast_seq += 1;
    let id = state.toast_seq;
    state.toast = Some(Toast {
        id,
        text: text.into(),
        tone,
    });
    ui().set(state);
}

/// Dismisses a toast after its time; installed once under the app's owner
/// so a toast made inside a listener still goes away.
pub fn install_toast_timer() {
    let expires = stored(0_u64);
    effect(move || {
        if ui().get().toast.is_some() {
            expires.replace(nsh_core::time::unix_ms() as u64 + 3400);
        } else {
            expires.replace(0);
        }
    });
    set_interval(Duration::from_millis(200), move || {
        let deadline = expires.get();
        if deadline == 0 || (nsh_core::time::unix_ms() as u64) < deadline {
            return;
        }
        expires.replace(0);
        let mut state = ui().get();
        state.toast = None;
        ui().set(state);
    });
}

pub fn clear_toast() {
    let mut state = ui().get();
    state.toast = None;
    ui().set(state);
}

pub fn request_search_focus() {
    let mut state = ui().get();
    state.search_focus = true;
    ui().set(state);
}

pub fn take_search_focus() -> bool {
    let mut state = ui().get();
    let focus = state.search_focus;
    if focus {
        state.search_focus = false;
        ui().set(state);
    }
    focus
}

pub fn go(route: Route) {
    let mut value = nav().get();
    value.go(route);
    nav().set(value);
}

pub fn back() {
    let mut value = nav().get();
    if !value.back() {
        value.route = if session().get().demo() {
            Route::Home
        } else {
            Route::Connect
        };
    }
    nav().set(value);
}

pub fn home() {
    let mut value = nav().get();
    value.home();
    nav().set(value);
}

pub fn open_search() {
    let mut value = nav().get();
    if value.route != Route::Search {
        value.go(Route::Search);
        nav().set(value);
    }
    refresh_search_history(false);
    request_search_focus();
}

pub fn enter_demo() {
    let mut state = session().get();
    state.mode = Mode::Demo;
    state.base = "demo".to_owned();
    state.username = "演示".to_owned();
    state.token.clear();
    state.expired = false;
    state.connecting = false;
    state.profile = "demo".to_owned();
    state.generation += 1;
    session().set(state);
    let mut stored = settings().get();
    stored.demo_active = true;
    stored.last_username = String::new();
    settings().set(stored.clone());
    save_settings(&stored);
    library().set(load_library("demo"));
    reset_search_history("demo");
    logbook::event("session", serde_json::json!({"mode": "demo"}));
    home();
    toast(
        "已进入演示模式：片单与记录独立保存在本机".to_owned(),
        ToastTone::Demo,
    );
}

pub fn leave_session() {
    crate::relay::clear();
    crate::session_store::clear();
    let mut state = session().get();
    let was_demo = state.mode == Mode::Demo;
    state.mode = Mode::None;
    state.token.clear();
    state.expired = false;
    state.connecting = false;
    state.generation += 1;
    session().set(state);
    reset_search_history("");
    if was_demo {
        let mut stored = settings().get();
        stored.demo_active = false;
        settings().set(stored.clone());
        save_settings(&stored);
    }
    logbook::event(
        "session",
        serde_json::json!({"mode": "disconnected", "was_demo": was_demo}),
    );
    go(Route::Connect);
}

/// A successful login: the request token stays in memory, and a DPAPI-sealed
/// copy is kept so the next launch can resume without the password.
pub fn begin_real_session(base: String, username: String, token: String) {
    let profile = profile_for(&base, &username);
    let mut state = session().get();
    state.mode = Mode::Real;
    state.base = base.clone();
    state.username = username.clone();
    state.token = token;
    state.expired = false;
    state.connecting = false;
    state.profile = profile.clone();
    state.generation += 1;
    session().set(state);
    crate::relay::configure(&base, &session().get().token);
    crate::session_store::save(&base, &username, &session().get().token);
    let mut stored = settings().get();
    stored.instance = base.clone();
    stored.demo_active = false;
    stored.last_username = username.clone();
    settings().set(stored.clone());
    save_settings(&stored);
    library().set(load_library(&profile));
    reset_search_history(&profile);
    let generation = session().get().generation;
    pull_instance_library(base.clone(), session().get().token.clone(), generation);
    logbook::event(
        "session",
        serde_json::json!({"mode": "real", "instance": logbook::safe_url(&base), "user": short_hash(&username)}),
    );
    home();
    toast(
        "已连接实例；密码与会话不会写入本机设置".to_owned(),
        ToastTone::Success,
    );
}

pub fn session_expired() {
    crate::relay::clear();
    crate::session_store::clear();
    let mut state = session().get();
    if state.expired {
        return;
    }
    state.expired = true;
    state.token.clear();
    state.connecting = false;
    state.generation += 1;
    session().set(state);
    logbook::event("session_expired", serde_json::json!({}));
    toast(
        "会话已过期，请回到设置重新登录".to_owned(),
        ToastTone::Error,
    );
}

pub fn update_settings(update: impl FnOnce(&mut Settings)) {
    let mut value = settings().get();
    update(&mut value);
    settings().set(value.clone());
    save_settings(&value);
}

pub fn save_library_now() {
    let state = session().get();
    save_library(&state.profile, &library().get());
}

pub fn toggle_favorite(candidate: &Candidate) {
    let key = candidate.key();
    let entry = FavoriteEntry::from_candidate(candidate);
    let mut lib = library().get();
    let now = lib.toggle_favorite(entry.clone());
    if now && session().get().demo() {
        lib.set_synced(&key, Some(false));
    }
    library().set(lib);
    save_library_now();
    if now {
        toast(
            if session().get().demo() {
                format!("已加入本机片单：{}（演示数据）", candidate.title)
            } else {
                format!("已收藏：{}", candidate.title)
            },
            ToastTone::Success,
        );
    } else {
        toast(format!("已取消收藏：{}", candidate.title), ToastTone::Info);
    }
    let state = session().get();
    if !state.demo() && state.connected() {
        let base = state.base.clone();
        let token = state.token.clone();
        let generation = state.generation;
        let key2 = key.clone();
        spawn_global(async move {
            let result = if now {
                api::save_favorite(&base, &token, &entry).await
            } else {
                api::delete_favorite(&base, &token, &key2).await
            };
            if session().get().generation != generation {
                return;
            }
            match result {
                Ok(()) => {
                    let mut lib = library().get();
                    lib.set_synced(&key2, Some(true));
                    library().set(lib);
                    save_library_now();
                    toast("实例已确认收藏变更".to_owned(), ToastTone::Success);
                }
                Err(error) => {
                    let mut lib = library().get();
                    lib.set_synced(&key2, Some(false));
                    library().set(lib);
                    save_library_now();
                    if error.is_auth() {
                        session_expired();
                    } else {
                        toast(
                            format!("本机已更新，但实例同步失败：{}", error.message()),
                            ToastTone::Error,
                        );
                    }
                }
            }
        });
    }
}

pub fn remove_favorite(key: &str) {
    let mut lib = library().get();
    lib.remove_favorite(key);
    library().set(lib);
    save_library_now();
    toast("已从本机片单移除".to_owned(), ToastTone::Info);
    let state = session().get();
    if !state.demo() && state.connected() {
        let base = state.base.clone();
        let token = state.token.clone();
        let generation = state.generation;
        let key = key.to_owned();
        spawn_global(async move {
            let result = api::delete_favorite(&base, &token, &key).await;
            if session().get().generation != generation {
                return;
            }
            match result {
                Ok(()) => toast("实例已确认移除".to_owned(), ToastTone::Success),
                Err(error) if error.is_auth() => session_expired(),
                Err(error) => toast(
                    format!("本机已移除，但实例同步失败：{}", error.message()),
                    ToastTone::Error,
                ),
            }
        });
    }
}

pub fn refresh_favorites_from_instance() {
    let state = session().get();
    if state.demo() || !state.connected() {
        toast(
            "演示模式没有云端收藏；本机片单就是全部".to_owned(),
            ToastTone::Demo,
        );
        return;
    }
    let base = state.base.clone();
    let token = state.token.clone();
    let generation = state.generation;
    spawn_global(async move {
        match api::favorites(&base, &token).await {
            Ok(mut entries) => {
                if session().get().generation != generation {
                    return;
                }
                let mut lib = library().get();
                for entry in entries.drain(..) {
                    if !lib.is_favorite(&entry.key) {
                        lib.favorites.push(entry);
                    } else {
                        lib.set_synced(&entry.key, Some(true));
                    }
                }
                lib.favorites.sort_by_key(|f| std::cmp::Reverse(f.saved_at));
                library().set(lib);
                save_library_now();
                toast("已从实例刷新收藏".to_owned(), ToastTone::Success);
            }
            Err(error) if error.is_auth() => session_expired(),
            Err(error) => toast(
                format!("刷新收藏失败：{}", error.message()),
                ToastTone::Error,
            ),
        }
    });
}

// ------------------------------- search ---------------------------------

/// How long the stream may stay silent before it is treated as broken. The
/// instance gives each source 20 seconds, so this must outlast that.
const STREAM_IDLE: Duration = Duration::from_secs(28);

pub fn run_search() {
    let mut state = search_state().get();
    state.generation += 1;
    let generation = state.generation;
    let query = state.draft.trim().to_string();
    state.query = query.clone();
    state.error.clear();
    state.notice.clear();
    state.has_run = true;
    state.results.clear();
    state.sources.clear();
    state.total_sources = 0;
    state.completed_sources = 0;
    state.partial = false;
    state.works.clear();
    state.phase = if query.is_empty() && !session().get().demo() {
        LoadPhase::Idle
    } else {
        LoadPhase::Loading
    };
    search_state().set(state);
    open_search();
    refresh_search_history(false);
    logbook::event(
        "search_start",
        serde_json::json!({"query_len": query.chars().count()}),
    );
    if session().get().demo() {
        let mut state = search_state().get();
        state.works = if query.is_empty() {
            demo::works()
        } else {
            demo::search(&query)
        };
        state.phase = LoadPhase::Ready;
        search_state().set(state);
        if !query.is_empty() {
            record_search_history(&query);
        }
        return;
    }
    let session_now = session().get();
    if !session_now.connected() {
        let mut state = search_state().get();
        state.phase = LoadPhase::Error;
        state.error = "请先连接实例并登录".to_string();
        search_state().set(state);
        return;
    }
    if query.is_empty() {
        return;
    }
    record_search_history(&query);
    let base = session_now.base.clone();
    let token = session_now.token.clone();
    let session_generation = session_now.generation;
    match search_stream::open(&base, &token, &query) {
        Ok(stream) => {
            spawn_stream(stream, base, token, query, generation, session_generation);
        }
        Err(error) => {
            logbook::event(
                "search_stream_unavailable",
                serde_json::json!({"why": error.message()}),
            );
            spawn_fallback(
                base,
                token,
                query,
                generation,
                session_generation,
                "流式搜索无法启动",
            );
        }
    }
}

fn still_current(generation: u64, session_generation: u64) -> bool {
    search_state().get().generation == generation
        && session().get().generation == session_generation
}

fn note_source(
    state: &mut SearchState,
    source: String,
    name: String,
    count: usize,
    error: Option<String>,
) {
    state.completed_sources += 1;
    if let Some(entry) = state
        .sources
        .iter_mut()
        .find(|entry| entry.source == source)
    {
        entry.name = if name.is_empty() {
            entry.name.clone()
        } else {
            name
        };
        entry.count = count;
        entry.error = error;
    } else {
        state.sources.push(SourceLog {
            source,
            name,
            count,
            error,
        });
    }
}

fn ingest(state: &mut SearchState, incoming: Vec<Candidate>) -> usize {
    crate::model::merge_candidates(&mut state.results, incoming)
}

fn spawn_stream(
    mut stream: search_stream::Stream,
    base: String,
    token: String,
    query: String,
    generation: u64,
    session_generation: u64,
) {
    spawn_global(async move {
        loop {
            if !still_current(generation, session_generation) {
                stream.cancel();
                return;
            }
            let next = tokio::time::timeout(STREAM_IDLE, stream.next()).await;
            let event = match next {
                Ok(Some(event)) => event,
                Ok(None) => {
                    stream.cancel();
                    fallback(
                        &base,
                        &token,
                        &query,
                        generation,
                        session_generation,
                        "流式搜索连接提前结束",
                    )
                    .await;
                    return;
                }
                Err(_) => {
                    stream.cancel();
                    fallback(
                        &base,
                        &token,
                        &query,
                        generation,
                        session_generation,
                        "片源响应超时",
                    )
                    .await;
                    return;
                }
            };
            if !still_current(generation, session_generation) {
                stream.cancel();
                return;
            }
            match event {
                search_stream::Event::Start { total_sources } => {
                    let mut state = search_state().get();
                    state.total_sources = total_sources;
                    search_state().set(state);
                }
                search_stream::Event::SourceResults {
                    source,
                    source_name,
                    results,
                } => {
                    let mut state = search_state().get();
                    let count = results.len();
                    ingest(&mut state, results);
                    state.works = crate::model::rank_works(
                        crate::model::group_results(state.results.clone()),
                        &state.query,
                    );
                    note_source(&mut state, source, source_name, count, None);
                    search_state().set(state);
                }
                search_stream::Event::SourceError {
                    source,
                    source_name,
                    error,
                } => {
                    let mut state = search_state().get();
                    note_source(&mut state, source, source_name, 0, Some(error));
                    search_state().set(state);
                }
                search_stream::Event::Complete {
                    total_results,
                    completed_sources,
                } => {
                    let mut state = search_state().get();
                    state.completed_sources = state.completed_sources.max(completed_sources);
                    state.works = crate::model::rank_works(
                        crate::model::group_results(state.results.clone()),
                        &state.query,
                    );
                    state.phase = LoadPhase::Ready;
                    state.partial = false;
                    state.notice.clear();
                    let works = state.works.len();
                    let results = state.results.len();
                    let sources = state.sources.len();
                    search_state().set(state);
                    logbook::event(
                        "search_done",
                        serde_json::json!({
                            "stream": true,
                            "query_len": query.chars().count(),
                            "works": works,
                            "results": results,
                            "total_results": total_results,
                            "sources": sources,
                        }),
                    );
                    return;
                }
                search_stream::Event::Failed { error } => {
                    if error.is_auth() {
                        session_expired();
                    }
                    stream.cancel();
                    fallback(
                        &base,
                        &token,
                        &query,
                        generation,
                        session_generation,
                        &error.message(),
                    )
                    .await;
                    return;
                }
            }
        }
    });
}

fn spawn_fallback(
    base: String,
    token: String,
    query: String,
    generation: u64,
    session_generation: u64,
    reason: &str,
) {
    let reason = reason.to_string();
    spawn_global(async move {
        fallback(
            &base,
            &token,
            &query,
            generation,
            session_generation,
            &reason,
        )
        .await;
    });
}

fn absorb_fallback(state: &mut SearchState, candidates: Vec<Candidate>, reason: &str) -> usize {
    let added = crate::model::merge_candidates(&mut state.results, candidates);
    state.works = crate::model::rank_works(
        crate::model::group_results(state.results.clone()),
        &state.query,
    );
    state.phase = LoadPhase::Ready;
    state.partial = false;
    state.notice = if state.results.is_empty() {
        format!("{reason}；普通搜索也没有返回结果")
    } else {
        format!("{reason}；已降级到普通搜索并补全结果")
    };
    added
}

fn keep_partial_after_failure(state: &mut SearchState, reason: &str, error: &str) {
    state.works = crate::model::rank_works(
        crate::model::group_results(state.results.clone()),
        &state.query,
    );
    state.phase = LoadPhase::Ready;
    state.partial = true;
    state.notice = format!("{reason}；普通搜索也失败（{error}），仅显示已到达的结果");
}

/// Ordinary JSON search, used when the stream breaks, stalls or cannot
/// start. Its results merge into everything already on screen: an early
/// source's results are never thrown away.
async fn fallback(
    base: &str,
    token: &str,
    query: &str,
    generation: u64,
    session_generation: u64,
    reason: &str,
) {
    if !still_current(generation, session_generation) {
        return;
    }
    let result = api::search(base, token, query).await;
    if !still_current(generation, session_generation) {
        return;
    }
    let mut state = search_state().get();
    match result {
        Ok(candidates) => {
            let had = state.results.len();
            let added = absorb_fallback(&mut state, candidates, reason);
            let works = state.works.len();
            let results = state.results.len();
            search_state().set(state);
            logbook::event(
                "search_fallback",
                serde_json::json!({
                    "reason": reason,
                    "query_len": query.chars().count(),
                    "had": had,
                    "added": added,
                    "works": works,
                    "results": results,
                }),
            );
        }
        Err(error) => {
            if error.is_auth() {
                session_expired();
            }
            if !state.results.is_empty() {
                keep_partial_after_failure(&mut state, reason, &error.message());
            } else {
                state.phase = LoadPhase::Error;
                state.error = error.message();
            }
            search_state().set(state);
        }
    }
}

pub fn cancel_search() {
    search_stream::cancel_current();
    let mut state = search_state().get();
    state.generation += 1;
    state.suggestion_generation += 1;
    state.suggestions.clear();
    if state.phase == LoadPhase::Loading {
        state.phase = if state.works.is_empty() {
            LoadPhase::Idle
        } else {
            LoadPhase::Ready
        };
        state.partial = true;
        state.notice = "已取消本次搜索；保留已到达的结果".to_string();
    }
    search_state().set(state);
    toast("已取消搜索", ToastTone::Info);
}

/// Updates the draft and schedules one debounced suggestion request. A newer
/// keystroke bumps the generation, so an old answer cannot replace it.
pub fn set_draft(value: &str) {
    let mut state = search_state().get();
    state.draft = value.to_string();
    state.suggestion_generation += 1;
    let generation = state.suggestion_generation;
    if value.trim().is_empty() {
        state.suggestions.clear();
    }
    search_state().set(state);
    let trimmed = value.trim().to_string();
    if trimmed.is_empty() {
        return;
    }
    let session_now = session().get();
    if session_now.demo() || !session_now.connected() {
        return;
    }
    let base = session_now.base.clone();
    let token = session_now.token.clone();
    let session_generation = session_now.generation;
    set_timeout(Duration::from_millis(300), move || {
        let current = search_state().get();
        if current.suggestion_generation != generation
            || current.draft.trim() != trimmed
            || session().get().generation != session_generation
        {
            return;
        }
        spawn_global(async move {
            let result = api::suggestions(&base, &token, &trimmed).await;
            if session().get().generation != session_generation {
                return;
            }
            let mut state = search_state().get();
            if state.suggestion_generation != generation {
                return;
            }
            if let Ok(list) = result {
                state.suggestions = list;
                search_state().set(state);
            }
        });
    });
}

/// Runs a query chosen from history, suggestions or a discovery card.
pub fn search_for(keyword: impl Into<String>) {
    let keyword = keyword.into();
    let mut state = search_state().get();
    state.draft = keyword;
    state.suggestion_generation += 1;
    state.suggestions.clear();
    search_state().set(state);
    run_search();
}

// ----------------------------- search history ---------------------------

pub fn refresh_search_history(force: bool) {
    let session_now = session().get();
    if !force && search_state().get().history_loaded {
        return;
    }
    let local = load_search_history(&session_now.profile);
    {
        let mut state = search_state().get();
        if force || state.history.is_empty() {
            state.history = local;
        }
        state.history_loaded = true;
        search_state().set(state);
    }
    if session_now.demo() || !session_now.connected() {
        return;
    }
    let base = session_now.base.clone();
    let token = session_now.token.clone();
    let session_generation = session_now.generation;
    spawn_global(async move {
        match api::search_history(&base, &token).await {
            Ok(list) => {
                if session().get().generation != session_generation {
                    return;
                }
                persist_history(list);
            }
            Err(error) if error.is_auth() => session_expired(),
            // A history read is advisory; the local copy stays usable.
            Err(_) => {}
        }
    });
}

/// Switches the visible search history to the profile that just became
/// active, without asking the instance until the page is opened.
fn reset_search_history(profile: &str) {
    let history = if profile.is_empty() {
        Vec::new()
    } else {
        load_search_history(profile)
    };
    let mut state = search_state().get();
    state.history = history;
    state.history_loaded = false;
    state.suggestions.clear();
    state.suggestion_generation += 1;
    search_state().set(state);
}

fn persist_history(list: Vec<String>) {
    let session_now = session().get();
    let mut history: Vec<String> = Vec::new();
    for item in list {
        let item = item.trim().to_string();
        if !item.is_empty() && !history.iter().any(|known| known == &item) {
            history.push(item);
        }
    }
    history.truncate(20);
    save_search_history(&session_now.profile, &history);
    let mut state = search_state().get();
    state.history = history;
    state.history_loaded = true;
    search_state().set(state);
}

pub fn record_search_history(keyword: &str) {
    let keyword = keyword.trim();
    if keyword.is_empty() {
        return;
    }
    let session_now = session().get();
    let mut history = search_state().get().history;
    history.retain(|known| known != keyword);
    history.insert(0, keyword.to_string());
    history.truncate(20);
    persist_history(history);
    if session_now.demo() || !session_now.connected() {
        return;
    }
    let base = session_now.base.clone();
    let token = session_now.token.clone();
    let session_generation = session_now.generation;
    let keyword = keyword.to_string();
    spawn_global(async move {
        match api::add_search_history(&base, &token, &keyword).await {
            Ok(list) => {
                if session().get().generation == session_generation {
                    persist_history(list);
                }
            }
            Err(error) if error.is_auth() => session_expired(),
            Err(error) => logbook::event(
                "search_history_sync_failed",
                serde_json::json!({"why": error.message()}),
            ),
        }
    });
}

pub fn remove_search_history(keyword: &str) {
    let session_now = session().get();
    let mut history = search_state().get().history;
    history.retain(|known| known != keyword);
    persist_history(history);
    if session_now.demo() || !session_now.connected() {
        return;
    }
    let base = session_now.base.clone();
    let token = session_now.token.clone();
    let session_generation = session_now.generation;
    let keyword = keyword.to_string();
    spawn_global(async move {
        match api::delete_search_history(&base, &token, Some(&keyword)).await {
            Ok(()) => {
                if session().get().generation == session_generation {
                    toast("已删除这条搜索历史", ToastTone::Info);
                }
            }
            Err(error) if error.is_auth() => session_expired(),
            Err(error) => toast(
                format!("本机已删除，但实例同步失败：{}", error.message()),
                ToastTone::Error,
            ),
        }
    });
}

pub fn clear_search_history() {
    let session_now = session().get();
    persist_history(Vec::new());
    if session_now.demo() || !session_now.connected() {
        toast("已清空本机搜索历史", ToastTone::Info);
        return;
    }
    let base = session_now.base.clone();
    let token = session_now.token.clone();
    let session_generation = session_now.generation;
    spawn_global(async move {
        match api::delete_search_history(&base, &token, None).await {
            Ok(()) => {
                if session().get().generation == session_generation {
                    toast("已清空搜索历史", ToastTone::Success);
                }
            }
            Err(error) if error.is_auth() => session_expired(),
            Err(error) => toast(
                format!("本机已清空，但实例同步失败：{}", error.message()),
                ToastTone::Error,
            ),
        }
    });
}

// ------------------------------- discover --------------------------------

fn discover_secondary(state: &DiscoverState) -> String {
    match state.kind {
        MediaKind::Movie => state.movie_region.clone(),
        MediaKind::Tv => state.tv_secondary.clone(),
        MediaKind::Show => state.show_secondary.clone(),
        MediaKind::Anime => "全部".to_string(),
    }
}

fn memory_key(kind: MediaKind, slot: &str) -> String {
    format!("{}.{slot}", kind.key())
}

fn remember_discover() {
    let state = discover_state().get();
    let kind = state.kind;
    let mut stored = settings().get();
    match kind {
        MediaKind::Movie => {
            stored
                .discover
                .insert(memory_key(kind, "primary"), state.movie_primary.clone());
            stored
                .discover
                .insert(memory_key(kind, "secondary"), state.movie_region.clone());
        }
        MediaKind::Tv => {
            stored
                .discover
                .insert(memory_key(kind, "primary"), state.tv_primary.clone());
            stored
                .discover
                .insert(memory_key(kind, "secondary"), state.tv_secondary.clone());
        }
        MediaKind::Show => {
            stored
                .discover
                .insert(memory_key(kind, "primary"), state.show_primary.clone());
            stored
                .discover
                .insert(memory_key(kind, "secondary"), state.show_secondary.clone());
        }
        MediaKind::Anime => {
            stored.discover.insert(
                memory_key(kind, "anime"),
                match state.anime {
                    AnimeTab::Daily => "daily",
                    AnimeTab::Tv => "tv",
                    AnimeTab::Movie => "movie",
                }
                .to_string(),
            );
            stored
                .discover
                .insert(memory_key(kind, "weekday"), state.weekday.clone());
        }
    }
    for key in ["type", "label", "region", "year", "platform", "sort"] {
        stored.discover.insert(
            memory_key(kind, &format!("filter.{key}")),
            state.filters.get(key).to_string(),
        );
    }
    settings().set(stored.clone());
    save_settings(&stored);
}

fn apply_discover_memory(state: &mut DiscoverState) {
    let stored = settings().get();
    let kind = state.kind;
    let read = |slot: &str| stored.discover.get(&memory_key(kind, slot)).cloned();
    match kind {
        MediaKind::Movie => {
            if let Some(value) =
                read("primary").filter(|v| discover::movie_primaries().contains(&v.as_str()))
            {
                state.movie_primary = value;
            }
            if let Some(value) =
                read("secondary").filter(|v| discover::movie_regions().contains(&v.as_str()))
            {
                state.movie_region = value;
            }
        }
        MediaKind::Tv => {
            if let Some(value) = read("primary").filter(|v| v == "全部" || v == "最近热门") {
                state.tv_primary = value;
            }
            if let Some(value) = read("secondary")
                .filter(|v| discover::tv_secondaries().iter().any(|(_, id)| id == v))
            {
                state.tv_secondary = value;
            }
        }
        MediaKind::Show => {
            if let Some(value) = read("primary").filter(|v| v == "全部" || v == "最近热门") {
                state.show_primary = value;
            }
            if let Some(value) = read("secondary")
                .filter(|v| discover::show_secondaries().iter().any(|(_, id)| id == v))
            {
                state.show_secondary = value;
            }
        }
        MediaKind::Anime => {
            if let Some(value) = read("anime") {
                state.anime = match value.as_str() {
                    "tv" => AnimeTab::Tv,
                    "movie" => AnimeTab::Movie,
                    _ => AnimeTab::Daily,
                };
            }
            if let Some(value) =
                read("weekday").filter(|v| discover::weekday_order().contains(&v.as_str()))
            {
                state.weekday = value;
            }
        }
    }
    for key in ["type", "label", "region", "year", "platform", "sort"] {
        if let Some(value) = read(&format!("filter.{key}")).filter(|v| !v.trim().is_empty()) {
            state.filters.set(key, &value);
        }
    }
    sanitize_filters(&mut state.filters, kind, state.anime);
}

fn sanitize_filters(filters: &mut Filters, kind: MediaKind, anime: AnimeTab) {
    let specs = discover::filter_specs(kind, anime);
    for spec in &specs {
        let value = filters.get(spec.key).to_string();
        if !spec.options.iter().any(|(_, id)| *id == value) {
            filters.set(spec.key, if spec.key == "sort" { "T" } else { "all" });
        }
    }
    for spec in discover::filter_specs(kind, AnimeTab::Daily) {
        if !specs.iter().any(|known| known.key == spec.key) {
            filters.set(spec.key, if spec.key == "sort" { "T" } else { "all" });
        }
    }
}

pub fn open_discover() {
    {
        let mut state = discover_state().get();
        apply_discover_memory(&mut state);
        discover_state().set(state);
    }
    go(Route::Discover);
    discover_load(false);
}

pub fn discover_set_kind(kind: MediaKind) {
    let mut state = discover_state().get();
    if state.kind == kind {
        return;
    }
    state.kind = kind;
    state.items.clear();
    state.days.clear();
    state.page = 0;
    state.has_more = false;
    state.loading_more = false;
    state.phase = LoadPhase::Idle;
    state.error.clear();
    state.notice.clear();
    state.filters = Filters::reset();
    state.generation += 1;
    apply_discover_memory(&mut state);
    discover_state().set(state);
    discover_load(false);
}

pub fn discover_set_movie_primary(value: &str) {
    let mut state = discover_state().get();
    if state.movie_primary == value {
        return;
    }
    state.movie_primary = value.to_string();
    state.items.clear();
    state.page = 0;
    state.has_more = false;
    state.phase = LoadPhase::Idle;
    state.error.clear();
    state.notice.clear();
    state.generation += 1;
    discover_state().set(state);
    remember_discover();
    discover_load(false);
}

pub fn discover_set_movie_region(value: &str) {
    let mut state = discover_state().get();
    if state.movie_region == value {
        return;
    }
    state.movie_region = value.to_string();
    state.items.clear();
    state.page = 0;
    state.has_more = false;
    state.phase = LoadPhase::Idle;
    state.error.clear();
    state.notice.clear();
    state.generation += 1;
    discover_state().set(state);
    remember_discover();
    discover_load(false);
}

pub fn discover_set_tv_primary(value: &str) {
    let mut state = discover_state().get();
    let current = if state.kind == MediaKind::Show {
        &mut state.show_primary
    } else {
        &mut state.tv_primary
    };
    if *current == value {
        return;
    }
    *current = value.to_string();
    state.items.clear();
    state.page = 0;
    state.has_more = false;
    state.phase = LoadPhase::Idle;
    state.error.clear();
    state.notice.clear();
    state.generation += 1;
    discover_state().set(state);
    remember_discover();
    discover_load(false);
}

pub fn discover_set_secondary(value: &str) {
    let mut state = discover_state().get();
    let current = match state.kind {
        MediaKind::Movie => &mut state.movie_region,
        MediaKind::Show => &mut state.show_secondary,
        _ => &mut state.tv_secondary,
    };
    if *current == value {
        return;
    }
    *current = value.to_string();
    state.items.clear();
    state.page = 0;
    state.has_more = false;
    state.phase = LoadPhase::Idle;
    state.error.clear();
    state.notice.clear();
    state.generation += 1;
    discover_state().set(state);
    remember_discover();
    discover_load(false);
}

pub fn discover_set_anime(tab: AnimeTab) {
    let mut state = discover_state().get();
    if state.anime == tab {
        return;
    }
    state.anime = tab;
    state.items.clear();
    state.page = 0;
    state.has_more = false;
    state.phase = LoadPhase::Idle;
    state.error.clear();
    state.notice.clear();
    state.generation += 1;
    sanitize_filters(&mut state.filters, state.kind, tab);
    discover_state().set(state);
    remember_discover();
    discover_load(false);
}

pub fn discover_set_filter(key: &str, value: &str) {
    let mut state = discover_state().get();
    if state.filters.get(key) == value {
        return;
    }
    state.filters.set(key, value);
    state.items.clear();
    state.page = 0;
    state.has_more = false;
    state.phase = LoadPhase::Idle;
    state.error.clear();
    state.notice.clear();
    state.generation += 1;
    discover_state().set(state);
    remember_discover();
    discover_load(false);
}

pub fn discover_reset_filters() {
    let mut state = discover_state().get();
    state.filters = Filters::reset();
    state.items.clear();
    state.page = 0;
    state.has_more = false;
    state.phase = LoadPhase::Idle;
    state.error.clear();
    state.notice.clear();
    state.generation += 1;
    discover_state().set(state);
    remember_discover();
    discover_load(false);
}

pub fn discover_set_weekday(weekday: &str) {
    let mut state = discover_state().get();
    if state.weekday == weekday || !discover::weekday_order().contains(&weekday) {
        return;
    }
    state.weekday = weekday.to_string();
    state.phase = LoadPhase::Ready;
    state.error.clear();
    discover_state().set(state);
    remember_discover();
}

pub fn discover_retry() {
    discover_load(true);
}

pub fn discover_more() {
    let state = discover_state().get();
    if state.loading_more
        || !state.has_more
        || state.kind == MediaKind::Anime && state.anime == AnimeTab::Daily
    {
        return;
    }
    if session().get().demo() {
        return;
    }
    let session_now = session().get();
    if !session_now.connected() {
        toast("请先连接实例并登录", ToastTone::Error);
        return;
    }
    let Some(query) = crate::discover::query_for(
        state.kind,
        &state.movie_primary,
        if state.kind == MediaKind::Show {
            &state.show_primary
        } else {
            &state.tv_primary
        },
        &discover_secondary(&state),
        state.anime,
        &state.filters,
        (state.page + 1) * discover::PAGE_LIMIT,
    ) else {
        return;
    };
    if matches!(query, crate::discover::Query::Calendar) {
        return;
    }
    let mut state = discover_state().get();
    state.loading_more = true;
    state.generation += 1;
    let generation = state.generation;
    state.notice.clear();
    discover_state().set(state);
    let base = session_now.base.clone();
    let token = session_now.token.clone();
    let session_generation = session_now.generation;
    spawn_global(async move {
        let result = api::discover(&base, &token, &query).await;
        if !discover_current(generation, session_generation) {
            return;
        }
        let mut state = discover_state().get();
        state.loading_more = false;
        match result {
            Ok(items) => {
                let full_page = items.len() >= discover::PAGE_LIMIT;
                let added = crate::discover::merge(&mut state.items, items);
                state.page += 1;
                state.has_more = full_page && added > 0;
                if !state.has_more && added == 0 {
                    state.notice = "这一页没有新条目，已停止加载".to_string();
                }
                state.phase = LoadPhase::Ready;
                discover_state().set(state);
            }
            Err(error) => {
                if error.is_auth() {
                    session_expired();
                }
                state.notice = format!("加载下一页失败：{}", error.message());
                state.has_more = true;
                discover_state().set(state);
            }
        }
    });
}

fn discover_current(generation: u64, session_generation: u64) -> bool {
    discover_state().get().generation == generation
        && session().get().generation == session_generation
}

pub fn discover_load(replace: bool) {
    let state = discover_state().get();
    if !replace && state.phase == LoadPhase::Ready {
        return;
    }
    if state.kind == MediaKind::Anime
        && state.anime == AnimeTab::Daily
        && !replace
        && !state.days.is_empty()
    {
        return;
    }
    if !replace && state.phase == LoadPhase::Loading {
        return;
    }
    if session().get().demo() {
        discover_load_demo();
        return;
    }
    let session_now = session().get();
    if !session_now.connected() {
        let mut state = discover_state().get();
        state.phase = LoadPhase::Error;
        state.error = "请先连接实例并登录；发现页使用实例的豆瓣与 Bangumi 代理".to_string();
        discover_state().set(state);
        return;
    }
    let Some(query) = crate::discover::query_for(
        state.kind,
        &state.movie_primary,
        if state.kind == MediaKind::Show {
            &state.show_primary
        } else {
            &state.tv_primary
        },
        &discover_secondary(&state),
        state.anime,
        &state.filters,
        0,
    ) else {
        return;
    };
    let mut state = discover_state().get();
    state.generation += 1;
    let generation = state.generation;
    state.phase = LoadPhase::Loading;
    state.error.clear();
    state.notice.clear();
    state.page = 0;
    state.has_more = false;
    state.loading_more = false;
    let kind_key = state.kind.key();
    discover_state().set(state);
    logbook::event(
        "discover_load",
        serde_json::json!({"kind": kind_key, "query": format!("{query:?}")}),
    );
    let base = session_now.base.clone();
    let token = session_now.token.clone();
    let session_generation = session_now.generation;
    spawn_global(async move {
        match query {
            crate::discover::Query::Calendar => {
                let result = api::bangumi_calendar(&base, &token).await;
                if !discover_current(generation, session_generation) {
                    return;
                }
                let mut state = discover_state().get();
                match result {
                    Ok(days) => {
                        state.days = days;
                        let weekday = state.weekday.clone();
                        if !state.days.iter().any(|day| day.en == weekday)
                            && let Some(first) = state.days.first()
                        {
                            state.weekday = first.en.clone();
                        }
                        state.phase = LoadPhase::Ready;
                        state.error.clear();
                        if state.days.iter().all(|day| day.items.is_empty()) {
                            state.notice = "日历返回了空的一天；换个星期或稍后重试".to_string();
                        }
                        discover_state().set(state);
                    }
                    Err(error) => {
                        if error.is_auth() {
                            session_expired();
                        }
                        state.phase = LoadPhase::Error;
                        state.error = error.message();
                        discover_state().set(state);
                    }
                }
            }
            query => {
                let result = api::discover(&base, &token, &query).await;
                if !discover_current(generation, session_generation) {
                    return;
                }
                let mut state = discover_state().get();
                match result {
                    Ok(items) => {
                        let full_page = items.len() >= discover::PAGE_LIMIT;
                        state.items = Vec::new();
                        crate::discover::merge(&mut state.items, items);
                        state.has_more = full_page;
                        state.page = 0;
                        state.phase = LoadPhase::Ready;
                        state.error.clear();
                        state.notice.clear();
                        discover_state().set(state);
                    }
                    Err(error) => {
                        if error.is_auth() {
                            session_expired();
                        }
                        state.items.clear();
                        state.phase = LoadPhase::Error;
                        state.error = error.message();
                        discover_state().set(state);
                    }
                }
            }
        }
    });
}

fn discover_load_demo() {
    let mut state = discover_state().get();
    state.generation += 1;
    state.days = demo::anime_days();
    state.items = demo::discover(
        state.kind,
        state.anime,
        &state.movie_primary,
        if state.kind == MediaKind::Show {
            &state.show_primary
        } else {
            &state.tv_primary
        },
        &state.filters,
    );
    if state.kind == MediaKind::Anime && state.anime == AnimeTab::Daily {
        state.items.clear();
        if !state.days.iter().any(|day| day.en == state.weekday)
            && let Some(first) = state.days.first()
        {
            state.weekday = first.en.clone();
        }
    }
    state.phase = LoadPhase::Ready;
    state.error.clear();
    state.page = 0;
    state.has_more = false;
    state.loading_more = false;
    state.notice = "演示样片：不请求豆瓣或 Bangumi；筛选只作用于这几条本机示例".to_string();
    discover_state().set(state);
}

/// Opens a discovery card as a search: discovery has metadata but no playable
/// source, so the next real step is aggregating the instance's sources.
pub fn discover_open_item(item: &Item) {
    search_for(item.title.clone());
}

// ---------------------------- source browsing ---------------------------

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
            .take(3)
            .collect(),
        candidates: vec![Candidate {
            source: item.source.clone(),
            id: item.id.clone(),
            source_name: item.source_name.clone(),
            title: item.title.clone(),
            year: item.year.clone(),
            poster: item.poster.clone(),
            desc: item.desc.clone(),
            remarks: item.remarks.clone(),
            episodes: item.episodes.clone(),
            episode_titles: item.episode_titles.clone(),
            ..Candidate::default()
        }],
        weight: 0,
    }
}

pub fn open_source_item(item: &source_browse::Item) {
    open_detail(browse_work(item), Some(item.source.clone()));
}

pub fn open_duanju_item(item: &source_browse::Item) {
    open_detail_duanju(browse_work(item), Some(item.source.clone()));
}

pub fn open_sources() {
    go(Route::Sources);
    let state = source_browse_state().get();
    if state.sites.is_empty() || state.sites_phase == LoadPhase::Error {
        source_load_sites(false);
        return;
    }
    if !state.source.is_empty() && !state.categories_loaded {
        source_load_categories();
    }
}

pub fn source_refresh_sites() {
    source_load_sites(true);
}

fn source_load_sites(force: bool) {
    if !force && source_browse_state().get().sites_phase == LoadPhase::Loading {
        return;
    }
    let session_now = session().get();
    let mut state = source_browse_state().get();
    state.sites_generation += 1;
    let generation = state.sites_generation;
    state.sites_phase = LoadPhase::Loading;
    state.sites_error.clear();
    source_browse_state().set(state);
    if !session_now.connected() {
        let mut state = source_browse_state().get();
        state.sites_phase = LoadPhase::Error;
        state.sites_error = "请先连接实例并登录".to_string();
        source_browse_state().set(state);
        return;
    }
    let base = session_now.base.clone();
    let token = session_now.token.clone();
    let session_generation = session_now.generation;
    spawn_global(async move {
        let result = api::source_sites(&base, &token).await;
        if source_browse_state().get().sites_generation != generation
            || session().get().generation != session_generation
        {
            return;
        }
        let mut state = source_browse_state().get();
        match result {
            Ok(sites) => {
                let previous = state.source.clone();
                state.sites = sites;
                state.sites_phase = LoadPhase::Ready;
                state.sites_error.clear();
                let keep = state.sites.iter().any(|site| site.key == previous);
                state.source = if keep {
                    previous
                } else {
                    state
                        .sites
                        .first()
                        .map(|site| site.key.clone())
                        .unwrap_or_default()
                };
                source_browse_state().set(state);
                if !source_browse_state().get().source.is_empty() {
                    source_load_categories();
                }
            }
            Err(error) => {
                if error.is_auth() {
                    session_expired();
                }
                state.sites_phase = LoadPhase::Error;
                state.sites_error = error.message();
                source_browse_state().set(state);
            }
        }
    });
}

pub fn source_select(key: String) {
    let state = source_browse_state().get();
    if state.source == key {
        return;
    }
    let mut state = state;
    state.source = key;
    state.categories.clear();
    state.categories_loaded = false;
    state.parent.clear();
    state.category.clear();
    state.items.clear();
    state.phase = LoadPhase::Loading;
    state.error.clear();
    state.notice.clear();
    state.page = 0;
    state.page_count = 0;
    state.total = 0;
    state.loading_more = false;
    state.generation += 1;
    state.list_generation += 1;
    source_browse_state().set(state);
    source_load_categories();
}

fn source_load_categories() {
    let session_now = session().get();
    let mut state = source_browse_state().get();
    let source = state.source.clone();
    if source.is_empty() {
        return;
    }
    if !session_now.connected() {
        state.phase = LoadPhase::Error;
        state.error = "请先连接实例并登录".to_string();
        source_browse_state().set(state);
        return;
    }
    state.generation += 1;
    let generation = state.generation;
    state.phase = LoadPhase::Loading;
    state.error.clear();
    state.notice.clear();
    state.categories_loaded = false;
    state.items.clear();
    state.page = 0;
    state.page_count = 0;
    state.total = 0;
    source_browse_state().set(state);
    let base = session_now.base.clone();
    let token = session_now.token.clone();
    let session_generation = session_now.generation;
    spawn_global(async move {
        let result = api::source_categories(&base, &token, &source).await;
        if source_browse_state().get().generation != generation
            || session().get().generation != session_generation
        {
            return;
        }
        let mut state = source_browse_state().get();
        match result {
            Ok(categories) => {
                state.categories = categories;
                state.categories_loaded = true;
                if state.categories.is_empty() {
                    state.phase = LoadPhase::Ready;
                    state.items.clear();
                    state.notice =
                        "这个源没有返回可用分类；它可能暂时不可用，换一个源或稍后重试".to_string();
                    source_browse_state().set(state);
                    return;
                }
                let (parent, category) = source_browse::default_selection(&state.categories, "");
                state.parent = parent;
                state.category = category;
                source_browse_state().set(state);
                source_load_list(true);
            }
            Err(error) => {
                if error.is_auth() {
                    session_expired();
                }
                state.categories.clear();
                state.items.clear();
                state.phase = LoadPhase::Error;
                state.error = error.message();
                source_browse_state().set(state);
            }
        }
    });
}

pub fn source_set_parent(parent: String) {
    let mut state = source_browse_state().get();
    if state.parent == parent {
        return;
    }
    let children = source_browse::children(&state.categories, &parent);
    state.parent = parent;
    state.category = children
        .first()
        .map(|child| child.id.clone())
        .unwrap_or_else(|| state.parent.clone());
    state.items.clear();
    state.page = 0;
    state.page_count = 0;
    state.phase = LoadPhase::Loading;
    state.error.clear();
    state.notice.clear();
    state.generation += 1;
    source_browse_state().set(state);
    source_load_list(true);
}

pub fn source_set_category(category: String) {
    let mut state = source_browse_state().get();
    if state.category == category {
        return;
    }
    state.category = category;
    state.items.clear();
    state.page = 0;
    state.page_count = 0;
    state.phase = LoadPhase::Loading;
    state.error.clear();
    state.notice.clear();
    state.generation += 1;
    source_browse_state().set(state);
    source_load_list(true);
}

fn source_load_list(reset: bool) {
    let session_now = session().get();
    let mut state = source_browse_state().get();
    let source = state.source.clone();
    let category = state.category.clone();
    let keyword = state.keyword.clone();
    let searching = state.searching;
    if source.is_empty() || category.is_empty() || (state.searching && keyword.is_empty()) {
        return;
    }
    if !session_now.connected() {
        state.phase = LoadPhase::Error;
        state.error = "请先连接实例并登录".to_string();
        source_browse_state().set(state);
        return;
    }
    let page = if reset { 1 } else { state.page + 1 };
    state.list_generation += 1;
    let generation = state.list_generation;
    let source_generation = state.generation;
    if reset {
        state.items.clear();
        state.page = 1;
        state.phase = LoadPhase::Loading;
    } else {
        state.loading_more = true;
    }
    state.error.clear();
    state.notice.clear();
    source_browse_state().set(state);
    let base = session_now.base.clone();
    let token = session_now.token.clone();
    let session_generation = session_now.generation;
    spawn_global(async move {
        let result = if searching {
            api::source_search(&base, &token, &source, &keyword, page).await
        } else {
            api::source_videos(&base, &token, &source, &category, page).await
        };
        let now = source_browse_state().get();
        if now.list_generation != generation
            || now.generation != source_generation
            || session().get().generation != session_generation
        {
            return;
        }
        let mut state = source_browse_state().get();
        state.loading_more = false;
        match result {
            Ok(page_data) => {
                let added = source_browse::merge(&mut state.items, page_data.items.clone());
                state.page = page_data.page.max(1);
                state.page_count = page_data.page_count.max(state.page);
                state.total = page_data.total;
                state.phase = LoadPhase::Ready;
                if !reset && added == 0 && state.page < state.page_count {
                    state.page_count = state.page;
                    state.notice = "这一页没有新条目，已停止加载".to_string();
                }
                source_browse_state().set(state);
            }
            Err(error) => {
                if error.is_auth() {
                    session_expired();
                }
                if reset {
                    state.phase = LoadPhase::Error;
                    state.error = error.message();
                } else {
                    state.notice = format!("加载下一页失败：{}", error.message());
                }
                source_browse_state().set(state);
            }
        }
    });
}

pub fn source_more() {
    let state = source_browse_state().get();
    if state.loading_more || state.page == 0 || state.page >= state.page_count {
        return;
    }
    source_load_list(false);
}

pub fn source_retry() {
    let state = source_browse_state().get();
    if state.sites.is_empty() || state.sites_phase == LoadPhase::Error {
        source_load_sites(true);
    } else if state.categories.is_empty() {
        source_load_categories();
    } else {
        source_load_list(true);
    }
}

pub fn source_search_input(value: String) {
    let mut state = source_browse_state().get();
    state.search_input = value;
    source_browse_state().set(state);
}

pub fn source_run_search() {
    let mut state = source_browse_state().get();
    let keyword = state.search_input.trim().to_string();
    if keyword.is_empty() {
        toast("请输入源内搜索关键词", ToastTone::Info);
        return;
    }
    if state.source.is_empty() || state.category.is_empty() {
        toast("请先选择一个可用的源和分类", ToastTone::Error);
        return;
    }
    state.searching = true;
    state.keyword = keyword;
    state.page = 0;
    state.items.clear();
    state.phase = LoadPhase::Loading;
    state.error.clear();
    state.notice.clear();
    source_browse_state().set(state);
    source_load_list(true);
}

pub fn source_back_browse() {
    let mut state = source_browse_state().get();
    if !state.searching {
        return;
    }
    state.searching = false;
    state.keyword.clear();
    state.search_input.clear();
    state.page = 0;
    state.items.clear();
    state.phase = LoadPhase::Loading;
    state.error.clear();
    state.notice.clear();
    source_browse_state().set(state);
    source_load_list(true);
}

// ------------------------------ short drama -----------------------------

pub fn open_duanju() {
    go(Route::Duanju);
    let state = duanju_state().get();
    if state.sites.is_empty() || state.sites_phase == LoadPhase::Error {
        duanju_load_sites(false);
    }
    if state.recommends_phase == LoadPhase::Idle && !state.sites.is_empty() {
        duanju_load_recommends();
    }
}

pub fn duanju_refresh_sites() {
    duanju_load_sites(true);
}

fn duanju_load_sites(force: bool) {
    if !force && duanju_state().get().sites_phase == LoadPhase::Loading {
        return;
    }
    let session_now = session().get();
    let mut state = duanju_state().get();
    state.sites_generation += 1;
    let generation = state.sites_generation;
    state.sites_phase = LoadPhase::Loading;
    state.sites_error.clear();
    duanju_state().set(state);
    if !session_now.connected() {
        let mut state = duanju_state().get();
        state.sites_phase = LoadPhase::Error;
        state.sites_error = "请先连接实例并登录".to_string();
        duanju_state().set(state);
        return;
    }
    let base = session_now.base.clone();
    let token = session_now.token.clone();
    let session_generation = session_now.generation;
    spawn_global(async move {
        let result = api::duanju_sites(&base, &token).await;
        if duanju_state().get().sites_generation != generation
            || session().get().generation != session_generation
        {
            return;
        }
        let mut state = duanju_state().get();
        match result {
            Ok(sites) => {
                let previous = state.source.clone();
                state.sites = sites;
                state.sites_phase = LoadPhase::Ready;
                state.sites_error.clear();
                let keep = state.sites.iter().any(|site| site.key == previous);
                state.source = if keep {
                    previous
                } else {
                    state
                        .sites
                        .first()
                        .map(|site| site.key.clone())
                        .unwrap_or_default()
                };
                duanju_state().set(state);
                if !duanju_state().get().source.is_empty() {
                    duanju_load_categories();
                }
                duanju_load_recommends();
            }
            Err(error) => {
                if error.is_auth() {
                    session_expired();
                }
                state.sites_phase = LoadPhase::Error;
                state.sites_error = error.message();
                duanju_state().set(state);
            }
        }
    });
}

pub fn duanju_select(key: String) {
    let mut state = duanju_state().get();
    if state.source == key {
        return;
    }
    state.source = key;
    state.categories.clear();
    state.parent.clear();
    state.category.clear();
    state.items.clear();
    state.phase = LoadPhase::Loading;
    state.error.clear();
    state.notice.clear();
    state.page = 0;
    state.page_count = 0;
    state.total = 0;
    state.loading_more = false;
    state.generation += 1;
    state.list_generation += 1;
    duanju_state().set(state);
    duanju_load_categories();
}

fn duanju_load_categories() {
    let session_now = session().get();
    let mut state = duanju_state().get();
    if state.source.is_empty() {
        return;
    }
    if !session_now.connected() {
        state.phase = LoadPhase::Error;
        state.error = "请先连接实例并登录".to_string();
        duanju_state().set(state);
        return;
    }
    let source = state.source.clone();
    state.generation += 1;
    let generation = state.generation;
    state.phase = LoadPhase::Loading;
    state.error.clear();
    state.notice.clear();
    state.items.clear();
    state.page = 0;
    state.page_count = 0;
    state.total = 0;
    duanju_state().set(state);
    let base = session_now.base.clone();
    let token = session_now.token.clone();
    let session_generation = session_now.generation;
    spawn_global(async move {
        let result = api::duanju_categories(&base, &token, &source).await;
        if duanju_state().get().generation != generation
            || session().get().generation != session_generation
        {
            return;
        }
        let mut state = duanju_state().get();
        let preferred = state
            .sites
            .iter()
            .find(|site| site.key == source)
            .map(|site| site.type_id.clone())
            .unwrap_or_default();
        match result {
            Ok(categories) => {
                state.categories = categories;
                if state.categories.is_empty() {
                    if !preferred.is_empty() {
                        state.category = preferred;
                        state.parent.clear();
                        duanju_state().set(state);
                        duanju_load_list(true);
                    } else {
                        state.phase = LoadPhase::Ready;
                        state.notice =
                            "这个源没有返回短剧分类；可能是分类缓存过期，稍后刷新".to_string();
                        duanju_state().set(state);
                    }
                    return;
                }
                let (parent, category) =
                    source_browse::default_selection(&state.categories, &preferred);
                state.parent = parent;
                state.category = category;
                duanju_state().set(state);
                duanju_load_list(true);
            }
            Err(error) => {
                if error.is_auth() {
                    session_expired();
                }
                if !preferred.is_empty() {
                    // The source list already told us which category is short
                    // drama; a category endpoint failure must not hide the list.
                    state.category = preferred;
                    state.parent.clear();
                    state.notice = format!("短剧分类加载失败，已用源标记分类：{}", error.message());
                    duanju_state().set(state);
                    duanju_load_list(true);
                } else {
                    state.phase = LoadPhase::Error;
                    state.error = error.message();
                    duanju_state().set(state);
                }
            }
        }
    });
}

pub fn duanju_set_parent(parent: String) {
    let mut state = duanju_state().get();
    if state.parent == parent {
        return;
    }
    let children = source_browse::children(&state.categories, &parent);
    state.parent = parent;
    state.category = children
        .first()
        .map(|child| child.id.clone())
        .unwrap_or_else(|| state.parent.clone());
    state.items.clear();
    state.page = 0;
    state.page_count = 0;
    state.phase = LoadPhase::Loading;
    state.error.clear();
    state.notice.clear();
    state.generation += 1;
    duanju_state().set(state);
    duanju_load_list(true);
}

pub fn duanju_set_category(category: String) {
    let mut state = duanju_state().get();
    if state.category == category {
        return;
    }
    state.category = category;
    state.items.clear();
    state.page = 0;
    state.page_count = 0;
    state.phase = LoadPhase::Loading;
    state.error.clear();
    state.notice.clear();
    state.generation += 1;
    duanju_state().set(state);
    duanju_load_list(true);
}

fn duanju_load_list(reset: bool) {
    let session_now = session().get();
    let mut state = duanju_state().get();
    let source = state.source.clone();
    let category = state.category.clone();
    if source.is_empty() || category.is_empty() {
        return;
    }
    if !session_now.connected() {
        state.phase = LoadPhase::Error;
        state.error = "请先连接实例并登录".to_string();
        duanju_state().set(state);
        return;
    }
    let page = if reset { 1 } else { state.page + 1 };
    state.list_generation += 1;
    let generation = state.list_generation;
    let source_generation = state.generation;
    if reset {
        state.items.clear();
        state.page = 1;
        state.phase = LoadPhase::Loading;
    } else {
        state.loading_more = true;
    }
    state.error.clear();
    duanju_state().set(state);
    let base = session_now.base.clone();
    let token = session_now.token.clone();
    let session_generation = session_now.generation;
    spawn_global(async move {
        let result = api::duanju_videos(&base, &token, &source, &category, page).await;
        let now = duanju_state().get();
        if now.list_generation != generation
            || now.generation != source_generation
            || session().get().generation != session_generation
        {
            return;
        }
        let mut state = duanju_state().get();
        state.loading_more = false;
        match result {
            Ok(page_data) => {
                let added = source_browse::merge(&mut state.items, page_data.items.clone());
                state.page = page_data.page.max(1);
                state.page_count = page_data.page_count.max(state.page);
                state.total = page_data.total;
                state.phase = LoadPhase::Ready;
                if !reset && added == 0 && state.page < state.page_count {
                    state.page_count = state.page;
                    state.notice = "这一页没有新条目，已停止加载".to_string();
                }
                duanju_state().set(state);
            }
            Err(error) => {
                if error.is_auth() {
                    session_expired();
                }
                if reset {
                    state.phase = LoadPhase::Error;
                    state.error = error.message();
                } else {
                    state.notice = format!("加载下一页失败：{}", error.message());
                }
                duanju_state().set(state);
            }
        }
    });
}

pub fn duanju_more() {
    let state = duanju_state().get();
    if state.loading_more || state.page == 0 || state.page >= state.page_count {
        return;
    }
    duanju_load_list(false);
}

pub fn duanju_retry() {
    let state = duanju_state().get();
    if state.sites.is_empty() || state.sites_phase == LoadPhase::Error {
        duanju_load_sites(true);
    } else if state.categories.is_empty() {
        duanju_load_categories();
    } else {
        duanju_load_list(true);
    }
}

fn duanju_load_recommends() {
    let state = duanju_state().get();
    if state.recommends_phase == LoadPhase::Loading || state.recommends_phase == LoadPhase::Ready {
        return;
    }
    let session_now = session().get();
    if !session_now.connected() {
        return;
    }
    let mut state = duanju_state().get();
    state.recommends_phase = LoadPhase::Loading;
    state.recommends_error.clear();
    duanju_state().set(state);
    let base = session_now.base.clone();
    let token = session_now.token.clone();
    let session_generation = session_now.generation;
    spawn_global(async move {
        let result = api::duanju_recommends(&base, &token).await;
        if session().get().generation != session_generation {
            return;
        }
        let mut state = duanju_state().get();
        match result {
            Ok(items) => {
                state.recommends = items;
                state.recommends_phase = LoadPhase::Ready;
            }
            Err(error) => {
                if error.is_auth() {
                    session_expired();
                }
                state.recommends_phase = LoadPhase::Error;
                state.recommends_error = error.message();
            }
        }
        duanju_state().set(state);
    });
}

pub fn duanju_retry_recommends() {
    let mut state = duanju_state().get();
    state.recommends_phase = LoadPhase::Idle;
    duanju_state().set(state);
    duanju_load_recommends();
}

// ------------------------------- detail ---------------------------------

pub fn open_detail(work: Work, source: Option<String>) {
    open_detail_mode(work, source, false);
}

/// Short-drama cards open the same detail, but the detail page offers the
/// portrait autoplay action.
pub fn open_detail_duanju(work: Work, source: Option<String>) {
    open_detail_mode(work, source, true);
}

fn open_detail_mode(mut work: Work, source: Option<String>, duanju: bool) {
    if session().get().demo()
        && let Some(full) = demo::works()
            .into_iter()
            .find(|full| full.key() == work.key())
    {
        work = full;
    }
    let source = source
        .filter(|source| {
            work.candidates
                .iter()
                .any(|candidate| &candidate.source == source)
        })
        .or_else(|| {
            work.candidates
                .first()
                .map(|candidate| candidate.source.clone())
        })
        .unwrap_or_default();
    let mut state = detail_state().get();
    state.work = Some(work);
    state.source = source;
    state.detail = None;
    state.selected_episode = 0;
    state.generation += 1;
    state.phase = LoadPhase::Loading;
    state.error.clear();
    state.duanju = duanju;
    detail_state().set(state);
    go(Route::Detail(detail_state().get().work.clone().unwrap()));
    fetch_detail();
}

pub fn select_source(source: String) {
    let mut state = detail_state().get();
    if state.source == source {
        return;
    }
    state.source = source;
    state.generation += 1;
    state.phase = LoadPhase::Loading;
    state.error.clear();
    detail_state().set(state);
    fetch_detail();
}

pub fn fetch_detail() {
    let state = detail_state().get();
    let Some(work) = state.work.clone() else {
        return;
    };
    let source = state.source.clone();
    let fallback = work
        .candidates
        .iter()
        .find(|candidate| candidate.source == source)
        .cloned();
    if let Some(fallback) = &fallback {
        let mut current = detail_state().get();
        current.detail = Some(fallback.clone());
        if let Some(progress) = library()
            .get()
            .progress
            .iter()
            .find(|p| p.key == fallback.key() && p.episode_index < fallback.episode_count())
        {
            current.selected_episode = progress.episode_index;
        }
        detail_state().set(current);
    }
    if session().get().demo() {
        let source = source.clone();
        let work = work.clone();
        let mut current = detail_state().get();
        current.detail = demo::detail(&work, &source).or(fallback);
        current.phase = LoadPhase::Ready;
        current.error.clear();
        detail_state().set(current);
        return;
    }
    let session_now = session().get();
    if !session_now.connected() {
        let mut current = detail_state().get();
        current.phase = LoadPhase::Error;
        current.error = "请先连接实例并登录".to_owned();
        detail_state().set(current);
        return;
    }
    let generation = state.generation;
    let base = session_now.base.clone();
    let token = session_now.token.clone();
    let id = fallback.as_ref().map(|c| c.id.clone()).unwrap_or_default();
    if id.is_empty() {
        let mut current = detail_state().get();
        current.phase = LoadPhase::Error;
        current.error = "这条搜索结果缺少 ID".to_owned();
        detail_state().set(current);
        return;
    }
    let fallback_for_call = fallback.clone();
    spawn_global(async move {
        let result = api::detail(&base, &token, &source, &id, fallback_for_call.as_ref()).await;
        if detail_state().get().generation != generation {
            return;
        }
        match result {
            Ok(candidate) => {
                let mut current = detail_state().get();
                // A search result may have fresher artwork; keep the fetched
                // detail as the source of truth.
                current.detail = Some(candidate);
                current.phase = LoadPhase::Ready;
                current.error.clear();
                detail_state().set(current);
            }
            Err(error) => {
                if error.is_auth() {
                    session_expired();
                }
                let mut current = detail_state().get();
                current.phase = LoadPhase::Error;
                current.error = error.message();
                detail_state().set(current);
            }
        }
    });
}

pub fn selected_candidate() -> Option<Candidate> {
    detail_state().get().detail
}

pub fn select_episode(index: usize) {
    let mut state = detail_state().get();
    state.selected_episode = index;
    detail_state().set(state);
}

pub fn play_selected(index: usize) {
    let Some(candidate) = selected_candidate() else {
        return;
    };
    start_playback(&candidate, index);
}

pub fn start_playback(candidate: &Candidate, index: usize) {
    start_playback_mode(candidate, index, false);
}

/// Short-drama entry: the same queue and progress, rendered as a 9:16
/// portrait column with autoplay. Called from the detail page's portrait
/// action and from a portrait resume record.
pub fn start_playback_portrait(candidate: &Candidate, index: usize) {
    start_playback_mode(candidate, index, true);
}

fn start_playback_mode(candidate: &Candidate, index: usize, portrait: bool) {
    if candidate.episodes.is_empty() {
        toast("这个片源没有可用剧集".to_owned(), ToastTone::Error);
        return;
    }
    let base = session().get().base;
    let demo = session().get().demo();
    let mut queue = Vec::new();
    for (i, _) in candidate.episodes.iter().enumerate() {
        let Some(raw) = api::media_url(&base, candidate, i) else {
            continue;
        };
        let url = if !demo && !candidate.proxy_mode {
            crate::relay::url(&raw).unwrap_or(raw)
        } else {
            raw
        };
        queue.push(QueueItem {
            index: i,
            title: candidate.episode_title(i),
            url,
        });
    }
    if queue.is_empty() {
        toast("这个片源没有可播放的地址".to_owned(), ToastTone::Error);
        return;
    }
    let queue_index = queue
        .iter()
        .position(|item| item.index == index)
        .unwrap_or(0);
    let selected = queue[queue_index].index;
    let resume = library()
        .get()
        .progress_for(&candidate.key())
        .filter(|p| p.episode_index == selected && !p.completed())
        .map(|p| p.position)
        .unwrap_or(0.0);
    let trace = PlayTrace {
        source: candidate.source.clone(),
        id: candidate.id.clone(),
        source_name: candidate.source_name.clone(),
        title: candidate.title.clone(),
        year: candidate.year.clone(),
        poster: candidate.poster.clone(),
        work_key: format!("{}\u{1f}{}", candidate.title, candidate.year),
        total_episodes: candidate.episodes.len(),
        proxy_mode: candidate.proxy_mode,
    };
    let first_url = queue
        .first()
        .map(|item| item.url.clone())
        .unwrap_or_default();
    let mut state = player().get();
    state.trace = Some(trace);
    state.work = detail_state().get().work.clone().or_else(|| {
        Some(Work {
            title: candidate.title.clone(),
            year: candidate.year.clone(),
            poster: candidate.poster.clone(),
            desc: String::new(),
            genres: Vec::new(),
            candidates: vec![candidate.clone()],
            weight: 0,
        })
    });
    state.queue = queue;
    state.index = queue_index;
    state.load_token += 1;
    state.playing = true;
    state.position = 0.0;
    state.duration = 0.0;
    state.buffering = true;
    state.phase = PlayPhase::Loading;
    state.error.clear();
    state.auto_in = None;
    state.auto_token += 1;
    state.resume_at = resume;
    state.resume_attempts = 0;
    state.resume_done = false;
    state.fullscreen = false;
    state.portrait = portrait;
    state.muted = false;
    state.notice.clear();
    player().set(state);
    go(Route::Player);
    logbook::event(
        "play_start",
        serde_json::json!({
            "source": candidate.source,
            "id": candidate.id,
            "episode": selected,
            "resume_at": resume,
            "proxy_mode": candidate.proxy_mode,
            "url": logbook::safe_url(&first_url),
        }),
    );
}

pub fn play_index(index: usize, resume: bool) {
    let mut state = player().get();
    if index >= state.queue.len() {
        return;
    }
    let episode_index = state.queue[index].index;
    state.index = index;
    state.load_token += 1;
    state.playing = true;
    state.position = 0.0;
    state.duration = 0.0;
    state.buffering = true;
    state.phase = PlayPhase::Loading;
    state.error.clear();
    state.auto_in = None;
    state.auto_token += 1;
    state.resume_at = if resume {
        library()
            .get()
            .progress_for(&trace_key(&state))
            .filter(|p| p.episode_index == episode_index && !p.completed())
            .map(|p| p.position)
            .unwrap_or(0.0)
    } else {
        0.0
    };
    state.resume_attempts = 0;
    state.resume_done = false;
    state.notice.clear();
    player().set(state);
    logbook::event("play_episode", serde_json::json!({"index": index}));
}

fn trace_key(state: &PlayerState) -> String {
    state
        .trace
        .as_ref()
        .map(|trace| format!("{}+{}", trace.source, trace.id))
        .unwrap_or_default()
}

pub fn switch_player_source(source: String) {
    let state = player().get();
    let Some(work) = state.work.clone() else {
        toast("当前播放没有可用的片源列表".to_owned(), ToastTone::Error);
        return;
    };
    let Some(candidate) = work.candidates.iter().find(|c| c.source == source).cloned() else {
        return;
    };
    let current_index = state.current().map(|item| item.index).unwrap_or(0);
    let index = current_index.min(candidate.episode_count().saturating_sub(1));
    let portrait = state.portrait;
    if candidate.episodes.is_empty() {
        if session().get().demo() {
            toast("这个演示片源没有剧集".to_owned(), ToastTone::Error);
            return;
        }
        let session_now = session().get();
        if !session_now.connected() {
            toast("请先重新登录后才能换源".to_owned(), ToastTone::Error);
            return;
        }
        let base = session_now.base.clone();
        let token = session_now.token.clone();
        let generation = session_now.generation;
        let mut player_state = player().get();
        player_state.phase = PlayPhase::Loading;
        player_state.error.clear();
        player_state.buffering = true;
        player().set(player_state);
        spawn_global(async move {
            match api::detail(
                &base,
                &token,
                &candidate.source,
                &candidate.id,
                Some(&candidate),
            )
            .await
            {
                Ok(fetched) => {
                    if session().get().generation != generation {
                        return;
                    }
                    let index = index.min(fetched.episode_count().saturating_sub(1));
                    start_playback_mode(&fetched, index, portrait);
                    toast("已切换到新的片源".to_owned(), ToastTone::Success);
                }
                Err(error) => {
                    if error.is_auth() {
                        session_expired();
                    }
                    observe_error(error.message());
                }
            }
        });
        return;
    }
    start_playback_mode(&candidate, index, portrait);
    toast("已切换到新的片源".to_owned(), ToastTone::Success);
}

pub fn trace_candidate() -> Option<Candidate> {
    let state = player().get();
    let trace = state.trace?;
    Some(Candidate {
        source: trace.source,
        id: trace.id,
        source_name: trace.source_name,
        title: trace.title,
        year: trace.year,
        poster: trace.poster,
        episodes: state.queue.iter().map(|item| item.url.clone()).collect(),
        episode_titles: state.queue.iter().map(|item| item.title.clone()).collect(),
        proxy_mode: trace.proxy_mode,
        ..Candidate::default()
    })
}

pub fn next_episode(auto: bool) {
    let state = player().get();
    if !state.can_next() {
        if auto {
            toast("已经是最后一集".to_owned(), ToastTone::Info);
        }
        return;
    }
    play_index(state.index + 1, true);
}

pub fn prev_episode() {
    let state = player().get();
    if state.can_prev() {
        play_index(state.index - 1, true);
    }
}

pub fn leave_player() {
    let state = player().get();
    if state.trace.is_some() {
        save_progress(true);
    }
    let mut state = player().get();
    state.auto_token += 1;
    state.auto_in = None;
    state.playing = false;
    player().set(state);
    back();
}

pub fn toggle_play() {
    let mut state = player().get();
    if state.phase == PlayPhase::Ended {
        state.playing = true;
        state.load_token += 1;
        state.position = 0.0;
        state.duration = 0.0;
        state.phase = PlayPhase::Loading;
        state.resume_at = 0.0;
        state.resume_done = true;
        state.auto_in = None;
        state.auto_token += 1;
        player().set(state);
        return;
    }
    state.playing = !state.playing;
    let was_playing = state.playing;
    player().set(state);
    if !was_playing {
        save_progress(true);
    }
}

pub fn observe_meta(duration: f64) {
    if duration <= 0.0 {
        return;
    }
    let mut state = player().get();
    if state.duration <= 0.0 || (state.phase == PlayPhase::Loading && !state.playing) {
        state.duration = duration;
    } else if (state.duration - duration).abs() > 0.5 {
        // HLS can report a placeholder duration first. Keep the larger
        // stable value and never reset or seek backwards because of it.
        state.duration = state.duration.max(duration);
    }
    if state.resume_at > duration - 2.0 && state.phase == PlayPhase::Loading {
        state.resume_at = 0.0;
        state.resume_done = true;
    }
    if state.phase == PlayPhase::Loading {
        state.phase = PlayPhase::Ready;
    }
    state.buffering = false;
    player().set(state);
}

pub fn observe_progress(position: f64, duration: f64) {
    let mut state = player().get();
    if duration > 0.0 {
        state.duration = state.duration.max(duration);
    }
    // Persist on pause, leave, end and errors instead of every few seconds:
    // rewriting the persisted library while the video plays recreates the
    // player page and reloads the source, which resets playback position.
    state.position = position.max(0.0);
    state.buffering = false;
    if state.phase == PlayPhase::Loading {
        state.phase = PlayPhase::Ready;
    }
    player().set(state);
}

pub fn observe_buffering(buffering: bool) {
    let mut state = player().get();
    state.buffering = buffering;
    player().set(state);
}

pub fn observe_error(message: impl Into<String>) {
    let message = message.into();
    let mut state = player().get();
    state.playing = false;
    state.phase = PlayPhase::Failed;
    state.error = friendly_play_error(&message);
    state.buffering = false;
    player().set(state);
    save_progress(true);
    logbook::event("play_error", serde_json::json!({"message": message}));
}

fn friendly_play_error(message: &str) -> String {
    let lower = message.to_lowercase();
    if lower.contains("0x80070005")
        || lower.contains("access is denied")
        || lower.contains("拒绝访问")
    {
        "媒体服务器拒绝访问。需要登录的片源应使用实例代理或本次会话中继；演示媒体请确认内置媒体服务已启动。".to_owned()
    } else if lower.contains("0xc00d") || lower.contains("media") {
        format!("播放器无法解码这个媒体：{message}")
    } else if message.trim().is_empty() {
        "播放失败，未收到具体错误".to_owned()
    } else {
        format!("播放失败：{message}")
    }
}

pub fn observe_end() {
    let mut state = player().get();
    state.position = state.duration.max(state.position);
    state.playing = false;
    state.phase = PlayPhase::Ended;
    state.buffering = false;
    let ended_index = state.index;
    let episodes_len = state.queue.len();
    let can_next = state.can_next();
    player().set(state);
    save_progress(true);
    logbook::event(
        "play_end",
        serde_json::json!({"index": ended_index, "episodes": episodes_len}),
    );
    let auto_next = settings().get().auto_next;
    if auto_next && can_next {
        start_auto_next();
    }
}

pub fn start_auto_next() {
    let mut state = player().get();
    if !state.can_next() {
        return;
    }
    state.auto_token += 1;
    state.auto_in = Some(5);
    player().set(state);
}

pub fn cancel_auto_next() {
    let mut state = player().get();
    state.auto_token += 1;
    state.auto_in = None;
    player().set(state);
}

pub fn seek_by(delta: f64) {
    let mut state = player().get();
    if state.duration <= 0.0 {
        return;
    }
    state.position = (state.position + delta).clamp(0.0, state.duration);
    player().set(state);
}

pub fn set_position_after_seek(position: f64) {
    let mut state = player().get();
    state.position = position.clamp(0.0, state.duration.max(position));
    state.phase = PlayPhase::Ready;
    player().set(state);
}

pub fn retry() {
    let mut state = player().get();
    state.load_token += 1;
    state.playing = true;
    state.phase = PlayPhase::Loading;
    state.error.clear();
    state.buffering = true;
    state.resume_at = state.position.max(0.0);
    state.resume_attempts = 0;
    state.resume_done = false;
    state.auto_in = None;
    state.auto_token += 1;
    // A same URL would leave the native video source untouched; force a
    // fresh load with a harmless retry query. The media/relay servers ignore
    // unknown query parameters.
    if let Some(item) = state.queue.get_mut(state.index) {
        let separator = if item.url.contains('?') { '&' } else { '?' };
        item.url = format!("{}{}_r={}", item.url, separator, state.load_token);
    }
    let retried_at = state.position;
    player().set(state);
    logbook::event("play_retry", serde_json::json!({"position": retried_at}));
}

pub fn save_progress(force: bool) {
    let state = player().get();
    let Some(trace) = state.trace.clone() else {
        return;
    };
    let Some(item) = state.current() else { return };
    if state.duration <= 0.0 && state.position <= 0.0 {
        return;
    }
    if !force && state.position < 1.0 {
        return;
    }
    let key = format!("{}+{}", trace.source, trace.id);
    let wire = library()
        .get()
        .progress_for(&key)
        .map(|p| p.wire.clone())
        .unwrap_or(serde_json::Value::Null);
    let entry = ProgressEntry {
        key: key.clone(),
        source: trace.source.clone(),
        id: trace.id.clone(),
        title: trace.title.clone(),
        year: trace.year.clone(),
        poster: trace.poster.clone(),
        source_name: trace.source_name.clone(),
        episode_index: item.index,
        episode_title: item.title.clone(),
        position: state.position,
        duration: state.duration,
        total_episodes: trace.total_episodes,
        updated_at: nsh_core::time::unix_ms() as u64,
        work_key: trace.work_key.clone(),
        duanju: state.portrait,
        synced: None,
        wire,
    };
    let mut lib = library().get();
    lib.upsert_progress(entry.clone());
    library().set(lib);
    save_library_now();
    let now = nsh_core::time::unix_ms() as u64;
    let last = LAST_PROGRESS_PUSH.load(std::sync::atomic::Ordering::Relaxed);
    if force || now.saturating_sub(last) >= PROGRESS_PUSH_INTERVAL_MS {
        LAST_PROGRESS_PUSH.store(now, std::sync::atomic::Ordering::Relaxed);
        push_progress(entry, session().get().generation);
    }
}

static LAST_PROGRESS_PUSH: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
const PROGRESS_PUSH_INTERVAL_MS: u64 = 30_000;

/// Mirrors one local save to the instance. The local entry stays the source
/// for the UI; `synced` only records whether the instance confirmed it.
fn push_progress(entry: ProgressEntry, generation: u64) {
    let state = session().get();
    if state.demo() || !state.connected() || !api::sync_supported() {
        return;
    }
    let base = state.base.clone();
    let token = state.token.clone();
    spawn_global(async move {
        let result = api::save_progress(&base, &token, &entry).await;
        if session().get().generation != generation {
            return;
        }
        match result {
            Ok(()) => {
                let mut lib = library().get();
                lib.set_progress_synced(&entry.key, Some(true));
                library().set(lib);
                save_library_now();
            }
            Err(error) if error.is_auth() => session_expired(),
            Err(error) => {
                let mut lib = library().get();
                lib.set_progress_synced(&entry.key, Some(false));
                library().set(lib);
                save_library_now();
                logbook::event(
                    "progress_sync_failed",
                    serde_json::json!({"key": entry.key, "why": error.message()}),
                );
            }
        }
    });
}

/// Pulls the instance's favorites and watch records after login. Local
/// entries the instance has never seen stay; only older local progress is
/// replaced by the server's copy.
fn pull_instance_library(base: String, token: String, generation: u64) {
    if !api::sync_supported() {
        return;
    }
    spawn_global(async move {
        let favorites = api::favorites(&base, &token).await;
        let records = api::playrecords(&base, &token).await;
        if session().get().generation != generation {
            return;
        }
        if matches!(&favorites, Err(error) if error.is_auth())
            || matches!(&records, Err(error) if error.is_auth())
        {
            session_expired();
            return;
        }
        let mut lib = library().get();
        let mut changed = false;
        if let Ok(entries) = favorites {
            for entry in entries {
                if !lib.is_favorite(&entry.key) {
                    lib.favorites.push(entry);
                } else {
                    lib.set_synced(&entry.key, Some(true));
                }
                changed = true;
            }
            lib.favorites.sort_by_key(|f| std::cmp::Reverse(f.saved_at));
        }
        if let Ok(entries) = records {
            for entry in entries {
                let newer_local = lib
                    .progress_for(&entry.key)
                    .is_some_and(|local| local.updated_at > entry.updated_at);
                if !newer_local {
                    lib.upsert_progress(entry);
                    changed = true;
                }
            }
        }
        if changed {
            library().set(lib);
            save_library_now();
            logbook::event("library_pulled", serde_json::json!({"records": true}));
        }
    });
}

pub fn clear_history() {
    let mut lib = library().get();
    lib.clear_history();
    library().set(lib);
    save_library_now();
    toast("已清除当前资料的本机历史".to_owned(), ToastTone::Info);
    logbook::event(
        "history_clear",
        serde_json::json!({"profile": session().get().profile}),
    );
}

pub fn clear_all_local() {
    let mut lib = library().get();
    lib.clear_all();
    library().set(lib);
    save_library_now();
    toast("已清除当前资料的本机收藏与历史".to_owned(), ToastTone::Info);
}

/// Starts playback from a saved progress entry; the detail is fetched again
/// because signed media URLs expire.
pub fn resume_progress(entry: &ProgressEntry) {
    let entry = entry.clone();
    if session().get().demo() {
        let works = demo::works();
        if let Some(work) = works.into_iter().find(|work| work.key() == entry.work_key)
            && let Some(candidate) = work
                .candidates
                .iter()
                .find(|candidate| candidate.source == entry.source)
        {
            let candidate = candidate.clone();
            // The catalog may have been regenerated; use the saved
            // episode identity only when it still exists.
            let index = entry
                .episode_index
                .min(candidate.episodes.len().saturating_sub(1));
            start_playback(&candidate, index);
            return;
        }
        toast(
            "这条演示记录对应的片源已不存在".to_owned(),
            ToastTone::Error,
        );
        return;
    }
    let candidate = Candidate {
        source: entry.source.clone(),
        id: entry.id.clone(),
        source_name: entry.source_name.clone(),
        title: entry.title.clone(),
        year: entry.year.clone(),
        poster: entry.poster.clone(),
        ..Candidate::default()
    };
    if !session().get().connected() {
        toast("请先重新登录，再继续观看".to_owned(), ToastTone::Error);
        go(Route::Connect);
        return;
    }
    let base = session().get().base;
    let token = session().get().token;
    let mut detail = detail_state().get();
    detail.work = Some(entry_to_work(&entry));
    detail.source = entry.source.clone();
    detail.detail = Some(candidate.clone());
    detail.phase = LoadPhase::Loading;
    detail.generation += 1;
    detail.selected_episode = entry.episode_index;
    detail_state().set(detail);
    go(Route::Player);
    let mut player_state = player().get();
    player_state.trace = Some(PlayTrace {
        source: entry.source.clone(),
        id: entry.id.clone(),
        source_name: entry.source_name.clone(),
        title: entry.title.clone(),
        year: entry.year.clone(),
        poster: entry.poster.clone(),
        work_key: entry.work_key.clone(),
        total_episodes: entry.total_episodes,
        proxy_mode: false,
    });
    player_state.work = Some(entry_to_work(&entry));
    player_state.queue.clear();
    player_state.phase = PlayPhase::Loading;
    player_state.error.clear();
    player_state.playing = true;
    player_state.portrait = entry.duanju;
    player_state.load_token += 1;
    player_state.position = 0.0;
    player_state.duration = 0.0;
    player_state.resume_at = if entry.completed() {
        0.0
    } else {
        entry.position
    };
    player_state.resume_attempts = 0;
    player_state.resume_done = false;
    player_state.index = 0;
    player_state.auto_in = None;
    player_state.auto_token += 1;
    player().set(player_state);
    spawn_global(async move {
        match api::detail(&base, &token, &entry.source, &entry.id, Some(&candidate)).await {
            Ok(fetched) => {
                if player().get().load_token == 0 {
                    return;
                }
                let index = entry
                    .episode_index
                    .min(fetched.episodes.len().saturating_sub(1));
                start_playback_mode(&fetched, index, entry.duanju);
                let mut state = player().get();
                state.resume_at = if entry.completed() {
                    0.0
                } else {
                    entry.position
                };
                state.resume_attempts = 0;
                state.resume_done = false;
                player().set(state);
            }
            Err(error) => {
                if error.is_auth() {
                    session_expired();
                }
                observe_error(error.message());
            }
        }
    });
}

fn entry_to_work(entry: &ProgressEntry) -> Work {
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

/// Opens the detail of a favorite or history card, refreshing it.
pub fn open_from_library(entry: &FavoriteEntry) {
    let work = entry.work();
    open_detail(work, Some(entry.source.clone()));
}

pub fn storage_usage() -> (usize, usize) {
    let lib = library().get();
    (lib.favorites.len(), lib.progress.len())
}

pub fn profile_count() -> usize {
    Storage::get_all_keys()
        .iter()
        .filter(|key| key.starts_with("moontv.library.v1."))
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate(id: &str, title: &str) -> Candidate {
        Candidate {
            source: "fixture-a".into(),
            id: id.into(),
            title: title.into(),
            year: "2026".into(),
            source_name: "夹具 A".into(),
            ..Candidate::default()
        }
    }

    #[test]
    fn fallback_keeps_streamed_results_and_dedupes() {
        let mut state = SearchState {
            results: vec![candidate("1", "回声档案")],
            partial: true,
            ..SearchState::default()
        };
        let added = absorb_fallback(
            &mut state,
            vec![candidate("1", "回声档案"), candidate("2", "暮色电台")],
            "片源响应超时",
        );
        assert_eq!(added, 1, "only the new item is merged");
        assert_eq!(state.results.len(), 2, "the early source is never lost");
        assert_eq!(state.works.len(), 2);
        assert_eq!(state.phase, LoadPhase::Ready);
        assert!(!state.partial);
        assert!(state.notice.contains("降级"), "{}", state.notice);
    }

    #[test]
    fn a_failed_fallback_keeps_arrived_results_partial() {
        let mut state = SearchState {
            results: vec![candidate("1", "回声档案")],
            ..SearchState::default()
        };
        keep_partial_after_failure(&mut state, "片源响应超时", "实例返回 HTTP 500");
        assert_eq!(state.results.len(), 1);
        assert_eq!(state.works.len(), 1);
        assert_eq!(state.phase, LoadPhase::Ready);
        assert!(state.partial);
        assert!(state.notice.contains("仅显示已到达的结果"));
    }

    #[test]
    fn fallback_into_an_empty_search_reports_no_results() {
        let mut state = SearchState::default();
        let added = absorb_fallback(&mut state, Vec::new(), "流式搜索连接提前结束");
        assert_eq!(added, 0);
        assert_eq!(state.phase, LoadPhase::Ready);
        assert!(state.notice.contains("没有返回结果"));
    }
}
