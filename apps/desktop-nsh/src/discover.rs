//! Discovery beyond search: the instance's Douban category/recommend proxies
//! and the Bangumi daily calendar. The filters mirror the web client's own
//! selector vocabulary (Chinese labels for Douban, English ids for options
//! on screen) so both clients ask the server for the same thing.

use serde_json::Value;

use crate::model::{api_url, encode_query, resolve_url};

/// The four discovery sections the sidebar offers.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MediaKind {
    #[default]
    Movie,
    Tv,
    Show,
    Anime,
}

impl MediaKind {
    pub fn key(self) -> &'static str {
        match self {
            Self::Movie => "movie",
            Self::Tv => "tv",
            Self::Show => "show",
            Self::Anime => "anime",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Movie => "电影",
            Self::Tv => "剧集",
            Self::Show => "综艺",
            Self::Anime => "动漫",
        }
    }

    pub fn all() -> [Self; 4] {
        [Self::Movie, Self::Tv, Self::Show, Self::Anime]
    }

    /// The Douban `kind` a recommendation query uses.
    pub fn douban_kind(self) -> &'static str {
        match self {
            Self::Movie | Self::Anime => "movie",
            Self::Tv | Self::Show => "tv",
        }
    }
}

/// 动漫's third selector: 每日放送 / 番剧 / 剧场版.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AnimeTab {
    #[default]
    Daily,
    Tv,
    Movie,
}

impl AnimeTab {
    pub fn label(self) -> &'static str {
        match self {
            Self::Daily => "每日放送",
            Self::Tv => "番剧",
            Self::Movie => "剧场版",
        }
    }
}

/// One card. `rate` stays empty when the source has no score; the UI never
/// invents one.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Item {
    pub id: String,
    pub title: String,
    pub poster: String,
    pub rate: String,
    pub year: String,
    pub kind: String,
    pub subtitle: String,
    /// Demo samples only: the option ids a local filter compares against.
    /// Real Douban/Bangumi items leave these empty.
    pub region: String,
    pub type_id: String,
}

/// The stored selection for one filter dimension. Values are the option ids
/// (`comedy`, `chinese`, `2020s`); `all` / `T` mean "no filter".
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Filters {
    pub type_sel: String,
    pub label: String,
    pub region: String,
    pub year: String,
    pub platform: String,
    pub sort: String,
}

impl Filters {
    pub fn reset() -> Self {
        Self {
            type_sel: "all".into(),
            label: "all".into(),
            region: "all".into(),
            year: "all".into(),
            platform: "all".into(),
            sort: "T".into(),
        }
    }

    pub fn get(&self, key: &str) -> &str {
        match key {
            "type" => &self.type_sel,
            "label" => &self.label,
            "region" => &self.region,
            "year" => &self.year,
            "platform" => &self.platform,
            "sort" => &self.sort,
            _ => "all",
        }
    }

    pub fn set(&mut self, key: &str, value: &str) {
        let slot = match key {
            "type" => &mut self.type_sel,
            "label" => &mut self.label,
            "region" => &mut self.region,
            "year" => &mut self.year,
            "platform" => &mut self.platform,
            "sort" => &mut self.sort,
            _ => return,
        };
        *slot = value.to_string();
    }

    pub fn is_default(&self) -> bool {
        self.type_sel == "all"
            && self.label == "all"
            && self.region == "all"
            && self.year == "all"
            && self.platform == "all"
            && self.sort == "T"
    }
}

/// A filter chip row: its stable key, title and ordered options.
#[derive(Clone, Debug, PartialEq)]
pub struct FilterSpec {
    pub key: &'static str,
    pub title: &'static str,
    pub options: Vec<(String, String)>,
}

const ALL: (&str, &str) = ("全部", "all");

fn options(list: &[(&str, &str)]) -> Vec<(String, String)> {
    list.iter()
        .map(|(label, value)| ((*label).to_string(), (*value).to_string()))
        .collect()
}

/// The movie categories: primary tab plus its region selector.
pub fn movie_primaries() -> Vec<&'static str> {
    vec!["全部", "热门", "最新", "豆瓣高分", "冷门佳片"]
}

