//! Adapter audited against MoonTVPlus 5b132342 source contracts.
use crate::api_transport::{self as transport, Response as HttpResponse, request, send};
pub use crate::api_transport::{ApiError, sync_supported};
use crate::{
    discover::{self, Day, Item, Query},
    logbook,
    model::{Candidate, FavoriteEntry, ProgressEntry, api_url, encode_query, resolve_url},
    source_browse::{self, Category, DuanjuSite, Page, Site},
};
use serde_json::Value;
pub struct Login {
    pub token: String,
    pub username: String,
}
fn parse_json(response: &HttpResponse) -> Result<Value, ApiError> {
    serde_json::from_str(response.text()).map_err(|_| ApiError::BadResponse("响应不是 JSON".into()))
}

pub async fn server_config(base: &str) -> Result<Value, ApiError> {
    parse_json(&send(request(api_url(base, "api/server-config"), ""), 20).await?)
}
pub fn validate_key(key: &str) -> Result<(), ApiError> {
    let parts: Vec<_> = key.split('+').collect();
    if parts.len() != 2 || parts.iter().any(|s| s.is_empty()) {
        Err(ApiError::BadResponse(
            "必须指定非空 source+id；已阻止可能清空全部数据的请求".into(),
        ))
    } else {
        Ok(())
    }
}
pub fn favorite_payload(entry: &FavoriteEntry) -> Value {
    let mut f = entry.wire.as_object().cloned().unwrap_or_default();
    for(k,v)in serde_json::json!({"title":entry.title,"source_name":entry.source_name,"cover":entry.poster,"year":entry.year,"search_title":entry.title,"total_episodes":entry.total_episodes,"save_time":entry.saved_at}).as_object().unwrap(){f.insert(k.clone(),v.clone());}
    f.entry("origin").or_insert(serde_json::json!("vod"));
    serde_json::json!({"key":entry.key,"favorite":f})
}
async fn resolve_episodes(
    base: &str,
    token: &str,
    candidate: &mut Candidate,
) -> Result<(), ApiError> {
    candidate.episode_headers = vec![Default::default(); candidate.episodes.len()];
    for i in 0..candidate.episodes.len() {
        let absolute = resolve_url(base, &candidate.episodes[i]);
        let mut parsed = url::Url::parse(&absolute)
            .map_err(|_| ApiError::BadResponse("剧集地址格式不正确".into()))?;
        if crate::model::same_origin(base, &absolute)
            && parsed.path().ends_with("/api/source-script/play")
        {
            parsed.query_pairs_mut().append_pair("format", "json");
            let v = parse_json(&send(request(parsed.to_string(), token), 20).await?)?;
            candidate.episodes[i] = resolve_url(
                base,
                v["url"]
                    .as_str()
                    .filter(|s| !s.is_empty())
                    .ok_or_else(|| ApiError::BadResponse("懒解析没有返回媒体地址".into()))?,
            );
            if let Some(headers) = v["headers"].as_object() {
                for (k, v) in headers {
                    if let Some(v) = v.as_str() {
                        candidate.episode_headers[i].insert(k.clone(), v.into());
                    }
                }
            }
        } else {
            candidate.episodes[i] = absolute;
        }
    }
    Ok(())
}
pub fn progress_payload(e: &ProgressEntry) -> Result<Value, ApiError> {
    validate_key(&e.key)?;
    if !e.position.is_finite()
        || !e.duration.is_finite()
        || e.duration <= 0.0
        || e.position < 0.0
        || e.title.is_empty()
        || e.source_name.is_empty()
    {
        return Err(ApiError::BadResponse(
            "进度尚未有效，不覆盖已保存记录".into(),
        ));
    }
    let mut r = e.wire.as_object().cloned().unwrap_or_default();
    for(k,v)in serde_json::json!({"title":e.title,"source_name":e.source_name,"cover":e.poster,"year":e.year,"index":e.episode_index+1,"total_episodes":e.total_episodes,"play_time":e.position.min(e.duration),"total_time":e.duration,"save_time":e.updated_at,"search_title":e.title}).as_object().unwrap(){r.insert(k.clone(),v.clone());}
    if e.duanju {
        r.insert("is_duanju".into(), serde_json::json!(true));
    }
    r.entry("origin").or_insert(serde_json::json!("vod"));
    Ok(serde_json::json!({"key":e.key,"record":r}))
}
pub fn progress_from_map(base: &str, map: &Value) -> Result<Vec<ProgressEntry>, ApiError> {
    let map = map
        .as_object()
        .ok_or_else(|| ApiError::BadResponse("历史列表不是对象".into()))?;
    let mut out = Vec::new();
    for (key, v) in map {
        if validate_key(key).is_err() || v["origin"] == "live" {
            continue;
        }
        let (source, id) = key.split_once('+').unwrap();
        // The record table has no `origin` column, so live TV entries stored by
        // the web client are only recognisable by their `live_` source prefix.
        if source.starts_with("live_") {
            continue;
        }
        let index = v["index"].as_u64().unwrap_or(0);
        if index == 0 || string(v, "title").is_empty() {
            continue;
        }
        let title = string(v, "title");
        let year = string(v, "year");
        let position = v["play_time"].as_f64().unwrap_or(0.0);
        let duration = v["total_time"].as_f64().unwrap_or(0.0);
        if duration <= 0.0 || position < 0.0 {
            continue;
        }
        out.push(ProgressEntry {
            key: key.clone(),
            source: source.into(),
            id: id.into(),
            title: title.clone(),
            year: year.clone(),
            poster: resolve_url(base, &string(v, "cover")),
            source_name: string(v, "source_name"),
            episode_index: index as usize - 1,
            episode_title: format!("第 {index} 集"),
            position,
            duration,
            total_episodes: v["total_episodes"].as_u64().unwrap_or(0) as usize,
            updated_at: v["save_time"].as_u64().unwrap_or(0),
            work_key: format!("{title}\u{1f}{year}"),
            duanju: boolean(v, "is_duanju"),
            synced: Some(true),
            wire: v.clone(),
        });
    }
    Ok(out)
}
pub async fn playrecords(base: &str, token: &str) -> Result<Vec<ProgressEntry>, ApiError> {
    if !sync_supported() {
        return Err(ApiError::Unsupported(
            "此实例为 localstorage 模式，收藏与观看记录仅保存在本机".into(),
        ));
    }
    let v = parse_json(&send(request(api_url(base, "api/playrecords"), token), 20).await?)?;
    progress_from_map(base, &v)
}
pub async fn save_progress(base: &str, token: &str, e: &ProgressEntry) -> Result<(), ApiError> {
    if !sync_supported() {
        return Err(ApiError::Unsupported("此实例不支持服务端记录同步".into()));
    }
    let body = progress_payload(e)?;
    let mut r = request(api_url(base, "api/playrecords"), token);
    r.method = "POST".into();
    r.headers
        .push(("Content-Type".into(), "application/json".into()));
    r.body = body.to_string().into_bytes();
    let v = parse_json(&send(r, 20).await?)?;
    if v["success"] == true {
        Ok(())
    } else {
        Err(ApiError::BadResponse("实例未确认记录保存".into()))
    }
}
fn string(value: &Value, key: &str) -> String {
    match value.get(key) {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Number(n)) => n.to_string(),
        Some(Value::Bool(b)) => b.to_string(),
        _ => String::new(),
    }
}

