//! The deliberately independent demo catalog. It exists to exercise the
//! product without an account or a live instance; every card says so and
//! its data lives in the `demo` storage profile, apart from accounts.

use crate::{
    media_server,
    model::{Candidate, Work, group_results},
};

fn media(path: &str) -> String {
    media_server::url(path)
}

// A single demo fixture row: keeping the eight fields positional keeps the
// catalog below readable as a table.
#[allow(clippy::too_many_arguments)]
fn candidate(
    source: &str,
    source_name: &str,
    id: &str,
    title: &str,
    year: &str,
    kind: &str,
    desc: &str,
    episodes: &[(&str, &str)],
) -> Candidate {
    Candidate {
        source: source.to_owned(),
        id: id.to_owned(),
        source_name: source_name.to_owned(),
        title: title.to_owned(),
        year: year.to_owned(),
        poster: String::new(),
        desc: desc.to_owned(),
        kind: kind.to_owned(),
        remarks: "演示片源".to_owned(),
        episodes: episodes.iter().map(|(_, path)| media(path)).collect(),
        episode_titles: episodes.iter().map(|(name, _)| name.to_string()).collect(),
        proxy_mode: false,
        weight: match source {
            "demo-direct" => 30,
            "demo-hls" => 20,
            "demo-fmp4" => 30,
            "demo-master" => 20,
            "demo-enc-ts" => 30,
            "demo-range" => 20,
            "demo-enc-fmp4" => 30,
            _ => 0,
        },
        ..Candidate::default()
    }
}

/// The demo works as search results; grouping makes the cards.
pub fn works() -> Vec<Work> {
    let results = vec![
        candidate(
            "demo-direct",
            "演示 · 原生 MP4",
            "1",
            "回声档案",
            "2026",
            "科幻 / 悬疑 / 剧情",
            "一段来自废弃中继站的录音，把调查员带回了十年前的雨夜。演示片源使用本次新生成的受控 H.264/AAC 媒体，不代表任何真实剧集。",
            &[
                ("第 1 集 · 信号", "clip.mp4"),
                ("第 2 集 · 余波", "second.mp4"),
            ],
        ),
        candidate(
            "demo-hls",
            "演示 · HLS 分片",
            "1",
            "回声档案",
            "2026",
            "科幻 / 悬疑 / 剧情",
            "同一演示作品的分片片源：用于验证原生 HLS 清单、分片与切源行为。",
            &[("第 1 集 · 信号（HLS）", "ts/index.m3u8")],
        ),
        candidate(
            "demo-fmp4",
            "演示 · HLS fMP4",
            "2",
            "暮色电台",
            "2026",
            "剧情 / 音乐",
            "午夜频道只剩一位主播。她每晚读一封没有寄件人的信，直到某封信写着她自己的名字。演示用 fMP4 分片，单集。",
            &[("第 1 集 · 午夜频率", "fmp4/index.m3u8")],
        ),
        candidate(
            "demo-master",
            "演示 · HLS Master",
            "2",
            "暮色电台",
            "2026",
            "剧情 / 音乐",
            "同一演示作品的主清单片源：由播放器选择一条变体。",
            &[("第 1 集 · 午夜频率（Master）", "master.m3u8")],
        ),
        candidate(
            "demo-enc-ts",
            "演示 · AES-128 TS",
            "3",
            "远山来信",
            "2025",
            "剧情 / 家庭",
            "父亲留下的地图没有终点，只有一排逐年加深的铅笔线。演示用 AES-128 加密的 TS 分片，单集。",
            &[("第 1 集 · 去往山谷", "enc-ts/index.m3u8")],
        ),
        candidate(
            "demo-range",
            "演示 · BYTERANGE fMP4",
            "3",
            "远山来信",
            "2025",
            "剧情 / 家庭",
            "同一演示作品的单文件分片片源：验证 BYTERANGE 与 Range 请求。",
            &[("第 1 集 · 去往山谷（Range）", "range/index.m3u8")],
        ),
        candidate(
            "demo-enc-fmp4",
            "演示 · AES-128 fMP4",
            "4",
            "蓝色航线",
            "2024",
            "冒险 / 纪录",
            "一艘货轮在冬季穿过整片海，船员们用各自的母语数着同一段航程。演示用 AES-128 加密的 fMP4 分片，单集。",
            &[("第 1 集 · 离岸", "enc-fmp4/index.m3u8")],
        ),
        candidate(
            "demo-short",
            "演示 · 短测试片",
            "5",
            "逆光档案",
            "2025",
            "纪录 / 实验",
            "十二秒的受控测试片段，用于快速验证暂停、seek、切集与自动下一集。",
            &[("第 1 集 · 底片", "second.mp4")],
        ),
    ];
    group_results(results)
}