pub fn movie_regions() -> Vec<&'static str> {
    vec!["全部", "华语", "欧美", "韩国", "日本"]
}

/// The tv/show second selector's raw values (they go to Douban as-is).
pub fn tv_secondaries() -> Vec<(&'static str, &'static str)> {
    vec![
        ("全部", "tv"),
        ("国产", "tv_domestic"),
        ("欧美", "tv_american"),
        ("日本", "tv_japanese"),
        ("韩国", "tv_korean"),
        ("动漫", "tv_animation"),
        ("纪录片", "tv_documentary"),
    ]
}

pub fn show_secondaries() -> Vec<(&'static str, &'static str)> {
    vec![
        ("全部", "show"),
        ("国内", "show_domestic"),
        ("国外", "show_foreign"),
    ]
}

/// The multi-level filter bar of one content type, in display order.
pub fn filter_specs(kind: MediaKind, anime: AnimeTab) -> Vec<FilterSpec> {
    let mut specs = Vec::new();
    let anime_tv = kind == MediaKind::Anime && anime == AnimeTab::Tv;
    let anime_movie = kind == MediaKind::Anime && anime == AnimeTab::Movie;
    let type_options = if anime_tv {
        let mut list = options(&[ALL]);
        list.extend(options(anime_tv_labels()));
        list
    } else if anime_movie {
        let mut list = options(&[ALL]);
        list.extend(options(anime_movie_labels()));
        list
    } else {
        let mut list = options(&[ALL]);
        list.extend(options(match kind {
            MediaKind::Movie | MediaKind::Anime => movie_types(),
            MediaKind::Tv => tv_types(),
            MediaKind::Show => show_types(),
        }));
        list
    };
    specs.push(FilterSpec {
        key: if anime_tv || anime_movie {
            "label"
        } else {
            "type"
        },
        title: "类型",
        options: type_options,
    });

    let mut regions = options(&[ALL]);
    regions.extend(options(match kind {
        MediaKind::Movie | MediaKind::Anime => movie_regions_all(),
        MediaKind::Tv | MediaKind::Show => tv_regions(),
    }));
    specs.push(FilterSpec {
        key: "region",
        title: "地区",
        options: regions,
    });

    specs.push(FilterSpec {
        key: "year",
        title: "年代",
        options: year_options(current_year()),
    });

    if matches!(kind, MediaKind::Tv | MediaKind::Show | MediaKind::Anime) && !anime_movie {
        let mut platforms = options(&[ALL]);
        platforms.extend(options(platforms_all()));
        specs.push(FilterSpec {
            key: "platform",
            title: "平台",
            options: platforms,
        });
    }

    specs.push(FilterSpec {
        key: "sort",
        title: "排序",
        options: options(&[
            ("综合排序", "T"),
            ("近期热度", "U"),
            ("首播时间", "R"),
            ("高分优先", "S"),
        ]),
    });
    specs
}

/// The label to show on a filter chip: the option's label, else the chip's
/// own title when nothing is selected.
pub fn filter_label(specs: &[FilterSpec], filters: &Filters, key: &str) -> String {
    let Some(spec) = specs.iter().find(|spec| spec.key == key) else {
        return String::new();
    };
    let value = filters.get(key);
    if value == "all" || value == "T" {
        return spec.title.to_string();
    }
    spec.options
        .iter()
        .find(|(_, option)| option == value)
        .map(|(label, _)| label.clone())
        .unwrap_or_else(|| spec.title.to_string())
}

/// Translates a stored value into the label the Douban proxy expects.
pub fn wire_label(specs: &[FilterSpec], key: &str, value: &str) -> String {
    if value == "all" {
        return if key == "sort" {
            "T".into()
        } else {
            "all".into()
        };
    }
    specs
        .iter()
        .find(|spec| spec.key == key)
        .and_then(|spec| {
            spec.options
                .iter()
                .find(|(_, option)| option == value)
                .map(|(label, _)| label.clone())
        })
        .unwrap_or_else(|| value.to_string())
}