fn boolean(value: &Value, key: &str) -> bool {
    value.get(key).and_then(Value::as_bool).unwrap_or(false)
}

fn number(value: &Value, key: &str) -> i64 {
    value.get(key).and_then(|v| v.as_i64()).unwrap_or(0)
}

fn strings(value: &Value, key: &str) -> Vec<String> {
    value
        .get(key)
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| match item {
                    Value::String(s) => Some(s.clone()),
                    Value::Number(n) => Some(n.to_string()),
                    _ => None,
                })
                .collect()
        })
        .unwrap_or_default()
}

pub(crate) fn candidate_from(
    base: &str,
    value: &Value,
    fallback: Option<&Candidate>,
) -> Option<Candidate> {
    let source = string(value, "source");
    let id = string(value, "id");
    if source.is_empty() || id.is_empty() {
        return None;
    }
    let title = {
        let title = string(value, "title");
        if title.is_empty() {
            fallback.map(|c| c.title.clone()).unwrap_or_default()
        } else {
            title
        }
    };
    let year = {
        let year = string(value, "year");
        if year.is_empty() {
            fallback.map(|c| c.year.clone()).unwrap_or_default()
        } else {
            year
        }
    };
    let poster_raw = {
        let poster = string(value, "poster");
        if poster.is_empty() {
            string(value, "cover")
        } else {
            poster
        }
    };
    let poster = if poster_raw.is_empty() {
        fallback.map(|c| c.poster.clone()).unwrap_or_default()
    } else {
        resolve_url(base, &poster_raw)
    };
    let source_name = {
        let name = string(value, "source_name");
        if name.is_empty() {
            fallback
                .map(|c| c.source_name.clone())
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| source.clone())
        } else {
            name
        }
    };
    let episodes = strings(value, "episodes");
    let episode_titles = strings(value, "episodes_titles");

    let desc = {
        let desc = string(value, "desc");
        if desc.is_empty() {
            fallback.map(|c| c.desc.clone()).unwrap_or_default()
        } else {
            desc
        }
    };
    let kind = {
        let kind = string(value, "type_name");
        if kind.is_empty() {
            string(value, "class")
        } else {
            kind
        }
    };
    let remarks = string(value, "vod_remarks");
    Some(Candidate {
        source,
        id,
        source_name,
        title,
        year,
        poster,
        desc,
        kind,
        remarks,
        episodes,
        episode_titles,
        proxy_mode: boolean(value, "proxyMode"),
        weight: number(value, "weight"),
        ..Candidate::default()
    })
}

