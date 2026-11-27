//! The product's domain shapes: works from search sources, episodes,
//! favorites, local progress, and the persisted library. No HTTP or UI code
//! lives here; both sides speak these types.

use std::collections::HashMap;

use nsh_core::storage::Storage;
use serde::{Deserialize, Serialize};

/// One search result: an entry of one source. Its identity is
/// `source + id`, never the title.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Candidate {
    pub source: String,
    pub id: String,
    pub source_name: String,
    pub title: String,
    pub year: String,
    pub poster: String,
    pub desc: String,
    pub kind: String,
    pub remarks: String,
    pub episodes: Vec<String>,
    pub episode_titles: Vec<String>,
    pub proxy_mode: bool,
    pub weight: i64,
    pub episode_headers: Vec<std::collections::BTreeMap<String, String>>,
}

impl Candidate {
    pub fn key(&self) -> String {
        format!("{}+{}", self.source, self.id)
    }

    pub fn episode_count(&self) -> usize {
        self.episodes.len().max(self.episode_titles.len())
    }

    pub fn episode_title(&self, index: usize) -> String {
        self.episode_titles
            .get(index)
            .filter(|s| !s.trim().is_empty())
            .cloned()
            .unwrap_or_else(|| format!("第 {} 集", index + 1))
    }
}

/// A work on screen: candidates that share an exact title and year. Same
/// titles from different years or sources stay apart unless both match;
/// a visual group never becomes a playback identity.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Work {
    pub title: String,
    pub year: String,
    pub poster: String,
    pub desc: String,
    pub genres: Vec<String>,
    pub candidates: Vec<Candidate>,
    pub weight: i64,
}

impl Work {
    pub fn key(&self) -> String {
        format!("{}\u{1f}{}", self.title, self.year)
    }

    pub fn from_candidates(mut candidates: Vec<Candidate>) -> Option<Self> {
        candidates
            .retain(|c| !c.source.is_empty() && !c.id.is_empty() && !c.title.trim().is_empty());
        if candidates.is_empty() {
            return None;
        }
        candidates.sort_by(|a, b| {
            b.weight
                .cmp(&a.weight)
                .then_with(|| a.source_name.cmp(&b.source_name))
                .then_with(|| a.source.cmp(&b.source))
        });
        let first = &candidates[0];
        let poster = candidates
            .iter()
            .find(|c| !c.poster.is_empty())
            .map(|c| c.poster.clone())
            .unwrap_or_default();
        let desc = candidates
            .iter()
            .find(|c| !c.desc.trim().is_empty())
            .map(|c| c.desc.clone())
            .unwrap_or_default();
        let genres = first
            .kind
            .split(['/', ',', '，', '、'])
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_owned)
            .take(3)
            .collect();
        Some(Self {
            title: first.title.trim().to_owned(),
            year: first.year.trim().to_owned(),
            poster,
            desc,
            genres,
            weight: candidates.iter().map(|c| c.weight).max().unwrap_or(0),
            candidates,
        })
    }
}

/// Merges results by `source+id`, keeping the first copy. The count is how
/// many were new, so a repeated page cannot grow the list silently.
pub fn merge_candidates(existing: &mut Vec<Candidate>, incoming: Vec<Candidate>) -> usize {
    let mut added = 0;
    for candidate in incoming {
        if existing.iter().any(|known| known.key() == candidate.key()) {
            continue;
        }
        existing.push(candidate);
        added += 1;
    }
    added
}

/// Grouping key: case and punctuation insensitive so "庆余年 第一季" and
/// "庆余年第一季" stay one work, while season and audio markers are kept so
/// different seasons and Cantonese/Mandarin cuts do not collapse together.
fn group_key(raw: &str) -> String {
    let lower = raw.to_lowercase();
    let mut out: String = lower.chars().filter(|c| c.is_alphanumeric()).collect();
    for token in ["4k", "1080p", "2160p", "高清", "蓝光", "完整版"] {
        out = out.replace(token, "");
    }
    out
}

/// The source a work opens with: the remembered one when it still exists,
/// then an explicit source from the caller, then the first candidate in the
/// existing sorted order. Never invents a source that is not in the work.
pub fn preferred_source(
    work: &Work,
    remembered: Option<&str>,
    explicit: Option<&str>,
) -> Option<String> {
    let valid = |source: &str| work.candidates.iter().any(|c| c.source == source);
    if let Some(source) = remembered.filter(|source| valid(source)) {
        return Some(source.to_string());
    }
    if let Some(source) = explicit.filter(|source| valid(source)) {
        return Some(source.to_string());
    }
    work.candidates.first().map(|c| c.source.clone())
}