/// One HTTP request the discovery page makes, independent of its transport.
#[derive(Clone, Debug, PartialEq)]
pub enum Query {
    Categories {
        kind: &'static str,
        category: String,
        type_: String,
        start: usize,
        limit: usize,
    },
    Recommends {
        kind: &'static str,
        category: String,
        format: String,
        label: String,
        region: String,
        year: String,
        platform: String,
        sort: String,
        start: usize,
        limit: usize,
    },
    /// The daily calendar is a single request; the weekday switch is local.
    Calendar,
}

pub const PAGE_LIMIT: usize = 25;

/// Builds the request for the current selection. `None` means the daily
/// calendar, which ignores the Douban filters.
#[allow(clippy::too_many_arguments)]
pub fn query_for(
    kind: MediaKind,
    movie_primary: &str,
    tv_primary: &str,
    secondary: &str,
    anime: AnimeTab,
    filters: &Filters,
    start: usize,
) -> Option<Query> {
    if kind == MediaKind::Anime && anime == AnimeTab::Daily {
        return Some(Query::Calendar);
    }
    let specs = filter_specs(kind, anime);
    if kind == MediaKind::Anime {
        let (dq_kind, format) = match anime {
            AnimeTab::Tv => ("tv", "电视剧"),
            _ => ("movie", ""),
        };
        return Some(Query::Recommends {
            kind: dq_kind,
            category: "动画".into(),
            format: format.into(),
            label: wire_label(&specs, "label", filters.label.as_str()),
            region: wire_label(&specs, "region", &filters.region),
            year: wire_label(&specs, "year", &filters.year),
            platform: wire_label(&specs, "platform", &filters.platform),
            sort: filters.sort.clone(),
            start,
            limit: PAGE_LIMIT,
        });
    }
    match kind {
        MediaKind::Movie if movie_primary != "全部" => Some(Query::Categories {
            kind: "movie",
            category: movie_primary.to_string(),
            type_: secondary.to_string(),
            start,
            limit: PAGE_LIMIT,
        }),
        MediaKind::Movie => Some(Query::Recommends {
            kind: "movie",
            category: wire_label(&specs, "type", &filters.type_sel),
            format: String::new(),
            label: String::new(),
            region: wire_label(&specs, "region", &filters.region),
            year: wire_label(&specs, "year", &filters.year),
            platform: String::new(),
            sort: filters.sort.clone(),
            start,
            limit: PAGE_LIMIT,
        }),
        MediaKind::Tv | MediaKind::Show if tv_primary != "全部" => Some(Query::Categories {
            kind: "tv",
            category: if kind == MediaKind::Tv { "tv" } else { "show" }.into(),
            type_: secondary.to_string(),
            start,
            limit: PAGE_LIMIT,
        }),
        MediaKind::Tv | MediaKind::Show => Some(Query::Recommends {
            kind: "tv",
            category: wire_label(&specs, "type", &filters.type_sel),
            format: if kind == MediaKind::Tv {
                "电视剧"
            } else {
                "综艺"
            }
            .into(),
            label: String::new(),
            region: wire_label(&specs, "region", &filters.region),
            year: wire_label(&specs, "year", &filters.year),
            platform: wire_label(&specs, "platform", &filters.platform),
            sort: filters.sort.clone(),
            start,
            limit: PAGE_LIMIT,
        }),
        MediaKind::Anime => None,
    }
}

pub fn url(base: &str, query: &Query) -> String {
    match query {
        Query::Categories {
            kind,
            category,
            type_,
            start,
            limit,
        } => api_url(
            base,
            &format!(
                "api/douban/categories?kind={}&category={}&type={}&limit={}&start={}",
                encode_query(kind),
                encode_query(category),
                encode_query(type_),
                limit,
                start
            ),
        ),
        Query::Recommends {
            kind,
            category,
            format,
            label,
            region,
            year,
            platform,
            sort,
            start,
            limit,
        } => api_url(
            base,
            &format!(
                "api/douban/recommends?kind={}&limit={}&start={}&category={}&format={}&region={}&year={}&platform={}&sort={}&label={}",
                encode_query(kind),
                limit,
                start,
                encode_query(category),
                encode_query(format),
                encode_query(region),
                encode_query(year),
                encode_query(platform),
                encode_query(sort),
                encode_query(label)
            ),
        ),
        Query::Calendar => api_url(base, "api/bangumi/calendar"),
    }
}