pub async fn login(base: &str, username: &str, password: &str) -> Result<Login, ApiError> {
    let config = server_config(base).await?;
    if config["LoginRequireTurnstile"].as_bool() == Some(true) {
        return Err(ApiError::Unsupported(
            "此实例要求浏览器人机验证；原生安全回传尚未验证，不能绕过".into(),
        ));
    }
    let storage = config["StorageType"]
        .as_str()
        .ok_or_else(|| ApiError::BadResponse("配置缺少 StorageType".into()))?;
    let url = api_url(base, "api/login");
    let body = serde_json::json!({"username": username, "password": password}).to_string();
    let mut request = request(url, "");
    request.method = "POST".to_owned();
    request
        .headers
        .push(("Content-Type".to_owned(), "application/json".to_owned()));
    request.body = body.into_bytes();
    let response = send(request, 15).await?;
    let value = parse_json(&response)?;
    let ok = value.get("ok").and_then(Value::as_bool).unwrap_or(false);
    let token = string(&value, "token");
    if !ok || token.is_empty() {
        return Err(ApiError::Unauthorized);
    }
    let name = value
        .get("auth")
        .and_then(|auth| auth.get("username"))
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .unwrap_or(username)
        .to_owned();
    logbook::event(
        "login",
        serde_json::json!({"base": logbook::safe_url(base), "user": crate::model::short_hash(&name)}),
    );
    transport::install(base, &token, storage);
    Ok(Login {
        token,
        username: name,
    })
}

pub async fn search(base: &str, token: &str, query: &str) -> Result<Vec<Candidate>, ApiError> {
    let url = api_url(base, &format!("api/search?q={}", encode_query(query)));
    let response = send(request(url, token), 20).await?;
    let value = parse_json(&response)?;
    let Some(results) = value.get("results").and_then(Value::as_array) else {
        return Err(ApiError::BadResponse("缺少 results 数组".to_owned()));
    };
    let candidates: Vec<Candidate> = results
        .iter()
        .filter_map(|item| candidate_from(base, item, None))
        .collect();
    logbook::event(
        "search",
        serde_json::json!({"query_len": query.chars().count(), "results": candidates.len()}),
    );
    Ok(candidates)
}

/// Search completion hints from the instance. They are advisory: a slow or
/// failed suggestion request never blocks typing or a real search.
pub async fn suggestions(base: &str, token: &str, query: &str) -> Result<Vec<String>, ApiError> {
    let url = api_url(
        base,
        &format!("api/search/suggestions?q={}", encode_query(query)),
    );
    let response = send(request(url, token), 12).await?;
    let value = parse_json(&response)?;
    let items = value["suggestions"].as_array().cloned().unwrap_or_default();
    let mut out: Vec<String> = Vec::new();
    for item in items {
        let text = item["text"].as_str().unwrap_or("").trim();
        if text.is_empty() || out.iter().any(|known| known == text) {
            continue;
        }
        out.push(text.to_string());
        if out.len() >= 8 {
            break;
        }
    }
    Ok(out)
}

