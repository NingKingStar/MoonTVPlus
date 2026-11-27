//! Single-source browsing and short-drama data: the instance's
//! `/api/source-search/*` and `/api/duanju/*` contracts. This module only
//! parses and models; HTTP and UI live elsewhere.

use serde_json::Value;

use crate::model::resolve_url;

/// One playable source visible to the signed-in account.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Site {
    pub key: String,
    pub name: String,
    pub api: String,
}

/// One short-drama-capable source. `type_id`/`type_name` are the category the
/// server detected as short drama; the page uses it as the default selection.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DuanjuSite {
    pub key: String,
    pub name: String,
    pub api: String,
    pub type_id: String,
    pub type_name: String,
}

/// A CMS category. `pid` is `"0"` for a root; some sources return a flat list.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Category {
    pub id: String,
    pub name: String,
    pub pid: String,
}

/// One result row from source browsing or source-scoped search.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Item {
    pub id: String,
    pub title: String,
    pub poster: String,
    pub year: String,
    pub episodes: Vec<String>,
    pub episode_titles: Vec<String>,
    pub source: String,
    pub source_name: String,
    pub desc: String,
    pub type_name: String,
    pub remarks: String,
}

/// A parsed page. `page_count` is the server's own count, so the client does
/// not guess where the last page is.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Page {
    pub items: Vec<Item>,
    pub page: usize,
    pub page_count: usize,
    pub total: usize,
}

fn text(value: &Value, key: &str) -> String {
    match value.get(key) {
        Some(Value::String(value)) => value.clone(),
        Some(Value::Number(value)) => value.to_string(),
        Some(Value::Bool(value)) => value.to_string(),
        _ => String::new(),
    }
}

fn number(value: &Value, key: &str) -> usize {
    value
        .get(key)
        .and_then(|value| match value {
            Value::Number(number) => number.as_u64(),
            Value::String(string) => string.parse().ok(),
            _ => None,
        })
        .unwrap_or(0) as usize
}

fn string_list(value: &Value, key: &str) -> Vec<String> {
    value
        .get(key)
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

/// `{sources:[{key,name,api}]}` from `/api/source-search/sources`.
pub fn parse_sites(value: &Value) -> Result<Vec<Site>, String> {
    error_or(value)?;
    let Some(items) = value["sources"].as_array() else {
        return Err("响应缺少 sources 数组".into());
    };
    Ok(items
        .iter()
        .filter_map(|item| {
            let key = text(item, "key");
            if key.is_empty() {
                return None;
            }
            Some(Site {
                key,
                name: {
                    let name = text(item, "name");
                    if name.is_empty() {
                        "未命名源".to_string()
                    } else {
                        name
                    }
                },
                api: text(item, "api"),
            })
        })
        .collect())
}

/// `{categories:[{id,name,pid}]}`.
pub fn parse_categories(value: &Value) -> Result<Vec<Category>, String> {
    error_or(value)?;
    let Some(items) = value["categories"].as_array() else {
        return Err("响应缺少 categories 数组".into());
    };
    Ok(categories_from(items))
}

/// The short-drama routes wrap their class list in `{code, data}`.
pub fn parse_duanju_categories(value: &Value) -> Result<Vec<Category>, String> {
    error_or(value)?;
    let Some(items) = value["data"].as_array() else {
        return Err("响应缺少 data 数组".into());
    };
    Ok(categories_from(items))
}

fn categories_from(items: &[Value]) -> Vec<Category> {
    items
        .iter()
        .filter_map(|item| {
            let id = text(item, "id");
            if id.is_empty() {
                return None;
            }
            let name = text(item, "name");
            Some(Category {
                id,
                name: if name.is_empty() {
                    "未命名分类".to_string()
                } else {
                    name
                },
                pid: {
                    let pid = text(item, "pid");
                    if pid.is_empty() { "0".to_string() } else { pid }
                },
            })
        })
        .collect()
}

/// `{code:200,data:[{key,name,api,typeId,typeName}]}` from `/api/duanju/sources`.
pub fn parse_duanju_sites(value: &Value) -> Result<Vec<DuanjuSite>, String> {
    error_or(value)?;
    let Some(items) = value["data"].as_array() else {
        return Err("响应缺少 data 数组".into());
    };
    Ok(items
        .iter()
        .filter_map(|item| {
            let key = text(item, "key");
            if key.is_empty() {
                return None;
            }
            let name = text(item, "name");
            Some(DuanjuSite {
                key,
                name: if name.is_empty() {
                    "未命名短剧源".to_string()
                } else {
                    name
                },
                api: text(item, "api"),
                type_id: text(item, "typeId"),
                type_name: text(item, "typeName"),
            })
        })
        .collect())
}

/// Parses the shared `{results|data, total, page, pageCount}` envelope used by
/// source browsing, source search and short drama.
pub fn parse_page(base: &str, value: &Value) -> Result<Page, String> {
    error_or(value)?;
    let items = value["results"]
        .as_array()
        .or_else(|| value["data"].as_array())
        .ok_or_else(|| "响应缺少 results/data 数组".to_string())?;
    let parsed: Vec<Item> = items
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
                year: {
                    let year = text(item, "year");
                    if year == "unknown" {
                        String::new()
                    } else {
                        year
                    }
                },
                episodes: string_list(item, "episodes"),
                episode_titles: string_list(item, "episodes_titles"),
                source: text(item, "source"),
                source_name: text(item, "source_name"),
                desc: text(item, "desc"),
                type_name: text(item, "type_name"),
                remarks: text(item, "vod_remarks"),
            })
        })
        .collect();
    let page = number(value, "page").max(1);
    let page_count = number(value, "pageCount");
    Ok(Page {
        items: parsed,
        page,
        page_count,
        total: number(value, "total"),
    })
}