/// Parses the instance's `{code, message, list}` envelope. A non-200 code
/// or an error body becomes a readable message, never an empty success.
pub fn parse_list(base: &str, value: &Value) -> Result<Vec<Item>, String> {
    if let Some(error) = value["error"].as_str().filter(|text| !text.is_empty()) {
        let details = value["details"].as_str().unwrap_or("");
        return Err(if details.is_empty() {
            error.to_string()
        } else {
            format!("{error}：{details}")
        });
    }
    if value["code"].as_i64().is_some_and(|code| code != 200) {
        return Err(value["message"]
            .as_str()
            .filter(|text| !text.is_empty())
            .unwrap_or("豆瓣数据没有返回成功状态")
            .to_string());
    }
    let Some(items) = value["list"].as_array() else {
        return Err("响应缺少 list 数组".into());
    };
    Ok(items
        .iter()
        .filter_map(|item| {
            let id = text(item, "id");
            let title = text(item, "title");
            if id.is_empty() || title.trim().is_empty() {
                return None;
            }
            Some(Item {
                id,
                title: title.trim().to_string(),
                poster: resolve_url(base, &text(item, "poster")),
                rate: text(item, "rate"),
                year: text(item, "year"),
                kind: "豆瓣".into(),
                ..Item::default()
            })
        })
        .collect())
}

fn text(value: &Value, key: &str) -> String {
    match value.get(key) {
        Some(Value::String(value)) => value.clone(),
        Some(Value::Number(value)) => value.to_string(),
        _ => String::new(),
    }
}

/// One weekday of the Bangumi calendar. The weekday ids are Bangumi's
/// English names; the labels are its Chinese names.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Day {
    pub en: String,
    pub cn: String,
    pub items: Vec<Item>,
}

pub fn parse_calendar(base: &str, value: &Value) -> Result<Vec<Day>, String> {
    if let Some(error) = value["error"].as_str().filter(|text| !text.is_empty()) {
        return Err(error.to_string());
    }
    let Some(days) = value.as_array() else {
        return Err("日历响应不是数组".into());
    };
    Ok(days
        .iter()
        .filter_map(|day| {
            let en = text(&day["weekday"], "en");
            if en.is_empty() {
                return None;
            }
            let cn = {
                let cn = text(&day["weekday"], "cn");
                if cn.is_empty() {
                    weekday_cn(&en).to_string()
                } else {
                    cn
                }
            };
            let items = day["items"]
                .as_array()
                .map(|items| {
                    items
                        .iter()
                        .filter_map(|item| {
                            let id = text(item, "id");
                            // Chinese name first; a title-less item falls back
                            // to its original name rather than disappearing.
                            let title = {
                                let cn = text(item, "name_cn");
                                if cn.trim().is_empty() {
                                    text(item, "name")
                                } else {
                                    cn
                                }
                            };
                            if id.is_empty() || title.trim().is_empty() {
                                return None;
                            }
                            let poster = {
                                let large = text(&item["images"], "large");
                                if large.is_empty() {
                                    text(&item["images"], "common")
                                } else {
                                    large
                                }
                            };
                            let air = text(item, "air_date");
                            let year = if air.len() >= 4 {
                                air[..4].to_string()
                            } else {
                                String::new()
                            };
                            let rate = match item["rating"]["score"].as_f64() {
                                Some(score) if score > 0.0 => format!("{score:.1}"),
                                _ => String::new(),
                            };
                            Some(Item {
                                id,
                                title: title.trim().to_string(),
                                poster: resolve_url(base, &poster),
                                rate,
                                year,
                                kind: "番剧".into(),
                                ..Item::default()
                            })
                        })
                        .collect()
                })
                .unwrap_or_default();
            Some(Day { en, cn, items })
        })
        .collect())
}

pub fn weekday_order() -> [&'static str; 7] {
    [
        "monday",
        "tuesday",
        "wednesday",
        "thursday",
        "friday",
        "saturday",
        "sunday",
    ]
}

