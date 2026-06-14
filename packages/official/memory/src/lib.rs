//! Memory package - SQLite-backed memory persistence with legacy KV migration.

use weft_package_sdk::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;

const MEMORY_SCHEMA_VERSION: u32 = 1;
const LEGACY_GLOBAL_AGENTS_INDEX_KEY: &str = "memory:__agents";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MemoryRecord {
    schema_version: u32,
    key: String,
    content: String,
    category: String,
    created_at: u64,
    updated_at: u64,
    #[serde(default)]
    expires_at: Option<u64>,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    metadata: Value,
}

#[derive(Debug, Serialize, Deserialize)]
struct StoreInput {
    agent: String,
    key: String,
    content: String,
    #[serde(default = "default_category")]
    category: String,
    #[serde(default)]
    expires_at: Option<u64>,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    metadata: Value,
    #[serde(default)]
    timestamp: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize)]
struct RecallInput {
    agent: String,
    #[serde(default)]
    query: String,
    #[serde(default = "default_limit")]
    limit: usize,
    #[serde(default)]
    category: String,
    #[serde(default)]
    categories: Vec<String>,
    #[serde(default)]
    include_expired: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct ForgetInput {
    agent: String,
    key: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ListInput {
    agent: String,
    #[serde(default)]
    category: String,
    #[serde(default)]
    categories: Vec<String>,
    #[serde(default)]
    include_expired: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct AgentInput {
    agent: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct CleanupInput {
    agent: String,
    #[serde(default)]
    now_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
struct MemoryView {
    key: String,
    category: String,
    content: String,
    created_at: u64,
    updated_at: u64,
    expires_at: Option<u64>,
    tags: Vec<String>,
    metadata: Value,
    score: i64,
}

fn default_category() -> String {
    "core".into()
}

fn default_limit() -> usize {
    10
}

fn current_time_ms() -> u64 {
    now_ms()
}

fn sanitize_category(category: &str) -> String {
    let trimmed = category.trim();
    if trimmed.is_empty() {
        default_category()
    } else {
        trimmed.to_string()
    }
}

fn memory_db_path() -> String {
    env_get("WEFT_MEMORY_DB_PATH").unwrap_or_else(|| "./data/memory.sqlite".into())
}

fn ensure_schema() -> Result<(), String> {
    let path = memory_db_path();
    sqlite_batch(
        &path,
        &[
            (
                "CREATE TABLE IF NOT EXISTS memory_records (
                    agent TEXT NOT NULL,
                    category TEXT NOT NULL,
                    key TEXT NOT NULL,
                    content TEXT NOT NULL,
                    created_at INTEGER NOT NULL,
                    updated_at INTEGER NOT NULL,
                    expires_at INTEGER,
                    tags_json TEXT NOT NULL,
                    metadata_json TEXT NOT NULL,
                    schema_version INTEGER NOT NULL,
                    PRIMARY KEY (agent, category, key)
                )"
                .into(),
                vec![],
            ),
            (
                "CREATE INDEX IF NOT EXISTS idx_memory_agent_updated
                 ON memory_records(agent, updated_at DESC)"
                    .into(),
                vec![],
            ),
            (
                "CREATE INDEX IF NOT EXISTS idx_memory_agent_category_updated
                 ON memory_records(agent, category, updated_at DESC)"
                    .into(),
                vec![],
            ),
            (
                "CREATE INDEX IF NOT EXISTS idx_memory_agent_expiry
                 ON memory_records(agent, expires_at)"
                    .into(),
                vec![],
            ),
        ],
    )
    .map(|_| ())
}

fn parse_legacy_record_value(raw: &str, key: &str, category: &str) -> MemoryRecord {
    serde_json::from_str::<MemoryRecord>(raw).unwrap_or_else(|_| MemoryRecord {
        schema_version: MEMORY_SCHEMA_VERSION,
        key: key.to_string(),
        content: raw.to_string(),
        category: category.to_string(),
        created_at: 0,
        updated_at: 0,
        expires_at: None,
        tags: Vec::new(),
        metadata: Value::Null,
    })
}

fn migrate_legacy_agent(agent: &str) -> Result<(), String> {
    let prefix = format!("memory:{}:", agent);
    let keys = kv_list(&prefix)?;
    if keys.is_empty() {
        return Ok(());
    }

    ensure_schema()?;
    for legacy_key in keys {
        if legacy_key.ends_with(":__index") || legacy_key == format!("memory:{}:__categories", agent)
        {
            let _ = kv_delete(&legacy_key);
            continue;
        }

        let Some(rest) = legacy_key.strip_prefix(&prefix) else {
            continue;
        };
        let Some((category, record_key)) = rest.split_once(':') else {
            continue;
        };
        let Some(raw) = kv_get(&legacy_key) else {
            continue;
        };
        if raw.trim().is_empty() {
            let _ = kv_delete(&legacy_key);
            continue;
        }

        let record = parse_legacy_record_value(&raw, record_key, category);
        // Legacy KV deletion may not flush before process shutdown.
        // Keep migration idempotent so repeated startups do not fail on duplicates.
        save_record_sqlite(agent, &record, true)?;
        let _ = kv_delete(&legacy_key);
    }

    let _ = kv_delete(&format!("memory:{}:__categories", agent));
    if let Some(global_agents_raw) = kv_get(LEGACY_GLOBAL_AGENTS_INDEX_KEY) {
        if let Ok(mut agents) = serde_json::from_str::<Vec<String>>(&global_agents_raw) {
            agents.retain(|item| item != agent);
            if agents.is_empty() {
                let _ = kv_delete(LEGACY_GLOBAL_AGENTS_INDEX_KEY);
            } else {
                kv_set(
                    LEGACY_GLOBAL_AGENTS_INDEX_KEY,
                    &serde_json::to_string(&agents).unwrap_or_else(|_| "[]".into()),
                );
            }
        }
    }

    Ok(())
}

fn migrate_legacy_agents(agents: &[String]) -> Result<(), String> {
    for agent in agents {
        migrate_legacy_agent(agent)?;
    }
    Ok(())
}

fn all_legacy_agents() -> Vec<String> {
    kv_get(LEGACY_GLOBAL_AGENTS_INDEX_KEY)
        .and_then(|raw| serde_json::from_str::<Vec<String>>(&raw).ok())
        .unwrap_or_default()
}

fn save_record_sqlite(agent: &str, record: &MemoryRecord, preserve_created_at: bool) -> Result<(), String> {
    let path = memory_db_path();
    let tags_json = serde_json::to_string(&record.tags).unwrap_or_else(|_| "[]".into());
    let metadata_json = serde_json::to_string(&record.metadata).unwrap_or_else(|_| "null".into());

    if preserve_created_at {
        sqlite_execute(
            &path,
            "INSERT INTO memory_records (
                agent, category, key, content, created_at, updated_at, expires_at,
                tags_json, metadata_json, schema_version
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(agent, category, key) DO UPDATE SET
                content = excluded.content,
                updated_at = excluded.updated_at,
                expires_at = excluded.expires_at,
                tags_json = excluded.tags_json,
                metadata_json = excluded.metadata_json,
                schema_version = excluded.schema_version",
            &[
                Value::String(agent.to_string()),
                Value::String(record.category.clone()),
                Value::String(record.key.clone()),
                Value::String(record.content.clone()),
                serde_json::json!(record.created_at),
                serde_json::json!(record.updated_at),
                record.expires_at.map(Value::from).unwrap_or(Value::Null),
                Value::String(tags_json),
                Value::String(metadata_json),
                serde_json::json!(record.schema_version),
            ],
        )?;
    } else {
        sqlite_execute(
            &path,
            "INSERT INTO memory_records (
                agent, category, key, content, created_at, updated_at, expires_at,
                tags_json, metadata_json, schema_version
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            &[
                Value::String(agent.to_string()),
                Value::String(record.category.clone()),
                Value::String(record.key.clone()),
                Value::String(record.content.clone()),
                serde_json::json!(record.created_at),
                serde_json::json!(record.updated_at),
                record.expires_at.map(Value::from).unwrap_or(Value::Null),
                Value::String(tags_json),
                Value::String(metadata_json),
                serde_json::json!(record.schema_version),
            ],
        )?;
    }

    Ok(())
}

fn read_string(row: &[Value], index: usize) -> String {
    row.get(index)
        .and_then(|value| value.as_str())
        .unwrap_or_default()
        .to_string()
}

fn read_u64(row: &[Value], index: usize) -> u64 {
    row.get(index).and_then(|value| value.as_u64()).unwrap_or(0)
}

fn read_optional_u64(row: &[Value], index: usize) -> Option<u64> {
    row.get(index).and_then(|value| value.as_u64())
}

fn parse_record_row(row: &[Value]) -> MemoryRecord {
    let tags_json = read_string(row, 6);
    let metadata_json = read_string(row, 7);
    let schema_version = row.get(8).and_then(|value| value.as_u64()).unwrap_or(MEMORY_SCHEMA_VERSION as u64) as u32;

    MemoryRecord {
        key: read_string(row, 0),
        category: read_string(row, 1),
        content: read_string(row, 2),
        created_at: read_u64(row, 3),
        updated_at: read_u64(row, 4),
        expires_at: read_optional_u64(row, 5),
        tags: serde_json::from_str(&tags_json).unwrap_or_default(),
        metadata: serde_json::from_str(&metadata_json).unwrap_or(Value::Null),
        schema_version,
    }
}

fn load_record_sqlite(agent: &str, category: &str, key: &str) -> Result<Option<MemoryRecord>, String> {
    let path = memory_db_path();
    let result = sqlite_query(
        &path,
        "SELECT key, category, content, created_at, updated_at, expires_at, tags_json, metadata_json, schema_version
         FROM memory_records
         WHERE agent = ? AND category = ? AND key = ?
         LIMIT 1",
        &[
            Value::String(agent.to_string()),
            Value::String(category.to_string()),
            Value::String(key.to_string()),
        ],
    )?;

    Ok(result.rows.first().map(|row| parse_record_row(row)))
}

fn query_agent_records(agent: &str) -> Result<Vec<MemoryRecord>, String> {
    let path = memory_db_path();
    let result = sqlite_query(
        &path,
        "SELECT key, category, content, created_at, updated_at, expires_at, tags_json, metadata_json, schema_version
         FROM memory_records
         WHERE agent = ?
         ORDER BY updated_at DESC",
        &[Value::String(agent.to_string())],
    )?;

    Ok(result.rows.iter().map(|row| parse_record_row(row)).collect())
}

fn query_agent_categories(agent: &str) -> Result<Vec<String>, String> {
    let path = memory_db_path();
    let result = sqlite_query(
        &path,
        "SELECT DISTINCT category FROM memory_records WHERE agent = ? ORDER BY category",
        &[Value::String(agent.to_string())],
    )?;
    Ok(result.rows.iter().map(|row| read_string(row, 0)).filter(|item| !item.is_empty()).collect())
}

fn selected_categories(
    agent: &str,
    category: &str,
    categories: &[String],
) -> Result<Vec<String>, String> {
    if !categories.is_empty() {
        let mut selected: Vec<String> = categories.iter().map(|item| sanitize_category(item)).collect();
        selected.sort();
        selected.dedup();
        return Ok(selected);
    }

    if !category.trim().is_empty() {
        return Ok(vec![sanitize_category(category)]);
    }

    query_agent_categories(agent)
}

fn is_record_expired(record: &MemoryRecord, now_ms: u64) -> bool {
    matches!(record.expires_at, Some(expires_at) if expires_at <= now_ms)
}

fn score_record(record: &MemoryRecord, query: &str) -> Option<i64> {
    let trimmed = query.trim();
    if trimmed.is_empty() {
        return Some(0);
    }

    let haystack = format!(
        "{}\n{}\n{}\n{}\n{}",
        record.key,
        record.category,
        record.content,
        record.tags.join(" "),
        record.metadata
    )
    .to_lowercase();
    let query_lower = trimmed.to_lowercase();
    let mut score = 0_i64;

    if haystack.contains(&query_lower) {
        score += 100;
    }

    for token in query_lower.split_whitespace().filter(|item| !item.is_empty()) {
        if haystack.contains(token) {
            score += 10;
        }
    }

    if score == 0 {
        None
    } else {
        Some(score)
    }
}

fn to_memory_view(record: &MemoryRecord, score: i64) -> MemoryView {
    MemoryView {
        key: record.key.clone(),
        category: record.category.clone(),
        content: record.content.clone(),
        created_at: record.created_at,
        updated_at: record.updated_at,
        expires_at: record.expires_at,
        tags: record.tags.clone(),
        metadata: record.metadata.clone(),
        score,
    }
}

fn cleanup_agent(agent: &str, now_ms: u64, remove_empty: bool) -> Result<usize, String> {
    let path = memory_db_path();
    let sql = if remove_empty {
        "DELETE FROM memory_records
         WHERE agent = ?
           AND ((expires_at IS NOT NULL AND expires_at <= ?)
                OR TRIM(content) = '')"
    } else {
        "DELETE FROM memory_records
         WHERE agent = ?
           AND expires_at IS NOT NULL
           AND expires_at <= ?"
    };

    let result = sqlite_execute(
        &path,
        sql,
        &[Value::String(agent.to_string()), serde_json::json!(now_ms)],
    )?;
    Ok(result.changes)
}

#[plugin_fn]
pub fn init(_input: String) -> FnResult<String> {
    ensure_schema().map_err(extism_pdk::Error::msg)?;
    log_info("memory package initialized");
    Ok(PackageResult::ok_empty().to_json())
}

#[plugin_fn]
pub fn handle_ws_message(input: String) -> FnResult<String> {
    let req: WsRequest = serde_json::from_str(&input).unwrap_or(WsRequest {
        action: String::new(),
        data: Value::Null,
    });

    let result = match req.action.as_str() {
        "store" => {
            let payload: StoreInput = serde_json::from_value(req.data).unwrap_or(StoreInput {
                agent: String::new(),
                key: String::new(),
                content: String::new(),
                category: default_category(),
                expires_at: None,
                tags: Vec::new(),
                metadata: Value::Null,
                timestamp: None,
            });
            do_store(&payload)
        }
        "recall" => {
            let payload: RecallInput = serde_json::from_value(req.data).unwrap_or(RecallInput {
                agent: String::new(),
                query: String::new(),
                limit: default_limit(),
                category: String::new(),
                categories: Vec::new(),
                include_expired: false,
            });
            do_recall(&payload)
        }
        "forget" | "delete" => {
            let payload: ForgetInput = serde_json::from_value(req.data).unwrap_or(ForgetInput {
                agent: String::new(),
                key: String::new(),
            });
            do_forget(&payload)
        }
        "list" => {
            let payload: ListInput = serde_json::from_value(req.data).unwrap_or(ListInput {
                agent: String::new(),
                category: String::new(),
                categories: Vec::new(),
                include_expired: false,
            });
            do_list(&payload)
        }
        "hygiene" => {
            let payload: AgentInput = serde_json::from_value(req.data).unwrap_or(AgentInput {
                agent: String::new(),
            });
            do_hygiene(&payload)
        }
        "cleanup_expired" => {
            let payload: CleanupInput = serde_json::from_value(req.data).unwrap_or(CleanupInput {
                agent: String::new(),
                now_ms: None,
            });
            do_cleanup_expired(&payload)
        }
        _ => PackageResult::err(format!("unknown action: {}", req.action)),
    };

    Ok(result.to_json())
}

#[plugin_fn]
pub fn store(input: String) -> FnResult<String> {
    let payload: StoreInput = serde_json::from_str(&input)
        .map_err(|e| extism_pdk::Error::msg(format!("parse error: {}", e)))?;
    Ok(do_store(&payload).to_json())
}

fn do_store(input: &StoreInput) -> PackageResult {
    if input.agent.trim().is_empty() || input.key.trim().is_empty() {
        return PackageResult::err("missing agent or key");
    }

    if let Err(error) = ensure_schema() {
        return PackageResult::err(error);
    }
    if let Err(error) = migrate_legacy_agent(input.agent.trim()) {
        return PackageResult::err(error);
    }

    let category = sanitize_category(&input.category);
    let now_ms = input.timestamp.unwrap_or_else(current_time_ms);
    let created_at = match load_record_sqlite(input.agent.trim(), &category, input.key.trim()) {
        Ok(Some(existing)) if existing.created_at > 0 => existing.created_at,
        Ok(_) => now_ms,
        Err(error) => return PackageResult::err(error),
    };

    let record = MemoryRecord {
        schema_version: MEMORY_SCHEMA_VERSION,
        key: input.key.trim().to_string(),
        content: input.content.clone(),
        category,
        created_at,
        updated_at: now_ms,
        expires_at: input.expires_at,
        tags: input.tags.clone(),
        metadata: input.metadata.clone(),
    };

    if let Err(error) = save_record_sqlite(input.agent.trim(), &record, true) {
        return PackageResult::err(error);
    }

    PackageResult::ok(serde_json::json!({
        "record": to_memory_view(&record, 0),
    }))
}

#[plugin_fn]
pub fn recall(input: String) -> FnResult<String> {
    let payload: RecallInput = serde_json::from_str(&input)
        .map_err(|e| extism_pdk::Error::msg(format!("parse error: {}", e)))?;
    Ok(do_recall(&payload).to_json())
}

fn do_recall(input: &RecallInput) -> PackageResult {
    if input.agent.trim().is_empty() {
        return PackageResult::err("missing agent");
    }

    if let Err(error) = ensure_schema() {
        return PackageResult::err(error);
    }
    if let Err(error) = migrate_legacy_agent(input.agent.trim()) {
        return PackageResult::err(error);
    }

    let now_ms = current_time_ms();
    let categories = match selected_categories(input.agent.trim(), &input.category, &input.categories) {
        Ok(items) => items,
        Err(error) => return PackageResult::err(error),
    };
    let all_records = match query_agent_records(input.agent.trim()) {
        Ok(records) => records,
        Err(error) => return PackageResult::err(error),
    };
    let mut matches = Vec::<MemoryView>::new();

    for record in all_records {
        if !categories.is_empty() && !categories.iter().any(|item| item == &record.category) {
            continue;
        }
        if !input.include_expired && is_record_expired(&record, now_ms) {
            continue;
        }
        let Some(score) = score_record(&record, &input.query) else {
            continue;
        };
        matches.push(to_memory_view(&record, score));
    }

    matches.sort_by(|left, right| {
        right
            .score
            .cmp(&left.score)
            .then_with(|| right.updated_at.cmp(&left.updated_at))
            .then_with(|| left.key.cmp(&right.key))
    });
    matches.truncate(input.limit.max(1));

    PackageResult::ok(serde_json::json!({ "memories": matches }))
}

#[plugin_fn]
pub fn forget(input: String) -> FnResult<String> {
    let payload: ForgetInput = serde_json::from_str(&input)
        .map_err(|e| extism_pdk::Error::msg(format!("parse error: {}", e)))?;
    Ok(do_forget(&payload).to_json())
}

fn do_forget(input: &ForgetInput) -> PackageResult {
    if input.agent.trim().is_empty() || input.key.trim().is_empty() {
        return PackageResult::err("missing agent or key");
    }

    if let Err(error) = ensure_schema() {
        return PackageResult::err(error);
    }
    if let Err(error) = migrate_legacy_agent(input.agent.trim()) {
        return PackageResult::err(error);
    }

    let path = memory_db_path();
    let result = sqlite_execute(
        &path,
        "DELETE FROM memory_records WHERE agent = ? AND key = ?",
        &[
            Value::String(input.agent.trim().to_string()),
            Value::String(input.key.trim().to_string()),
        ],
    );

    match result {
        Ok(result) => PackageResult::ok(serde_json::json!({
            "removed": result.changes > 0,
            "removed_count": result.changes,
        })),
        Err(error) => PackageResult::err(error),
    }
}

#[plugin_fn]
pub fn list_by_category(input: String) -> FnResult<String> {
    let payload: ListInput = serde_json::from_str(&input)
        .map_err(|e| extism_pdk::Error::msg(format!("parse error: {}", e)))?;
    Ok(do_list(&payload).to_json())
}

fn do_list(input: &ListInput) -> PackageResult {
    if input.agent.trim().is_empty() {
        return PackageResult::err("missing agent");
    }

    if let Err(error) = ensure_schema() {
        return PackageResult::err(error);
    }
    if let Err(error) = migrate_legacy_agent(input.agent.trim()) {
        return PackageResult::err(error);
    }

    let now_ms = current_time_ms();
    let categories = match selected_categories(input.agent.trim(), &input.category, &input.categories) {
        Ok(items) => items,
        Err(error) => return PackageResult::err(error),
    };
    let all_records = match query_agent_records(input.agent.trim()) {
        Ok(records) => records,
        Err(error) => return PackageResult::err(error),
    };
    let mut memories = Vec::<MemoryView>::new();

    for record in all_records {
        if !categories.is_empty() && !categories.iter().any(|item| item == &record.category) {
            continue;
        }
        if !input.include_expired && is_record_expired(&record, now_ms) {
            continue;
        }
        memories.push(to_memory_view(&record, 0));
    }

    memories.sort_by(|left, right| {
        right
            .updated_at
            .cmp(&left.updated_at)
            .then_with(|| left.category.cmp(&right.category))
            .then_with(|| left.key.cmp(&right.key))
    });

    PackageResult::ok(serde_json::json!({ "memories": memories }))
}

#[plugin_fn]
pub fn hygiene(input: String) -> FnResult<String> {
    let payload: AgentInput = serde_json::from_str(&input)
        .map_err(|e| extism_pdk::Error::msg(format!("parse error: {}", e)))?;
    Ok(do_hygiene(&payload).to_json())
}

fn do_hygiene(input: &AgentInput) -> PackageResult {
    if input.agent.trim().is_empty() {
        return PackageResult::err("missing agent");
    }

    if let Err(error) = ensure_schema() {
        return PackageResult::err(error);
    }
    if let Err(error) = migrate_legacy_agent(input.agent.trim()) {
        return PackageResult::err(error);
    }

    match cleanup_agent(input.agent.trim(), current_time_ms(), true) {
        Ok(cleaned) => PackageResult::ok(serde_json::json!({ "cleaned": cleaned })),
        Err(error) => PackageResult::err(error),
    }
}

#[plugin_fn]
pub fn cleanup_expired(input: String) -> FnResult<String> {
    let payload: CleanupInput = serde_json::from_str(&input)
        .map_err(|e| extism_pdk::Error::msg(format!("parse error: {}", e)))?;
    Ok(do_cleanup_expired(&payload).to_json())
}

fn do_cleanup_expired(input: &CleanupInput) -> PackageResult {
    if let Err(error) = ensure_schema() {
        return PackageResult::err(error);
    }

    let now_ms = input.now_ms.unwrap_or_else(current_time_ms);
    let agents: Vec<String> = if input.agent.trim() == "*" {
        let sqlite_agents = sqlite_query(
            &memory_db_path(),
            "SELECT DISTINCT agent FROM memory_records ORDER BY agent",
            &[],
        )
        .map(|result| {
            result
                .rows
                .iter()
                .map(|row| read_string(row, 0))
                .filter(|item| !item.is_empty())
                .collect::<Vec<String>>()
        })
        .unwrap_or_default();
        let mut all_agents = sqlite_agents;
        for agent in all_legacy_agents() {
            if !all_agents.iter().any(|item| item == &agent) {
                all_agents.push(agent);
            }
        }
        all_agents
    } else if input.agent.trim().is_empty() {
        return PackageResult::err("missing agent");
    } else {
        vec![input.agent.trim().to_string()]
    };

    if let Err(error) = migrate_legacy_agents(&agents) {
        return PackageResult::err(error);
    }

    let mut total_cleaned = 0_usize;
    let mut cleaned_agents = Vec::<String>::new();
    for agent in agents {
        match cleanup_agent(&agent, now_ms, false) {
            Ok(cleaned) if cleaned > 0 => {
                total_cleaned += cleaned;
                cleaned_agents.push(agent);
            }
            Ok(_) => {}
            Err(error) => return PackageResult::err(error),
        }
    }

    cleaned_agents.sort();
    cleaned_agents.dedup();

    PackageResult::ok(serde_json::json!({
        "cleaned": total_cleaned,
        "agents": cleaned_agents,
        "now_ms": now_ms,
    }))
}