fn error_or(value: &Value) -> Result<(), String> {
    if let Some(error) = value["error"].as_str().filter(|text| !text.is_empty()) {
        return Err(error.to_string());
    }
    if let Some(code) = value["code"].as_i64()
        && code != 200
    {
        let message = value["message"].as_str().unwrap_or("接口没有返回成功状态");
        let detail = value["error"].as_str().unwrap_or("");
        return Err(if detail.is_empty() {
            message.to_string()
        } else {
            format!("{message}：{detail}")
        });
    }
    if value["error"].is_string() || value["success"] == false {
        return Err(value["error"]
            .as_str()
            .unwrap_or("接口没有返回成功状态")
            .to_string());
    }
    Ok(())
}

/// A CMS class list with at least one child pointing at a real parent is
/// hierarchical; orphan children are treated as roots, like the web client.
pub fn is_hierarchical(list: &[Category]) -> bool {
    let ids: Vec<&str> = list.iter().map(|item| item.id.as_str()).collect();
    list.iter()
        .any(|item| item.pid != "0" && ids.contains(&item.pid.as_str()))
}

pub fn parents(list: &[Category]) -> Vec<Category> {
    let ids: Vec<&str> = list.iter().map(|item| item.id.as_str()).collect();
    list.iter()
        .filter(|item| item.pid == "0" || !ids.contains(&item.pid.as_str()))
        .cloned()
        .collect()
}

pub fn children(list: &[Category], parent: &str) -> Vec<Category> {
    list.iter()
        .filter(|item| item.pid == parent)
        .cloned()
        .collect()
}

/// The default selection for a source: first child of the first parent, or
/// the parent itself when it has no children; first category for flat lists.
/// `preferred` is the short-drama source's marked type id, when there is one.
pub fn default_selection(list: &[Category], preferred: &str) -> (String, String) {
    if list.is_empty() {
        return (String::new(), String::new());
    }
    if !preferred.is_empty()
        && let Some(found) = list.iter().find(|item| item.id == preferred)
    {
        if found.pid != "0" {
            return (found.pid.clone(), found.id.clone());
        }
        let kids = children(list, &found.id);
        return (
            found.id.clone(),
            kids.first()
                .map(|child| child.id.clone())
                .unwrap_or_else(|| found.id.clone()),
        );
    }
    if !is_hierarchical(list) {
        return (String::new(), list[0].id.clone());
    }
    let first = parents(list).into_iter().next().unwrap_or_default();
    let kids = children(list, &first.id);
    (
        first.id.clone(),
        kids.first()
            .map(|child| child.id.clone())
            .unwrap_or(first.id),
    )
}

/// Appends a page, ignoring repeated ids. The count is what was new.
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