pub fn weekday_cn(en: &str) -> &'static str {
    match en {
        "monday" => "周一",
        "tuesday" => "周二",
        "wednesday" => "周三",
        "thursday" => "周四",
        "friday" => "周五",
        "saturday" => "周六",
        "sunday" => "周日",
        _ => "未知",
    }
}

/// Today's weekday id in the machine's local time.
pub fn today() -> &'static str {
    let offset = local_weekday().unwrap_or_else(|| weekday_from_unix_ms(nsh_core::time::unix_ms()));
    weekday_order()[offset as usize % 7]
}

fn weekday_from_unix_ms(ms: f64) -> u8 {
    // 1970-01-01 was a Thursday; weekday_order starts at Monday.
    let days = (ms / 86_400_000.0).floor() as i64;
    let sunday_based = (days + 4).rem_euclid(7);
    ((sunday_based + 6) % 7) as u8
}

#[cfg(windows)]
fn local_weekday() -> Option<u8> {
    #[repr(C)]
    struct SystemTime {
        year: u16,
        month: u16,
        day_of_week: u16,
        day: u16,
        hour: u16,
        minute: u16,
        second: u16,
        milliseconds: u16,
    }
    unsafe extern "system" {
        fn GetLocalTime(time: *mut SystemTime);
    }
    let mut time = SystemTime {
        year: 0,
        month: 0,
        day_of_week: 0,
        day: 0,
        hour: 0,
        minute: 0,
        second: 0,
        milliseconds: 0,
    };
    // SAFETY: `GetLocalTime` writes a `SYSTEMTIME`; the pointer is valid for
    // the duration of the call and the struct matches its C layout.
    unsafe { GetLocalTime(&mut time) };
    (time.day_of_week <= 6).then_some(((time.day_of_week + 6) % 7) as u8)
}

#[cfg(not(windows))]
fn local_weekday() -> Option<u8> {
    None
}

#[cfg(windows)]
pub fn current_year() -> i32 {
    #[repr(C)]
    struct SystemTime {
        year: u16,
        month: u16,
        day_of_week: u16,
        day: u16,
        hour: u16,
        minute: u16,
        second: u16,
        milliseconds: u16,
    }
    unsafe extern "system" {
        fn GetLocalTime(time: *mut SystemTime);
    }
    let mut time = SystemTime {
        year: 0,
        month: 0,
        day_of_week: 0,
        day: 0,
        hour: 0,
        minute: 0,
        second: 0,
        milliseconds: 0,
    };
    // SAFETY: as above.
    unsafe { GetLocalTime(&mut time) };
    if time.year == 0 {
        2026
    } else {
        i32::from(time.year)
    }
}

#[cfg(not(windows))]
pub fn current_year() -> i32 {
    2026
}

fn year_options(year: i32) -> Vec<(String, String)> {
    let mut options = vec![("全部".to_string(), "all".to_string())];
    let current_decade = (year / 10) * 10;
    options.push((
        format!("{current_decade}年代"),
        format!("{current_decade}s"),
    ));
    let mut value = year;
    while value >= current_decade {
        options.push((value.to_string(), value.to_string()));
        value -= 1;
    }
    let mut decade = current_decade - 10;
    while decade >= 1960 {
        options.push((format!("{decade}年代"), format!("{decade}s")));
        decade -= 10;
    }
    options.push(("更早".to_string(), "earlier".to_string()));
    options
}

fn movie_types() -> &'static [(&'static str, &'static str)] {
    &[
        ("喜剧", "comedy"),
        ("爱情", "romance"),
        ("动作", "action"),
        ("科幻", "sci-fi"),
        ("悬疑", "suspense"),
        ("犯罪", "crime"),
        ("惊悚", "thriller"),
        ("冒险", "adventure"),
        ("音乐", "music"),
        ("历史", "history"),
        ("奇幻", "fantasy"),
        ("恐怖", "horror"),
        ("战争", "war"),
        ("传记", "biography"),
        ("歌舞", "musical"),
        ("武侠", "wuxia"),
        ("灾难", "disaster"),
        ("西部", "western"),
        ("纪录片", "documentary"),
        ("短片", "short"),
    ]
}

