//! RSS Reader package — subscribe, fetch, parse, and AI-summarize RSS/Atom feeds.
//! AI capabilities go through weft core via the host chat_completion bridge.

use quick_xml::events::Event;
use quick_xml::Reader;
use serde_json::{json, Value};
use weft_package_sdk::*;

mod recommend;

const PACKAGE_NAME: &str = "rss-reader";
const CAPABILITY_NAME: &str = "rss.reader";

fn db_path() -> String {
    env_get("WEFT_RSS_DB_PATH").unwrap_or_else(|| "./data/rss.sqlite".into())
}

// ── parsed feed item ──

#[derive(Debug, Default, Clone)]
struct ParsedItem {
    guid: String,
    title: String,
    link: String,
    published_at: String,
    content: String,
}

// ── entrypoints ──

#[plugin_fn]
pub fn init(_input: String) -> FnResult<String> {
    ensure_schema().map_err(extism_pdk::Error::msg)?;
    log_info("rss-reader package initialized");
    Ok(PackageResult::ok_empty().to_json())
}

#[plugin_fn]
pub fn handle_ws_message(input: String) -> FnResult<String> {
    let req: WsRequest = serde_json::from_str(&input).unwrap_or(WsRequest {
        action: String::new(),
        data: Value::Null,
    });

    let result = match req.action.as_str() {
        "add_feed" => do_add_feed(&req.data),
        "remove_feed" => do_remove_feed(&req.data),
        "list_feeds" => do_list_feeds(),
        "refresh_feed" => do_refresh_feed(&req.data),
        "refresh_all" => do_refresh_all(),
        "list_articles" => do_list_articles(&req.data),
        "mark_read" => do_mark_read(&req.data),
        "mark_all_read" => do_mark_all_read(&req.data),
        "mark_favorite" => do_mark_favorite(&req.data),
        "summarize_article" => do_summarize_article(&req.data),
        "recommend_articles" => recommend::do_recommend_articles(&req.data),
        "set_ai_config" => do_set_ai_config(&req.data),
        "get_ai_config" => do_get_ai_config(),
        "explain_selection" => do_explain_selection(&req.data),
        "analyze_sections" => do_analyze_sections(&req.data),
        "chat_with_article" => do_chat_with_article(&req.data),
        "proxy_page" => do_proxy_page(&req.data),
        "translate_text" => do_translate_text(&req.data),
        "web_search" => do_web_search(&req.data),
        "describe" => do_describe(),
        "health" => do_health(),
        _ => PackageResult::err(format!("unknown action: {}", req.action)),
    };

    Ok(result.to_json())
}

#[plugin_fn]
pub fn describe(_input: String) -> FnResult<String> {
    Ok(do_describe().to_json())
}

#[plugin_fn]
pub fn health(_input: String) -> FnResult<String> {
    Ok(do_health().to_json())
}

fn do_describe() -> PackageResult {
    PackageResult::ok(json!({
        "package": PACKAGE_NAME,
        "runtime": "wasm",
        "capabilities": [CAPABILITY_NAME],
        "actions": {
            CAPABILITY_NAME: [
                "describe",
                "health",
                "add_feed",
                "remove_feed",
                "list_feeds",
                "refresh_feed",
                "refresh_all",
                "list_articles",
                "mark_read",
                "mark_all_read",
                "mark_favorite",
                "summarize_article",
                "recommend_articles",
                "set_ai_config",
                "get_ai_config",
                "explain_selection",
                "analyze_sections",
                "chat_with_article",
                "proxy_page",
                "translate_text",
                "web_search"
            ]
        }
    }))
}

fn do_health() -> PackageResult {
    PackageResult::ok(json!({
        "healthy": true,
        "package": PACKAGE_NAME,
        "capabilities": [CAPABILITY_NAME],
    }))
}

// ── schema ──

fn ensure_schema() -> Result<(), String> {
    let path = db_path();
    let statements: Vec<(String, Vec<Value>)> = vec![
        (
            "CREATE TABLE IF NOT EXISTS feeds (\
                id INTEGER PRIMARY KEY AUTOINCREMENT, \
                url TEXT NOT NULL UNIQUE, \
                title TEXT NOT NULL DEFAULT '', \
                added_at INTEGER NOT NULL)"
                .to_string(),
            vec![],
        ),
        (
            "CREATE TABLE IF NOT EXISTS articles (\
                id INTEGER PRIMARY KEY AUTOINCREMENT, \
                feed_id INTEGER NOT NULL, \
                guid TEXT NOT NULL, \
                title TEXT NOT NULL DEFAULT '', \
                link TEXT NOT NULL DEFAULT '', \
                published_at TEXT NOT NULL DEFAULT '', \
                content TEXT NOT NULL DEFAULT '', \
                summary TEXT NOT NULL DEFAULT '', \
                is_read INTEGER NOT NULL DEFAULT 0, \
                is_favorite INTEGER NOT NULL DEFAULT 0, \
                fetched_at INTEGER NOT NULL, \
                UNIQUE(feed_id, guid))"
                .to_string(),
            vec![],
        ),
        (
            "CREATE TABLE IF NOT EXISTS recommendations (\
                id INTEGER PRIMARY KEY AUTOINCREMENT, \
                profile_fingerprint TEXT NOT NULL, \
                candidate_hash TEXT NOT NULL, \
                result TEXT NOT NULL, \
                token_used INTEGER NOT NULL DEFAULT 1, \
                built_at INTEGER NOT NULL, \
                UNIQUE(profile_fingerprint, candidate_hash))"
                .to_string(),
            vec![],
        ),
        (
            "CREATE INDEX IF NOT EXISTS idx_articles_signal ON articles(is_read, is_favorite)".to_string(),
            vec![],
        ),
        (
            "CREATE TABLE IF NOT EXISTS settings (\
                key TEXT PRIMARY KEY, \
                value TEXT NOT NULL, \
                updated_at INTEGER NOT NULL)"
                .to_string(),
            vec![],
        ),
        (
            "CREATE TABLE IF NOT EXISTS conversations (\
                id INTEGER PRIMARY KEY AUTOINCREMENT, \
                article_id INTEGER NOT NULL, \
                role TEXT NOT NULL, \
                content TEXT NOT NULL, \
                created_at INTEGER NOT NULL)"
                .to_string(),
            vec![],
        ),
        (
            "CREATE INDEX IF NOT EXISTS idx_conv_article ON conversations(article_id, created_at)"
                .to_string(),
            vec![],
        ),
    ];
    sqlite_batch(&path, &statements).map(|_| ())?;

    // 老库迁移:为已存在的 articles 表补 is_favorite 列。
    // CREATE TABLE IF NOT EXISTS 不会给旧表加列,故单独 ALTER;
    // 列已存在时 ALTER 报 "duplicate column",此处忽略该错误(幂等)。
    let _ = sqlite_execute(
        &path,
        "ALTER TABLE articles ADD COLUMN is_favorite INTEGER NOT NULL DEFAULT 0",
        &[],
    );
    Ok(())
}