/// Whether a label/type looks like short drama. Used only for UI hints; the
/// server has the authoritative source list and category selection.
pub fn looks_like_duanju(text: &str) -> bool {
    let text = text.to_lowercase();
    text.contains("短剧") || text.contains("短视频") || text.contains("微短剧")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn category(id: &str, name: &str, pid: &str) -> Category {
        Category {
            id: id.into(),
            name: name.into(),
            pid: pid.into(),
        }
    }

    #[test]
    fn hierarchies_match_the_web_rules() {
        let hierarchical = vec![
            category("1", "电影", "0"),
            category("2", "剧集", "0"),
            category("21", "国产剧", "2"),
            category("22", "日剧", "2"),
        ];
        assert!(is_hierarchical(&hierarchical));
        assert_eq!(
            parents(&hierarchical)
                .iter()
                .map(|item| item.id.as_str())
                .collect::<Vec<_>>(),
            ["1", "2"]
        );
        assert_eq!(
            children(&hierarchical, "2")
                .iter()
                .map(|item| item.id.as_str())
                .collect::<Vec<_>>(),
            ["21", "22"]
        );
        assert_eq!(
            default_selection(&hierarchical, ""),
            ("1".into(), "1".into()),
            "first parent without children falls back to itself"
        );

        let flat = vec![category("1", "电影", "0"), category("2", "剧集", "0")];
        assert!(!is_hierarchical(&flat));
        assert_eq!(default_selection(&flat, ""), (String::new(), "1".into()));

        let orphan = vec![category("1", "电影", "0"), category("2", "子类", "999")];
        assert!(!is_hierarchical(&orphan));
        assert_eq!(parents(&orphan).len(), 2);
    }

    #[test]
    fn short_drama_preferred_category_is_honoured() {
        let list = vec![
            category("8", "短剧", "0"),
            category("81", "甜宠", "8"),
            category("82", "逆袭", "8"),
        ];
        assert_eq!(
            default_selection(&list, "8"),
            ("8".into(), "81".into()),
            "the marked parent selects its first child"
        );
        assert_eq!(default_selection(&list, "82"), ("8".into(), "82".into()));
    }

    #[test]
    fn parses_sites_categories_and_pages() {
        let sites = parse_sites(&serde_json::json!({
            "sources": [{"key": "a", "name": "源 A", "api": "https://a/api"}]
        }))
        .unwrap();
        assert_eq!(sites[0].key, "a");
        assert_eq!(sites[0].name, "源 A");

        let categories = parse_categories(&serde_json::json!({
            "categories": [
                {"id": 1, "name": "电影", "pid": "0"},
                {"id": 11, "name": "华语", "pid": 1}
            ]
        }))
        .unwrap();
        assert_eq!(categories[1].pid, "1");

        let page = parse_page(
            "https://tv.example/moon/",
            &serde_json::json!({                "results": [{
                    "id": 1001, "title": "回声档案", "poster": "/p.jpg",
                    "year": "unknown", "episodes": ["a.m3u8"],
                    "source": "a", "source_name": "源 A", "type_name": "短剧"
                }],
                "total": 1, "page": 1, "pageCount": 3
            }),
        )
        .unwrap();
        assert_eq!(page.page_count, 3);
        assert_eq!(page.items[0].id, "1001");
        assert_eq!(page.items[0].poster, "https://tv.example/p.jpg");
        assert!(page.items[0].year.is_empty());
        assert_eq!(page.items[0].episodes.len(), 1);

        let duanju = parse_page(
            "https://tv.example/",
            &serde_json::json!({"code": 200, "data": [], "page": 1, "pageCount": 0}),
        )
        .unwrap();
        assert!(duanju.items.is_empty());
    }

    #[test]
    fn errors_are_not_empty_successes() {
        assert!(
            parse_page(
                "x",
                &serde_json::json!({"error": "未找到指定的视频源: nope"})
            )
            .unwrap_err()
            .contains("nope")
        );
        assert!(
            parse_categories(&serde_json::json!({"code": 500, "message": "获取分类列表失败"}))
                .unwrap_err()
                .contains("分类")
        );
        assert!(
            parse_sites(&serde_json::json!({"error": "Unauthorized"}))
                .unwrap_err()
                .contains("Unauthorized")
        );
    }

    #[test]
    fn merge_skips_repeated_ids() {
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
    fn duanju_labels_are_recognised() {
        assert!(looks_like_duanju("短剧"));
        assert!(looks_like_duanju("微短剧"));
        assert!(looks_like_duanju("SHORT短视频"));
        assert!(!looks_like_duanju("电影"));
    }
}