/// The demo search: local, instant, and labelled as a demo everywhere.
pub fn search(query: &str) -> Vec<Work> {
    let query = query.trim().to_lowercase();
    let works = works();
    if query.is_empty() {
        return works;
    }
    works
        .into_iter()
        .filter(|work| {
            work.title.to_lowercase().contains(&query)
                || work.desc.to_lowercase().contains(&query)
                || work
                    .genres
                    .iter()
                    .any(|genre| genre.to_lowercase().contains(&query))
                || "演示".contains(&query)
        })
        .collect()
}

fn sample(
    id: &str,
    title: &str,
    year: &str,
    region: &str,
    type_id: &str,
    rate: &str,
    kind: &str,
) -> crate::discover::Item {
    crate::discover::Item {
        id: id.to_string(),
        title: title.to_string(),
        poster: String::new(),
        rate: rate.to_string(),
        year: year.to_string(),
        kind: kind.to_string(),
        subtitle: "演示样片".to_string(),
        region: region.to_string(),
        type_id: type_id.to_string(),
    }
}

fn samples(kind: crate::discover::MediaKind) -> Vec<crate::discover::Item> {
    use crate::discover::MediaKind;
    match kind {
        MediaKind::Movie => vec![
            sample(
                "demo-movie-1",
                "回声档案",
                "2026",
                "chinese",
                "sci-fi",
                "8.4",
                "科幻",
            ),
            sample(
                "demo-movie-2",
                "暮色电台",
                "2025",
                "japanese",
                "drama",
                "8.6",
                "剧情",
            ),
            sample(
                "demo-movie-3",
                "远山来信",
                "2025",
                "chinese",
                "family",
                "8.1",
                "家庭",
            ),
            sample(
                "demo-movie-4",
                "蓝色航线",
                "2024",
                "western",
                "adventure",
                "7.9",
                "冒险",
            ),
            sample(
                "demo-movie-5",
                "逆光档案",
                "2023",
                "chinese",
                "documentary",
                "8.8",
                "纪录",
            ),
            sample(
                "demo-movie-6",
                "第七码头",
                "2022",
                "korean",
                "crime",
                "7.4",
                "犯罪",
            ),
            sample(
                "demo-movie-7",
                "雪线",
                "2021",
                "western",
                "drama",
                "8.2",
                "剧情",
            ),
            sample(
                "demo-movie-8",
                "旧城电影",
                "2018",
                "chinese",
                "comedy",
                "7.1",
                "喜剧",
            ),
        ],
        MediaKind::Tv => vec![
            sample(
                "demo-tv-1",
                "长夜将尽",
                "2026",
                "chinese",
                "suspense",
                "8.7",
                "悬疑",
            ),
            sample(
                "demo-tv-2",
                "海雾旅馆",
                "2025",
                "korean",
                "drama",
                "8.2",
                "剧情",
            ),
            sample(
                "demo-tv-3",
                "十二号站台",
                "2024",
                "japanese",
                "sci-fi",
                "8.0",
                "科幻",
            ),
            sample(
                "demo-tv-4",
                "铜色年代",
                "2023",
                "chinese",
                "history",
                "7.8",
                "历史",
            ),
            sample(
                "demo-tv-5",
                "小镇医生",
                "2022",
                "western",
                "family",
                "7.6",
                "家庭",
            ),
            sample(
                "demo-tv-6",
                "雨巷",
                "2021",
                "chinese",
                "romance",
                "7.3",
                "爱情",
            ),
        ],
        MediaKind::Show => vec![
            sample(
                "demo-show-1",
                "厨房夜谈",
                "2026",
                "chinese",
                "talkshow",
                "7.9",
                "脱口秀",
            ),
            sample(
                "demo-show-2",
                "荒野合宿",
                "2025",
                "chinese",
                "reality",
                "7.5",
                "真人秀",
            ),
            sample(
                "demo-show-3",
                "声音的现场",
                "2024",
                "chinese",
                "music",
                "8.3",
                "音乐",
            ),
            sample(
                "demo-show-4",
                "舞台之后",
                "2023",
                "korean",
                "reality",
                "7.1",
                "真人秀",
            ),
        ],
        MediaKind::Anime => vec![
            sample(
                "demo-anime-1",
                "星轨列车",
                "2026",
                "japanese",
                "sci_fi",
                "8.5",
                "番剧",
            ),
            sample(
                "demo-anime-2",
                "纸灯与海",
                "2025",
                "japanese",
                "healing",
                "8.8",
                "番剧",
            ),
            sample(
                "demo-anime-3",
                "竹林小径",
                "2024",
                "chinese",
                "chinese_anime",
                "7.9",
                "番剧",
            ),
            sample(
                "demo-anime-4",
                "雨中的信",
                "2023",
                "japanese",
                "love",
                "8.1",
                "番剧",
            ),
            sample(
                "demo-anime-5",
                "夏日短章",
                "2025",
                "japanese",
                "youth",
                "8.4",
                "剧场版",
            ),
            sample(
                "demo-anime-6",
                "旧书页",
                "2022",
                "chinese",
                "healing",
                "7.7",
                "剧场版",
            ),
        ],
    }
}

