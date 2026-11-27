//! Live contract test: drives this client's real API code (`src/api.rs`)
//! against an isolated instance started by
//! `tests/contract/isolated-run.mjs`.
//!
//! It is skipped unless `MOONTV_CONTRACT_BASE` is set, so `cargo test` stays
//! offline and green without a server. Nothing here writes to a shared
//! database: the runner creates a throwaway SQLite file and a loopback server.

use moontv_native::api;
use moontv_native::api_transport::{self, ApiError};
use moontv_native::model::{FavoriteEntry, ProgressEntry, group_results};

fn env(key: &str) -> Option<String> {
    std::env::var(key).ok().filter(|v| !v.trim().is_empty())
}

fn require(key: &str) -> String {
    env(key).unwrap_or_else(|| panic!("{key} must be set for the live contract test"))
}

/// Returns the token metadata with one field replaced, keeping the rest.
fn stale_token(token: &str, patch: impl Fn(&mut serde_json::Value)) -> String {
    let mut value = api_transport::token_metadata(token).expect("login token is JSON metadata");
    patch(&mut value);
    moontv_native::model::encode_query(&value.to_string())
}

#[test]
fn live_contract() {
    let Some(base) = env("MOONTV_CONTRACT_BASE") else {
        println!("live contract skipped: MOONTV_CONTRACT_BASE is not set");
        return;
    };
    let user = require("MOONTV_CONTRACT_USER");
    let pass = require("MOONTV_CONTRACT_PASS");

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio runtime");

    runtime.block_on(async move {
        // --- instance ---------------------------------------------------
        let config = api::server_config(&base).await.expect("server-config");
        assert_eq!(
            config["StorageType"].as_str(),
            Some("d1"),
            "isolated instance must report its storage type"
        );
        assert_eq!(config["LoginRequireTurnstile"].as_bool(), Some(false));

        // --- login ------------------------------------------------------
        let wrong = api::login(&base, &user, "definitely-wrong").await;
        assert!(
            matches!(wrong, Err(ApiError::Unauthorized)),
            "wrong password must map to Unauthorized, got {:?}",
            wrong.err()
        );

        let login = api::login(&base, &user, &pass).await.expect("login");
        assert_eq!(login.username, user);
        assert!(api_transport::sync_supported(), "d1 storage supports sync");

        let token = login.token.clone();
        let metadata = api_transport::token_metadata(&token).expect("token metadata");
        let access_ms = metadata["timestamp"].as_u64().expect("timestamp");
        let refresh_ms = metadata["refreshExpires"].as_u64().expect("refreshExpires");
        let sixty_days = 60 * 24 * 60 * 60 * 1000;
        let age = refresh_ms.saturating_sub(access_ms);
        assert!(
            age.abs_diff(sixty_days) < 5 * 60 * 1000,
            "refresh window should be 60 days, got {age} ms"
        );

        // --- search -----------------------------------------------------
        let results = api::search(&base, &token, "回声").await.expect("search");
        assert!(
            results.len() >= 4,
            "expected both fixture sources, got {results:?}"
        );
        let current = results
            .iter()
            .find(|c| c.source == "fixture-a" && c.id == "1001")
            .expect("fixture-a/1001 in results");
        assert_eq!(current.title, "回声档案");
        assert_eq!(current.year, "2026");
        assert_eq!(current.source_name, "夹具 A");
        assert_eq!(current.kind, "科幻");
        assert_eq!(current.episodes.len(), 2, "two .m3u8 episodes");
        assert_eq!(current.episode_titles.len(), 2);
        assert!(current.episodes.iter().all(|e| e.ends_with(".m3u8")));
        assert!(
            current.desc.contains("录音"),
            "html tags stripped: {}",
            current.desc
        );
        assert!(!current.desc.contains("<p>"));

        let mirror = results
            .iter()
            .find(|c| c.source == "fixture-c" && c.id == "2001")
            .expect("fixture-c mirror in results");
        assert_eq!(mirror.title, "回声档案");
        assert_eq!(mirror.year, "2026");
        assert_eq!(mirror.source_name, "夹具 C（慢）");
        assert_eq!(mirror.episodes.len(), 2);

        let works = group_results(results.clone());
        let years: Vec<&str> = works
            .iter()
            .filter(|w| w.title == "回声档案")
            .map(|w| w.year.as_str())
            .collect();
        assert!(years.contains(&"2026") && years.contains(&"2020"));
        assert_ne!(
            works.iter().filter(|w| w.title == "回声档案").count(),
            1,
            "same title in two years must not be merged"
        );
        let echo = works
            .iter()
            .find(|w| w.title == "回声档案" && w.year == "2026")
            .expect("2026 work");
        assert_eq!(echo.candidates.len(), 2, "two sources for one work");
        let backup = moontv_native::model::next_backup_candidate(
            echo,
            "fixture-a",
            &["fixture-a".to_string()],
        )
        .expect("a backup source");
        assert_eq!(backup.source, "fixture-c");
        assert_eq!(
            moontv_native::model::preferred_source(echo, Some("fixture-c"), None),
            Some("fixture-c".to_string())
        );
        assert_eq!(
            moontv_native::model::preferred_source(echo, Some("gone"), None),
            Some(echo.candidates[0].source.clone())
        );

        // --- incremental SSE search --------------------------------------
        let started = std::time::Instant::now();
        // A different query from the JSON search above: the instance caches
        // per-query results, and this test needs the slow source to be slow.
        let mut stream =
            moontv_native::search_stream::open(&base, &token, "暮色").expect("search stream");
        let mut first_source_at = None;
        let mut complete_at = None;
        let mut streamed = 0_usize;
        let mut completions = (0_usize, 0_usize);
        let mut order: Vec<String> = Vec::new();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(40);
        loop {
            let remaining = deadline.saturating_duration_since(std::time::Instant::now());
            if remaining.is_zero() {
                panic!("stream did not complete; events={order:?}");
            }
            let event = tokio::time::timeout(remaining, stream.next())
                .await
                .expect("stream idle timeout");
            let Some(event) = event else { break };
            match event {
                moontv_native::search_stream::Event::Start { total_sources } => {
                    assert_eq!(total_sources, 3, "three configured sources");
                    order.push("start".into());
                }
                moontv_native::search_stream::Event::SourceResults {
                    source,
                    results,
                    ..
                } => {
                    if source == "fixture-a" {
                        first_source_at = Some(started.elapsed());
                        assert_eq!(results.len(), 3, "catalog from fixture A");
                        streamed += results.len();
                    }
                    order.push(format!("source:{source}"));
                }
                moontv_native::search_stream::Event::SourceError { source, .. } => {
                    order.push(format!("error:{source}"));
                }
                moontv_native::search_stream::Event::Complete {
                    total_results,
                    completed_sources,
                } => {
                    complete_at = Some(started.elapsed());
                    completions = (total_results, completed_sources);
                    order.push("complete".into());
                    break;
                }
                moontv_native::search_stream::Event::Failed { error } => {
                    panic!("stream failed: {:?}", error.message())
                }
            }
        }
        assert_eq!(streamed, 3, "fixture A returned its three usable items");
        assert_eq!(
            completions,
            (4, 3),
            "two sources returned four results in total"
        );
        let first_at = first_source_at.expect("a source result arrived");
        let complete_at = complete_at.expect("complete arrived");
        assert!(
            complete_at - first_at >= std::time::Duration::from_millis(900),
            "the fast source must be delivered before the slow one completes: first={first_at:?} complete={complete_at:?} order={order:?}"
        );
        assert!(
            order.iter().position(|item| item == "source:fixture-a")
                < order.iter().position(|item| item == "complete"),
            "source_result precedes complete: {order:?}"
        );

        // --- suggestions -------------------------------------------------
        let hints = api::suggestions(&base, &token, "回")
            .await
            .expect("suggestions");
        assert!(
            hints.iter().any(|hint| hint == "回声档案"),
            "suggestions come from the first source's titles: {hints:?}"
        );

        // --- search history CRUD -----------------------------------------
        let initial = api::search_history(&base, &token)
            .await
            .expect("empty history");
        assert!(initial.is_empty());
        let after_one = api::add_search_history(&base, &token, "回声")
            .await
            .expect("add history");
        assert_eq!(after_one.first().map(String::as_str), Some("回声"));
        let after_two = api::add_search_history(&base, &token, "暮色")
            .await
            .expect("add second history");
        assert_eq!(after_two.first().map(String::as_str), Some("暮色"));
        assert!(after_two.iter().any(|item| item == "回声"));
        api::delete_search_history(&base, &token, Some("回声"))
            .await
            .expect("delete one keyword");
        let after_delete = api::search_history(&base, &token)
            .await
            .expect("history after delete");
        assert!(!after_delete.iter().any(|item| item == "回声"));
        assert!(after_delete.iter().any(|item| item == "暮色"));
        api::delete_search_history(&base, &token, None)
            .await
            .expect("clear history");
        assert!(
            api::search_history(&base, &token)
                .await
                .expect("cleared history")
                .is_empty()
        );

        // --- Bangumi calendar (fixture base URL configured by the runner) -
        let days = api::bangumi_calendar(&base, &token)
            .await
            .expect("bangumi calendar");
        assert_eq!(days.len(), 7, "a full week");
        let monday = days
            .iter()
            .find(|day| day.en == "monday")
            .expect("monday");
        assert_eq!(monday.cn, "星期一");
        assert_eq!(monday.items[0].title, "夹具番剧 1", "Chinese name first");
        assert_eq!(
            monday.items[1].title, "Original Only 1",
            "no Chinese name falls back to the original title"
        );
        assert!(
            monday.items[1].rate.is_empty(),
            "a zero score is not turned into a rating"
        );

        // --- source browsing (single source) -----------------------------
        let sites = api::source_sites(&base, &token)
            .await
            .expect("source sites");
        assert!(
            sites.iter().any(|site| site.key == "fixture-a"),
            "fixture A is available: {sites:?}"
        );
        assert_eq!(sites.len(), 3, "three enabled sources");
        let categories = api::source_categories(&base, &token, "fixture-a")
            .await
            .expect("source categories");
        assert!(
            categories.iter().any(|category| category.name == "电影"),
            "CMS classes reach the client: {categories:?}"
        );
        assert!(
            categories
                .iter()
                .any(|category| category.id == "81" && category.pid == "8"),
            "hierarchical children keep their parent"
        );
        let source_page = api::source_videos(&base, &token, "fixture-a", "1", 1)
            .await
            .expect("source videos page 1");
        assert_eq!(source_page.page_count, 2, "fixture pages hold two rows");
        assert_eq!(source_page.items.len(), 2);
        assert!(
            source_page.items.iter().all(|item| item.source == "fixture-a"
                && !item.episodes.is_empty()
                && item.episodes.iter().all(|episode| episode.ends_with(".m3u8"))),
            "source browse keeps the source identity and playable episodes"
        );
        let source_page2 = api::source_videos(&base, &token, "fixture-a", "1", 2)
            .await
            .expect("source videos page 2");
        assert_eq!(source_page2.page, 2);
        assert_eq!(source_page2.items.len(), 1);
        assert!(
            !source_page2.items.iter().any(|item| source_page
                .items
                .iter()
                .any(|first| first.id == item.id)),
            "the next page does not repeat page 1"
        );
        let source_hits = api::source_search(&base, &token, "fixture-a", "回声", 1)
            .await
            .expect("source search");
        assert_eq!(source_hits.items.len(), 2, "keyword matches two titles");
        assert!(
            source_hits
                .items
                .iter()
                .all(|item| item.title.contains("回声")),
            "the source-scoped search uses the source's own results"
        );
        let missing_source = api::source_categories(&base, &token, "nope").await;
        assert!(
            matches!(missing_source, Err(ApiError::BadResponse(ref message)) if message.contains("可用列表")),
            "an unavailable source is a precise error, got {missing_source:?}"
        );

        // --- short drama -------------------------------------------------
        let duanju_sites = api::duanju_sites(&base, &token)
            .await
            .expect("duanju sites");
        assert_eq!(duanju_sites.len(), 1, "only fixture A declares short drama");
        assert_eq!(duanju_sites[0].key, "fixture-a");
        assert_eq!(duanju_sites[0].type_id, "8");
        let duanju_categories = api::duanju_categories(&base, &token, "fixture-a")
            .await
            .expect("duanju categories");
        assert!(
            duanju_categories
                .iter()
                .any(|category| category.id == "8")
                && duanju_categories
                    .iter()
                    .any(|category| category.id == "81" && category.pid == "8"),
            "short drama keeps its parent and children: {duanju_categories:?}"
        );
        let duanju_page = api::duanju_videos(&base, &token, "fixture-a", "8", 1)
            .await
            .expect("duanju page 1");
        assert_eq!(duanju_page.page_count, 2);
        assert_eq!(duanju_page.items.len(), 2);
        assert!(
            duanju_page.items.iter().all(|item| !item.episodes.is_empty()),
            "short drama rows carry episodes for the detail page"
        );
        let duanju_page2 = api::duanju_videos(&base, &token, "fixture-a", "8", 2)
            .await
            .expect("duanju page 2");
        assert_eq!(duanju_page2.items.len(), 1);
        assert!(
            !duanju_page2.items.iter().any(|item| duanju_page
                .items
                .iter()
                .any(|first| first.id == item.id)),
            "short-drama pages do not repeat ids"
        );
        let recommends = api::duanju_recommends(&base, &token)
            .await
            .expect("duanju recommends");
        assert!(
            !recommends.is_empty()
                && recommends.iter().all(|item| !item.episodes.is_empty()),
            "recommendations are real rows with playable episodes: {recommends:?}"
        );

        // --- source detail ----------------------------------------------
        let detail = api::detail(&base, &token, "fixture-a", "1001", Some(current))
            .await
            .expect("source-detail");
        assert_eq!(
            detail.episodes.len(),
            detail.episode_headers.len(),
            "every episode carries its own header set"
        );
        assert!(
            detail.poster.starts_with("http"),
            "poster resolved: {}",
            detail.poster
        );
        let app_ua = api::media_url(&base, &detail, 0).expect("media url");
        assert!(
            app_ua.contains("/api/proxy-m3u8?url="),
            "the instance wraps configured sources for app user agents: {app_ua}"
        );

        // --- favorites --------------------------------------------------
        let before = api::favorites(&base, &token).await.expect("favorites");
        assert!(
            before.is_empty(),
            "fresh isolated database has no favorites"
        );

        let mut favorite = FavoriteEntry::from_candidate(&detail);
        favorite.saved_at = api_transport::now_ms();
        api::save_favorite(&base, &token, &favorite)
            .await
            .expect("save favorite");

        let saved = api::favorites(&base, &token).await.expect("favorites");
        let entry = saved
            .iter()
            .find(|f| f.key == favorite.key)
            .expect("saved entry");
        assert_eq!(entry.title, "回声档案");
        assert_eq!(entry.year, "2026");
        assert_eq!(entry.source_name, "夹具 A");
        assert_eq!(entry.total_episodes, 2);
        assert_eq!(entry.saved_at, favorite.saved_at);
        assert_eq!(
            entry.poster, favorite.poster,
            "cover comes back as a full URL"
        );
        assert_eq!(entry.synced, Some(true), "server copy is authoritative");
        assert_eq!(entry.wire["origin"], "vod");

        // --- play records ----------------------------------------------
        let mut progress = ProgressEntry {
            key: format!("{}+{}", detail.source, detail.id),
            source: detail.source.clone(),
            id: detail.id.clone(),
            title: detail.title.clone(),
            year: detail.year.clone(),
            poster: detail.poster.clone(),
            source_name: detail.source_name.clone(),
            episode_index: 1,
            episode_title: detail.episode_title(1),
            position: 37.5,
            duration: 1201.5,
            total_episodes: detail.episode_count(),
            updated_at: api_transport::now_ms(),
            work_key: format!("{}\u{1f}{}", detail.title, detail.year),
            duanju: false,
            synced: None,
            wire: serde_json::Value::Null,
        };
        api::save_progress(&base, &token, &progress)
            .await
            .expect("save progress");

        let records = api::playrecords(&base, &token).await.expect("playrecords");
        let stored = records
            .iter()
            .find(|r| r.key == progress.key)
            .expect("stored record");
        assert_eq!(stored.episode_index, 1, "1-based episode index round trip");
        assert!(
            (stored.position - 37.5).abs() < 0.001,
            "seconds are not scaled"
        );
        assert!((stored.duration - 1201.5).abs() < 0.001);
        assert_eq!(stored.year, "2026");
        assert_eq!(stored.total_episodes, 2);
        assert_eq!(stored.synced, Some(true));

        progress.position = 300.0;
        progress.updated_at = api_transport::now_ms();
        progress.synced = None;
        api::save_progress(&base, &token, &progress)
            .await
            .expect("update progress");
        let updated = api::playrecords(&base, &token).await.expect("playrecords");
        let same: Vec<&ProgressEntry> = updated.iter().filter(|r| r.key == progress.key).collect();
        assert_eq!(same.len(), 1, "re-saving the same key updates in place");
        assert!((same[0].position - 300.0).abs() < 0.001);

        // Short-drama records keep `is_duanju` so a resume can reopen the
        // portrait stage; a normal record must not overwrite it with false.
        progress.duanju = true;
        progress.position = 42.0;
        progress.updated_at = api_transport::now_ms();
        api::save_progress(&base, &token, &progress)
            .await
            .expect("save portrait progress");
        let portrait = api::playrecords(&base, &token)
            .await
            .expect("playrecords");
        let stored_portrait = portrait
            .iter()
            .find(|record| record.key == progress.key)
            .expect("portrait record");
        assert!(stored_portrait.duanju, "is_duanju survives the round trip");
        assert!((stored_portrait.position - 42.0).abs() < 0.001);

        // --- transparent refresh ---------------------------------------
        let stale = stale_token(&token, |value| {
            value["timestamp"] = serde_json::json!(
                api_transport::now_ms().saturating_sub(api_transport::ACCESS_AGE_MS + 60_000)
            );
        });
        api_transport::install(&base, &stale, "d1");
        let refreshed = api::favorites(&base, &stale)
            .await
            .expect("refresh then retry");
        assert!(refreshed.iter().any(|f| f.key == favorite.key));
        let renewed = api_transport::current_token().expect("renewed token");
        assert_ne!(renewed, stale, "the client keeps the rotated access token");
        api_transport::clear();

        // --- refresh window closed -------------------------------------
        let expired = stale_token(&token, |value| {
            value["refreshExpires"] =
                serde_json::json!(api_transport::now_ms().saturating_sub(1000));
        });
        api_transport::install(&base, &expired, "d1");
        let closed = api::favorites(&base, &expired).await;
        assert!(
            matches!(closed, Err(ApiError::Unauthorized)),
            "past 60 days the client must report an expired session, got {closed:?}"
        );
        api_transport::clear();

        // --- logout revokes --------------------------------------------
        api_transport::install(&base, &renewed, "d1");
        let logout = api_transport::take_logout().expect("logout request");
        let response = api_transport::raw(logout, 20).await.expect("logout call");
        assert!((200..300).contains(&response.status()), "logout accepted");

        api_transport::install(&base, &renewed, "d1");
        let revoked = api::favorites(&base, &renewed).await;
        assert!(
            matches!(revoked, Err(ApiError::Unauthorized)),
            "a revoked session cannot refresh, got {revoked:?}"
        );
        api_transport::clear();

        // --- cleanup ----------------------------------------------------
        let again = api::login(&base, &user, &pass).await.expect("second login");
        api::delete_favorite(&base, &again.token, &favorite.key)
            .await
            .expect("delete favorite");
        let empty = api::favorites(&base, &again.token)
            .await
            .expect("favorites");
        assert!(empty.is_empty(), "deleting by key clears only that entry");

        println!("live contract: client paths verified against {base}");
    });
}
