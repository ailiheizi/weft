use weft_package_sdk::*;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};

const PACKAGE_NAME: &str = "workflow-orchestrator";
const CAPABILITY_NAME: &str = "workflow.orchestration";
const BOARD_INDEX_KEY: &str = "team-task-board:boards";
const TASK_BOARD_PACKAGE: &str = "team-task-board";
const TEAM_RUNTIME_PACKAGE: &str = "team-runtime";
const WORKFLOW_TEMPLATE_PACKAGE: &str = "workflow-template-devteam";

#[derive(Debug, Clone, Deserialize, Default)]
struct BoardIndexEntry {
    #[serde(default)]
    board_id: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
struct TaskRecord {
    #[serde(default)]
    board_id: String,
    #[serde(default)]
    task_id: String,
    #[serde(default)]
    title: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    kind: String,
    #[serde(default)]
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
    metadata: Value,
}

#[derive(Debug, Clone, Deserialize, Default)]
struct HandoffRecord {
    #[serde(default)]
    handoff_id: String,
    #[serde(default)]
    from_member_id: String,
    #[serde(default)]
    to_member_id: String,
    #[serde(default)]
    task_id: String,
    #[serde(default)]
    board_id: String,
    #[serde(default)]
    status: String,
    #[serde(default)]
    context_snapshot_ref: String,
    #[serde(default)]
    reason: String,
}

#[derive(Debug, Clone, Serialize)]
struct PendingHandoffSummary {
    board_id: String,
    handoff_id: String,
    task_id: String,
    from_member_id: String,
    to_member_id: String,
}

#[derive(Debug, Deserialize, Default)]
struct DispatchOneInput {
    #[serde(default)]
    board_id: String,
    #[serde(default)]
    handoff_id: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
struct PhaseDefinition {
    #[serde(default)]
    id: String,
    #[serde(default)]
    default_role: String,
    #[serde(default)]
    entry_statuses: Vec<String>,
    #[serde(default)]
    exit_statuses: Vec<String>,
    #[serde(default)]
    next: Vec<String>,
    #[serde(default)]
    terminal: bool,
}

#[derive(Debug, Clone, Deserialize, Default)]
struct PackageEnvelope {
    #[serde(default)]
    status: String,
    #[serde(default)]
    data: Option<Value>,
    #[serde(default)]
    error: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
struct PlannedSubtask {
    #[serde(default)]
    title: String,
    #[serde(default)]
    description: String,
}

#[plugin_fn]
pub fn init(_input: String) -> FnResult<String> {
    log_info("workflow-orchestrator initialized");
    Ok(PackageResult::ok_empty().to_json())
}

#[plugin_fn]
pub fn handle_ws_message(input: String) -> FnResult<String> {
    let req: WsRequest = serde_json::from_str(&input).unwrap_or(WsRequest {
        action: String::new(),
        data: serde_json::Value::Null,
    });

    let result = match req.action.as_str() {
        "describe" => PackageResult::ok(json!({
            "package": PACKAGE_NAME,
            "capability": CAPABILITY_NAME,
            "runtime": "wasm",
            "actions": [
                "describe",
                "health",
                "plan",
                "call",
                "tick",
                "dispatch",
                "list_pending_handoffs",
                "dispatch_one"
            ],
            "tick": {
                "interval_hint_ms": 10_000,
                "summary": "Advances pending handoffs and promotes tasks across workflow phases"
            }
        })),
        "health" => PackageResult::ok(json!({"healthy": true, "package": PACKAGE_NAME})),
        "plan" | "call" => PackageResult::ok(json!({
            "package": PACKAGE_NAME,
            "accepted": true,
            "workflow": req.data,
        })),
        _ => PackageResult::err(format!("unknown action: {}", req.action)),
    };

    Ok(result.to_json())
}

#[plugin_fn]
pub fn tick(_input: String) -> FnResult<String> {
    let started_at = now_ms();
    let board_ids = load_board_ids();

    for board_id in &board_ids {
        let tasks = load_tasks(board_id);
        let handoffs = load_handoffs(board_id);
        advance_task_phases(board_id, &tasks, &handoffs);
    }

    log_info(&format!(
        "workflow-orchestrator tick completed at {} for {} boards",
        started_at,
        board_ids.len()
    ));
    Ok(PackageResult::ok_empty().to_json())
}

#[plugin_fn]
pub fn dispatch(_input: String) -> FnResult<String> {
    let board_ids = load_board_ids();

    for board_id in &board_ids {
        let tasks = load_tasks(board_id);
        let handoffs = load_handoffs(board_id);
        for handoff in handoffs.iter().filter(|item| item.status == "pending") {
            dispatch_single_handoff(board_id, handoff, &tasks);
        }
    }

    log_info(&format!(
        "workflow-orchestrator dispatch completed for {} boards",
        board_ids.len()
    ));
    Ok(PackageResult::ok_empty().to_json())
}

#[plugin_fn]
pub fn list_pending_handoffs(_input: String) -> FnResult<String> {
    let handoffs = load_board_ids()
        .into_iter()
        .flat_map(|board_id| {
            load_handoffs(&board_id)
                .into_iter()
                .filter(|handoff| handoff.status == "pending")
                .map(move |handoff| PendingHandoffSummary {
                    board_id: board_id.clone(),
                    handoff_id: handoff.handoff_id,
                    task_id: handoff.task_id,
                    from_member_id: handoff.from_member_id,
                    to_member_id: handoff.to_member_id,
                })
        })
        .collect::<Vec<_>>();

    Ok(PackageResult::ok(json!({ "handoffs": handoffs })).to_json())
}

#[plugin_fn]
pub fn dispatch_one(input: String) -> FnResult<String> {
    let parsed: DispatchOneInput = serde_json::from_str(&input).unwrap_or_default();
    let board_id = parsed.board_id.trim();
    let handoff_id = parsed.handoff_id.trim();

    if board_id.is_empty() || handoff_id.is_empty() {
        return Ok(PackageResult::err("missing board_id or handoff_id".to_string()).to_json());
    }

    let tasks = load_tasks(board_id);
    let handoffs = load_handoffs(board_id);
    let Some(handoff) = handoffs.iter().find(|item| item.handoff_id == handoff_id) else {
        return Ok(PackageResult::err(format!(
            "handoff '{}' not found on board '{}'",
            handoff_id, board_id
        ))
        .to_json());
    };

    if handoff.status != "pending" {
        return Ok(PackageResult::ok(json!({
            "board_id": board_id,
            "handoff_id": handoff_id,
            "status": handoff.status,
            "dispatched": false,
        }))
        .to_json());
    }

    dispatch_single_handoff(board_id, handoff, &tasks);

    Ok(PackageResult::ok(json!({
        "board_id": board_id,
        "handoff_id": handoff_id,
        "status": "dispatched",
        "dispatched": true,
    }))
    .to_json())
}

fn load_board_ids() -> Vec<String> {
    // 区分"无 board"与"索引损坏":后者会让 orchestrator 静默空转(用户看不到任何进度,
    // 误以为卡死或崩溃),必须上报。
    let raw = kv_get(BOARD_INDEX_KEY);
    if let Some(ref json) = raw {
        if !json.trim().is_empty()
            && serde_json::from_str::<Vec<BoardIndexEntry>>(json).is_err()
        {
            log_warn(&format!(
                "board index '{}' present but failed to parse (len={}); orchestrator will see 0 boards. \
                 Raw head: {}",
                BOARD_INDEX_KEY,
                json.len(),
                json.chars().take(120).collect::<String>()
            ));
        }
    }
    parse_json_or_default::<Vec<BoardIndexEntry>>(raw)
        .into_iter()
        .filter_map(|entry| {
            let board_id = entry.board_id.trim().to_string();
            if board_id.is_empty() {
                None
            } else {
                Some(board_id)
            }
        })
        .collect()
}

fn load_tasks(board_id: &str) -> Vec<TaskRecord> {
    let payload = json!({ "board_id": board_id });
    match call_package_data(TASK_BOARD_PACKAGE, "list_tasks", &payload) {
        Ok(data) => serde_json::from_value::<Vec<TaskRecord>>(
            data.get("tasks").cloned().unwrap_or(Value::Array(Vec::new())),
        )
        .unwrap_or_default(),
        Err(error) => {
            log_warn(&format!("list_tasks failed for board '{}': {}", board_id, error));
            Vec::new()
        }
    }
}

fn load_handoffs(board_id: &str) -> Vec<HandoffRecord> {
    let payload = json!({ "board_id": board_id });
    match call_package_data(TASK_BOARD_PACKAGE, "list_handoffs", &payload) {
        Ok(data) => serde_json::from_value::<Vec<HandoffRecord>>(
            data.get("handoffs")
                .cloned()
                .unwrap_or(Value::Array(Vec::new())),
        )
        .unwrap_or_default(),
        Err(error) => {
            log_warn(&format!("list_handoffs failed for board '{}': {}", board_id, error));
            Vec::new()
        }
    }
}

// ===== P1: review 阶段 adversarial verify(3 个独立 reviewer 并行投票)=====

/// review 投票的三个视角。每个对应一个并行 reviewer handoff。
const REVIEW_PERSPECTIVES: [&str; 3] = ["correctness", "security", "requirements"];
/// 最多打回轮数:第 3 次进 review(round>=2 且结论 reject)强制放行,防 execute↔review 无限循环。
const REVIEW_MAX_ROUNDS: u64 = 2;

/// review-vote handoff 的 id 命名:`review-vote-<task_id>-<round>-<perspective>`。
/// HandoffRecord 无 metadata 字段,故用 id 模式承载投票元信息。
fn review_vote_handoff_id(task_id: &str, round: u64, perspective: &str) -> String {
    format!("review-vote-{}-{}-{}", task_id.trim(), round, perspective)
}

/// 解析 review-vote handoff id,返回 (round, perspective)。非投票 handoff 返回 None。
fn parse_review_vote_handoff(handoff_id: &str) -> Option<(u64, String)> {
    let rest = handoff_id.strip_prefix("review-vote-")?;
    // 后两段是 round 和 perspective;task_id 自身可能含 '-',故从右侧切两刀。
    let last_dash = rest.rfind('-')?;
    let (head, perspective) = (&rest[..last_dash], &rest[last_dash + 1..]);
    let round_dash = head.rfind('-')?;
    let round: u64 = head[round_dash + 1..].parse().ok()?;
    Some((round, perspective.to_string()))
}

fn review_round_of(task: &TaskRecord) -> u64 {
    task.metadata
        .get("review_round")
        .and_then(Value::as_u64)
        .unwrap_or(0)
}

/// 每票写独立 KV key,规避 3 个 reviewer 并发写同一 key 丢票的竞争。
/// key = `review:vote:<board>:<task>:<round>:<perspective>`,value = "approve"|"reject"。
fn review_vote_key(board_id: &str, task_id: &str, round: u64, perspective: &str) -> String {
    format!("review:vote:{}:{}:{}:{}", board_id, task_id, round, perspective)
}

fn record_review_vote(
    board_id: &str,
    task_id: &str,
    round: u64,
    perspective: &str,
    verdict: &str,
) {
    kv_set(
        &review_vote_key(board_id, task_id, round, perspective),
        verdict,
    );
}

/// 读回本轮所有已落票(按视角逐个读独立 key),返回 (approve数, reject数, 总票数)。
fn tally_review_votes(board_id: &str, task_id: &str, round: u64) -> (u64, u64, u64) {
    let mut approve = 0u64;
    let mut reject = 0u64;
    let mut total = 0u64;
    for perspective in REVIEW_PERSPECTIVES.iter() {
        let v = kv_get(&review_vote_key(board_id, task_id, round, perspective)).unwrap_or_default();
        match v.trim() {
            "approve" => {
                approve += 1;
                total += 1;
            }
            "reject" => {
                reject += 1;
                total += 1;
            }
            _ => {}
        }
    }
    (approve, reject, total)
}

/// 三视角 reviewer prompt,强制输出 verdict JSON。
fn build_review_prompt(perspective: &str, task: &TaskRecord) -> String {
    let lens = match perspective {
        "correctness" => "CORRECTNESS — logic errors, wrong behavior, broken edge cases, incorrect outputs.",
        "security" => "SECURITY & ROBUSTNESS — injection, missing validation/auth, unsafe handling, crashes, resource leaks.",
        "requirements" => "REQUIREMENTS COMPLETENESS — does it fully satisfy the stated task? missing features, unmet acceptance criteria.",
        _ => "GENERAL QUALITY.",
    };
    let title = task.title.trim();
    let description = task.description.trim();
    let context = if !title.is_empty() && !description.is_empty() {
        format!("{}\n\n{}", title, description)
    } else if !title.is_empty() {
        title.to_string()
    } else {
        description.to_string()
    };
    format!(
        "You are a REVIEWER inspecting completed work through ONE lens only:\n{}\n\nTASK UNDER REVIEW:\n{}\n\nInspect the produced artifacts in the workspace. Judge ONLY through your assigned lens. Default to \"reject\" if you find a real problem in your lens; \"approve\" only if your lens is genuinely satisfied.\n\nYour ENTIRE reply must be ONLY a raw JSON object, nothing else, exactly:\n{{\"verdict\":\"approve\",\"reason\":\"one concise sentence\"}}\nor\n{{\"verdict\":\"reject\",\"reason\":\"one concise sentence naming the specific problem\"}}\n\nNo prose, no markdown fences, no tool calls beyond reading artifacts.",
        lens, context
    )
}

/// 容错解析 reviewer 的 verdict。解析失败按 approve 计(宽松,避免误打回),返回 (verdict, parsed)。
fn parse_review_verdict(reply: &str) -> (&'static str, bool) {
    let trimmed = reply.trim();
    let fenced = trimmed
        .strip_prefix("```json")
        .and_then(|v| v.strip_suffix("```"))
        .map(str::trim)
        .unwrap_or(trimmed);
    for candidate in [trimmed, fenced] {
        if let Ok(Value::Object(map)) = serde_json::from_str::<Value>(candidate) {
            if let Some(v) = map.get("verdict").and_then(Value::as_str) {
                return match v.trim().to_lowercase().as_str() {
                    "reject" => ("reject", true),
                    "approve" => ("approve", true),
                    _ => ("approve", false),
                };
            }
        }
    }
    // 兜底:裸文本含 reject 字样也算 reject。
    if fenced.to_lowercase().contains("\"reject\"") || fenced.to_lowercase().contains("verdict: reject")
    {
        return ("reject", true);
    }
    ("approve", false)
}

/// review 阶段父 task 首次到达:建 3 个并行 reviewer-vote handoff,父 task 设 in_review 等待。
fn initialize_review_fanout(board_id: &str, task: &TaskRecord, handoff: &HandoffRecord) -> Result<(), String> {
    let round = review_round_of(task);
    // 清理本轮可能的残留票(重试同 round 时)。
    for perspective in REVIEW_PERSPECTIVES.iter() {
        let _ = kv_delete(&review_vote_key(board_id, &task.task_id, round, perspective));
    }
    for perspective in REVIEW_PERSPECTIVES.iter() {
        let handoff_id = review_vote_handoff_id(&task.task_id, round, perspective);
        let handoff_payload = json!({
            "board_id": board_id,
            "handoff_id": handoff_id,
            "task_id": task.task_id,
            "from_member_id": handoff.to_member_id,
            "to_member_id": "reviewer",
            "reason": format!("adversarial review {} round {}", perspective, round),
            "expected_outcome": "Return a verdict JSON: approve or reject",
            "context_snapshot_ref": "",
        });
        call_package_data(TASK_BOARD_PACKAGE, "create_handoff", &handoff_payload)?;
    }
    // 父 task 置 in_review(review 阶段 entry status,不前进),等投票聚合改写 status。
    save_task_phase_status(task, "review", Some("in_review"))?;
    log_info(&format!(
        "review fan-out created {} reviewers for task '{}' round {} on board '{}'",
        REVIEW_PERSPECTIVES.len(),
        task.task_id,
        round,
        board_id
    ));
    Ok(())
}

/// 齐 N 票后聚合:≥2 reject 打回 execute(round+1);否则 review_completed 进 integrate。
/// round>=REVIEW_MAX_ROUNDS 且结论 reject 时强制放行。幂等(重复调用设确定 status)。
fn aggregate_review_votes(board_id: &str, task: &TaskRecord, round: u64) {
    let (approve, reject, total) = tally_review_votes(board_id, &task.task_id, round);
    if total < REVIEW_PERSPECTIVES.len() as u64 {
        return; // 票未齐,等最后一票触发。
    }
    let rejected = reject >= 2;
    if rejected && round < REVIEW_MAX_ROUNDS {
        // 打回 execute,round+1。
        let mut metadata = metadata_object(&task.metadata);
        metadata.insert("review_round".to_string(), json!(round + 1));
        metadata.insert("phase".to_string(), json!("review"));
        let payload = json!({
            "board_id": task.board_id,
            "task_id": task.task_id,
            "title": task.title,
            "description": task.description,
            "kind": task.kind,
            "status": "changes_requested",
            "owner_member_id": task.owner_member_id,
            "depends_on": task.depends_on,
            "artifact_refs": task.artifact_refs,
            "review_state": task.review_state,
            "metadata": Value::Object(metadata),
        });
        let _ = call_package_data(TASK_BOARD_PACKAGE, "save_task", &payload);
        log_info(&format!(
            "review verdict: CHANGES_REQUESTED task '{}' round {} (approve={} reject={}) -> back to execute",
            task.task_id, round, approve, reject
        ));
    } else {
        // 放行(approve 多数,或达打回上限强制放行)。
        if rejected {
            log_info(&format!(
                "review max rounds reached for task '{}' (reject={}), force-passing",
                task.task_id, reject
            ));
        }
        if let Err(error) = save_task_phase_status(task, "review", Some("review_completed")) {
            log_warn(&format!(
                "failed to set review_completed for task '{}': {}",
                task.task_id, error
            ));
        }
        log_info(&format!(
            "review verdict: APPROVED task '{}' round {} (approve={} reject={}) -> integrate",
            task.task_id, round, approve, reject
        ));
    }
}

/// 处理单个 review-vote handoff:用视角 prompt 调 reviewer,解析 verdict,落票,齐票则聚合。
fn dispatch_review_vote(
    board_id: &str,
    handoff: &HandoffRecord,
    task: &TaskRecord,
    round: u64,
    perspective: &str,
) {
    let prompt = build_review_prompt(perspective, task);
    let payload = json!({
        "board_id": board_id,
        "task_id": handoff.task_id,
        "from_role_id": handoff.from_member_id,
        "to_role_id": "reviewer",
        "prompt": prompt,
        "reason": format!("adversarial review {}", perspective),
        "must_act": true,
        "planning_only": false,
    });
    let verdict = match call_package_data(TEAM_RUNTIME_PACKAGE, "execute_delegate", &payload) {
        Ok(data) => {
            let reply = data.get("reply").and_then(Value::as_str).unwrap_or("");
            let (v, parsed) = parse_review_verdict(reply);
            log_info(&format!(
                "review vote [{}] task '{}' round {} -> {} (parsed={})",
                perspective, handoff.task_id, round, v, parsed
            ));
            v
        }
        Err(error) => {
            log_warn(&format!(
                "review vote [{}] execute_delegate failed for task '{}': {} (counting as REJECT — \
                 a review that could not run must not silently pass)",
                perspective, handoff.task_id, error
            ));
            "reject"
        }
    };
    // 无论 LLM 成功/失败,投票 handoff 处理完即标记终态,防止 dispatch 循环反复重投(死循环根因)。
    let _ = call_package_data(
        TASK_BOARD_PACKAGE,
        "update_handoff_status",
        &json!({"board_id": board_id, "handoff_id": handoff.handoff_id, "status": "accepted"}),
    );
    record_review_vote(board_id, &handoff.task_id, round, perspective, verdict);
    // 重新读取 task(metadata 可能已被 initialize 改成 in_review)并尝试聚合。
    let fresh_tasks = load_tasks(board_id);
    if let Some(fresh) = fresh_tasks.iter().find(|t| t.task_id == handoff.task_id) {
        aggregate_review_votes(board_id, fresh, round);
    }
}

fn dispatch_single_handoff(board_id: &str, handoff: &HandoffRecord, tasks: &[TaskRecord]) {
    // P1: review-vote handoff 走独立投票路径,不走默认 delegate 流程。
    if let Some((round, perspective)) = parse_review_vote_handoff(&handoff.handoff_id) {
        if let Some(task) = tasks.iter().find(|t| t.task_id == handoff.task_id) {
            dispatch_review_vote(board_id, handoff, task, round, &perspective);
        }
        return;
    }
    // P1: review 阶段父 task 首次 dispatch → 扇出 3 个并行 reviewer,不走橡皮图章放行。
    // 仅在 review 未开始投票时触发(排除 in_review 投票中 / review_completed 已决 / changes_requested 打回中),
    // 配合"触发 handoff 即标 accepted"双重防止重复扇出。
    if let Some(task) = tasks.iter().find(|t| t.task_id == handoff.task_id) {
        let st = task.status.as_str();
        let already = st == "in_review" || st == "review_completed" || st == "changes_requested";
        if task_phase(task) == "review" && !already {
            match initialize_review_fanout(board_id, task, handoff) {
                Ok(()) => {
                    let _ = call_package_data(
                        TASK_BOARD_PACKAGE,
                        "update_handoff_status",
                        &json!({"board_id": board_id, "handoff_id": handoff.handoff_id, "status": "accepted"}),
                    );
                    return;
                }
                Err(error) => log_warn(&format!(
                    "review fan-out failed for task '{}' on board '{}': {} (falling back to default)",
                    task.task_id, board_id, error
                )),
            }
        }
    }
    let task = tasks.iter().find(|task| task.task_id == handoff.task_id);
    let prompt = build_delegate_prompt(task, &handoff.task_id);
    // plan 阶段的 planner 是"纯规划"(只做分解、输出子任务 JSON),禁用工具,
    // 防止带 fs_write 的 LLM 直接动手造文件而非克制分解。
    let planning_only = task.map(task_phase).as_deref() == Some("plan");
    let payload = json!({
        "board_id": board_id,
        "task_id": handoff.task_id,
        "from_role_id": handoff.from_member_id,
        "to_role_id": handoff.to_member_id,
        "prompt": prompt,
        "reason": "orchestrated handoff",
        "must_act": true,
        "planning_only": planning_only,
    });

    match call_package_data(TEAM_RUNTIME_PACKAGE, "execute_delegate", &payload) {
        Ok(data) => {
            if data.get("status").and_then(Value::as_str) == Some("executed") {
                let update_payload = json!({
                    "board_id": board_id,
                    "handoff_id": handoff.handoff_id,
                    "status": "accepted",
                });
                match call_package_data(TASK_BOARD_PACKAGE, "update_handoff_status", &update_payload)
                {
                    Ok(_) => log_info(&format!(
                        "accepted handoff '{}' for task '{}' on board '{}'",
                        handoff.handoff_id, handoff.task_id, board_id
                    )),
                    Err(error) => log_warn(&format!(
                        "execute_delegate succeeded but update_handoff_status failed for '{}' on board '{}': {}",
                        handoff.handoff_id, board_id, error
                    )),
                }
                let mut planned_fanout = false;
                if let Some(task) = task {
                    let phase = task_phase(task);
                    if phase == "plan" {
                        if let Some(subtasks) = data
                            .get("reply")
                            .and_then(Value::as_str)
                            .and_then(parse_planned_subtasks)
                        {
                            match initialize_plan_fanout(board_id, task, handoff, &subtasks) {
                                Ok(()) => planned_fanout = true,
                                Err(error) => log_warn(&format!(
                                    "failed to fan out planner task '{}' on board '{}': {}",
                                    task.task_id, board_id, error
                                )),
                            }
                        }
                    }
                    if !planned_fanout {
                        if let Some(exit) = forward_exit_status(&phase) {
                            if task.status != exit {
                                // execute 阶段:回填 implementer 写过的文件到 artifact_refs。
                                let artifacts = if phase == "execute" {
                                    take_recorded_artifacts(
                                        board_id,
                                        &task.task_id,
                                        &handoff.to_member_id,
                                    )
                                } else {
                                    Vec::new()
                                };
                                if let Err(error) = save_task_phase_status_artifacts(
                                    task,
                                    &phase,
                                    Some(&exit),
                                    &artifacts,
                                ) {
                                    log_warn(&format!(
                                        "failed to advance task '{}' status to '{}' on board '{}': {}",
                                        task.task_id, exit, board_id, error
                                    ));
                                } else if !artifacts.is_empty() {
                                    log_info(&format!(
                                        "backfilled {} artifact(s) to task '{}' on board '{}'",
                                        artifacts.len(),
                                        task.task_id,
                                        board_id
                                    ));
                                }
                            }
                        }
                    }
                }
                if let Some(reply) = data.get("reply").and_then(Value::as_str) {
                    if !reply.trim().is_empty() {
                        let ts = now_ms();
                        let activity_payload = json!({
                            "board_id": board_id,
                            "event_id": format!("act-{}-{}", handoff.handoff_id, ts),
                            "event_type": "delegate_reply",
                            "task_id": handoff.task_id,
                            "handoff_id": handoff.handoff_id,
                            "actor_member_id": handoff.to_member_id,
                            "summary": reply,
                            "timestamp": ts,
                        });
                        if let Err(error) = call_package_data(
                            TASK_BOARD_PACKAGE,
                            "append_activity",
                            &activity_payload,
                        ) {
                            log_warn(&format!(
                                "append_activity for delegate reply failed on handoff '{}': {}",
                                handoff.handoff_id, error
                            ));
                        }
                    }
                }
            } else {
                log_warn(&format!(
                    "execute_delegate returned non-executed status for handoff '{}' on board '{}': {}",
                    handoff.handoff_id,
                    board_id,
                    data
                ));
            }
        }
        Err(error) => {
            log_warn(&format!(
                "execute_delegate failed for handoff '{}' on board '{}': {}",
                handoff.handoff_id, board_id, error
            ));
            let fail_payload = json!({
                "board_id": board_id,
                "handoff_id": handoff.handoff_id,
                "status": "failed",
            });
            if let Err(e) = call_package_data(TASK_BOARD_PACKAGE, "update_handoff_status", &fail_payload)
            {
                log_warn(&format!(
                    "failed to mark handoff '{}' as failed on board '{}': {}",
                    handoff.handoff_id, board_id, e
                ));
            }
        }
    }
}

fn advance_task_phases(board_id: &str, tasks: &[TaskRecord], handoffs: &[HandoffRecord]) {
    for task in tasks {
        let current_phase = task_phase(task);
        let phase_def = match get_phase(&current_phase) {
            Ok(phase) => phase,
            Err(error) => {
                log_warn(&format!(
                    "get_phase failed for task '{}' phase '{}' on board '{}': {}",
                    task.task_id, current_phase, board_id, error
                ));
                continue;
            }
        };

        let Some(next_phase) = pick_next_phase(task, &phase_def, tasks) else {
            continue;
        };

        match verify_transition(&current_phase, &next_phase) {
            Ok(true) => {}
            Ok(false) => {
                log_warn(&format!(
                    "transition denied for task '{}' on board '{}': {} -> {}",
                    task.task_id, board_id, current_phase, next_phase
                ));
                continue;
            }
            Err(error) => {
                log_warn(&format!(
                    "verify_transition failed for task '{}' on board '{}': {}",
                    task.task_id, board_id, error
                ));
                continue;
            }
        }

        let next_def = match get_phase(&next_phase) {
            Ok(phase) => phase,
            Err(error) => {
                log_warn(&format!(
                    "get_phase failed for next phase '{}' for task '{}' on board '{}': {}",
                    next_phase, task.task_id, board_id, error
                ));
                continue;
            }
        };

        let handoff_id = auto_handoff_id(&task.task_id, &next_phase);
        let handoff_exists = handoffs.iter().any(|handoff| handoff.handoff_id == handoff_id);
        // 同角色阶段转换通常无需委托 handoff（直接推进）。
        // 例外:intake→plan 虽然都是 planner,但 plan 阶段需要 planner 真正执行
        // 一次(做规划/分解,可能扇出多个并行子任务),所以强制建 planner 自委托 handoff,
        // 让 dispatch_single_handoff 跑 planner agent 并解析其 reply 做 fan-out。
        let plan_needs_planner = next_phase == "plan";
        let same_role = phase_def.default_role == next_def.default_role && !plan_needs_planner;
        let handoff_ready = same_role
            || handoff_exists
            || create_phase_handoff(
                board_id,
                &handoff_id,
                task,
                &phase_def.default_role,
                &next_def.default_role,
                &next_phase,
            );

        if !handoff_ready {
            continue;
        }

        // 同角色阶段（如 intake→plan 都是 planner）没有 delegate 来推进状态，
        // 直接把新 phase 的 exit_status 写上，使其在下一个 tick 继续前进；
        // 跨角色阶段则保留原状态，等 delegate 执行成功后由 dispatch 写 exit_status。
        let new_status = if same_role {
            forward_exit_status(&next_phase)
        } else if is_fanout_parent(task) && current_phase == "execute" && next_phase == "review" {
            phase_entry_status(&next_def)
        } else {
            None
        };
        if save_task_phase_status(task, &next_phase, new_status.as_deref()).is_ok() {
            log_info(&format!(
                "advanced task '{}' on board '{}' from '{}' to '{}'{}",
                task.task_id,
                board_id,
                current_phase,
                next_phase,
                new_status
                    .as_deref()
                    .map(|s| format!(" (auto status={})", s))
                    .unwrap_or_default()
            ));
            // P3: fan-out 父任务从 execute 进入 review 前,检查所有子任务是否都真有产出。
            if is_fanout_parent(task) && current_phase == "execute" && next_phase == "review" {
                check_fanout_completeness(board_id, task, tasks);
            }
        }
    }
}

fn create_phase_handoff(
    board_id: &str,
    handoff_id: &str,
    task: &TaskRecord,
    from_role_id: &str,
    to_role_id: &str,
    next_phase: &str,
) -> bool {
    let payload = json!({
        "board_id": board_id,
        "handoff_id": handoff_id,
        "task_id": task.task_id,
        "from_member_id": from_role_id,
        "to_member_id": to_role_id,
        "reason": format!("auto phase transition to {}", next_phase),
        "expected_outcome": format!("Advance task into {} phase", next_phase),
        "context_snapshot_ref": "",
        "metadata": {
            "phase": next_phase,
            "auto": true,
        }
    });

    match call_package_data(TASK_BOARD_PACKAGE, "create_handoff", &payload) {
        Ok(_) => true,
        Err(error) => {
            log_warn(&format!(
                "create_handoff failed for task '{}' on board '{}': {}",
                task.task_id, board_id, error
            ));
            false
        }
    }
}

/// 返回某 phase 的"前进方向" exit status（确定性，来自模板，不依赖 LLM 输出）。
/// review 有两个 exit（review_completed / changes_requested），取前进的 review_completed。
/// terminal phase 或无 exit_status 返回 None。
fn forward_exit_status(phase_id: &str) -> Option<String> {
    let def = get_phase(phase_id).ok()?;
    if def.terminal || def.exit_statuses.is_empty() {
        return None;
    }
    if def.exit_statuses.iter().any(|s| s == "review_completed") {
        return Some("review_completed".to_string());
    }
    def.exit_statuses.first().cloned()
}

/// 写回任务的 phase，并可选地同时把 status 改成给定值（None 则保留原 status）。
fn save_task_phase(task: &TaskRecord, next_phase: &str) -> Result<(), String> {
    save_task_phase_status(task, next_phase, None)
}

fn save_task_phase_status(
    task: &TaskRecord,
    next_phase: &str,
    new_status: Option<&str>,
) -> Result<(), String> {
    save_task_phase_status_artifacts(task, next_phase, new_status, &[])
}

/// 同 save_task_phase_status,但把 extra_artifacts 合并进 task.artifact_refs(去重)。
/// 用于 execute 推进时回填 implementer 产出的文件路径。
fn save_task_phase_status_artifacts(
    task: &TaskRecord,
    next_phase: &str,
    new_status: Option<&str>,
    extra_artifacts: &[String],
) -> Result<(), String> {
    let mut metadata = metadata_object(&task.metadata);
    metadata.insert("phase".to_string(), Value::String(next_phase.to_string()));
    let status = new_status.unwrap_or(&task.status);

    let mut artifact_refs = task.artifact_refs.clone();
    for a in extra_artifacts {
        if !a.trim().is_empty() && !artifact_refs.iter().any(|x| x == a) {
            artifact_refs.push(a.clone());
        }
    }

    let payload = json!({
        "board_id": task.board_id,
        "task_id": task.task_id,
        "title": task.title,
        "description": task.description,
        "kind": task.kind,
        "status": status,
        "owner_member_id": task.owner_member_id,
        "depends_on": task.depends_on,
        "artifact_refs": artifact_refs,
        "review_state": task.review_state,
        "metadata": Value::Object(metadata),
    });

    match call_package_data(TASK_BOARD_PACKAGE, "save_task", &payload) {
        Ok(_) => Ok(()),
        Err(error) => {
            log_warn(&format!(
                "save_task failed while advancing task '{}' on board '{}': {}",
                task.task_id, task.board_id, error
            ));
            Err(error)
        }
    }
}

/// 读取 implementer 写过的文件路径(agent-core 累加到 KV `artifacts:<session_id>`),
/// 并清掉该 key(避免打回重做时累积旧路径)。session_id 公式同 team-runtime delegate session。
fn take_recorded_artifacts(board_id: &str, task_id: &str, role: &str) -> Vec<String> {
    let session_id = format!("team-delegate-{}-{}-{}", board_id, task_id, role);
    let key = format!("artifacts:{}", session_id);
    let paths: Vec<String> = kv_get(&key)
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default();
    if !paths.is_empty() {
        let _ = kv_delete(&key);
    }
    paths
}

fn get_phase(phase: &str) -> Result<PhaseDefinition, String> {
    let data = call_package_data(
        WORKFLOW_TEMPLATE_PACKAGE,
        "get_phase",
        &json!({ "phase_id": phase }),
    )?;
    serde_json::from_value::<PhaseDefinition>(
        data.get("phase")
            .cloned()
            .ok_or_else(|| format!("missing phase payload for '{}'", phase))?,
    )
    .map_err(|error| error.to_string())
}

fn verify_transition(from_phase: &str, to_phase: &str) -> Result<bool, String> {
    let data = call_package_data(
        WORKFLOW_TEMPLATE_PACKAGE,
        "verify_transition",
        &json!({
            "from_phase": from_phase,
            "to_phase": to_phase,
        }),
    )?;
    Ok(data
        .get("allowed")
        .and_then(Value::as_bool)
        .unwrap_or(false))
}

fn pick_next_phase(task: &TaskRecord, phase: &PhaseDefinition, tasks: &[TaskRecord]) -> Option<String> {
    if phase.terminal || phase.next.is_empty() {
        return None;
    }
    if !dependencies_satisfied(task, tasks) {
        return None;
    }
    if is_fanout_child(task) && phase.id == "execute" && task.status == "review_requested" {
        return None;
    }
    if is_fanout_parent(task) && phase.id == "execute" {
        return phase
            .next
            .iter()
            .find(|candidate| candidate.as_str() == "review")
            .cloned();
    }
    if !phase.exit_statuses.iter().any(|status| status == &task.status) {
        return None;
    }

    if phase.id == "review" {
        if task.status == "changes_requested"
            && phase.next.iter().any(|candidate| candidate == "execute")
        {
            return Some("execute".to_string());
        }
        if task.status == "review_completed"
            && phase.next.iter().any(|candidate| candidate == "integrate")
        {
            return Some("integrate".to_string());
        }
    }

    phase.next.first().cloned()
}

fn task_phase(task: &TaskRecord) -> String {
    task.metadata
        .get("phase")
        .and_then(Value::as_str)
        .map(|phase| phase.trim().to_string())
        .filter(|phase| !phase.is_empty())
        .unwrap_or_else(|| "intake".to_string())
}

fn build_delegate_prompt(task: Option<&TaskRecord>, task_id: &str) -> String {
    let phase = task
        .map(task_phase)
        .unwrap_or_else(|| "intake".to_string());
    if let Some(task) = task {
        let title = task.title.trim();
        let description = task.description.trim();
        if phase == "plan" {
            let body = if !title.is_empty() && !description.is_empty() {
                format!("{}\n\n{}", title, description)
            } else if !title.is_empty() {
                title.to_string()
            } else if !description.is_empty() {
                description.to_string()
            } else {
                format!("Continue task {}", task_id)
            };
            return format!(
                "{}\n\nYou are the PLANNER. Break this work into the 2-4 MOST INDEPENDENT parts that separate implementers can build in PARALLEL. Each subtask must be a self-contained deliverable (e.g. one file, one page, or one module) that does not depend on the others being done first.\n\nIMPORTANT: Do NOT call any tools. Do NOT write files. Do NOT delegate. Output ONLY a JSON object, nothing else, in exactly this form:\n{{\"subtasks\":[{{\"title\":\"short subtask title\",\"description\":\"what to build for this subtask\"}}]}}\n\nReturn 2-4 subtasks whenever the work has independent parts (it almost always does). Only return a single subtask if the work is genuinely atomic and cannot be split. No prose, no markdown fences — output the raw JSON object as your reply.",
                body
            );
        }
        if !title.is_empty() && !description.is_empty() {
            let body = format!("{}\n\n{}", title, description);
            return decorate_phase_prompt(&phase, &body);
        }
        if !title.is_empty() {
            return decorate_phase_prompt(&phase, title);
        }
        if !description.is_empty() {
            return decorate_phase_prompt(&phase, description);
        }
    }
    format!("Continue task {}", task_id)
}

/// P2: 给 execute/integrate 阶段的 delegate prompt 注入"努力度 + 工具边界 + 完成契约",
/// 让 worker 不重复劳动、不偷工、产出可验证。其他阶段原样返回 body。
fn decorate_phase_prompt(phase: &str, body: &str) -> String {
    match phase {
        "execute" => format!(
            "{}\n\n--- EXECUTION CONTRACT ---\nYou are the IMPLEMENTER. Build EXACTLY this subtask — no more, no less.\nEFFORT: Do the complete job in this turn. Write all required files to the workspace using your tools. Don't stop at a sketch or a plan; produce working artifacts.\nBOUNDARIES: Stay within THIS subtask's scope. Do NOT re-decompose or delegate further. Do NOT touch unrelated files. Reuse what already exists in the workspace rather than rewriting it.\nDONE WHEN: the described functionality is fully implemented and the files are written. Before finishing, re-read your own output and confirm it actually satisfies the subtask.",
            body
        ),
        "integrate" => format!(
            "{}\n\n--- INTEGRATION CONTRACT ---\nYou are the INTEGRATOR. Combine the completed pieces into a coherent whole.\nEFFORT: Verify the parts fit together — consistent naming, shared entry points, no duplicated or conflicting files. Fix integration gaps directly.\nBOUNDARIES: Do NOT rebuild features from scratch; only wire and reconcile what implementers produced. Do NOT delegate further.\nDONE WHEN: the combined result is runnable/usable as a single deliverable and you've confirmed the pieces are actually connected.",
            body
        ),
        _ => body.to_string(),
    }
}

fn parse_planned_subtasks(reply: &str) -> Option<Vec<PlannedSubtask>> {
    let trimmed = reply.trim();
    let fenced = trimmed
        .strip_prefix("```json")
        .and_then(|value| value.strip_suffix("```"))
        .map(str::trim)
        .unwrap_or(trimmed);
    let candidates = [trimmed, fenced];

    for candidate in candidates {
        if let Some(parsed) = parse_planned_subtasks_candidate(candidate) {
            return Some(parsed);
        }
        // 对象形式 {"subtasks":[...]} 兜底:取 subtasks 字段再解析。
        if let Ok(Value::Object(map)) = serde_json::from_str::<Value>(candidate) {
            if let Some(arr) = map.get("subtasks") {
                let arr_str = serde_json::to_string(arr).unwrap_or_default();
                if let Some(parsed) = parse_planned_subtasks_candidate(&arr_str) {
                    return Some(parsed);
                }
            }
        }
        if let Some(array) = extract_json_array(candidate) {
            if let Some(parsed) = parse_planned_subtasks_candidate(&array) {
                return Some(parsed);
            }
        }
    }

    None
}

fn parse_planned_subtasks_candidate(candidate: &str) -> Option<Vec<PlannedSubtask>> {
    let items = serde_json::from_str::<Vec<PlannedSubtask>>(candidate).ok()?;
    let planned = items
        .into_iter()
        .filter_map(|item| {
            let title = item.title.trim().to_string();
            let description = item.description.trim().to_string();
            if title.is_empty() || description.is_empty() {
                None
            } else {
                Some(PlannedSubtask { title, description })
            }
        })
        .take(4)
        .collect::<Vec<_>>();
    if planned.len() > 1 {
        Some(planned)
    } else {
        None
    }
}

fn extract_json_array(reply: &str) -> Option<String> {
    let start = reply.find('[')?;
    let end = reply.rfind(']')?;
    if end < start {
        return None;
    }
    Some(reply[start..=end].to_string())
}

fn initialize_plan_fanout(
    board_id: &str,
    parent: &TaskRecord,
    handoff: &HandoffRecord,
    subtasks: &[PlannedSubtask],
) -> Result<(), String> {
    let mut child_ids = Vec::with_capacity(subtasks.len());

    for (index, subtask) in subtasks.iter().enumerate() {
        let child_id = format!("{}-exec-{}", parent.task_id.trim(), index + 1);
        let task_payload = json!({
            "board_id": board_id,
            "task_id": child_id,
            "title": subtask.title,
            "description": subtask.description,
            "kind": parent.kind,
            "status": "in_progress",
            "owner_member_id": "",
            "depends_on": [],
            "artifact_refs": [],
            "review_state": "",
            "metadata": {
                "phase": "execute",
                "fanout_child": true,
                "parent_task_id": parent.task_id,
                "source_handoff_id": handoff.handoff_id,
            },
        });
        call_package_data(TASK_BOARD_PACKAGE, "save_task", &task_payload)?;

        let handoff_id = auto_handoff_id(&child_id, "execute");
        let handoff_payload = json!({
            "board_id": board_id,
            "handoff_id": handoff_id,
            "task_id": child_id,
            "from_member_id": handoff.to_member_id,
            "to_member_id": "implementer",
            "reason": format!("planner fan-out from {}", parent.task_id),
            "expected_outcome": "Complete this implementation subtask",
            "context_snapshot_ref": "",
            "metadata": {
                "phase": "execute",
                "fanout_child": true,
                "parent_task_id": parent.task_id,
            }
        });
        call_package_data(TASK_BOARD_PACKAGE, "create_handoff", &handoff_payload)?;
        child_ids.push(child_id);
    }

    let mut metadata = metadata_object(&parent.metadata);
    metadata.insert("phase".to_string(), Value::String("execute".to_string()));
    metadata.insert("fanout_parent".to_string(), Value::Bool(true));
    metadata.insert(
        "fanout_children".to_string(),
        Value::Array(
            child_ids
                .iter()
                .cloned()
                .map(Value::String)
                .collect::<Vec<_>>(),
        ),
    );
    let parent_payload = json!({
        "board_id": parent.board_id,
        "task_id": parent.task_id,
        "title": parent.title,
        "description": parent.description,
        "kind": parent.kind,
        "status": "blocked",
        "owner_member_id": parent.owner_member_id,
        "depends_on": child_ids,
        "artifact_refs": parent.artifact_refs,
        "review_state": parent.review_state,
        "metadata": Value::Object(metadata),
    });
    call_package_data(TASK_BOARD_PACKAGE, "save_task", &parent_payload)?;

    log_info(&format!(
        "fan-out created {} execute children for parent task '{}' on board '{}'",
        subtasks.len(),
        parent.task_id,
        board_id
    ));
    Ok(())
}

fn dependencies_satisfied(task: &TaskRecord, tasks: &[TaskRecord]) -> bool {
    if task.depends_on.is_empty() {
        return true;
    }

    task.depends_on.iter().all(|dependency_id| {
        tasks.iter()
            .find(|candidate| candidate.task_id == *dependency_id)
            .map(task_is_dependency_complete)
            .unwrap_or(false)
    })
}

/// P3 纯逻辑: 统计 fan-out 父任务的子任务完成情况,返回 (已完成数, 子任务总数, 未完成的子任务id)。
/// "完成"判据: 子任务有 artifact_refs,或已离开 in_progress/blocked(达到 execute 完成态
/// review_requested 及之后),即真正被处理过。artifact_refs 当前未必回填,故以状态为主信号。
/// 抽成纯函数(无 host 依赖)以便单测。
fn fanout_completeness_stats<'a>(
    parent_id: &str,
    tasks: &'a [TaskRecord],
) -> (usize, usize, Vec<&'a str>) {
    fn child_done(c: &TaskRecord) -> bool {
        if !c.artifact_refs.is_empty() {
            return true;
        }
        // execute 未完成的子任务停在 in_progress/blocked;完成后状态前进到 review_requested 等。
        !matches!(c.status.as_str(), "" | "in_progress" | "blocked" | "new" | "captured")
    }
    let children: Vec<&TaskRecord> = tasks
        .iter()
        .filter(|t| {
            t.metadata
                .get("parent_task_id")
                .and_then(Value::as_str)
                .map(|p| p == parent_id)
                .unwrap_or(false)
        })
        .collect();
    let produced = children.iter().filter(|c| child_done(c)).count();
    let empty: Vec<&str> = children
        .iter()
        .filter(|c| !child_done(c))
        .map(|c| c.task_id.as_str())
        .collect();
    (produced, children.len(), empty)
}

/// P3 completeness check: 当 fan-out 父任务进入 review/integrate 前,统计哪些子任务
/// 没有产出 artifact(空手而归)。把缺口写进日志并告警,避免"静默漏做"被当成完成。
/// 返回有产出的子任务数 / 总子任务数。
fn check_fanout_completeness(board_id: &str, parent: &TaskRecord, tasks: &[TaskRecord]) -> (usize, usize) {
    let (produced, total, empty) = fanout_completeness_stats(&parent.task_id, tasks);
    if total == 0 {
        return (0, 0);
    }
    if !empty.is_empty() {
        log_warn(&format!(
            "fan-out completeness: {}/{} subtasks produced artifacts on board '{}'; EMPTY subtasks (no artifacts): {}",
            produced,
            total,
            board_id,
            empty.join(", ")
        ));
    } else {
        log_info(&format!(
            "fan-out completeness: all {}/{} subtasks produced artifacts on board '{}'",
            produced,
            total,
            board_id
        ));
    }
    (produced, total)
}

fn task_is_dependency_complete(task: &TaskRecord) -> bool {
    let phase_id = task_phase(task);
    let Ok(phase) = get_phase(&phase_id) else {
        return false;
    };
    if phase.terminal {
        return true;
    }
    forward_exit_status(&phase_id)
        .map(|status| status == task.status)
        .unwrap_or(false)
}

fn phase_entry_status(phase: &PhaseDefinition) -> Option<String> {
    phase.entry_statuses.first().cloned()
}

fn is_fanout_parent(task: &TaskRecord) -> bool {
    task.metadata
        .get("fanout_parent")
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn is_fanout_child(task: &TaskRecord) -> bool {
    task.metadata
        .get("fanout_child")
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn auto_handoff_id(task_id: &str, next_phase: &str) -> String {
    format!("auto-{}-{}", task_id.trim(), next_phase.trim())
}

fn metadata_object(value: &Value) -> Map<String, Value> {
    match value {
        Value::Object(map) => map.clone(),
        _ => Map::new(),
    }
}

fn parse_json_or_default<T>(raw: Option<String>) -> T
where
    T: for<'de> Deserialize<'de> + Default,
{
    raw.and_then(|json| serde_json::from_str(&json).ok())
        .unwrap_or_default()
}

fn call_package_data(package: &str, action: &str, payload: &Value) -> Result<Value, String> {
    let raw = call_package_ws_action(package, action, payload).map_err(|error| {
        format!(
            "package '{}' action '{}' host call failed: {}",
            package, action, error
        )
    })?;
    parse_package_data(&raw, package, action)
}

fn parse_package_data(raw: &str, package: &str, action: &str) -> Result<Value, String> {
    let envelope: PackageEnvelope =
        serde_json::from_str(raw).map_err(|error| format!("invalid package result: {}", error))?;
    if envelope.status != "ok" {
        return Err(
            envelope
                .error
                .unwrap_or_else(|| format!("package '{}' action '{}' failed", package, action)),
        );
    }
    envelope.data.ok_or_else(|| {
        format!(
            "package '{}' action '{}' returned ok without data",
            package, action
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn child(task_id: &str, parent: &str, artifacts: &[&str]) -> TaskRecord {
        TaskRecord {
            task_id: task_id.to_string(),
            artifact_refs: artifacts.iter().map(|s| s.to_string()).collect(),
            metadata: json!({ "parent_task_id": parent }),
            ..Default::default()
        }
    }

    #[test]
    fn fanout_completeness_all_produced() {
        let tasks = vec![
            child("t-exec-1", "t", &["a.html"]),
            child("t-exec-2", "t", &["b.html"]),
        ];
        let (produced, total, empty) = fanout_completeness_stats("t", &tasks);
        assert_eq!(produced, 2);
        assert_eq!(total, 2);
        assert!(empty.is_empty());
    }

    #[test]
    fn fanout_completeness_detects_empty_subtasks() {
        let tasks = vec![
            child("t-exec-1", "t", &["a.html"]),
            child("t-exec-2", "t", &[]), // 空手而归
            child("t-exec-3", "t", &[]), // 空手而归
        ];
        let (produced, total, empty) = fanout_completeness_stats("t", &tasks);
        assert_eq!(produced, 1);
        assert_eq!(total, 3);
        assert_eq!(empty, vec!["t-exec-2", "t-exec-3"]);
    }

    #[test]
    fn fanout_completeness_ignores_other_parents() {
        let tasks = vec![
            child("t-exec-1", "t", &["a.html"]),
            child("x-exec-1", "x", &[]), // 别的父任务的子任务,不计入
        ];
        let (produced, total, _empty) = fanout_completeness_stats("t", &tasks);
        assert_eq!(produced, 1);
        assert_eq!(total, 1);
    }

    fn child_with_status(task_id: &str, parent: &str, status: &str) -> TaskRecord {
        TaskRecord {
            task_id: task_id.to_string(),
            status: status.to_string(),
            metadata: json!({ "parent_task_id": parent }),
            ..Default::default()
        }
    }

    #[test]
    fn fanout_completeness_uses_status_when_no_artifacts() {
        // 无 artifact_refs(未回填),靠状态判断:review_requested=已完成 execute,in_progress=没做完。
        let tasks = vec![
            child_with_status("t-exec-1", "t", "review_requested"), // 完成
            child_with_status("t-exec-2", "t", "in_progress"),      // 没做完
        ];
        let (produced, total, empty) = fanout_completeness_stats("t", &tasks);
        assert_eq!(produced, 1);
        assert_eq!(total, 2);
        assert_eq!(empty, vec!["t-exec-2"]);
    }
}