// ── actions ──

fn do_add_feed(data: &Value) -> PackageResult {
    let url = data.get("url").and_then(|v| v.as_str()).unwrap_or("").trim();
    if url.is_empty() {
        return PackageResult::err("add_feed requires data.url");
    }
    let path = db_path();
    let now = now_ms() as i64;

    if let Err(e) = sqlite_execute(
        &path,
        "INSERT OR IGNORE INTO feeds (url, title, added_at) VALUES (?, '', ?)",
        &[json!(url), json!(now)],
    ) {
        return PackageResult::err(e);
    }

    match sqlite_query(
        &path,
        "SELECT id, url, title, added_at FROM feeds WHERE url = ?",
        &[json!(url)],
    ) {
        Ok(res) => match res.rows.first() {
            Some(row) => PackageResult::ok(feed_row_to_json(&res.columns, row, None)),
            None => PackageResult::err("feed insert succeeded but row not found"),
        },
        Err(e) => PackageResult::err(e),
    }
}

fn do_remove_feed(data: &Value) -> PackageResult {
    let feed_id = match data.get("feed_id").and_then(|v| v.as_i64()) {
        Some(v) => v,
        None => return PackageResult::err("remove_feed requires data.feed_id"),
    };
    let path = db_path();

    let statements: Vec<(String, Vec<Value>)> = vec![
        (
            "DELETE FROM articles WHERE feed_id = ?".to_string(),
            vec![json!(feed_id)],
        ),
        (
            "DELETE FROM feeds WHERE id = ?".to_string(),
            vec![json!(feed_id)],
        ),
    ];
    match sqlite_batch(&path, &statements) {
        Ok(_) => PackageResult::ok(json!({ "feed_id": feed_id, "removed": true })),
        Err(e) => PackageResult::err(e),
    }
}

fn do_list_feeds() -> PackageResult {
    let path = db_path();
    let sql = "SELECT f.id, f.url, f.title, f.added_at, \
        (SELECT COUNT(*) FROM articles a WHERE a.feed_id = f.id AND a.is_read = 0) AS unread \
        FROM feeds f ORDER BY f.added_at DESC";
    match sqlite_query(&path, sql, &[]) {
        Ok(res) => {
            let feeds: Vec<Value> = res
                .rows
                .iter()
                .map(|row| feed_row_to_json(&res.columns, row, Some("unread")))
                .collect();
            PackageResult::ok(json!({ "feeds": feeds }))
        }
        Err(e) => PackageResult::err(e),
    }
}

fn do_refresh_feed(data: &Value) -> PackageResult {
    let feed_id = match data.get("feed_id").and_then(|v| v.as_i64()) {
        Some(v) => v,
        None => return PackageResult::err("refresh_feed requires data.feed_id"),
    };
    let path = db_path();

    let url = match sqlite_query(
        &path,
        "SELECT url FROM feeds WHERE id = ?",
        &[json!(feed_id)],
    ) {
        Ok(res) => match res.rows.first().and_then(|r| r.first()).and_then(|v| v.as_str()) {
            Some(u) => u.to_string(),
            None => return PackageResult::err(format!("feed {} not found", feed_id)),
        },
        Err(e) => return PackageResult::err(e),
    };

    match refresh_one(&path, feed_id, &url) {
        Ok(added) => PackageResult::ok(json!({ "feed_id": feed_id, "added": added })),
        Err(e) => PackageResult::err(e),
    }
}

fn do_refresh_all() -> PackageResult {
    let path = db_path();
    let feeds = match sqlite_query(&path, "SELECT id, url FROM feeds", &[]) {
        Ok(res) => res.rows,
        Err(e) => return PackageResult::err(e),
    };

    let mut added_total: i64 = 0;
    let mut errors: Vec<Value> = Vec::new();
    for row in &feeds {
        let id = row.first().and_then(|v| v.as_i64()).unwrap_or(0);
        let url = row.get(1).and_then(|v| v.as_str()).unwrap_or("").to_string();
        if url.is_empty() {
            continue;
        }
        match refresh_one(&path, id, &url) {
            Ok(added) => added_total += added,
            Err(e) => errors.push(json!({ "feed_id": id, "error": e })),
        }
    }

    PackageResult::ok(json!({ "added_total": added_total, "errors": errors }))
}

