use weft_package_sdk::*;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};

const PACKAGE_NAME: &str = "team-task-board";
const TASKBOARD_CAPABILITY: &str = "team.taskboard";
const HANDOFF_CAPABILITY: &str = "team.handoff";
const SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct TeamTaskBoard {
    schema_version: u32,
    board_id: String,
    #[serde(default)]
    session_id: String,
    #[serde(default)]
    title: String,
    #[serde(default)]
    status: String,
    created_at: u64,
    updated_at: u64,
    #[serde(default)]
    review_items: Vec<String>,
    #[serde(default)]
    activity_log: Vec<String>,
    #[serde(default)]
    metadata: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct TeamReviewItem {
    schema_version: u32,
    board_id: String,
    review_item_id: String,
    #[serde(default)]
    task_id: String,
    #[serde(default)]
    reviewer_member_id: String,
    #[serde(default)]
    status: String,
    #[serde(default)]
    summary: String,
    #[serde(default)]
    artifact_refs: Vec<String>,
    created_at: u64,
    updated_at: u64,
    #[serde(default)]
    metadata: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct TeamActivityEvent {
    schema_version: u32,
    board_id: String,
    event_id: String,
    event_type: String,
    #[serde(default)]
    task_id: String,
    #[serde(default)]
    handoff_id: String,
    #[serde(default)]
    actor_member_id: String,
    #[serde(default)]
    summary: String,
    created_at: u64,
    #[serde(default)]
    metadata: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct TeamTask {
    schema_version: u32,
    board_id: String,
    task_id: String,
    title: String,
    #[serde(default)]
    description: String,
    #[serde(default = "default_task_kind")]
    kind: String,
    #[serde(default = "default_task_status")]
    status: String,
    #[serde(default)]
    owner_member_id: String,
    #[serde(default)]
    depends_on: Vec<String>,
    #[serde(default)]
    artifact_refs: Vec<String>,
    #[serde(default)]
    review_state: String,
    created_at: u64,
    updated_at: u64,
    #[serde(default)]
    metadata: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct TeamHandoff {
    schema_version: u32,
    handoff_id: String,
    board_id: String,
    #[serde(default)]
    task_id: String,
    #[serde(default)]
    from_member_id: String,
    #[serde(default)]
    to_member_id: String,
    #[serde(default)]
    reason: String,
    #[serde(default)]
    expected_outcome: String,
    #[serde(default)]
    context_snapshot_ref: String,
    #[serde(default = "default_handoff_status")]
    status: String,
    created_at: u64,
    updated_at: u64,
    #[serde(default)]
    metadata: Value,
}

#[derive(Debug, Clone, Deserialize)]
struct CreateBoardInput {
    board_id: String,
    #[serde(default)]
    session_id: String,
    #[serde(default)]
    title: String,
    #[serde(default = "default_board_status")]
    status: String,
    #[serde(default)]
    timestamp: Option<u64>,
    #[serde(default)]
    metadata: Value,
}

#[derive(Debug, Clone, Deserialize)]
struct GetBoardInput {
    board_id: String,
}

#[derive(Debug, Clone, Deserialize)]
struct SaveTaskInput {
    board_id: String,
    task_id: String,
    title: String,
    #[serde(default)]
    description: String,
    #[serde(default = "default_task_kind")]
    kind: String,
    #[serde(default = "default_task_status")]
    status: String,
    #[serde(default)]
    owner_member_id: String,
    #[serde(default)]
    depends_on: Vec<String>,
    #[serde(default)]
    artifact_refs: Vec<String>,
    #[serde(default)]
    review_state: String,
    #[serde(default)]
    timestamp: Option<u64>,
    #[serde(default)]
    metadata: Value,
}

#[derive(Debug, Clone, Deserialize)]
struct ListTasksInput {
    board_id: String,
}

#[derive(Debug, Clone, Deserialize)]
struct CreateHandoffInput {
    board_id: String,
    handoff_id: String,
    #[serde(default)]
    task_id: String,
    #[serde(default)]
    from_member_id: String,
    #[serde(default)]
    to_member_id: String,
    #[serde(default)]
    reason: String,
    #[serde(default)]
    expected_outcome: String,
    #[serde(default)]
    context_snapshot_ref: String,
    #[serde(default)]
    timestamp: Option<u64>,
    #[serde(default)]
    metadata: Value,
}

#[derive(Debug, Clone, Deserialize)]
struct UpdateHandoffStatusInput {
    board_id: String,
    handoff_id: String,
    status: String,
}

#[derive(Debug, Clone, Deserialize)]
struct SaveReviewItemInput {
    board_id: String,
    review_item_id: String,
    #[serde(default)]
    task_id: String,
    #[serde(default)]
    reviewer_member_id: String,
    #[serde(default)]
    status: String,
    #[serde(default)]
    summary: String,
    #[serde(default)]
    artifact_refs: Vec<String>,
    #[serde(default)]
    timestamp: Option<u64>,
    #[serde(default)]
    metadata: Value,
}

#[derive(Debug, Clone, Deserialize)]
struct ListReviewItemsInput {
    board_id: String,
}

#[derive(Debug, Clone, Deserialize)]
struct AppendActivityInput {
    board_id: String,
    event_id: String,
    event_type: String,
    #[serde(default)]
    task_id: String,
    #[serde(default)]
    handoff_id: String,
    #[serde(default)]
    actor_member_id: String,
    #[serde(default)]
    summary: String,
    #[serde(default)]
    timestamp: Option<u64>,
    #[serde(default)]
    metadata: Value,
}

#[derive(Debug, Clone, Deserialize)]
struct ListActivityInput {
    board_id: String,
}

#[derive(Debug, Clone, Deserialize)]
struct ListHandoffsInput {
    board_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct BoardIndexEntry {
    board_id: String,
    updated_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct TaskIndexEntry {
    task_id: String,
    updated_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct HandoffIndexEntry {
    handoff_id: String,
    updated_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct ReviewItemIndexEntry {
    review_item_id: String,
    updated_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct ActivityEventIndexEntry {
    event_id: String,
    created_at: u64,
}

fn default_board_status() -> String {
    "active".into()
}

fn default_task_kind() -> String {
    "task".into()
}

fn default_task_status() -> String {
    "todo".into()
}

fn default_handoff_status() -> String {
    "pending".into()
}

fn now_ts() -> u64 {
    now_ms()
}

fn boards_index_key() -> String {
    format!("{}:boards", PACKAGE_NAME)
}

fn board_key(board_id: &str) -> String {
    format!("{}:board:{}", PACKAGE_NAME, board_id.trim())
}

fn tasks_index_key(board_id: &str) -> String {
    format!("{}:board:{}:tasks", PACKAGE_NAME, board_id.trim())
}

fn task_key(board_id: &str, task_id: &str) -> String {
    format!(
        "{}:board:{}:task:{}",
        PACKAGE_NAME,
        board_id.trim(),
        task_id.trim()
    )
}

fn handoffs_index_key(board_id: &str) -> String {
    format!("{}:board:{}:handoffs", PACKAGE_NAME, board_id.trim())
}

fn handoff_key(board_id: &str, handoff_id: &str) -> String {
    format!(
        "{}:board:{}:handoff:{}",
        PACKAGE_NAME,
        board_id.trim(),
        handoff_id.trim()
    )
}

fn review_items_index_key(board_id: &str) -> String {
    format!("{}:board:{}:review-items", PACKAGE_NAME, board_id.trim())
}

fn review_item_key(board_id: &str, review_item_id: &str) -> String {
    format!(
        "{}:board:{}:review-item:{}",
        PACKAGE_NAME,
        board_id.trim(),
        review_item_id.trim()
    )
}

fn activity_log_index_key(board_id: &str) -> String {
    format!("{}:board:{}:activity", PACKAGE_NAME, board_id.trim())
}

fn activity_event_key(board_id: &str, event_id: &str) -> String {
    format!(
        "{}:board:{}:activity:{}",
        PACKAGE_NAME,
        board_id.trim(),
        event_id.trim()
    )
}

fn parse_json_or_default<T>(raw: Option<String>) -> T
where
    T: for<'de> Deserialize<'de> + Default,
{
    raw.and_then(|json| serde_json::from_str(&json).ok())
        .unwrap_or_default()
}

fn write_json<T: Serialize>(key: &str, value: &T) -> Result<(), String> {
    let json = serde_json::to_string(value).map_err(|error| error.to_string())?;
    kv_set(key, &json);
    Ok(())
}

fn push_or_replace<T, F>(items: &mut Vec<T>, new_item: T, same: F)
where
    F: Fn(&T, &T) -> bool,
{
    if let Some(index) = items.iter().position(|existing| same(existing, &new_item)) {
        items[index] = new_item;
    } else {
        items.push(new_item);
    }
}

fn ensure_non_empty(value: &str, field: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        Err(format!("missing {}", field))
    } else {
        Ok(())
    }
}

fn normalize_metadata(metadata: Value) -> Value {
    match metadata {
        Value::Null => Value::Object(Map::new()),
        other => other,
    }
}

fn save_board_record(input: CreateBoardInput) -> PackageResult {
    if let Err(error) = ensure_non_empty(&input.board_id, "board_id") {
        return PackageResult::err(error);
    }

    let timestamp = input.timestamp.unwrap_or_else(now_ts);
    let existing_board = load_board_record(&input.board_id);
    let board = TeamTaskBoard {
        schema_version: SCHEMA_VERSION,
        board_id: input.board_id.trim().to_string(),
        session_id: input.session_id.trim().to_string(),
        title: input.title,
        status: if input.status.trim().is_empty() {
            default_board_status()
        } else {
            input.status.trim().to_string()
        },
        created_at: existing_board
            .as_ref()
            .map(|board| board.created_at)
            .unwrap_or(timestamp),
        updated_at: timestamp,
        review_items: existing_board
            .as_ref()
            .map(|board| board.review_items.clone())
            .unwrap_or_default(),
        activity_log: existing_board
            .as_ref()
            .map(|board| board.activity_log.clone())
            .unwrap_or_default(),
        metadata: normalize_metadata(input.metadata),
    };

    if let Err(error) = persist_board(&board) {
        return PackageResult::err(error);
    }

    PackageResult::ok(serde_json::json!({ "board": board }))
}

fn load_board_record(board_id: &str) -> Option<TeamTaskBoard> {
    kv_get(&board_key(board_id)).and_then(|json| serde_json::from_str(&json).ok())
}

fn persist_board(board: &TeamTaskBoard) -> Result<(), String> {
    write_json(&board_key(&board.board_id), board)?;
    let mut boards: Vec<BoardIndexEntry> = parse_json_or_default(kv_get(&boards_index_key()));
    push_or_replace(
        &mut boards,
        BoardIndexEntry {
            board_id: board.board_id.clone(),
            updated_at: board.updated_at,
        },
        |left, right| left.board_id == right.board_id,
    );
    boards.sort_by(|left, right| {
        right
            .updated_at
            .cmp(&left.updated_at)
            .then(left.board_id.cmp(&right.board_id))
    });
    write_json(&boards_index_key(), &boards)
}

fn sync_board_summary(board_id: &str) -> Result<(), String> {
    if let Some(mut board) = load_board_record(board_id) {
        let review_index: Vec<ReviewItemIndexEntry> =
            parse_json_or_default(kv_get(&review_items_index_key(board_id)));
        let activity_index: Vec<ActivityEventIndexEntry> =
            parse_json_or_default(kv_get(&activity_log_index_key(board_id)));
        board.review_items = review_index
            .into_iter()
            .map(|entry| entry.review_item_id)
            .collect();
        board.activity_log = activity_index
            .into_iter()
            .map(|entry| entry.event_id)
            .collect();
        persist_board(&board)?;
    }
    Ok(())
}

fn get_board_result(input: GetBoardInput) -> PackageResult {
    if let Err(error) = ensure_non_empty(&input.board_id, "board_id") {
        return PackageResult::err(error);
    }

    match load_board_record(&input.board_id) {
        Some(board) => PackageResult::ok(serde_json::json!({ "board": board })),
        None => PackageResult::err(format!("board '{}' not found", input.board_id.trim())),
    }
}

fn save_task_result(input: SaveTaskInput) -> PackageResult {
    if let Err(error) = ensure_non_empty(&input.board_id, "board_id") {
        return PackageResult::err(error);
    }
    if let Err(error) = ensure_non_empty(&input.task_id, "task_id") {
        return PackageResult::err(error);
    }
    if let Err(error) = ensure_non_empty(&input.title, "title") {
        return PackageResult::err(error);
    }
    if load_board_record(&input.board_id).is_none() {
        return PackageResult::err(format!("board '{}' not found", input.board_id.trim()));
    }

    let timestamp = input.timestamp.unwrap_or_else(now_ts);
    let existing_task = load_task_record(&input.board_id, &input.task_id);
    let task = TeamTask {
        schema_version: SCHEMA_VERSION,
        board_id: input.board_id.trim().to_string(),
        task_id: input.task_id.trim().to_string(),
        title: input.title,
        description: input.description,
        kind: if input.kind.trim().is_empty() {
            default_task_kind()
        } else {
            input.kind.trim().to_string()
        },
        status: if input.status.trim().is_empty() {
            default_task_status()
        } else {
            input.status.trim().to_string()
        },
        owner_member_id: input.owner_member_id.trim().to_string(),
        depends_on: input.depends_on,
        artifact_refs: input.artifact_refs,
        review_state: input.review_state.trim().to_string(),
        created_at: existing_task
            .as_ref()
            .map(|task| task.created_at)
            .unwrap_or(timestamp),
        updated_at: timestamp,
        metadata: normalize_metadata(input.metadata),
    };

    if let Err(error) = persist_task(&task) {
        return PackageResult::err(error);
    }

    let _ = append_activity_event(TeamActivityEvent {
        schema_version: SCHEMA_VERSION,
        board_id: task.board_id.clone(),
        event_id: format!("task:{}:{}", task.task_id, task.updated_at),
        event_type: match task.status.as_str() {
            "todo" => "task_created".to_string(),
            "in_progress" => "task_started".to_string(),
            "blocked" => "task_blocked".to_string(),
            "done" => "task_completed".to_string(),
            _ => "task_updated".to_string(),
        },
        task_id: task.task_id.clone(),
        handoff_id: String::new(),
        actor_member_id: task.owner_member_id.clone(),
        summary: format!(
            "Task '{}' saved with status '{}'",
            task.task_id, task.status
        ),
        created_at: task.updated_at,
        metadata: task.metadata.clone(),
    });

    PackageResult::ok(serde_json::json!({ "task": task }))
}

fn load_task_record(board_id: &str, task_id: &str) -> Option<TeamTask> {
    kv_get(&task_key(board_id, task_id)).and_then(|json| serde_json::from_str(&json).ok())
}

fn persist_task(task: &TeamTask) -> Result<(), String> {
    write_json(&task_key(&task.board_id, &task.task_id), task)?;
    let mut tasks: Vec<TaskIndexEntry> =
        parse_json_or_default(kv_get(&tasks_index_key(&task.board_id)));
    push_or_replace(
        &mut tasks,
        TaskIndexEntry {
            task_id: task.task_id.clone(),
            updated_at: task.updated_at,
        },
        |left, right| left.task_id == right.task_id,
    );
    tasks.sort_by(|left, right| {
        right
            .updated_at
            .cmp(&left.updated_at)
            .then(left.task_id.cmp(&right.task_id))
    });
    write_json(&tasks_index_key(&task.board_id), &tasks)
}

fn list_tasks_result(input: ListTasksInput) -> PackageResult {
    if let Err(error) = ensure_non_empty(&input.board_id, "board_id") {
        return PackageResult::err(error);
    }
    if load_board_record(&input.board_id).is_none() {
        return PackageResult::err(format!("board '{}' not found", input.board_id.trim()));
    }

    let index: Vec<TaskIndexEntry> =
        parse_json_or_default(kv_get(&tasks_index_key(&input.board_id)));
    let tasks = index
        .into_iter()
        .filter_map(|item| load_task_record(&input.board_id, &item.task_id))
        .collect::<Vec<_>>();

    PackageResult::ok(serde_json::json!({
        "board_id": input.board_id.trim(),
        "tasks": tasks,
    }))
}

fn create_handoff_result(input: CreateHandoffInput) -> PackageResult {
    if let Err(error) = ensure_non_empty(&input.board_id, "board_id") {
        return PackageResult::err(error);
    }
    if let Err(error) = ensure_non_empty(&input.handoff_id, "handoff_id") {
        return PackageResult::err(error);
    }
    if load_board_record(&input.board_id).is_none() {
        return PackageResult::err(format!("board '{}' not found", input.board_id.trim()));
    }

    let timestamp = input.timestamp.unwrap_or_else(now_ts);
    let handoff = TeamHandoff {
        schema_version: SCHEMA_VERSION,
        handoff_id: input.handoff_id.trim().to_string(),
        board_id: input.board_id.trim().to_string(),
        task_id: input.task_id.trim().to_string(),
        from_member_id: input.from_member_id.trim().to_string(),
        to_member_id: input.to_member_id.trim().to_string(),
        reason: input.reason,
        expected_outcome: input.expected_outcome,
        context_snapshot_ref: input.context_snapshot_ref.trim().to_string(),
        status: default_handoff_status(),
        created_at: timestamp,
        updated_at: timestamp,
        metadata: normalize_metadata(input.metadata),
    };

    if let Err(error) = persist_handoff(&handoff) {
        return PackageResult::err(error);
    }

    let _ = append_activity_event(TeamActivityEvent {
        schema_version: SCHEMA_VERSION,
        board_id: handoff.board_id.clone(),
        event_id: format!("handoff:{}:{}", handoff.handoff_id, handoff.updated_at),
        event_type: "handoff_requested".to_string(),
        task_id: handoff.task_id.clone(),
        handoff_id: handoff.handoff_id.clone(),
        actor_member_id: handoff.from_member_id.clone(),
        summary: format!(
            "Handoff '{}' requested for task '{}'",
            handoff.handoff_id, handoff.task_id
        ),
        created_at: handoff.updated_at,
        metadata: handoff.metadata.clone(),
    });

    PackageResult::ok(serde_json::json!({ "handoff": handoff }))
}

fn update_handoff_status_result(input: UpdateHandoffStatusInput) -> PackageResult {
    if let Err(error) = ensure_non_empty(&input.board_id, "board_id") {
        return PackageResult::err(error);
    }
    if let Err(error) = ensure_non_empty(&input.handoff_id, "handoff_id") {
        return PackageResult::err(error);
    }
    let status = input.status.trim();
    if !matches!(status, "pending" | "accepted" | "completed") {
        return PackageResult::err("invalid handoff status".to_string());
    }

    let mut handoff = match load_handoff_record(&input.board_id, &input.handoff_id) {
        Some(record) => record,
        None => {
            return PackageResult::err(format!(
                "handoff '{}' not found on board '{}'",
                input.handoff_id.trim(),
                input.board_id.trim()
            ))
        }
    };

    handoff.status = status.to_string();
    handoff.updated_at = now_ts();
    if let Err(error) = persist_handoff(&handoff) {
        return PackageResult::err(error);
    }

    let _ = append_activity_event(TeamActivityEvent {
        schema_version: SCHEMA_VERSION,
        board_id: handoff.board_id.clone(),
        event_id: format!("handoff-status:{}:{}", handoff.handoff_id, handoff.updated_at),
        event_type: "handoff_status_updated".to_string(),
        task_id: handoff.task_id.clone(),
        handoff_id: handoff.handoff_id.clone(),
        actor_member_id: handoff.to_member_id.clone(),
        summary: format!("Handoff '{}' status -> {}", handoff.handoff_id, handoff.status),
        created_at: handoff.updated_at,
        metadata: json!({ "status": handoff.status }),
    });

    PackageResult::ok(serde_json::json!({ "handoff": handoff }))
}

fn load_handoff_record(board_id: &str, handoff_id: &str) -> Option<TeamHandoff> {
    kv_get(&handoff_key(board_id, handoff_id)).and_then(|json| serde_json::from_str(&json).ok())
}

fn persist_handoff(handoff: &TeamHandoff) -> Result<(), String> {
    write_json(
        &handoff_key(&handoff.board_id, &handoff.handoff_id),
        handoff,
    )?;
    let mut handoffs: Vec<HandoffIndexEntry> =
        parse_json_or_default(kv_get(&handoffs_index_key(&handoff.board_id)));
    push_or_replace(
        &mut handoffs,
        HandoffIndexEntry {
            handoff_id: handoff.handoff_id.clone(),
            updated_at: handoff.updated_at,
        },
        |left, right| left.handoff_id == right.handoff_id,
    );
    handoffs.sort_by(|left, right| {
        right
            .updated_at
            .cmp(&left.updated_at)
            .then(left.handoff_id.cmp(&right.handoff_id))
    });
    write_json(&handoffs_index_key(&handoff.board_id), &handoffs)
}

fn list_handoffs_result(input: ListHandoffsInput) -> PackageResult {
    if let Err(error) = ensure_non_empty(&input.board_id, "board_id") {
        return PackageResult::err(error);
    }
    if load_board_record(&input.board_id).is_none() {
        return PackageResult::err(format!("board '{}' not found", input.board_id.trim()));
    }

    let index: Vec<HandoffIndexEntry> =
        parse_json_or_default(kv_get(&handoffs_index_key(&input.board_id)));
    let handoffs = index
        .into_iter()
        .filter_map(|item| load_handoff_record(&input.board_id, &item.handoff_id))
        .collect::<Vec<_>>();

    PackageResult::ok(serde_json::json!({
        "board_id": input.board_id.trim(),
        "handoffs": handoffs,
    }))
}

fn load_review_item_record(board_id: &str, review_item_id: &str) -> Option<TeamReviewItem> {
    kv_get(&review_item_key(board_id, review_item_id))
        .and_then(|json| serde_json::from_str(&json).ok())
}

fn persist_review_item(item: &TeamReviewItem) -> Result<(), String> {
    write_json(&review_item_key(&item.board_id, &item.review_item_id), item)?;
    let mut items: Vec<ReviewItemIndexEntry> =
        parse_json_or_default(kv_get(&review_items_index_key(&item.board_id)));
    push_or_replace(
        &mut items,
        ReviewItemIndexEntry {
            review_item_id: item.review_item_id.clone(),
            updated_at: item.updated_at,
        },
        |left, right| left.review_item_id == right.review_item_id,
    );
    items.sort_by(|left, right| {
        right
            .updated_at
            .cmp(&left.updated_at)
            .then(left.review_item_id.cmp(&right.review_item_id))
    });
    write_json(&review_items_index_key(&item.board_id), &items)?;
    sync_board_summary(&item.board_id)
}

fn save_review_item_result(input: SaveReviewItemInput) -> PackageResult {
    if let Err(error) = ensure_non_empty(&input.board_id, "board_id") {
        return PackageResult::err(error);
    }
    if let Err(error) = ensure_non_empty(&input.review_item_id, "review_item_id") {
        return PackageResult::err(error);
    }
    if load_board_record(&input.board_id).is_none() {
        return PackageResult::err(format!("board '{}' not found", input.board_id.trim()));
    }
    let timestamp = input.timestamp.unwrap_or_else(now_ts);
    let existing = load_review_item_record(&input.board_id, &input.review_item_id);
    let item = TeamReviewItem {
        schema_version: SCHEMA_VERSION,
        board_id: input.board_id.trim().to_string(),
        review_item_id: input.review_item_id.trim().to_string(),
        task_id: input.task_id.trim().to_string(),
        reviewer_member_id: input.reviewer_member_id.trim().to_string(),
        status: if input.status.trim().is_empty() {
            "pending".to_string()
        } else {
            input.status.trim().to_string()
        },
        summary: input.summary,
        artifact_refs: input.artifact_refs,
        created_at: existing
            .as_ref()
            .map(|item| item.created_at)
            .unwrap_or(timestamp),
        updated_at: timestamp,
        metadata: normalize_metadata(input.metadata),
    };
    if let Err(error) = persist_review_item(&item) {
        return PackageResult::err(error);
    }
    let event_type = if item.status == "completed" {
        "review_completed"
    } else {
        "review_requested"
    }
    .to_string();
    let _ = append_activity_event(TeamActivityEvent {
        schema_version: SCHEMA_VERSION,
        board_id: item.board_id.clone(),
        event_id: format!("review:{}:{}", item.review_item_id, item.updated_at),
        event_type,
        task_id: item.task_id.clone(),
        handoff_id: String::new(),
        actor_member_id: item.reviewer_member_id.clone(),
        summary: format!(
            "Review item '{}' stored with status '{}'",
            item.review_item_id, item.status
        ),
        created_at: item.updated_at,
        metadata: item.metadata.clone(),
    });
    PackageResult::ok(serde_json::json!({ "review_item": item }))
}

fn list_review_items_result(input: ListReviewItemsInput) -> PackageResult {
    if let Err(error) = ensure_non_empty(&input.board_id, "board_id") {
        return PackageResult::err(error);
    }
    if load_board_record(&input.board_id).is_none() {
        return PackageResult::err(format!("board '{}' not found", input.board_id.trim()));
    }
    let index: Vec<ReviewItemIndexEntry> =
        parse_json_or_default(kv_get(&review_items_index_key(&input.board_id)));
    let review_items = index
        .into_iter()
        .filter_map(|item| load_review_item_record(&input.board_id, &item.review_item_id))
        .collect::<Vec<_>>();
    PackageResult::ok(
        serde_json::json!({ "board_id": input.board_id.trim(), "review_items": review_items }),
    )
}

fn load_activity_event_record(board_id: &str, event_id: &str) -> Option<TeamActivityEvent> {
    kv_get(&activity_event_key(board_id, event_id))
        .and_then(|json| serde_json::from_str(&json).ok())
}

fn append_activity_event(event: TeamActivityEvent) -> Result<(), String> {
    write_json(
        &activity_event_key(&event.board_id, &event.event_id),
        &event,
    )?;
    let mut index: Vec<ActivityEventIndexEntry> =
        parse_json_or_default(kv_get(&activity_log_index_key(&event.board_id)));
    push_or_replace(
        &mut index,
        ActivityEventIndexEntry {
            event_id: event.event_id.clone(),
            created_at: event.created_at,
        },
        |left, right| left.event_id == right.event_id,
    );
    index.sort_by(|left, right| {
        right
            .created_at
            .cmp(&left.created_at)
            .then(left.event_id.cmp(&right.event_id))
    });
    write_json(&activity_log_index_key(&event.board_id), &index)?;
    sync_board_summary(&event.board_id)
}

fn append_activity_result(input: AppendActivityInput) -> PackageResult {
    if let Err(error) = ensure_non_empty(&input.board_id, "board_id") {
        return PackageResult::err(error);
    }
    if let Err(error) = ensure_non_empty(&input.event_id, "event_id") {
        return PackageResult::err(error);
    }
    if let Err(error) = ensure_non_empty(&input.event_type, "event_type") {
        return PackageResult::err(error);
    }
    if load_board_record(&input.board_id).is_none() {
        return PackageResult::err(format!("board '{}' not found", input.board_id.trim()));
    }
    let event = TeamActivityEvent {
        schema_version: SCHEMA_VERSION,
        board_id: input.board_id.trim().to_string(),
        event_id: input.event_id.trim().to_string(),
        event_type: input.event_type.trim().to_string(),
        task_id: input.task_id.trim().to_string(),
        handoff_id: input.handoff_id.trim().to_string(),
        actor_member_id: input.actor_member_id.trim().to_string(),
        summary: input.summary,
        created_at: input.timestamp.unwrap_or_else(now_ts),
        metadata: normalize_metadata(input.metadata),
    };
    if let Err(error) = append_activity_event(event.clone()) {
        return PackageResult::err(error);
    }
    PackageResult::ok(serde_json::json!({ "event": event }))
}

fn list_activity_result(input: ListActivityInput) -> PackageResult {
    if let Err(error) = ensure_non_empty(&input.board_id, "board_id") {
        return PackageResult::err(error);
    }
    if load_board_record(&input.board_id).is_none() {
        return PackageResult::err(format!("board '{}' not found", input.board_id.trim()));
    }
    let index: Vec<ActivityEventIndexEntry> =
        parse_json_or_default(kv_get(&activity_log_index_key(&input.board_id)));
    let activity = index
        .into_iter()
        .filter_map(|entry| load_activity_event_record(&input.board_id, &entry.event_id))
        .collect::<Vec<_>>();
    PackageResult::ok(serde_json::json!({ "board_id": input.board_id.trim(), "activity": activity }))
}

fn describe_result() -> PackageResult {
    PackageResult::ok(serde_json::json!({
        "package": PACKAGE_NAME,
        "runtime": "wasm",
        "capabilities": [TASKBOARD_CAPABILITY, HANDOFF_CAPABILITY],
        "actions": {
            TASKBOARD_CAPABILITY: ["describe", "health", "create_board", "get_board", "save_task", "list_tasks"],
            HANDOFF_CAPABILITY: ["describe", "health", "create_handoff", "update_handoff_status", "list_handoffs", "save_review_item", "list_review_items", "append_activity", "list_activity"]
        }
    }))
}

#[plugin_fn]
pub fn init(_input: String) -> FnResult<String> {
    log_info("team-task-board initialized");
    Ok(PackageResult::ok_empty().to_json())
}

#[plugin_fn]
pub fn handle_ws_message(input: String) -> FnResult<String> {
    let req: WsRequest = serde_json::from_str(&input).unwrap_or(WsRequest {
        action: String::new(),
        data: Value::Null,
    });

    let result = match req.action.as_str() {
        "describe" => describe_result(),
        "health" => PackageResult::ok(serde_json::json!({
            "healthy": true,
            "package": PACKAGE_NAME,
            "capabilities": [TASKBOARD_CAPABILITY, HANDOFF_CAPABILITY],
        })),
        "create_board" | "call" => {
            let payload: CreateBoardInput =
                serde_json::from_value(req.data).unwrap_or(CreateBoardInput {
                    board_id: String::new(),
                    session_id: String::new(),
                    title: String::new(),
                    status: default_board_status(),
                    timestamp: None,
                    metadata: Value::Null,
                });
            save_board_record(payload)
        }
        "get_board" => {
            let payload: GetBoardInput =
                serde_json::from_value(req.data).unwrap_or(GetBoardInput {
                    board_id: String::new(),
                });
            get_board_result(payload)
        }
        "save_task" => {
            let payload: SaveTaskInput =
                serde_json::from_value(req.data).unwrap_or(SaveTaskInput {
                    board_id: String::new(),
                    task_id: String::new(),
                    title: String::new(),
                    description: String::new(),
                    kind: default_task_kind(),
                    status: default_task_status(),
                    owner_member_id: String::new(),
                    depends_on: Vec::new(),
                    artifact_refs: Vec::new(),
                    review_state: String::new(),
                    timestamp: None,
                    metadata: Value::Null,
                });
            save_task_result(payload)
        }
        "list_tasks" => {
            let payload: ListTasksInput =
                serde_json::from_value(req.data).unwrap_or(ListTasksInput {
                    board_id: String::new(),
                });
            list_tasks_result(payload)
        }
        "create_handoff" => {
            let payload: CreateHandoffInput =
                serde_json::from_value(req.data).unwrap_or(CreateHandoffInput {
                    board_id: String::new(),
                    handoff_id: String::new(),
                    task_id: String::new(),
                    from_member_id: String::new(),
                    to_member_id: String::new(),
                    reason: String::new(),
                    expected_outcome: String::new(),
                    context_snapshot_ref: String::new(),
                    timestamp: None,
                    metadata: Value::Null,
                });
            create_handoff_result(payload)
        }
        "update_handoff_status" => {
            let payload: UpdateHandoffStatusInput =
                serde_json::from_value(req.data).unwrap_or(UpdateHandoffStatusInput {
                    board_id: String::new(),
                    handoff_id: String::new(),
                    status: String::new(),
                });
            update_handoff_status_result(payload)
        }
        "list_handoffs" => {
            let payload: ListHandoffsInput =
                serde_json::from_value(req.data).unwrap_or(ListHandoffsInput {
                    board_id: String::new(),
                });
            list_handoffs_result(payload)
        }
        "save_review_item" => {
            let payload: SaveReviewItemInput =
                serde_json::from_value(req.data).unwrap_or(SaveReviewItemInput {
                    board_id: String::new(),
                    review_item_id: String::new(),
                    task_id: String::new(),
                    reviewer_member_id: String::new(),
                    status: String::new(),
                    summary: String::new(),
                    artifact_refs: Vec::new(),
                    timestamp: None,
                    metadata: Value::Null,
                });
            save_review_item_result(payload)
        }
        "list_review_items" => {
            let payload: ListReviewItemsInput =
                serde_json::from_value(req.data).unwrap_or(ListReviewItemsInput {
                    board_id: String::new(),
                });
            list_review_items_result(payload)
        }
        "append_activity" => {
            let payload: AppendActivityInput =
                serde_json::from_value(req.data).unwrap_or(AppendActivityInput {
                    board_id: String::new(),
                    event_id: String::new(),
                    event_type: String::new(),
                    task_id: String::new(),
                    handoff_id: String::new(),
                    actor_member_id: String::new(),
                    summary: String::new(),
                    timestamp: None,
                    metadata: Value::Null,
                });
            append_activity_result(payload)
        }
        "list_activity" => {
            let payload: ListActivityInput =
                serde_json::from_value(req.data).unwrap_or(ListActivityInput {
                    board_id: String::new(),
                });
            list_activity_result(payload)
        }
        _ => PackageResult::err(format!("unknown action: {}", req.action)),
    };

    Ok(result.to_json())
}