/// The next candidate to try after a playback failure. `tried` includes the
/// current source; candidates keep the work's existing weight order.
pub fn next_backup_candidate(work: &Work, current: &str, tried: &[String]) -> Option<Candidate> {
    work.candidates
        .iter()
        .find(|c| c.source != current && !tried.iter().any(|seen| seen == &c.source))
        .cloned()
}

/// Groups results for display. Identity is still each candidate; grouping
/// needs an equal normalised title and an equal year.
pub fn group_results(results: Vec<Candidate>) -> Vec<Work> {
    let mut seen = HashMap::new();
    let mut groups: Vec<(String, String, Vec<Candidate>)> = Vec::new();
    for candidate in results {
        let key = candidate.key();
        if seen.contains_key(&key) {
            continue;
        }
        seen.insert(key, true);
        let title_key = group_key(&candidate.title);
        let year_key = candidate.year.trim();
        if let Some((_, _, list)) = groups
            .iter_mut()
            .find(|(t, y, _)| *t == title_key && y == year_key)
        {
            list.push(candidate);
        } else {
            groups.push((title_key, year_key.to_owned(), vec![candidate]));
        }
    }
    let mut works: Vec<Work> = groups
        .into_iter()
        .filter_map(|(_, _, list)| Work::from_candidates(list))
        .collect();
    works.sort_by(|a, b| {
        b.weight
            .cmp(&a.weight)
            .then_with(|| b.year.cmp(&a.year))
            .then_with(|| a.title.cmp(&b.title))
    });
    works
}

/// Case/punctuation-insensitive form used for relevance, with common
/// release-tag noise removed so "庆余年 第二季 4K" still matches "庆余年".
fn search_key(raw: &str) -> String {
    let lower = raw.to_lowercase();
    let mut out: String = lower.chars().filter(|c| c.is_alphanumeric()).collect();
    for token in [
        "第二季",
        "第三季",
        "第四季",
        "第一季",
        "第2季",
        "第3季",
        "第4季",
        "第1季",
        "season2",
        "season3",
        "season4",
        "season1",
        "4k",
        "1080p",
        "2160p",
        "国语",
        "粤语",
        "中字",
        "无删减",
        "未删减",
        "完整版",
        "合集",
    ] {
        out = out.replace(token, "");
    }
    out
}

fn relevance(title: &str, query: &str) -> i32 {
    let q = search_key(query);
    if q.is_empty() {
        return 0;
    }
    let raw_title = title.trim().to_lowercase();
    let raw_query = query.trim().to_lowercase();
    if raw_title == raw_query {
        // The plain title beats anything carrying release tags.
        return 200;
    }
    let t = search_key(title);
    if t == q {
        120
    } else if t.starts_with(&q) {
        90
    } else if t.contains(&q) {
        60
    } else {
        0
    }
}

fn looks_like_noise(title: &str) -> bool {
    let t = title.to_lowercase();
    [
        "预告",
        "花絮",
        "解说",
        "速看",
        "混剪",
        "片段",
        "合集",
        "reaction",
        "拍摄花絮",
    ]
    .iter()
    .any(|marker| t.contains(marker))
}

/// Orders grouped works for a query: the closest title first, then the one
/// carried by more sources, then the newer year. Trailers and clips are
/// pushed down rather than dropped.
pub fn rank_works(mut works: Vec<Work>, query: &str) -> Vec<Work> {
    works.sort_by(|a, b| {
        let score = |w: &Work| {
            let mut value =
                relevance(&w.title, query) * 100 + (w.candidates.len().min(20) as i32) * 5;
            if let Ok(year) = w.year.trim().parse::<i32>() {
                value += (year - 1900).clamp(0, 200);
            }
            if looks_like_noise(&w.title) {
                value -= 5000;
            }
            value
        };
        score(b)
            .cmp(&score(a))
            .then_with(|| b.weight.cmp(&a.weight))
            .then_with(|| b.year.cmp(&a.year))
            .then_with(|| a.title.cmp(&b.title))
    });
    works
}