/// Fetch + parse + dedup-insert one feed. Returns number of newly added articles.
fn refresh_one(path: &str, feed_id: i64, url: &str) -> Result<i64, String> {
    let xml = http_request(
        "GET",
        url,
        &[("User-Agent", "weft-rss-reader/0.1")],
        "",
    )?;

    let (feed_title, items) = parse_feed(&xml);
    let now = now_ms() as i64;

    // Backfill feed title if parsed non-empty and stored is empty.
    if let Some(ft) = feed_title.as_deref() {
        let ft = ft.trim();
        if !ft.is_empty() {
            let _ = sqlite_execute(
                path,
                "UPDATE feeds SET title = ? WHERE id = ? AND (title IS NULL OR title = '')",
                &[json!(ft), json!(feed_id)],
            );
        }
    }

    let mut added: i64 = 0;
    for item in &items {
        let mut guid = item.guid.trim().to_string();
        if guid.is_empty() {
            guid = item.link.trim().to_string();
        }
        if guid.is_empty() {
            // No stable identity; skip to avoid duplicate spam.
            continue;
        }
        let res = sqlite_execute(
            path,
            "INSERT OR IGNORE INTO articles \
                (feed_id, guid, title, link, published_at, content, summary, is_read, fetched_at) \
                VALUES (?, ?, ?, ?, ?, ?, '', 0, ?)",
            &[
                json!(feed_id),
                json!(guid),
                json!(item.title),
                json!(item.link),
                json!(item.published_at),
                json!(item.content),
                json!(now),
            ],
        )?;
        added += res.rows_affected as i64;
    }
    Ok(added)
}

fn do_list_articles(data: &Value) -> PackageResult {
    let path = db_path();
    let feed_id = data.get("feed_id").and_then(|v| v.as_i64());
    let unread_only = data
        .get("unread_only")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let favorites_only = data
        .get("favorites_only")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let limit = data.get("limit").and_then(|v| v.as_i64()).unwrap_or(50);

    let mut sql = String::from(
        "SELECT id, feed_id, guid, title, link, published_at, content, summary, is_read, is_favorite, fetched_at \
         FROM articles WHERE 1 = 1",
    );
    let mut params: Vec<Value> = Vec::new();
    if let Some(fid) = feed_id {
        sql.push_str(" AND feed_id = ?");
        params.push(json!(fid));
    }
    if favorites_only {
        sql.push_str(" AND is_favorite = 1");
    }
    if unread_only {
        sql.push_str(" AND is_read = 0");
    }
    sql.push_str(" ORDER BY published_at DESC, fetched_at DESC LIMIT ?");
    params.push(json!(limit));

    match sqlite_query(&path, &sql, &params) {
        Ok(res) => {
            let articles: Vec<Value> = res
                .rows
                .iter()
                .map(|row| row_to_object(&res.columns, row))
                .collect();
            PackageResult::ok(json!({ "articles": articles }))
        }
        Err(e) => PackageResult::err(e),
    }
}

fn do_mark_read(data: &Value) -> PackageResult {
    let article_id = match data.get("article_id").and_then(|v| v.as_i64()) {
        Some(v) => v,
        None => return PackageResult::err("mark_read requires data.article_id"),
    };
    let is_read = data.get("is_read").and_then(|v| v.as_bool()).unwrap_or(true);
    let flag: i64 = if is_read { 1 } else { 0 };
    let path = db_path();

    match sqlite_execute(
        &path,
        "UPDATE articles SET is_read = ? WHERE id = ?",
        &[json!(flag), json!(article_id)],
    ) {
        Ok(res) => PackageResult::ok(json!({
            "article_id": article_id,
            "is_read": is_read,
            "updated": res.rows_affected,
        })),
        Err(e) => PackageResult::err(e),
    }
}

fn do_mark_all_read(data: &Value) -> PackageResult {
    let path = db_path();
    let feed_id = data.get("feed_id").and_then(|v| v.as_i64());

    let (sql, params): (String, Vec<Value>) = match feed_id {
        Some(fid) => (
            "UPDATE articles SET is_read = 1 WHERE is_read = 0 AND feed_id = ?".to_string(),
            vec![json!(fid)],
        ),
        None => (
            "UPDATE articles SET is_read = 1 WHERE is_read = 0".to_string(),
            vec![],
        ),
    };

    match sqlite_execute(&path, &sql, &params) {
        Ok(res) => PackageResult::ok(json!({ "updated": res.rows_affected })),
        Err(e) => PackageResult::err(e),
    }
}

fn do_mark_favorite(data: &Value) -> PackageResult {
    let article_id = match data.get("article_id").and_then(|v| v.as_i64()) {
        Some(v) => v,
        None => return PackageResult::err("mark_favorite requires data.article_id"),
    };
    // 默认置为收藏;传 is_favorite:false 取消收藏。
    let is_favorite = data
        .get("is_favorite")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    let flag = if is_favorite { 1 } else { 0 };
    let path = db_path();

    match sqlite_execute(
        &path,
        "UPDATE articles SET is_favorite = ? WHERE id = ?",
        &[json!(flag), json!(article_id)],
    ) {
        Ok(res) => PackageResult::ok(json!({
            "article_id": article_id,
            "is_favorite": is_favorite,
            "updated": res.rows_affected,
        })),
        Err(e) => PackageResult::err(e),
    }
}

// ── AI config (settings-based, package-level API key) ──