fn tv_types() -> &'static [(&'static str, &'static str)] {
    &[
        ("喜剧", "comedy"),
        ("爱情", "romance"),
        ("悬疑", "suspense"),
        ("武侠", "wuxia"),
        ("古装", "costume"),
        ("家庭", "family"),
        ("犯罪", "crime"),
        ("科幻", "sci-fi"),
        ("恐怖", "horror"),
        ("历史", "history"),
        ("战争", "war"),
        ("动作", "action"),
        ("冒险", "adventure"),
        ("传记", "biography"),
        ("剧情", "drama"),
        ("奇幻", "fantasy"),
        ("惊悚", "thriller"),
        ("灾难", "disaster"),
        ("歌舞", "musical"),
        ("音乐", "music"),
    ]
}

fn show_types() -> &'static [(&'static str, &'static str)] {
    &[
        ("真人秀", "reality"),
        ("脱口秀", "talkshow"),
        ("音乐", "music"),
        ("歌舞", "musical"),
    ]
}

fn anime_tv_labels() -> &'static [(&'static str, &'static str)] {
    &[
        ("黑色幽默", "dark_humor"),
        ("历史", "history"),
        ("歌舞", "musical"),
        ("励志", "inspirational"),
        ("恶搞", "parody"),
        ("治愈", "healing"),
        ("运动", "sports"),
        ("后宫", "harem"),
        ("国漫", "chinese_anime"),
        ("人性", "human_nature"),
        ("悬疑", "suspense"),
        ("恋爱", "love"),
        ("魔幻", "fantasy"),
        ("科幻", "sci_fi"),
    ]
}

fn anime_movie_labels() -> &'static [(&'static str, &'static str)] {
    &[
        ("定格动画", "stop_motion"),
        ("传记", "biography"),
        ("美国动画", "us_animation"),
        ("爱情", "romance"),
        ("黑色幽默", "dark_humor"),
        ("歌舞", "musical"),
        ("儿童", "children"),
        ("二次元", "anime"),
        ("动物", "animal"),
        ("青春", "youth"),
        ("历史", "history"),
        ("励志", "inspirational"),
        ("恶搞", "parody"),
        ("治愈", "healing"),
        ("运动", "sports"),
        ("后宫", "harem"),
        ("人性", "human_nature"),
        ("悬疑", "suspense"),
        ("恋爱", "love"),
        ("魔幻", "fantasy"),
        ("科幻", "sci_fi"),
    ]
}

fn movie_regions_all() -> &'static [(&'static str, &'static str)] {
    &[
        ("华语", "chinese"),
        ("欧美", "western"),
        ("韩国", "korean"),
        ("日本", "japanese"),
        ("中国大陆", "mainland_china"),
        ("美国", "usa"),
        ("中国香港", "hong_kong"),
        ("中国台湾", "taiwan"),
        ("英国", "uk"),
        ("法国", "france"),
        ("德国", "germany"),
        ("意大利", "italy"),
        ("西班牙", "spain"),
        ("印度", "india"),
        ("泰国", "thailand"),
        ("俄罗斯", "russia"),
        ("加拿大", "canada"),
        ("澳大利亚", "australia"),
        ("爱尔兰", "ireland"),
        ("瑞典", "sweden"),
        ("巴西", "brazil"),
        ("丹麦", "denmark"),
    ]
}

fn tv_regions() -> &'static [(&'static str, &'static str)] {
    &[
        ("华语", "chinese"),
        ("欧美", "western"),
        ("国外", "foreign"),
        ("韩国", "korean"),
        ("日本", "japanese"),
        ("中国大陆", "mainland_china"),
        ("中国香港", "hong_kong"),
        ("美国", "usa"),
        ("英国", "uk"),
        ("泰国", "thailand"),
        ("中国台湾", "taiwan"),
        ("意大利", "italy"),
        ("法国", "france"),
        ("德国", "germany"),
        ("西班牙", "spain"),
        ("俄罗斯", "russia"),
        ("瑞典", "sweden"),
        ("巴西", "brazil"),
        ("丹麦", "denmark"),
        ("印度", "india"),
        ("加拿大", "canada"),
        ("爱尔兰", "ireland"),
        ("澳大利亚", "australia"),
    ]
}