/// A favorite as this app keeps it locally. `synced` says whether the
/// instance confirmed it (`None`: demo or not attempted).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct FavoriteEntry {
    pub key: String,
    pub source: String,
    pub id: String,
    pub title: String,
    pub year: String,
    pub poster: String,
    pub source_name: String,
    pub total_episodes: usize,
    pub saved_at: u64,
    #[serde(default)]
    pub synced: Option<bool>,
    #[serde(default)]
    pub wire: serde_json::Value,
}

impl FavoriteEntry {
    pub fn from_candidate(candidate: &Candidate) -> Self {
        Self {
            key: candidate.key(),
            source: candidate.source.clone(),
            id: candidate.id.clone(),
            title: candidate.title.clone(),
            year: candidate.year.clone(),
            poster: candidate.poster.clone(),
            source_name: candidate.source_name.clone(),
            total_episodes: candidate.episode_count(),
            saved_at: nsh_core::time::unix_ms() as u64,
            synced: None,
            wire: serde_json::json!({"origin":"vod","vod_remarks":candidate.remarks}),
        }
    }

    pub fn work(&self) -> Work {
        Work {
            title: self.title.clone(),
            year: self.year.clone(),
            poster: self.poster.clone(),
            desc: String::new(),
            genres: Vec::new(),
            candidates: vec![Candidate {
                source: self.source.clone(),
                id: self.id.clone(),
                source_name: self.source_name.clone(),
                title: self.title.clone(),
                year: self.year.clone(),
                poster: self.poster.clone(),
                episodes: Vec::new(),
                ..Candidate::default()
            }],
            weight: 0,
        }
    }
}

/// The last place in one work the user watched, and enough to resume.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ProgressEntry {
    pub key: String,
    pub source: String,
    pub id: String,
    pub title: String,
    pub year: String,
    pub poster: String,
    pub source_name: String,
    pub episode_index: usize,
    pub episode_title: String,
    pub position: f64,
    pub duration: f64,
    pub total_episodes: usize,
    pub updated_at: u64,
    pub work_key: String,
    /// True when this record came from the short-drama portrait player; the
    /// server keeps `is_duanju` and the client uses it to resume in portrait.
    #[serde(default)]
    pub duanju: bool,
    #[serde(default)]
    pub synced: Option<bool>,
    #[serde(default)]
    pub wire: serde_json::Value,
}

impl ProgressEntry {
    pub fn percent(&self) -> f32 {
        if self.duration <= 0.0 {
            return 0.0;
        }
        (self.position / self.duration).clamp(0.0, 1.0) as f32
    }

    pub fn completed(&self) -> bool {
        self.duration > 0.0 && self.position >= self.duration - 4.0
    }
}

/// Everything kept for one profile (demo, or an instance's account).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Library {
    #[serde(default)]
    pub favorites: Vec<FavoriteEntry>,
    #[serde(default)]
    pub progress: Vec<ProgressEntry>,
    /// Last source the user picked or played for a work, keyed by the work's
    /// `title\u{1f}year`. Local only: never sent to the instance.
    #[serde(default)]
    pub last_sources: std::collections::BTreeMap<String, String>,
}

impl Library {
    pub fn favorite(&self, key: &str) -> Option<&FavoriteEntry> {
        self.favorites.iter().find(|f| f.key == key)
    }

    pub fn is_favorite(&self, key: &str) -> bool {
        self.favorite(key).is_some()
    }

    /// Adds or removes `entry`; the answer is whether it is now saved.
    pub fn toggle_favorite(&mut self, entry: FavoriteEntry) -> bool {
        if let Some(index) = self.favorites.iter().position(|f| f.key == entry.key) {
            self.favorites.remove(index);
            false
        } else {
            self.favorites.push(entry);
            self.favorites
                .sort_by_key(|f| std::cmp::Reverse(f.saved_at));
            true
        }
    }

    pub fn remove_favorite(&mut self, key: &str) {
        self.favorites.retain(|f| f.key != key);
    }

    pub fn set_synced(&mut self, key: &str, synced: Option<bool>) {
        if let Some(favorite) = self.favorites.iter_mut().find(|f| f.key == key) {
            favorite.synced = synced;
        }
    }

    pub fn set_progress_synced(&mut self, key: &str, synced: Option<bool>) {
        if let Some(entry) = self.progress.iter_mut().find(|p| p.key == key) {
            entry.synced = synced;
        }
    }

    pub fn upsert_progress(&mut self, entry: ProgressEntry) {
        self.progress.retain(|p| p.key != entry.key);
        self.progress.push(entry);
        self.progress
            .sort_by_key(|p| std::cmp::Reverse(p.updated_at));
        self.progress.truncate(300);
    }

