use weft_package_sdk::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;

const CAPABILITY: &str = "session.events";
const SCHEMA_VERSION: u32 = 1;
const DEFAULT_DB_PATH: &str = "./data/session-events/session-events.sqlite";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SessionEvent {
    schema_version: u32,
    session_id: String,
    seq: u64,
    event_id: String,
    #[serde(rename = "type")]
    event_type: String,
    created_at: u64,
    #[serde(default)]
    payload: Value,
}

#[derive(Debug, Clone, Deserialize, Default)]
struct AppendEventInput {
    session_id: String,
    #[serde(default)]
    #[serde(rename = "type")]
    event_type: String,
    #[serde(default)]
    payload: Value,
    #[serde(default)]
    created_at: Option<u64>,
}

#[derive(Debug, Clone, Deserialize, Default)]
struct AppendEventsInput {
    session_id: String,
    #[serde(default)]
    events: Vec<AppendEventItem>,
}

#[derive(Debug, Clone, Deserialize, Default)]
struct AppendEventItem {
    #[serde(default)]
    #[serde(rename = "type")]
    event_type: String,
    #[serde(default)]
    payload: Value,
    #[serde(default)]
    created_at: Option<u64>,
}

#[derive(Debug, Clone, Deserialize, Default)]
struct ListEventsInput {
    session_id: String,
    #[serde(default)]
    after_seq: Option<u64>,
    #[serde(default)]
    limit: Option<u64>,
}

#[derive(Debug, Clone, Deserialize, Default)]
struct DeleteSessionInput {
    session_id: String,
}

fn db_path() -> String {
    env_get("WEFT_SESSION_EVENTS_DB")
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_DB_PATH.to_string())
}

fn ensure_schema() -> Result<(), String> {
    sqlite_batch(
        &db_path(),
        &[
            (
                "CREATE TABLE IF NOT EXISTS session_events (session_id TEXT NOT NULL, seq INTEGER NOT NULL, event_id TEXT NOT NULL, event_type TEXT NOT NULL, payload_json TEXT NOT NULL, created_at INTEGER NOT NULL, PRIMARY KEY(session_id, seq))".to_string(),
                vec![],
            ),
            (
                "CREATE INDEX IF NOT EXISTS idx_session_events_created_at ON session_events(created_at)".to_string(),
                vec![],
            ),
        ],
    )?;
    Ok(())
}

fn next_seq(session_id: &str) -> Result<u64, String> {
    ensure_schema()?;
    let result = sqlite_query(
        &db_path(),
        "SELECT COALESCE(MAX(seq), 0) + 1 AS next_seq FROM session_events WHERE session_id = ?1",
        &[Value::String(session_id.to_string())],
    )?;
    Ok(result
        .rows
        .first()
        .and_then(|row| row.first())
        .and_then(Value::as_i64)
        .unwrap_or(1) as u64)
}

fn append_event(session_id: &str, event_type: &str, payload: Value, created_at: Option<u64>) -> Result<SessionEvent, String> {
    let session_id = session_id.trim();
    if session_id.is_empty() {
        return Err("append_event requires session_id".to_string());
    }
    let event_type = event_type.trim();
    if event_type.is_empty() {
        return Err("append_event requires type".to_string());
    }

    let seq = next_seq(session_id)?;
    let created_at = created_at.unwrap_or_else(now_ms);
    let event_id = format!("{session_id}-{seq}");
    let payload_json = serde_json::to_string(&payload)
        .map_err(|error| format!("failed to serialize event payload: {error}"))?;

    sqlite_execute(
        &db_path(),
        "INSERT INTO session_events (session_id, seq, event_id, event_type, payload_json, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        &[
            Value::String(session_id.to_string()),
            Value::from(seq),
            Value::String(event_id.clone()),
            Value::String(event_type.to_string()),
            Value::String(payload_json),
            Value::from(created_at),
        ],
    )?;

    Ok(SessionEvent {
        schema_version: SCHEMA_VERSION,
        session_id: session_id.to_string(),
        seq,
        event_id,
        event_type: event_type.to_string(),
        created_at,
        payload,
    })
}

fn row_to_event(row: &[Value]) -> Option<SessionEvent> {
    let session_id = row.get(0)?.as_str()?.to_string();
    let seq = row.get(1)?.as_i64()? as u64;
    let event_id = row.get(2)?.as_str()?.to_string();
    let event_type = row.get(3)?.as_str()?.to_string();
    let payload_json = row.get(4)?.as_str()?.to_string();
    let created_at = row.get(5)?.as_i64()? as u64;
    let payload = serde_json::from_str(&payload_json).unwrap_or(Value::Null);
    Some(SessionEvent {
        schema_version: SCHEMA_VERSION,
        session_id,
        seq,
        event_id,
        event_type,
        created_at,
        payload,
    })
}

fn do_describe() -> PackageResult {
    PackageResult::ok(serde_json::json!({
        "package": "session-events",
        "capability": CAPABILITY,
        "storage": "sqlite",
        "actions": ["describe", "health", "append_event", "append_events", "list_events", "delete_session_events"],
    }))
}