fn platforms_all() -> &'static [(&'static str, &'static str)] {
    &[
        ("腾讯视频", "tencent"),
        ("爱奇艺", "iqiyi"),
        ("优酷", "youku"),
        ("湖南卫视", "hunan_tv"),
        ("Netflix", "netflix"),
        ("HBO", "hbo"),
        ("BBC", "bbc"),
        ("NHK", "nhk"),
        ("CBS", "cbs"),
        ("NBC", "nbc"),
        ("tvN", "tvn"),
    ]
}

/// Merges a new page into the list, keyed by item id. The answer is how many
/// items were actually new, so a repeated last page cannot loop forever.
pub fn merge(existing: &mut Vec<Item>, incoming: Vec<Item>) -> usize {
    let mut added = 0;
    for item in incoming {
        if existing.iter().any(|known| known.id == item.id) {
            continue;
        }
        existing.push(item);
        added += 1;
    }
    added
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn movie_categories_ask_like_the_web_client() {
        let query = query_for(
            MediaKind::Movie,
            "热门",
            "全部",
            "华语",
            AnimeTab::Daily,
            &Filters::reset(),
            50,
        )
        .unwrap();
        assert_eq!(
            url("https://tv.example/moon/", &query),
            "https://tv.example/moon/api/douban/categories?kind=movie&category=%E7%83%AD%E9%97%A8&type=%E5%8D%8E%E8%AF%AD&limit=25&start=50"
        );
    }

    #[test]
    fn movie_all_uses_recommendations_with_selected_labels() {
        let mut filters = Filters::reset();
        filters.type_sel = "comedy".into();
        filters.region = "chinese".into();
        filters.year = "2020s".into();
        filters.sort = "S".into();
        let query = query_for(
            MediaKind::Movie,
            "全部",
            "全部",
            "全部",
            AnimeTab::Daily,
            &filters,
            0,
        )
        .unwrap();
        let url = url("https://tv.example/", &query);
        assert!(url.contains("category=%E5%96%9C%E5%89%A7"), "{url}"); // 喜剧
        assert!(url.contains("region=%E5%8D%8E%E8%AF%AD"), "{url}"); // 华语
        assert!(url.contains("year=2020%E5%B9%B4%E4%BB%A3"), "{url}"); // 2020年代
        assert!(url.contains("sort=S"), "{url}");
        assert!(url.contains("format=&"), "movie has no format: {url}");
    }

    #[test]
    fn tv_show_and_anime_use_their_own_kind_and_format() {
        let tv = query_for(
            MediaKind::Tv,
            "全部",
            "全部",
            "tv",
            AnimeTab::Daily,
            &Filters::reset(),
            25,
        )
        .unwrap();
        assert!(url("https://x/", &tv).contains("kind=tv&limit=25&start=25"));
        assert!(url("https://x/", &tv).contains("format=%E7%94%B5%E8%A7%86%E5%89%A7")); // 电视剧

        let show = query_for(
            MediaKind::Show,
            "全部",
            "最近热门",
            "show_domestic",
            AnimeTab::Daily,
            &Filters::reset(),
            0,
        )
        .unwrap();
        let show_url = url("https://x/", &show);
        assert!(show_url.contains("api/douban/categories?kind=tv&category=show"));
        assert!(show_url.contains("type=show_domestic"));

        let anime = query_for(
            MediaKind::Anime,
            "全部",
            "全部",
            "全部",
            AnimeTab::Daily,
            &Filters::reset(),
            0,
        )
        .unwrap();
        assert_eq!(anime, Query::Calendar);

        let bangumi = query_for(
            MediaKind::Anime,
            "全部",
            "全部",
            "全部",
            AnimeTab::Tv,
            &Filters::reset(),
            0,
        )
        .unwrap();
        let bangumi_url = url("https://x/", &bangumi);
        assert!(bangumi_url.contains("category=%E5%8A%A8%E7%94%BB")); // 动画
        assert!(bangumi_url.contains("format=%E7%94%B5%E8%A7%86%E5%89%A7"));
    }

    #[test]
    fn filter_specs_follow_the_content_type() {
        let movie = filter_specs(MediaKind::Movie, AnimeTab::Daily);
        let keys: Vec<&str> = movie.iter().map(|spec| spec.key).collect();
        assert_eq!(
            keys,
            ["type", "region", "year", "sort"],
            "movies have no platform"
        );

        let tv = filter_specs(MediaKind::Tv, AnimeTab::Daily);
        assert!(tv.iter().any(|spec| spec.key == "platform"));
        assert!(
            tv.iter()
                .find(|spec| spec.key == "type")
                .unwrap()
                .options
                .iter()
                .any(|(_, value)| value == "costume")
        );

        let anime_movie = filter_specs(MediaKind::Anime, AnimeTab::Movie);
        assert!(anime_movie.iter().any(|spec| spec.key == "label"));
        assert!(!anime_movie.iter().any(|spec| spec.key == "type"));
        assert!(!anime_movie.iter().any(|spec| spec.key == "platform"));
    }

    #[test]
    fn parse_list_rejects_fake_empty_success() {
        let error = parse_list(
            "https://x/",
            &serde_json::json!({"error": "获取豆瓣数据失败"}),
        );
        assert!(error.unwrap_err().contains("豆瓣"));
        let items = parse_list(
            "https://x/",
            &serde_json::json!({"code":200,"list":[{"id":1,"title":"回 echo","poster":"/p.jpg","rate":"8.1","year":"2026"}]}),
        )
        .unwrap();
        assert_eq!(items[0].id, "1");
        assert_eq!(items[0].poster, "https://x/p.jpg");
        assert_eq!(items[0].rate, "8.1");
    }

    #[test]
    fn calendar_prefers_chinese_names_and_keeps_originals() {
        let days = parse_calendar(
            "https://x/",
            &serde_json::json!([{
                "weekday": {"en": "monday", "cn": "星期一"},
                "items": [
                    {"id": 1, "name": "Original", "name_cn": "中文名", "air_date": "2025-10-06", "images": {"large": "https://img/x.jpg"}, "rating": {"score": 7.6}},
                    {"id": 2, "name": "No Chinese", "name_cn": "", "air_date": "", "images": {"common": "https://img/y.jpg"}, "rating": {"score": 0}}
                ]
            }]),
        )
        .unwrap();
        assert_eq!(days.len(), 1);
        assert_eq!(days[0].cn, "星期一");
        assert_eq!(days[0].items[0].title, "中文名");
        assert_eq!(days[0].items[0].year, "2025");
        assert_eq!(days[0].items[0].rate, "7.6");
        assert_eq!(days[0].items[1].title, "No Chinese");
        assert!(days[0].items[1].rate.is_empty());
        assert_eq!(days[0].items[1].poster, "https://img/y.jpg");
    }

    #[test]
    fn merge_skips_repeats_and_reports_new_items() {
        let mut existing = vec![Item {
            id: "1".into(),
            ..Item::default()
        }];
        let added = merge(
            &mut existing,
            vec![
                Item {
                    id: "1".into(),
                    ..Item::default()
                },
                Item {
                    id: "2".into(),
                    ..Item::default()
                },
            ],
        );
        assert_eq!(added, 1);
        assert_eq!(existing.len(), 2);
    }

    #[test]
    fn years_cover_the_current_decade_and_earlier() {
        let options = year_options(2026);
        assert!(
            options
                .iter()
                .any(|(label, value)| label == "2020年代" && value == "2020s")
        );
        assert!(
            options
                .iter()
                .any(|(label, value)| label == "2026" && value == "2026")
        );
        assert!(options.iter().any(|(_, value)| value == "earlier"));
        assert!(options.iter().any(|(_, value)| value == "1960s"));
    }

    #[test]
    fn weekday_conversion_matches_a_known_date() {
        // 2026-11-24 is a Tuesday (2).
        let ms = 1_795_478_400_000.0; // 2026-11-24T00:00:00Z
        assert_eq!(weekday_from_unix_ms(ms), 1);
        assert_eq!(weekday_order()[1], "tuesday");
    }
}