fn string_list(value: &Value) -> Result<Vec<String>, ApiError> {
    let Some(items) = value.as_array() else {
        if let Some(error) = value["error"].as_str() {
            return Err(ApiError::BadResponse(error.to_string()));
        }
        return Err(ApiError::BadResponse("搜索历史不是数组".to_string()));
    };
    Ok(items
        .iter()
        .filter_map(Value::as_str)
        .map(|text| text.trim().to_string())
        .filter(|text| !text.is_empty())
        .collect())
}

pub async fn search_history(base: &str, token: &str) -> Result<Vec<String>, ApiError> {
    let response = send(request(api_url(base, "api/searchhistory"), token), 12).await?;
    string_list(&parse_json(&response)?)
}

pub async fn add_search_history(
    base: &str,
    token: &str,
    keyword: &str,
) -> Result<Vec<String>, ApiError> {
    let mut request = request(api_url(base, "api/searchhistory"), token);
    request.method = "POST".into();
    request
        .headers
        .push(("Content-Type".into(), "application/json".into()));
    request.body = serde_json::json!({"keyword": keyword})
        .to_string()
        .into_bytes();
    let response = send(request, 12).await?;
    string_list(&parse_json(&response)?)
}

/// Deletes one keyword, or the whole history when `keyword` is `None`. The
/// server's DELETE without a keyword is the documented clear-all path.
pub async fn delete_search_history(
    base: &str,
    token: &str,
    keyword: Option<&str>,
) -> Result<(), ApiError> {
    let path = match keyword {
        Some(keyword) => format!("api/searchhistory?keyword={}", encode_query(keyword)),
        None => "api/searchhistory".to_string(),
    };
    let mut request = request(api_url(base, &path), token);
    request.method = "DELETE".into();
    let response = send(request, 12).await?;
    let value = parse_json(&response)?;
    if value["success"].as_bool() == Some(true) {
        Ok(())
    } else {
        Err(ApiError::BadResponse(
            "实例没有确认搜索历史删除".to_string(),
        ))
    }
}

/// Douban discover data through the instance's proxy routes.
pub async fn discover(base: &str, token: &str, query: &Query) -> Result<Vec<Item>, ApiError> {
    let response = send(request(discover::url(base, query), token), 20).await?;
    let value = parse_json(&response)?;
    discover::parse_list(base, &value).map_err(ApiError::BadResponse)
}

/// The Bangumi daily calendar, proxied by the instance.
pub async fn bangumi_calendar(base: &str, token: &str) -> Result<Vec<Day>, ApiError> {
    let response = send(request(api_url(base, "api/bangumi/calendar"), token), 20).await?;
    let value = parse_json(&response)?;
    discover::parse_calendar(base, &value).map_err(ApiError::BadResponse)
}

fn source_error(what: &str, error: ApiError) -> ApiError {
    match error {
        ApiError::Http(404) => ApiError::BadResponse(format!(
            "{what}：这个源不在当前账号的可用列表里（可能被禁用、取消授权或已删除）；刷新源列表后重试"
        )),
        ApiError::Http(500) => ApiError::BadResponse(format!(
            "{what}：源站暂时不可用或返回了异常数据；稍后重试或换一个源"
        )),
        other => other,
    }
}

/// Sources the signed-in account may browse.
pub async fn source_sites(base: &str, token: &str) -> Result<Vec<Site>, ApiError> {
    let response = send(
        request(api_url(base, "api/source-search/sources"), token),
        20,
    )
    .await
    .map_err(|error| source_error("读取片源列表失败", error))?;
    source_browse::parse_sites(&parse_json(&response)?).map_err(ApiError::BadResponse)
}

pub async fn source_categories(
    base: &str,
    token: &str,
    source: &str,
) -> Result<Vec<Category>, ApiError> {
    let response = send(
        request(
            api_url(
                base,
                &format!(
                    "api/source-search/categories?source={}",
                    encode_query(source)
                ),
            ),
            token,
        ),
        20,
    )
    .await
    .map_err(|error| source_error("读取片源分类失败", error))?;
    source_browse::parse_categories(&parse_json(&response)?).map_err(ApiError::BadResponse)
}

pub async fn source_videos(
    base: &str,
    token: &str,
    source: &str,
    category: &str,
    page: usize,
) -> Result<Page, ApiError> {
    let response = send(
        request(
            api_url(
                base,
                &format!(
                    "api/source-search/videos?source={}&categoryId={}&page={page}",
                    encode_query(source),
                    encode_query(category)
                ),
            ),
            token,
        ),
        25,
    )
    .await
    .map_err(|error| source_error("读取片源列表失败", error))?;
    source_browse::parse_page(base, &parse_json(&response)?).map_err(ApiError::BadResponse)
}