    pub fn progress_for(&self, key: &str) -> Option<&ProgressEntry> {
        self.progress.iter().find(|p| p.key == key)
    }

    pub fn last_source(&self, work_key: &str) -> Option<&str> {
        self.last_sources.get(work_key).map(String::as_str)
    }

    pub fn remember_source(&mut self, work_key: &str, source: &str) {
        if work_key.is_empty() || source.is_empty() {
            return;
        }
        self.last_sources
            .insert(work_key.to_string(), source.to_string());
    }

    /// Unfinished entries, newest first: what "continue watching" shows.
    pub fn continue_watching(&self) -> Vec<ProgressEntry> {
        let mut entries: Vec<ProgressEntry> = self
            .progress
            .iter()
            .filter(|p| p.position >= 5.0 && !p.completed())
            .cloned()
            .collect();
        entries.sort_by_key(|e| std::cmp::Reverse(e.updated_at));
        entries
    }

    pub fn clear_history(&mut self) {
        self.progress.clear();
    }

    pub fn clear_all(&mut self) {
        self.favorites.clear();
        self.progress.clear();
        self.last_sources.clear();
    }
}

/// Settings that may persist. Credentials (password, session token) are
/// deliberately absent.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub instance: String,
    pub demo_active: bool,
    pub auto_next: bool,
    pub volume: f32,
    /// Playback speed, one of `PLAYBACK_RATES`; stored per user.
    #[serde(default = "default_playback_rate")]
    pub playback_rate: f32,
    /// Seconds to skip at the start of every episode; 0 disables the window.
    #[serde(default)]
    pub intro_seconds: f64,
    /// Seconds before the end where the next-episode shortcut appears; 0
    /// disables it.
    #[serde(default)]
    pub outro_seconds: f64,
    /// Seek past the intro automatically when inside the intro window.
    #[serde(default)]
    pub auto_skip_intro: bool,
    pub last_username: String,
    pub show_spoilers: bool,
    /// Remembered discovery filters, keyed by `kind.slot` (`movie.primary`,
    /// `anime.filters.region`, ...). Stable option ids, not labels.
    #[serde(default)]
    pub discover: std::collections::BTreeMap<String, String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            instance: String::new(),
            demo_active: false,
            auto_next: true,
            volume: 0.8,
            playback_rate: 1.0,
            intro_seconds: 0.0,
            outro_seconds: 0.0,
            auto_skip_intro: false,
            last_username: String::new(),
            show_spoilers: false,
            discover: std::collections::BTreeMap::new(),
        }
    }
}

// Namespaced so a stray MoonTVPlus NSH build sharing the host's data folder
// can never be read as this product's settings or library.
const SETTINGS_KEY: &str = "moontv.nsh.settings.v1";

fn default_playback_rate() -> f32 {
    1.0
}

/// Instance the client offers on first launch. Override at build time with
/// `MOONTV_DEFAULT_INSTANCE`; users can still type a self-hosted address.
pub const DEFAULT_INSTANCE: &str = match option_env!("MOONTV_DEFAULT_INSTANCE") {
    Some(value) => value,
    None => "https://moontv-plus.zhuning978.workers.dev",
};