/// A small, clearly-labelled local sample for the discovery page. Filters
/// apply to these rows only; a real instance asks Douban instead.
pub fn discover(
    kind: crate::discover::MediaKind,
    anime: crate::discover::AnimeTab,
    movie_primary: &str,
    tv_primary: &str,
    filters: &crate::discover::Filters,
) -> Vec<crate::discover::Item> {
    use crate::discover::{AnimeTab, MediaKind};
    if kind == MediaKind::Anime && anime == AnimeTab::Daily {
        return Vec::new();
    }
    let mut items = samples(kind);
    let year = filters.year.as_str();
    items.retain(|item| {
        let year_ok = match year {
            "all" => true,
            "earlier" => item.year.as_str() < "1960",
            value if value.ends_with('s') => item.year.starts_with(&value[..4]),
            value => item.year == value,
        };
        let region_ok = filters.region == "all" || item.region == filters.region;
        let type_ok = if kind == MediaKind::Anime {
            filters.label == "all" || item.type_id == filters.label
        } else {
            filters.type_sel == "all" || item.type_id == filters.type_sel
        };
        year_ok && region_ok && type_ok
    });
    let primary = if kind == MediaKind::Movie {
        movie_primary
    } else {
        tv_primary
    };
    let score = |item: &crate::discover::Item| item.rate.parse::<f32>().unwrap_or(0.0);
    match primary {
        "豆瓣高分" => items.sort_by(|a, b| score(b).total_cmp(&score(a))),
        "冷门佳片" => items.sort_by(|a, b| score(a).total_cmp(&score(b))),
        "最新" => items.sort_by(|a, b| b.year.cmp(&a.year)),
        _ => items.sort_by(|a, b| score(b).total_cmp(&score(a))),
    }
    items
}

/// Seven sample days for the anime calendar. The weekday names and the
/// CN-first title fallback mirror the real Bangumi response.
pub fn anime_days() -> Vec<crate::discover::Day> {
    use crate::discover::{Day, MediaKind, weekday_cn, weekday_order};
    let pool = samples(MediaKind::Anime);
    weekday_order()
        .iter()
        .enumerate()
        .map(|(index, en)| Day {
            en: (*en).to_string(),
            cn: weekday_cn(en).to_string(),
            items: pool
                .iter()
                .cycle()
                .skip(index % pool.len().max(1))
                .take(2)
                .cloned()
                .collect(),
        })
        .collect()
}

/// The demo detail of one candidate.
pub fn detail(work: &Work, source: &str) -> Option<Candidate> {
    work.candidates
        .iter()
        .find(|candidate| candidate.source == source)
        .cloned()
}

/// Whether a source belongs to the demo catalog.
pub fn is_demo(candidate: &Candidate) -> bool {
    candidate.source.starts_with("demo-")
}