pub async fn source_search(
    base: &str,
    token: &str,
    source: &str,
    keyword: &str,
    page: usize,
) -> Result<Page, ApiError> {
    let response = send(
        request(
            api_url(
                base,
                &format!(
                    "api/source-search/search?source={}&keyword={}&page={page}",
                    encode_query(source),
                    encode_query(keyword)
                ),
            ),
            token,
        ),
        25,
    )
    .await
    .map_err(|error| source_error("源内搜索失败", error))?;
    source_browse::parse_page(base, &parse_json(&response)?).map_err(ApiError::BadResponse)
}

/// Sources that expose a short-drama category. The server checks the source
/// list itself; no account token is required by the route.
pub async fn duanju_sites(base: &str, token: &str) -> Result<Vec<DuanjuSite>, ApiError> {
    let response = send(request(api_url(base, "api/duanju/sources"), token), 30).await?;
    source_browse::parse_duanju_sites(&parse_json(&response)?).map_err(ApiError::BadResponse)
}

pub async fn duanju_categories(
    base: &str,
    token: &str,
    source: &str,
) -> Result<Vec<Category>, ApiError> {
    let response = send(
        request(
            api_url(
                base,
                &format!("api/duanju/categories?source={}", encode_query(source)),
            ),
            token,
        ),
        25,
    )
    .await
    .map_err(|error| source_error("读取短剧分类失败", error))?;
    source_browse::parse_duanju_categories(&parse_json(&response)?).map_err(ApiError::BadResponse)
}

pub async fn duanju_videos(
    base: &str,
    token: &str,
    source: &str,
    category: &str,
    page: usize,
) -> Result<Page, ApiError> {
    let response = send(
        request(
            api_url(
                base,
                &format!(
                    "api/duanju/videos?source={}&categoryId={}&page={page}",
                    encode_query(source),
                    encode_query(category)
                ),
            ),
            token,
        ),
        25,
    )
    .await
    .map_err(|error| source_error("读取短剧列表失败", error))?;
    source_browse::parse_page(base, &parse_json(&response)?).map_err(ApiError::BadResponse)
}

pub async fn duanju_recommends(
    base: &str,
    token: &str,
) -> Result<Vec<source_browse::Item>, ApiError> {
    let response = send(request(api_url(base, "api/duanju/recommends"), token), 30).await?;
    let page =
        source_browse::parse_page(base, &parse_json(&response)?).map_err(ApiError::BadResponse)?;
    Ok(page.items)
}

pub async fn detail(
    base: &str,
    token: &str,
    source: &str,
    id: &str,
    fallback: Option<&Candidate>,
) -> Result<Candidate, ApiError> {
    let url = api_url(
        base,
        &format!(
            "api/source-detail?source={}&id={}",
            encode_query(source),
            encode_query(id)
        ),
    );
    let response = send(request(url, token), 20).await?;
    let value = parse_json(&response)?;
    let mut candidate = candidate_from(base, &value, fallback)
        .filter(|c| !c.source.is_empty() && !c.id.is_empty())
        .ok_or_else(|| ApiError::BadResponse("详情缺少片源或 ID".to_owned()))?;
    if candidate.episodes.is_empty() {
        logbook::event(
            "detail_without_episodes",
            serde_json::json!({"source": source, "id": id}),
        );
    }
    resolve_episodes(base, token, &mut candidate).await?;
    Ok(candidate)
}

/// The instance's favorites, keyed by `source+id`, as local entries.
pub async fn favorites(base: &str, token: &str) -> Result<Vec<FavoriteEntry>, ApiError> {
    let response = send(request(api_url(base, "api/favorites"), token), 20).await?;
    let value = parse_json(&response)?;
    let Some(map) = value.as_object() else {
        return Err(ApiError::BadResponse("收藏列表不是对象".to_owned()));
    };
    let mut entries = Vec::new();
    for (key, favorite) in map {
        let Some((source, id)) = key.split_once('+') else {
            continue;
        };
        if source.is_empty() || id.is_empty() {
            continue;
        }
        let title = string(favorite, "title");
        if title.is_empty() {
            continue;
        }
        let poster_raw = {
            let poster = string(favorite, "cover");
            if poster.is_empty() {
                string(favorite, "poster")
            } else {
                poster
            }
        };
        entries.push(FavoriteEntry {
            key: key.clone(),
            source: source.to_owned(),
            id: id.to_owned(),
            title,
            year: string(favorite, "year"),
            poster: resolve_url(base, &poster_raw),
            source_name: {
                let name = string(favorite, "source_name");
                if name.is_empty() {
                    source.to_owned()
                } else {
                    name
                }
            },
            total_episodes: favorite
                .get("total_episodes")
                .and_then(Value::as_u64)
                .unwrap_or(0) as usize,
            saved_at: favorite
                .get("save_time")
                .and_then(Value::as_u64)
                .unwrap_or_else(|| nsh_core::time::unix_ms() as u64),
            synced: Some(true),
            wire: favorite.clone(),
        });
    }
    entries.sort_by_key(|e| std::cmp::Reverse(e.saved_at));
    Ok(entries)
}