fn do_set_ai_config(data: &Value) -> PackageResult {
    let path = db_path();
    let now = now_ms() as i64;

    let fields = [
        ("ai.base_url", data.get("base_url").and_then(|v| v.as_str())),
        ("ai.api_key", data.get("api_key").and_then(|v| v.as_str())),
        ("ai.model", data.get("model").and_then(|v| v.as_str())),
        ("ai.provider", data.get("provider").and_then(|v| v.as_str())),
    ];
    for (key, val) in &fields {
        if let Some(v) = val {
            if let Err(e) = sqlite_execute(
                &path,
                "INSERT OR REPLACE INTO settings (key, value, updated_at) VALUES (?, ?, ?)",
                &[json!(key), json!(v), json!(now)],
            ) {
                return PackageResult::err(e);
            }
        }
    }
    // Return current config (key masked).
    do_get_ai_config()
}

fn do_get_ai_config() -> PackageResult {
    let path = db_path();
    let base_url = setting_get(&path, "ai.base_url").unwrap_or_default();
    let api_key = setting_get(&path, "ai.api_key").unwrap_or_default();
    let model = setting_get(&path, "ai.model").unwrap_or_default();
    PackageResult::ok(json!({
        "base_url": base_url,
        "model": model,
        "has_api_key": !api_key.is_empty(),
        "api_key_masked": if api_key.len() > 8 {
            format!("{}...{}", &api_key[..4], &api_key[api_key.len()-4..])
        } else if !api_key.is_empty() {
            "****".to_string()
        } else {
            String::new()
        },
    }))
}

fn setting_get(path: &str, key: &str) -> Option<String> {
    sqlite_query(path, "SELECT value FROM settings WHERE key = ?", &[json!(key)])
        .ok()
        .and_then(|r| r.rows.first()?.first()?.as_str().map(|s| s.to_string()))
}

// ── AI reading assistant actions ──

fn do_explain_selection(data: &Value) -> PackageResult {
    let text = match data.get("text").and_then(|v| v.as_str()) {
        Some(t) if !t.trim().is_empty() => t.trim(),
        _ => return PackageResult::err("explain_selection requires data.text"),
    };
    let question = data.get("question").and_then(|v| v.as_str()).unwrap_or("请解释这段内容");
    let context = data.get("context").and_then(|v| v.as_str()).unwrap_or("");

    let system = "你是AI阅读助手。用户在阅读文章时选中了一段文本并提问。请针对选中内容给出准确、简洁的回答。如果有上下文，结合上下文理解。用中文回答。";
    let user_msg = if context.is_empty() {
        format!("选中文本：「{}」\n\n问题：{}", text, question)
    } else {
        format!("文章上下文：{}\n\n选中文本：「{}」\n\n问题：{}", context, text, question)
    };

    match ai_chat(system, &user_msg, "rss:explain") {
        Ok(result) => PackageResult::ok(json!({ "explanation": result })),
        Err(e) => PackageResult::err(e),
    }
}

fn do_analyze_sections(data: &Value) -> PackageResult {
    let article_id = match data.get("article_id").and_then(|v| v.as_i64()) {
        Some(v) => v,
        None => return PackageResult::err("analyze_sections requires data.article_id"),
    };
    let path = db_path();

    let res = match sqlite_query(
        &path,
        "SELECT title, content FROM articles WHERE id = ?",
        &[json!(article_id)],
    ) {
        Ok(r) => r,
        Err(e) => return PackageResult::err(e),
    };
    let row = match res.rows.first() {
        Some(r) => r,
        None => return PackageResult::err(format!("article {} not found", article_id)),
    };
    let title = row.first().and_then(|v| v.as_str()).unwrap_or("");
    let content = row.get(1).and_then(|v| v.as_str()).unwrap_or("");

    if content.trim().is_empty() {
        return PackageResult::err("文章无正文内容，无法分段分析");
    }

    let system = "你是AI阅读助手。对文章进行渐进式分析：\n1. 先给出一段总体概要（2-3句）\n2. 将正文按逻辑分段，每段给出：段落序号、该段核心内容一句话概括、该段的前几个关键词\n严格输出JSON格式：{\"overview\":\"总体概要\",\"sections\":[{\"index\":1,\"summary\":\"该段讲什么\",\"keywords\":[\"关键词1\",\"关键词2\"],\"start_text\":\"该段开头的前20个字（用于前端定位锚点）\"}]}。不要markdown。";
    let user_msg = format!("标题：{}\n\n正文：{}", title, &content[..content.len().min(6000)]);

    match ai_chat(system, &user_msg, "rss:sections") {
        Ok(result) => {
            // 尝试解析为JSON，失败则原样返回
            let cleaned = extract_json_str(&result);
            match serde_json::from_str::<Value>(&cleaned) {
                Ok(parsed) => PackageResult::ok(json!({ "analysis": parsed, "article_id": article_id })),
                Err(_) => PackageResult::ok(json!({ "analysis_raw": result, "article_id": article_id })),
            }
        }
        Err(e) => PackageResult::err(e),
    }
}