fn do_health() -> PackageResult {
    match ensure_schema() {
        Ok(()) => PackageResult::ok(serde_json::json!({
            "healthy": true,
            "capability": CAPABILITY,
            "db_path": db_path(),
        })),
        Err(error) => PackageResult::err(error),
    }
}

fn do_append_event(data: Value) -> PackageResult {
    let input: AppendEventInput = serde_json::from_value(data).unwrap_or_default();
    match append_event(&input.session_id, &input.event_type, input.payload, input.created_at) {
        Ok(event) => PackageResult::ok(serde_json::json!({ "event": event })),
        Err(error) => PackageResult::err(error),
    }
}

fn do_append_events(data: Value) -> PackageResult {
    let input: AppendEventsInput = serde_json::from_value(data).unwrap_or_default();
    let mut events = Vec::new();
    for item in input.events {
        match append_event(&input.session_id, &item.event_type, item.payload, item.created_at) {
            Ok(event) => events.push(event),
            Err(error) => return PackageResult::err(error),
        }
    }
    PackageResult::ok(serde_json::json!({ "session_id": input.session_id, "events": events }))
}

fn do_list_events(data: Value) -> PackageResult {
    let input: ListEventsInput = serde_json::from_value(data).unwrap_or_default();
    if input.session_id.trim().is_empty() {
        return PackageResult::err("list_events requires session_id");
    }
    if let Err(error) = ensure_schema() {
        return PackageResult::err(error);
    }

    let after_seq = input.after_seq.unwrap_or(0);
    let limit = input.limit.unwrap_or(200).clamp(1, 1000);
    match sqlite_query(
        &db_path(),
        "SELECT session_id, seq, event_id, event_type, payload_json, created_at FROM session_events WHERE session_id = ?1 AND seq > ?2 ORDER BY seq ASC LIMIT ?3",
        &[
            Value::String(input.session_id.clone()),
            Value::from(after_seq),
            Value::from(limit),
        ],
    ) {
        Ok(result) => {
            let events = result.rows.iter().filter_map(|row| row_to_event(row)).collect::<Vec<_>>();
            // Query the true global MAX(seq) so callers can use it as a baseline
            // regardless of the limit applied to the event list.
            let global_max_seq = sqlite_query(
                &db_path(),
                "SELECT COALESCE(MAX(seq), 0) as max_seq FROM session_events WHERE session_id = ?1",
                &[Value::String(input.session_id.clone())],
            )
            .ok()
            .and_then(|r| r.rows.first().cloned())
            .and_then(|row| row.first().cloned())
            .and_then(|v| match v {
                Value::Number(n) => n.as_u64(),
                _ => None,
            })
            .unwrap_or(after_seq);
            PackageResult::ok(serde_json::json!({
                "session_id": input.session_id,
                "after_seq": after_seq,
                "latest_seq": global_max_seq,
                "events": events,
            }))
        }
        Err(error) => PackageResult::err(error),
    }
}

fn do_delete_session_events(data: Value) -> PackageResult {
    let input: DeleteSessionInput = serde_json::from_value(data).unwrap_or_default();
    if input.session_id.trim().is_empty() {
        return PackageResult::err("delete_session_events requires session_id");
    }
    match sqlite_execute(
        &db_path(),
        "DELETE FROM session_events WHERE session_id = ?1",
        &[Value::String(input.session_id.clone())],
    ) {
        Ok(result) => PackageResult::ok(serde_json::json!({
            "session_id": input.session_id,
            "rows_affected": result.rows_affected,
        })),
        Err(error) => PackageResult::err(error),
    }
}

fn dispatch(action: &str, data: Value) -> PackageResult {
    match action {
        "describe" => do_describe(),
        "health" => do_health(),
        "append_event" => do_append_event(data),
        "append_events" => do_append_events(data),
        "list_events" | "get_session_events" => do_list_events(data),
        "delete_session_events" => do_delete_session_events(data),
        other => PackageResult::err(format!("unknown action: {other}")),
    }
}

#[plugin_fn]
pub fn init(_input: String) -> FnResult<String> {
    log_info("session-events initialized");
    Ok(do_health().to_json())
}

#[plugin_fn]
pub fn handle_ws_message(input: String) -> FnResult<String> {
    let req: WsRequest = serde_json::from_str(&input).unwrap_or(WsRequest {
        action: String::new(),
        data: Value::Null,
    });
    Ok(dispatch(&req.action, req.data).to_json())
}

#[plugin_fn]
pub fn call(input: String) -> FnResult<String> {
    let req: WsRequest = serde_json::from_str(&input).unwrap_or(WsRequest {
        action: String::new(),
        data: Value::Null,
    });
    Ok(dispatch(&req.action, req.data).to_json())
}

#[plugin_fn]
pub fn describe(_input: String) -> FnResult<String> {
    Ok(do_describe().to_json())
}

#[plugin_fn]
pub fn health(_input: String) -> FnResult<String> {
    Ok(do_health().to_json())
}