pub async fn save_favorite(base: &str, token: &str, entry: &FavoriteEntry) -> Result<(), ApiError> {
    validate_key(&entry.key)?;
    let body = favorite_payload(entry).to_string();
    let mut request = request(api_url(base, "api/favorites"), token);
    request.method = "POST".to_owned();
    request
        .headers
        .push(("Content-Type".to_owned(), "application/json".to_owned()));
    request.body = body.into_bytes();
    let response = send(request, 20).await?;
    let value = parse_json(&response)?;
    if value.get("success").and_then(Value::as_bool) == Some(true) {
        Ok(())
    } else {
        Err(ApiError::BadResponse("实例没有确认收藏保存".to_owned()))
    }
}

pub async fn delete_favorite(base: &str, token: &str, key: &str) -> Result<(), ApiError> {
    validate_key(key)?;
    let url = api_url(base, &format!("api/favorites?key={}", encode_query(key)));
    let mut request = request(url, token);
    request.method = "DELETE".to_owned();
    let response = send(request, 20).await?;
    let value = parse_json(&response)?;
    if value.get("success").and_then(Value::as_bool) == Some(true) {
        Ok(())
    } else {
        Err(ApiError::BadResponse("实例没有确认收藏删除".to_owned()))
    }
}

/// The media URL to hand to the native player. A source in proxy mode is
/// played through the instance's own `/api/proxy/vod/m3u8`, matching the
/// upstream client's contract; the raw URL is used otherwise.
pub fn media_url(base: &str, candidate: &Candidate, episode_index: usize) -> Option<String> {
    let raw = candidate.episodes.get(episode_index)?;
    let raw = raw.trim();
    if raw.is_empty() {
        return None;
    }
    let resolved = resolve_url(base, raw);
    if candidate.proxy_mode {
        Some(api_url(
            base,
            &format!(
                "api/proxy/vod/m3u8?source={}&url={}",
                encode_query(&candidate.source),
                encode_query(&resolved)
            ),
        ))
    } else {
        Some(resolved)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn candidate_reads_tolerantly() {
        let value = serde_json::json!({
            "id": 42, "source": "src", "title": "片名", "year": 2024,
            "poster": "/p.jpg", "episodes": ["a.m3u8", "b.m3u8"],
            "episodes_titles": ["一", "二"], "proxyMode": true
        });
        let candidate = candidate_from("https://tv.example/moon/", &value, None).unwrap();
        assert_eq!(candidate.id, "42");
        assert_eq!(candidate.year, "2024");
        assert_eq!(candidate.poster, "https://tv.example/p.jpg");
        assert_eq!(candidate.episodes.len(), 2);
        assert!(candidate.proxy_mode);
    }

    #[test]
    fn proxy_url_encodes_the_media() {
        let candidate = Candidate {
            source: "a b".into(),
            proxy_mode: true,
            episodes: vec!["https://cdn.example/x.m3u8?token=1".into()],
            ..Candidate::default()
        };
        let url = media_url("https://tv.example/", &candidate, 0).unwrap();
        assert!(url.starts_with("https://tv.example/api/proxy/vod/m3u8?source=a%20b&url="));
        assert!(url.contains("https%3A%2F%2Fcdn.example"));
    }

    #[test]
    fn direct_media_resolves_relative_episode() {
        let candidate = Candidate {
            source: "s".into(),
            episodes: vec!["media/a.m3u8".into()],
            ..Candidate::default()
        };
        assert_eq!(
            media_url("https://tv.example/moon/", &candidate, 0).unwrap(),
            "https://tv.example/moon/media/a.m3u8"
        );
    }
}