fn do_chat_with_article(data: &Value) -> PackageResult {
    let article_id = match data.get("article_id").and_then(|v| v.as_i64()) {
        Some(v) => v,
        None => return PackageResult::err("chat_with_article requires data.article_id"),
    };
    let message = match data.get("message").and_then(|v| v.as_str()) {
        Some(m) if !m.trim().is_empty() => m.trim().to_string(),
        _ => return PackageResult::err("chat_with_article requires data.message"),
    };
    let path = db_path();
    let now = now_ms() as i64;

    // 获取文章标题和摘要作为系统上下文
    let article_ctx = sqlite_query(
        &path,
        "SELECT title, content FROM articles WHERE id = ?",
        &[json!(article_id)],
    )
    .ok()
    .and_then(|r| r.rows.first().cloned())
    .map(|row| {
        let t = row.first().and_then(|v| v.as_str()).unwrap_or("");
        let c = row.get(1).and_then(|v| v.as_str()).unwrap_or("");
        let snippet = if c.len() > 2000 { &c[..2000] } else { c };
        format!("文章标题：{}\n正文摘录：{}", t, snippet)
    })
    .unwrap_or_default();

    // 读取历史对话（最近20条）
    let history = sqlite_query(
        &path,
        "SELECT role, content FROM conversations WHERE article_id = ? ORDER BY created_at DESC LIMIT 20",
        &[json!(article_id)],
    )
    .ok()
    .map(|r| r.rows)
    .unwrap_or_default();

    // 构建 messages（按时间正序）
    let system = format!(
        "你是AI阅读助手。用户正在阅读一篇文章并与你讨论。基于文章内容回答问题，准确简洁，用中文。\n\n{}",
        article_ctx
    );
    let mut messages_str = format!("{{\"role\":\"system\",\"content\":{}}}", json!(system));

    // 历史（倒序读的，需反转）
    for row in history.iter().rev() {
        let role = row.first().and_then(|v| v.as_str()).unwrap_or("user");
        let content = row.get(1).and_then(|v| v.as_str()).unwrap_or("");
        messages_str.push_str(&format!(",{{\"role\":{},\"content\":{}}}", json!(role), json!(content)));
    }
    // 当前用户消息
    messages_str.push_str(&format!(",{{\"role\":\"user\",\"content\":{}}}", json!(message)));

    let model = setting_get(&path, "ai.model").unwrap_or_else(|| "deepseek-v4-flash".into());
    let body = format!(
        "{{\"model\":{},\"messages\":[{}],\"temperature\":0.4}}",
        json!(model),
        messages_str
    );

    // 保存用户消息
    let _ = sqlite_execute(
        &path,
        "INSERT INTO conversations (article_id, role, content, created_at) VALUES (?, 'user', ?, ?)",
        &[json!(article_id), json!(message), json!(now)],
    );

    // 调 AI
    let base_url = setting_get(&path, "ai.base_url").unwrap_or_default();
    let api_key = setting_get(&path, "ai.api_key").unwrap_or_default();

    let reply = if !base_url.is_empty() && !api_key.is_empty() {
        let url = format!("{}/v1/chat/completions", base_url.trim_end_matches('/'));
        let auth = format!("Bearer {}", api_key);
        http_request("POST", &url, &[
            ("Authorization", &auth),
            ("Content-Type", "application/json"),
        ], &body)
        .and_then(|r| parse_chat_content(&r))
    } else {
        chat_completion("rss:chat", "", &body)
            .and_then(|r| parse_chat_content(&r))
    };

    match reply {
        Ok(assistant_msg) => {
            // 保存助手回复
            let _ = sqlite_execute(
                &path,
                "INSERT INTO conversations (article_id, role, content, created_at) VALUES (?, 'assistant', ?, ?)",
                &[json!(article_id), json!(assistant_msg), json!(now)],
            );
            PackageResult::ok(json!({
                "reply": assistant_msg,
                "article_id": article_id,
            }))
        }
        Err(e) => PackageResult::err(e),
    }
}