pub fn load_settings() -> Settings {
    Storage::get_item(SETTINGS_KEY)
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

pub fn save_settings(settings: &Settings) {
    match serde_json::to_string(settings) {
        Ok(raw) => {
            if let Err(full) = Storage::set_item(SETTINGS_KEY, raw) {
                log::warn!("settings storage refused: {full}");
            }
        }
        Err(error) => log::warn!("could not encode settings: {error}"),
    }
}

/// The storage namespace of a profile: demo data is never mixed with an
/// account's, and two accounts on one instance are kept apart.
pub fn profile_for(base: &str, username: &str) -> String {
    if base == "demo" || base.is_empty() {
        return "demo".to_owned();
    }
    use sha2::{Digest, Sha256};
    let canonical =
        normalize_instance(base).unwrap_or_else(|_| base.trim_end_matches('/').to_owned());
    let digest = Sha256::digest(format!("{canonical}\u{0}{username}").as_bytes());
    format!("acct-{digest:x}")
}

fn library_key(profile: &str) -> String {
    format!("moontv.nsh.library.v1.{profile}")
}

pub fn load_library(profile: &str) -> Library {
    Storage::get_item(&library_key(profile))
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

pub fn save_library(profile: &str, library: &Library) {
    match serde_json::to_string(library) {
        Ok(raw) => {
            if let Err(full) = Storage::set_item(library_key(profile), raw) {
                log::warn!("library storage refused: {full}");
            }
        }
        Err(error) => log::warn!("could not encode library: {error}"),
    }
}

fn search_history_key(profile: &str) -> String {
    format!("moontv.nsh.searchhistory.v1.{profile}")
}

/// The local copy of search history: the server's list when it answers,
/// this cache when it does not. Kept per profile so demo history never
/// leaks into an account's list.
pub fn load_search_history(profile: &str) -> Vec<String> {
    Storage::get_item(&search_history_key(profile))
        .and_then(|raw| serde_json::from_str::<Vec<String>>(&raw).ok())
        .map(|items| {
            let mut out: Vec<String> = Vec::new();
            for item in items {
                let item = item.trim().to_string();
                if !item.is_empty() && !out.iter().any(|known| known == &item) {
                    out.push(item);
                }
            }
            out.truncate(20);
            out
        })
        .unwrap_or_default()
}

pub fn save_search_history(profile: &str, history: &[String]) {
    if let Ok(raw) = serde_json::to_string(history)
        && let Err(full) = Storage::set_item(search_history_key(profile), raw)
    {
        log::warn!("search history storage refused: {full}");
    }
}

/// `h:mm:ss`, or `mm:ss` under an hour. Unknown durations show `--:--`.
pub fn format_time(seconds: f64) -> String {
    if !seconds.is_finite() || seconds < 0.0 {
        return "--:--".to_owned();
    }
    let total = seconds.round() as u64;
    let h = total / 3600;
    let m = (total % 3600) / 60;
    let s = total % 60;
    if h > 0 {
        format!("{h}:{m:02}:{s:02}")
    } else {
        format!("{m:02}:{s:02}")
    }
}

/// The playback speeds offered by the player. Keep this list small and
/// ordered; the UI and the `[` / `]` keyboard steps share it.
pub const PLAYBACK_RATES: [f32; 6] = [0.5, 0.75, 1.0, 1.25, 1.5, 2.0];

/// A persisted rate may come from an older or hand-edited settings file.
/// Only the six supported rates are accepted; anything else means 1x.
pub fn normalize_rate(rate: f32) -> f32 {
    PLAYBACK_RATES
        .iter()
        .copied()
        .find(|candidate| (*candidate - rate).abs() < 0.001)
        .unwrap_or(1.0)
}

pub fn step_rate(rate: f32, delta: i32) -> f32 {
    let current = normalize_rate(rate);
    let index = PLAYBACK_RATES
        .iter()
        .position(|candidate| (*candidate - current).abs() < 0.001)
        .unwrap_or(2) as i32;
    let next = (index + delta).clamp(0, PLAYBACK_RATES.len() as i32 - 1);
    PLAYBACK_RATES[next as usize]
}

pub fn format_rate(rate: f32) -> String {
    let value = format!("{:.2}", normalize_rate(rate));
    let value = value.trim_end_matches('0').trim_end_matches('.');
    format!("{value}×")
}

/// Where the intro skip should land, if the window is meaningful. A zero,
/// unknown, or overlong intro must not seek to the end or past it.
pub fn intro_skip_target(duration: f64, intro_seconds: f64) -> Option<f64> {
    if !duration.is_finite()
        || !intro_seconds.is_finite()
        || duration <= 0.0
        || intro_seconds <= 0.0
        || intro_seconds >= duration - 0.25
    {
        return None;
    }
    Some(intro_seconds)
}

pub fn in_intro_window(position: f64, duration: f64, intro_seconds: f64) -> bool {
    intro_skip_target(duration, intro_seconds)
        .is_some_and(|target| position >= 0.0 && position < target)
}

/// True only inside a finite outro window that does not cover the whole
/// episode. At exactly the end the end screen owns the UI.
pub fn in_outro_window(position: f64, duration: f64, outro_seconds: f64) -> bool {
    if !position.is_finite()
        || !duration.is_finite()
        || !outro_seconds.is_finite()
        || duration <= 0.0
        || outro_seconds <= 0.0
        || outro_seconds >= duration
    {
        return false;
    }
    position >= duration - outro_seconds && position < duration
}

pub fn percent_label(percent: f32) -> String {
    format!("{}%", (percent.clamp(0.0, 1.0) * 100.0).round() as u32)
}

/// A base URL with a trailing slash, validated: HTTP(S), no credentials,
/// no query or fragment; a path is kept for sub-path deployments.
pub fn normalize_instance(input: &str) -> Result<String, String> {
    let raw = input.trim();
    if raw.is_empty() {
        return Err("请输入实例地址".to_owned());
    }
    let with_scheme = if raw.contains("://") {
        raw.to_owned()
    } else {
        format!("https://{raw}")
    };
    let mut url = url::Url::parse(&with_scheme).map_err(|_| "实例地址格式不正确".to_owned())?;
    if !matches!(url.scheme(), "http" | "https") {
        return Err("实例只支持 http 或 https".to_owned());
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err("实例地址不能包含账号或密码".to_owned());
    }
    if url.query().is_some() || url.fragment().is_some() {
        return Err("实例地址不能带查询参数或锚点".to_owned());
    }
    if url.host_str().unwrap_or("").is_empty() {
        return Err("实例地址缺少主机名".to_owned());
    }
    let path = url.path().trim_end_matches('/').to_owned();
    url.set_path(
        &(if path.is_empty() {
            "/".to_owned()
        } else {
            format!("{path}/")
        }),
    );
    Ok(url.to_string())
}

/// The host (and port) of an instance for labels.
pub fn instance_host(base: &str) -> String {
    url::Url::parse(base)
        .ok()
        .and_then(|u| {
            let host = u.host_str()?.to_owned();
            match u.port() {
                Some(port) => Some(format!("{host}:{port}")),
                None => Some(host),
            }
        })
        .unwrap_or_else(|| base.trim_end_matches('/').to_owned())
}

pub fn api_url(base: &str, path: &str) -> String {
    format!(
        "{}/{}",
        base.trim_end_matches('/'),
        path.trim_start_matches('/')
    )
}

/// Percent-encodes one query value (UTF-8), leaving only unreserved bytes.
pub fn encode_query(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for byte in value.as_bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(*byte as char)
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

/// Resolves a URL from the API: absolute, protocol-relative, root-relative
/// or relative to the instance base.
pub fn resolve_url(base: &str, value: &str) -> String {
    let value = value.trim();
    if value.is_empty() {
        return String::new();
    }
    if value.starts_with("http://") || value.starts_with("https://") {
        return value.to_owned();
    }
    if let Some(rest) = value.strip_prefix("//") {
        let scheme = if base.starts_with("https://") {
            "https"
        } else {
            "http"
        };
        return format!("{scheme}://{rest}");
    }
    let Ok(parsed) = url::Url::parse(base) else {
        return value.to_owned();
    };
    if let Ok(joined) = parsed.join(value) {
        return joined.to_string();
    }
    format!("{}{}", base.trim_end_matches('/'), value)
}

pub fn same_origin(a: &str, b: &str) -> bool {
    match (url::Url::parse(a), url::Url::parse(b)) {
        (Ok(a), Ok(b)) => {
            a.scheme() == b.scheme()
                && a.host_str() == b.host_str()
                && a.port_or_known_default() == b.port_or_known_default()
        }
        _ => false,
    }
}

/// A stable key for logs and storage, without secrets.
pub fn short_hash(value: &str) -> String {
    let mut hash: u32 = 2166136261;
    for b in value.as_bytes() {
        hash ^= *b as u32;
        hash = hash.wrapping_mul(16777619);
    }
    format!("{hash:08x}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate(title: &str, year: &str, source: &str, id: &str) -> Candidate {
        Candidate {
            title: title.into(),
            year: year.into(),
            source: source.into(),
            id: id.into(),
            source_name: source.into(),
            ..Candidate::default()
        }
    }

    #[test]
    fn ranks_the_exact_title_first_and_sinks_trailers() {
        let works = group_results(vec![
            candidate("庆余年 第二季 4K", "2024", "a", "1"),
            candidate("庆余年", "2019", "b", "2"),
            candidate("庆余年预告片", "2019", "c", "3"),
            candidate("庆余年 合集", "2022", "d", "4"),
        ]);
        let ranked = rank_works(works, "庆余年");
        assert_eq!(ranked[0].title, "庆余年");
        assert_eq!(ranked.last().unwrap().title, "庆余年预告片");
    }

    #[test]
    fn more_sources_break_a_title_tie() {
        let works = group_results(vec![
            candidate("庆余年", "2019", "a", "1"),
            candidate("庆余年", "2019", "b", "2"),
            candidate("庆余年", "2019", "c", "3"),
            candidate("庆余年", "2024", "d", "4"),
        ]);
        let ranked = rank_works(works, "庆余年");
        assert_eq!(ranked[0].year, "2019");
        assert_eq!(ranked[0].candidates.len(), 3);
    }

    #[test]
    fn groups_only_exact_title_and_year() {
        let works = group_results(vec![
            candidate("夜行", "2024", "a", "1"),
            candidate("夜行", "2024", "b", "2"),
            candidate("夜行", "2023", "c", "3"),
            candidate("夜行 第二季", "2024", "a", "4"),
            candidate("夜行", "2024", "a", "1"),
        ]);
        assert_eq!(works.len(), 3);
        let exact = works
            .iter()
            .find(|w| w.year == "2024" && w.title == "夜行")
            .unwrap();
        assert_eq!(exact.candidates.len(), 2);
    }

    #[test]
    fn profile_separates_instances_and_accounts() {
        let a = profile_for("https://tv.example/", "alice");
        let b = profile_for("https://tv.example/", "bob");
        let c = profile_for("https://other.example/", "alice");
        assert_ne!(a, b);
        assert_ne!(a, c);
        assert_ne!(profile_for("https://tv.example", "ALICE"), a);
        assert_eq!(profile_for("demo", "x"), "demo");
    }

    #[test]
    fn normalizes_instances() {
        assert_eq!(
            normalize_instance("tv.example").unwrap(),
            "https://tv.example/"
        );
        assert_eq!(
            normalize_instance("http://127.0.0.1:8080/moon/").unwrap(),
            "http://127.0.0.1:8080/moon/"
        );
        assert!(normalize_instance("https://tv.example/?a=1").is_err());
        assert!(normalize_instance("ftp://tv.example").is_err());
        assert!(normalize_instance("https://u:p@tv.example").is_err());
    }

    #[test]
    fn merges_results_without_duplicates() {
        let mut existing = vec![candidate("回声", "2026", "a", "1")];
        let added = merge_candidates(
            &mut existing,
            vec![
                candidate("回声", "2026", "a", "1"),
                candidate("回声", "2026", "a", "2"),
            ],
        );
        assert_eq!(added, 1);
        assert_eq!(existing.len(), 2);
        assert_eq!(existing[1].id, "2");
    }

    #[test]
    fn remembers_one_source_per_work_locally() {
        let mut library = Library::default();
        assert_eq!(library.last_source("回声\u{1f}2026"), None);
        library.remember_source("回声\u{1f}2026", "fixture-a");
        library.remember_source("回声\u{1f}2026", "fixture-c");
        assert_eq!(library.last_source("回声\u{1f}2026"), Some("fixture-c"));
        library.remember_source("另一个\u{1f}2025", "fixture-a");
        assert_eq!(library.last_source("另一个\u{1f}2025"), Some("fixture-a"));
        library.clear_history();
        assert_eq!(
            library.last_source("回声\u{1f}2026"),
            Some("fixture-c"),
            "clearing watch history does not forget the chosen source"
        );
        library.clear_all();
        assert_eq!(library.last_source("回声\u{1f}2026"), None);
    }

    #[test]
    fn preferred_and_backup_sources_follow_the_contract() {
        let mut first = candidate("回声", "2026", "fixture-a", "1");
        first.source_name = "夹具 A".into();
        first.weight = 40;
        let mut second = candidate("回声", "2026", "fixture-c", "2");
        second.source_name = "夹具 C".into();
        second.weight = 20;
        let work = Work::from_candidates(vec![second, first]).unwrap();
        assert_eq!(
            preferred_source(&work, Some("fixture-c"), Some("fixture-a")),
            Some("fixture-c".into()),
            "remembered source wins when it still exists"
        );
        assert_eq!(
            preferred_source(&work, Some("gone"), Some("fixture-c")),
            Some("fixture-c".into()),
            "a missing remembered source falls back to the explicit one"
        );
        assert_eq!(
            preferred_source(&work, Some("gone"), Some("also-gone")),
            Some("fixture-a".into()),
            "then to the first sorted candidate"
        );
        let backup = next_backup_candidate(&work, "fixture-a", &["fixture-a".into()])
            .expect("second source");
        assert_eq!(backup.source, "fixture-c");
        assert!(
            next_backup_candidate(
                &work,
                "fixture-c",
                &["fixture-a".into(), "fixture-c".into()]
            )
            .is_none()
        );
    }

    #[test]
    fn playback_rates_step_and_stay_in_range() {
        assert_eq!(PLAYBACK_RATES, [0.5, 0.75, 1.0, 1.25, 1.5, 2.0]);
        assert_eq!(step_rate(0.5, -1), 0.5);
        assert_eq!(step_rate(0.5, 1), 0.75);
        assert_eq!(step_rate(1.0, 1), 1.25);
        assert_eq!(step_rate(2.0, 1), 2.0);
        assert_eq!(normalize_rate(1.3), 1.0);
        assert_eq!(normalize_rate(f32::NAN), 1.0);
        assert_eq!(format_rate(0.5), "0.5×");
        assert_eq!(format_rate(1.0), "1×");
    }

    #[test]
    fn intro_and_outro_windows_have_safe_boundaries() {
        assert_eq!(intro_skip_target(30.0, 10.0), Some(10.0));
        assert_eq!(intro_skip_target(30.0, 0.0), None);
        assert_eq!(intro_skip_target(30.0, 30.0), None);
        assert_eq!(intro_skip_target(0.0, 10.0), None);
        assert!(in_intro_window(0.0, 30.0, 10.0));
        assert!(in_intro_window(9.99, 30.0, 10.0));
        assert!(!in_intro_window(10.0, 30.0, 10.0));
        assert!(!in_intro_window(1.0, 30.0, 0.0));

        assert!(in_outro_window(20.0, 30.0, 10.0));
        assert!(in_outro_window(29.99, 30.0, 10.0));
        assert!(!in_outro_window(19.99, 30.0, 10.0));
        assert!(!in_outro_window(30.0, 30.0, 10.0));
        assert!(!in_outro_window(0.0, 30.0, 30.0));
        assert!(!in_outro_window(0.0, 0.0, 10.0));
    }

    #[test]
    fn settings_defaults_leave_daily_playback_unchanged() {
        let settings = Settings::default();
        assert_eq!(settings.playback_rate, 1.0);
        assert_eq!(settings.intro_seconds, 0.0);
        assert_eq!(settings.outro_seconds, 0.0);
        assert!(!settings.auto_skip_intro);
    }

    #[test]
    fn old_settings_json_keeps_new_playback_defaults() {
        let old = r#"{"instance":"http://127.0.0.1:1/","demo_active":false,"auto_next":true,"volume":0.4,"last_username":"u","show_spoilers":false,"discover":{}}"#;
        let settings: Settings = serde_json::from_str(old).expect("old settings");
        assert_eq!(settings.volume, 0.4);
        assert_eq!(settings.playback_rate, 1.0);
        assert_eq!(settings.intro_seconds, 0.0);
        assert_eq!(settings.outro_seconds, 0.0);
        assert!(!settings.auto_skip_intro);
    }

    #[test]
    fn formats_time() {
        assert_eq!(format_time(0.0), "00:00");
        assert_eq!(format_time(75.4), "01:15");
        assert_eq!(format_time(3725.0), "1:02:05");
        assert_eq!(format_time(-1.0), "--:--");
    }

    #[test]
    fn continues_only_unfinished_entries() {
        let mut library = Library::default();
        library.upsert_progress(ProgressEntry {
            key: "a+1".into(),
            position: 20.0,
            duration: 100.0,
            updated_at: 2,
            ..ProgressEntry::default()
        });
        library.upsert_progress(ProgressEntry {
            key: "b+1".into(),
            position: 99.0,
            duration: 100.0,
            updated_at: 3,
            ..ProgressEntry::default()
        });
        library.upsert_progress(ProgressEntry {
            key: "c+1".into(),
            position: 2.0,
            duration: 100.0,
            updated_at: 4,
            ..ProgressEntry::default()
        });
        let list = library.continue_watching();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].key, "a+1");
    }

    #[test]
    fn resolves_urls_around_a_subpath() {
        assert_eq!(
            resolve_url("https://x.example/moon/", "api/a"),
            "https://x.example/moon/api/a"
        );
        assert_eq!(
            resolve_url("https://x.example/moon/", "/root.png"),
            "https://x.example/root.png"
        );
        assert_eq!(
            resolve_url("https://x.example/moon/", "https://cdn.example/p.png"),
            "https://cdn.example/p.png"
        );
        assert!(same_origin(
            "https://x.example/moon/",
            "https://x.example/other"
        ));
        assert!(!same_origin("https://x.example/", "https://cdn.example/"));
    }
}