fn do_web_search(data: &Value) -> PackageResult {
    let query = match data.get("query").and_then(|v| v.as_str()) {
        Some(q) if !q.trim().is_empty() => q.trim(),
        _ => return PackageResult::err("web_search requires data.query"),
    };
    let limit = data.get("limit").and_then(|v| v.as_i64()).unwrap_or(8);

    // 用 Firecrawl 免 key 搜索端点（不需要 API key，国内可达）。
    let body = json!({ "query": query, "limit": limit }).to_string();
    let raw = match http_request(
        "POST",
        "https://api.firecrawl.dev/v1/search",
        &[("Content-Type", "application/json")],
        &body,
    ) {
        Ok(r) => r,
        Err(e) => return PackageResult::err(format!("web_search 请求 Firecrawl 失败: {}", e)),
    };

    let parsed: Value = match serde_json::from_str(&raw) {
        Ok(v) => v,
        Err(e) => return PackageResult::err(format!("web_search 解析 Firecrawl 返回失败: {}", e)),
    };

    // Firecrawl: {"success":true,"data":[{url,title,description},...]}
    if parsed.get("success").and_then(|v| v.as_bool()) != Some(true) {
        let msg = parsed.get("error").and_then(|v| v.as_str()).unwrap_or("Firecrawl 搜索失败");
        return PackageResult::err(format!("web_search: {}", msg));
    }

    // 精简结果：只保留 title/url/description，减少 token。
    let results: Vec<Value> = parsed
        .get("data")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .map(|item| {
                    json!({
                        "title": item.get("title").and_then(|v| v.as_str()).unwrap_or(""),
                        "url": item.get("url").and_then(|v| v.as_str()).unwrap_or(""),
                        "description": item.get("description").and_then(|v| v.as_str()).unwrap_or(""),
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    PackageResult::ok(json!({ "query": query, "results": results }))
}

fn do_proxy_page(data: &Value) -> PackageResult {
    let url = match data.get("url").and_then(|v| v.as_str()) {
        Some(u) if !u.trim().is_empty() => u.trim(),
        _ => return PackageResult::err("proxy_page requires data.url"),
    };
    // 安全：只允许 http/https
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return PackageResult::err("proxy_page only supports http/https URLs");
    }

    match http_request("GET", url, &[
        ("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36"),
        ("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8"),
    ], "") {
        Ok(html) => PackageResult::ok(json!({ "html": html, "url": url })),
        Err(e) => PackageResult::err(format!("proxy_page fetch failed: {}", e)),
    }
}

fn do_translate_text(data: &Value) -> PackageResult {
    let text = match data.get("text").and_then(|v| v.as_str()) {
        Some(t) if !t.trim().is_empty() => t.trim().to_string(),
        _ => return PackageResult::err("translate_text requires data.text"),
    };
    let target_lang = data.get("target").and_then(|v| v.as_str()).unwrap_or("zh");
    let source_lang = data.get("source").and_then(|v| v.as_str()).unwrap_or("auto");

    let path = db_path();
    let provider = setting_get(&path, "translate.provider").unwrap_or_else(|| "mymemory".into());

    match provider.as_str() {
        "microsoft" => translate_microsoft(&text, source_lang, target_lang, &path),
        "llm" => translate_llm(&text, target_lang),
        "google" => translate_google(&text, source_lang, target_lang),
        _ => translate_mymemory(&text, source_lang, target_lang), // 默认用 MyMemory（免费、不需key、国内可用）
    }
}

/// Google Translate 免费端点（不需要 API key）
fn translate_google(text: &str, source: &str, target: &str) -> PackageResult {
    let url = format!(
        "https://translate.googleapis.com/translate_a/single?client=gtx&sl={}&tl={}&dt=t&q={}",
        source, target, urlencod(text)
    );
    match http_request("GET", &url, &[
        ("User-Agent", "Mozilla/5.0"),
    ], "") {
        Ok(resp) => {
            // Google 返回嵌套 JSON 数组：[[["翻译结果","原文",...],...],...]
            let parsed: Value = serde_json::from_str(&resp).unwrap_or(Value::Null);
            let mut result = String::new();
            if let Some(sentences) = parsed.get(0).and_then(|v| v.as_array()) {
                for s in sentences {
                    if let Some(translated) = s.get(0).and_then(|v| v.as_str()) {
                        result.push_str(translated);
                    }
                }
            }
            if result.is_empty() {
                PackageResult::err("Google Translate 返回为空")
            } else {
                PackageResult::ok(json!({ "translated": result, "provider": "google", "source": source, "target": target }))
            }
        }
        Err(e) => PackageResult::err(format!("Google Translate 请求失败: {}", e)),
    }
}

/// MyMemory 免费翻译 API（不需要 key，国内可用，每天 5000 词免费）
fn translate_mymemory(text: &str, source: &str, target: &str) -> PackageResult {
    let sl = if source == "auto" { "en" } else { source };
    // MyMemory 需要 langpair=en|zh 格式
    let url = format!(
        "https://api.mymemory.translated.net/get?q={}&langpair={}|{}",
        urlencod(text), sl, target
    );
    match http_request("GET", &url, &[("User-Agent", "Mozilla/5.0")], "") {
        Ok(resp) => {
            let parsed: Value = serde_json::from_str(&resp).unwrap_or(Value::Null);
            let translated = parsed.get("responseData")
                .and_then(|v| v.get("translatedText"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if translated.is_empty() {
                PackageResult::err("MyMemory 翻译返回为空")
            } else {
                PackageResult::ok(json!({ "translated": translated, "provider": "mymemory", "source": sl, "target": target }))
            }
        }
        Err(e) => PackageResult::err(format!("MyMemory 翻译请求失败: {}", e)),
    }
}

/// 微软翻译（需要 Azure Translator key，存在 settings translate.api_key）
fn translate_microsoft(text: &str, source: &str, target: &str, settings_path: &str) -> PackageResult {
    let api_key = match setting_get(settings_path, "translate.api_key") {
        Some(k) if !k.is_empty() => k,
        _ => return PackageResult::err("微软翻译需要配置 translate.api_key（Azure Translator Key）"),
    };
    let region = setting_get(settings_path, "translate.region").unwrap_or_else(|| "global".into());
    let url = format!(
        "https://api.cognitive.microsofttranslator.com/translate?api-version=3.0&from={}&to={}",
        if source == "auto" { "" } else { source }, target
    );
    let body = serde_json::to_string(&vec![json!({"Text": text})]).unwrap_or_default();
    match http_request("POST", &url, &[
        ("Ocp-Apim-Subscription-Key", &api_key),
        ("Ocp-Apim-Subscription-Region", &region),
        ("Content-Type", "application/json"),
    ], &body) {
        Ok(resp) => {
            let parsed: Value = serde_json::from_str(&resp).unwrap_or(Value::Null);
            let translated = parsed.get(0)
                .and_then(|v| v.get("translations"))
                .and_then(|v| v.get(0))
                .and_then(|v| v.get("text"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if translated.is_empty() {
                PackageResult::err("微软翻译返回为空")
            } else {
                PackageResult::ok(json!({ "translated": translated, "provider": "microsoft", "source": source, "target": target }))
            }
        }
        Err(e) => PackageResult::err(format!("微软翻译请求失败: {}", e)),
    }
}

/// LLM 翻译（走 ai_chat）
fn translate_llm(text: &str, target: &str) -> PackageResult {
    let lang_name = match target {
        "zh" | "zh-CN" => "简体中文",
        "en" => "English",
        "ja" => "日本語",
        _ => target,
    };
    let system = &format!("你是翻译器。将下面的文本翻译成{}，只输出翻译结果，不要解释。", lang_name);
    match ai_chat(system, text, "rss:translate") {
        Ok(result) => PackageResult::ok(json!({ "translated": result, "provider": "llm", "target": target })),
        Err(e) => PackageResult::err(e),
    }
}

/// 简单 URL 编码（WASM 里没有 percent_encoding crate，手写基础版）
fn urlencod(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(b as char),
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}

/// 提取JSON字符串（剥离```json围栏）
fn extract_json_str(s: &str) -> String {
    let mut t = s.trim();
    if let Some(rest) = t.strip_prefix("```json") {
        t = rest;
    } else if let Some(rest) = t.strip_prefix("```") {
        t = rest;
    }
    if let Some(rest) = t.strip_suffix("```") {
        t = rest;
    }
    let t = t.trim();
    if let (Some(start), Some(end)) = (t.find('{'), t.rfind('}')) {
        if end >= start {
            return t[start..=end].to_string();
        }
    }
    t.to_string()
}

/// Unified AI chat call: uses package-level config if set, else falls back to core default.
/// Exposed as pub(crate) so recommend.rs can use it too.
pub(crate) fn ai_chat(system_prompt: &str, user_content: &str, label: &str) -> Result<String, String> {
    let path = db_path();
    let base_url = setting_get(&path, "ai.base_url").unwrap_or_default();
    let api_key = setting_get(&path, "ai.api_key").unwrap_or_default();
    let model = setting_get(&path, "ai.model").unwrap_or_else(|| "deepseek-v4-flash".into());
    let provider = setting_get(&path, "ai.provider").unwrap_or_default();

    let mut body_json = serde_json::json!({
        "model": model,
        "messages": [
            {"role": "system", "content": system_prompt},
            {"role": "user", "content": user_content},
        ],
        "temperature": 0.3,
    });
    // 指定供应商路由（core pipeline 的 x_provider 支持）
    if !provider.is_empty() {
        body_json["x_provider"] = serde_json::json!(provider);
    }

    let body = serde_json::to_string(&body_json)
        .map_err(|e| format!("build chat body: {}", e))?;

    if !base_url.is_empty() && !api_key.is_empty() {
        // Use package-level config: direct http_request to external endpoint.
        let url = format!("{}/v1/chat/completions", base_url.trim_end_matches('/'));
        let auth = format!("Bearer {}", api_key);
        let resp = http_request("POST", &url, &[
            ("Authorization", &auth),
            ("Content-Type", "application/json"),
        ], &body)?;
        parse_chat_content(&resp)
    } else {
        // Fallback: core default via host chat_completion.
        let resp = chat_completion(label, "", &body)?;
        parse_chat_content(&resp)
    }
}

fn parse_chat_content(resp_json: &str) -> Result<String, String> {
    let v: Value = serde_json::from_str(resp_json)
        .map_err(|e| format!("parse chat response: {}", e))?;
    v.get("choices")
        .and_then(|c| c.get(0))
        .and_then(|c| c.get("message"))
        .and_then(|m| m.get("content"))
        .and_then(|c| c.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| "chat response missing choices[0].message.content".into())
}

fn do_summarize_article(data: &Value) -> PackageResult {
    let article_id = match data.get("article_id").and_then(|v| v.as_i64()) {
        Some(v) => v,
        None => return PackageResult::err("summarize_article requires data.article_id"),
    };
    let mode = data
        .get("mode")
        .and_then(|v| v.as_str())
        .unwrap_or("summary");
    let path = db_path();

    let res = match sqlite_query(
        &path,
        "SELECT title, link, content, summary FROM articles WHERE id = ?",
        &[json!(article_id)],
    ) {
        Ok(res) => res,
        Err(e) => return PackageResult::err(e),
    };
    let row = match res.rows.first() {
        Some(r) => r,
        None => return PackageResult::err(format!("article {} not found", article_id)),
    };
    let title = row.first().and_then(|v| v.as_str()).unwrap_or("");
    let link = row.get(1).and_then(|v| v.as_str()).unwrap_or("");
    let content = row.get(2).and_then(|v| v.as_str()).unwrap_or("");
    let cached = row.get(3).and_then(|v| v.as_str()).unwrap_or("");

    // Return cached summary if present and we're in summary mode (no recompute).
    if mode == "summary" && !cached.trim().is_empty() {
        return PackageResult::ok(json!({ "summary": cached, "cached": true }));
    }

    let source = if content.trim().is_empty() {
        format!("{}\n{}", title, link)
    } else {
        content.to_string()
    };

    let system_prompt = match mode {
        "translate" => "把下面内容翻译成简体中文，保留原意",
        _ => "用简体中文 2-3 句概括这篇文章要点，不要寒暄",
    };

    let summary = match ai_chat(system_prompt, &source, "rss:summarize") {
        Ok(v) => v,
        Err(e) => return PackageResult::err(e),
    };

    // Cache the summary back onto the article.
    let _ = sqlite_execute(
        &path,
        "UPDATE articles SET summary = ? WHERE id = ?",
        &[json!(summary), json!(article_id)],
    );

    PackageResult::ok(json!({ "summary": summary, "cached": false, "mode": mode }))
}

// ── row helpers ──

fn row_to_object(columns: &[String], row: &[Value]) -> Value {
    let mut obj = serde_json::Map::new();
    for (i, col) in columns.iter().enumerate() {
        obj.insert(col.clone(), row.get(i).cloned().unwrap_or(Value::Null));
    }
    Value::Object(obj)
}

fn feed_row_to_json(columns: &[String], row: &[Value], unread_col: Option<&str>) -> Value {
    let mut obj = row_to_object(columns, row);
    if let (Some(col), Value::Object(map)) = (unread_col, &mut obj) {
        if !map.contains_key(col) {
            map.insert(col.to_string(), json!(0));
        }
    }
    obj
}

// ── RSS / Atom parsing (quick-xml) ──

fn parse_feed(xml: &str) -> (Option<String>, Vec<ParsedItem>) {
    let mut reader = Reader::from_str(xml);
    reader.trim_text(true);

    let mut items: Vec<ParsedItem> = Vec::new();
    let mut feed_title: Option<String> = None;

    let mut in_item = false; // inside <item> or <entry>
    let mut is_atom = false; // current container is an Atom <entry>
    let mut current = ParsedItem::default();

    // Track which element's text we are currently capturing.
    // Empty string = not capturing.
    let mut capture: &'static str = "";
    let mut text_buf = String::new();

    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) => {
                let name = local_name(e.name().as_ref());
                match name.as_str() {
                    "item" => {
                        in_item = true;
                        is_atom = false;
                        current = ParsedItem::default();
                    }
                    "entry" => {
                        in_item = true;
                        is_atom = true;
                        current = ParsedItem::default();
                    }
                    "title" => {
                        capture = "title";
                        text_buf.clear();
                    }
                    "link" => {
                        if in_item && is_atom {
                            // Atom link: prefer href attribute.
                            if let Some(href) = attr_value(&e, "href") {
                                if !href.is_empty() {
                                    current.link = href;
                                }
                            }
                        } else if in_item {
                            // RSS link: text content.
                            capture = "link";
                            text_buf.clear();
                        }
                    }
                    "guid" | "id" => {
                        if in_item {
                            capture = "guid";
                            text_buf.clear();
                        }
                    }
                    "pubDate" | "updated" | "published" => {
                        if in_item {
                            capture = "published";
                            text_buf.clear();
                        }
                    }
                    "description" | "summary" => {
                        if in_item {
                            capture = "content";
                            text_buf.clear();
                        }
                    }
                    // content:encoded (RSS) and Atom <content> both local-name "content".
                    "content" => {
                        if in_item {
                            capture = "content";
                            text_buf.clear();
                        }
                    }
                    "encoded" => {
                        // content:encoded — preferred RSS body.
                        if in_item {
                            capture = "encoded";
                            text_buf.clear();
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::Empty(e)) => {
                // Self-closing Atom <link href="..."/>.
                let name = local_name(e.name().as_ref());
                if name == "link" && in_item && is_atom {
                    if let Some(href) = attr_value(&e, "href") {
                        if !href.is_empty() {
                            current.link = href;
                        }
                    }
                }
            }
            Ok(Event::Text(t)) => {
                if !capture.is_empty() {
                    if let Ok(txt) = t.unescape() {
                        text_buf.push_str(&txt);
                    }
                }
            }
            Ok(Event::CData(t)) => {
                if !capture.is_empty() {
                    text_buf.push_str(&String::from_utf8_lossy(t.as_ref()));
                }
            }
            Ok(Event::End(e)) => {
                let name = local_name(e.name().as_ref());
                match name.as_str() {
                    "item" | "entry" => {
                        items.push(std::mem::take(&mut current));
                        in_item = false;
                        is_atom = false;
                    }
                    "title" => {
                        let val = std::mem::take(&mut text_buf);
                        if in_item {
                            current.title = val;
                        } else if feed_title.is_none() {
                            // Top-level <channel><title> or <feed><title>.
                            if !val.trim().is_empty() {
                                feed_title = Some(val);
                            }
                        }
                        capture = "";
                    }
                    "link" => {
                        if !capture.is_empty() && capture == "link" {
                            current.link = std::mem::take(&mut text_buf);
                        }
                        capture = "";
                    }
                    "guid" | "id" => {
                        if in_item && capture == "guid" {
                            current.guid = std::mem::take(&mut text_buf);
                        }
                        capture = "";
                    }
                    "pubDate" | "updated" | "published" => {
                        if in_item && capture == "published" {
                            let val = std::mem::take(&mut text_buf);
                            if current.published_at.is_empty() {
                                current.published_at = val;
                            }
                        }
                        capture = "";
                    }
                    "description" | "summary" => {
                        if in_item && capture == "content" {
                            let val = std::mem::take(&mut text_buf);
                            // description/summary 是最低优先级正文来源:
                            // 仅当还没有任何正文时才用(content:encoded / Atom <content> 更优)。
                            if current.content.trim().is_empty() {
                                current.content = val;
                            }
                        }
                        capture = "";
                    }
                    "content" => {
                        if in_item && capture == "content" {
                            let val = std::mem::take(&mut text_buf);
                            // Atom <content> / content:encoded 优于 summary:
                            // 只要新值更长(更完整),就覆盖先前的 summary 摘要。
                            if val.trim().len() > current.content.trim().len() {
                                current.content = val;
                            }
                        }
                        capture = "";
                    }
                    "encoded" => {
                        if in_item && capture == "encoded" {
                            // content:encoded takes priority over description.
                            current.content = std::mem::take(&mut text_buf);
                        }
                        capture = "";
                    }
                    _ => {
                        // Clear stray capture on unrelated end tags.
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
    }

    (feed_title, items)
}

/// Strip any namespace prefix from an XML element name (e.g. "content:encoded" -> "encoded").
fn local_name(raw: &[u8]) -> String {
    let s = String::from_utf8_lossy(raw);
    match s.rsplit_once(':') {
        Some((_, local)) => local.to_string(),
        None => s.to_string(),
    }
}

fn attr_value(e: &quick_xml::events::BytesStart, key: &str) -> Option<String> {
    for attr in e.attributes().flatten() {
        let attr_key = local_name(attr.key.as_ref());
        if attr_key == key {
            if let Ok(val) = attr.unescape_value() {
                return Some(val.to_string());
            }
        }
    }
    None
}
