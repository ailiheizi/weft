//! Skills package - skill registry, discovery, execution with built-in tools.

use base64::Engine;
use weft_package_sdk::*;
use serde::{Deserialize, Serialize};

#[cfg(test)]
#[allow(improper_ctypes_definitions)]
mod test_extism_host_stubs {
    #[no_mangle]
    pub extern "C" fn error_set(_: u64) {}

    #[no_mangle]
    pub extern "C" fn input_length() -> u64 {
        0
    }

    #[no_mangle]
    pub extern "C" fn input_load_u64(_: u64) -> u64 {
        0
    }

    #[no_mangle]
    pub extern "C" fn input_load_u8(_: u64) -> u8 {
        0
    }

    #[no_mangle]
    pub extern "C" fn load_u8(_: u64) -> u8 {
        0
    }

    #[no_mangle]
    pub extern "C" fn load_u64(_: u64) -> u64 {
        0
    }

    #[no_mangle]
    pub extern "C" fn store_u8(_: u64, _: u8) {}

    #[no_mangle]
    pub extern "C" fn store_u64(_: u64, _: u64) {}

    #[no_mangle]
    pub extern "C" fn output_set(_: u64, _: u64) {}

    #[no_mangle]
    pub extern "C" fn log_trace(_: u64) {}

    #[no_mangle]
    pub extern "C" fn log_debug(_: u64) {}

    #[no_mangle]
    pub extern "C" fn log_info(_: u64) {}

    #[no_mangle]
    pub extern "C" fn log_warn(_: u64) {}

    #[no_mangle]
    pub extern "C" fn log_error(_: u64) {}

    #[no_mangle]
    pub extern "C" fn alloc(_: u64) -> u64 {
        0
    }

    #[no_mangle]
    pub extern "C" fn length(_: u64) -> u64 {
        0
    }

    #[no_mangle]
    pub extern "C" fn length_unsafe(_: u64) -> u64 {
        0
    }

    #[no_mangle]
    pub extern "C" fn host_log(_: String) -> String {
        String::new()
    }

    #[no_mangle]
    pub extern "C" fn host_kv_get(_: String) -> String {
        String::new()
    }

    #[no_mangle]
    pub extern "C" fn host_kv_set(_: String) {}

    #[no_mangle]
    pub extern "C" fn host_kv_list(_: String) -> String {
        String::new()
    }

    #[no_mangle]
    pub extern "C" fn host_kv_delete(_: String) {}

    #[no_mangle]
    pub extern "C" fn host_env_get(_: String) -> String {
        String::new()
    }

    #[no_mangle]
    pub extern "C" fn host_read_file(_: String) -> String {
        String::new()
    }

    #[no_mangle]
    pub extern "C" fn host_write_file(_: String) {}

    #[no_mangle]
    pub extern "C" fn host_list_dir(_: String) -> String {
        String::new()
    }

    #[no_mangle]
    pub extern "C" fn host_exec(_: String) -> String {
        String::new()
    }

    #[no_mangle]
    pub extern "C" fn host_exec_advanced(_: String) -> String {
        String::new()
    }

    #[no_mangle]
    pub extern "C" fn host_chat_completion(_: String) -> String {
        String::new()
    }

    #[no_mangle]
    pub extern "C" fn host_call_package(_: String) -> String {
        String::new()
    }

    #[no_mangle]
    pub extern "C" fn host_call_package_ws(_: String) -> String {
        String::new()
    }

    #[no_mangle]
    pub extern "C" fn host_process_spawn(_: String) -> String {
        String::new()
    }

    #[no_mangle]
    pub extern "C" fn host_process_stop(_: String) -> String {
        String::new()
    }

    #[no_mangle]
    pub extern "C" fn host_process_status(_: String) -> String {
        String::new()
    }

    #[no_mangle]
    pub extern "C" fn host_process_write_stdin(_: String) -> String {
        String::new()
    }

    #[no_mangle]
    pub extern "C" fn host_process_read_stdout(_: String) -> String {
        String::new()
    }

    #[no_mangle]
    pub extern "C" fn host_sqlite_query(_: String) -> String {
        String::new()
    }

    #[no_mangle]
    pub extern "C" fn host_sqlite_execute(_: String) -> String {
        String::new()
    }

    #[no_mangle]
    pub extern "C" fn host_sqlite_batch(_: String) -> String {
        String::new()
    }
}

#[derive(Serialize, Deserialize, Clone)]
struct SkillDef {
    name: String,
    description: String,
    parameters: serde_json::Value,
}

#[derive(Serialize, Deserialize, Clone)]
struct ExternalToolDef {
    server: String,
    name: String,
    #[serde(default)]
    description: String,
    #[serde(rename = "inputSchema", default)]
    input_schema: serde_json::Value,
}

#[derive(Serialize, Deserialize, Clone)]
struct SkillStep {
    title: String,
    instruction: String,
    #[serde(default)]
    tools: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone)]
struct VerificationStep {
    title: String,
    expected: String,
    #[serde(default)]
    required: bool,
}

#[derive(Serialize, Deserialize, Clone)]
struct SkillQualityCheck {
    name: String,
    passed: bool,
    reason: String,
}

#[derive(Serialize, Deserialize, Clone)]
struct SkillQualityReport {
    passed: bool,
    score: f64,
    checks: Vec<SkillQualityCheck>,
}

#[derive(Serialize, Deserialize, Clone)]
struct EvolvedSkill {
    id: String,
    title: String,
    description: String,
    #[serde(default = "empty_object")]
    metadata: serde_json::Value,
    #[serde(default)]
    triggers: Vec<String>,
    #[serde(default)]
    anti_triggers: Vec<String>,
    #[serde(default)]
    required_tools: Vec<String>,
    procedure: String,
    #[serde(default)]
    steps: Vec<SkillStep>,
    verification: String,
    #[serde(default)]
    verification_steps: Vec<VerificationStep>,
    version: u32,
    source: String,
    source_trajectory_id: String,
    created_at: u64,
    updated_at: u64,
    #[serde(default = "default_skill_status")]
    status: String,
    #[serde(default = "default_skill_risk")]
    risk_level: String,
    #[serde(default)]
    quality_score: f64,
    #[serde(default)]
    confidence: f64,
    #[serde(default)]
    review_required: bool,
    #[serde(default)]
    reviewed_at: u64,
    #[serde(default)]
    promoted_at: u64,
    #[serde(default)]
    archived_at: u64,
    #[serde(default)]
    last_used_at: u64,
    #[serde(default)]
    last_failure_at: u64,
    #[serde(default)]
    successful_uses: u64,
    #[serde(default)]
    failed_uses: u64,
    #[serde(default)]
    archived: bool,
}

#[derive(Serialize, Clone)]
struct ScoredEvolvedSkill {
    score: i64,
    skill: EvolvedSkill,
}

#[derive(Deserialize)]
struct RetrieveSkillsInput {
    agent: String,
    query: String,
    #[serde(default = "default_evolved_skill_limit")]
    limit: usize,
}

#[derive(Deserialize)]
struct CrystallizeTrajectoryInput {
    agent: String,
    trajectory_id: String,
    task: String,
    #[serde(default)]
    summary: String,
    #[serde(default)]
    steps: Vec<String>,
    #[serde(default)]
    tools: Vec<String>,
    final_result: String,
    #[serde(default)]
    success: bool,
    #[serde(default)]
    verification_passed: bool,
    #[serde(default)]
    verification_evidence: String,
    #[serde(default)]
    auto_activate: bool,
    #[serde(default = "default_skill_risk")]
    risk_level: String,
}

#[derive(Deserialize)]
struct SkillUsageInput {
    agent: String,
    skill_id: String,
    #[serde(default)]
    success: bool,
    #[serde(default)]
    note: String,
}

#[derive(Deserialize)]
struct ReviewEvolvedInput {
    agent: String,
    skill_id: String,
    #[serde(default)]
    approve: bool,
    #[serde(default)]
    notes: String,
}

#[derive(Deserialize)]
struct PatchSuggestionInput {
    agent: String,
    skill_id: String,
    trajectory_id: String,
    reason: String,
    #[serde(default)]
    patch: String,
}

const EVOLVED_SKILLS_INDEX_KEY: &str = "skills:evolved:__index";

fn empty_object() -> serde_json::Value {
    serde_json::json!({})
}

fn default_skill_status() -> String {
    "active".into()
}

fn default_skill_risk() -> String {
    "low".into()
}

fn default_evolved_skill_limit() -> usize {
    3
}

fn evolved_skill_key(agent: &str, id: &str) -> String {
    format!("skills:evolved:{}:{}", agent.trim(), id.trim())
}

fn evolved_skill_usage_key(agent: &str, id: &str) -> String {
    format!("skills:evolved_usage:{}:{}", agent.trim(), id.trim())
}

fn load_skill_usage_history(agent: &str, id: &str) -> Vec<serde_json::Value> {
    kv_get(&evolved_skill_usage_key(agent, id))
        .and_then(|json| serde_json::from_str::<Vec<serde_json::Value>>(&json).ok())
        .unwrap_or_default()
}

fn save_skill_usage_history(agent: &str, id: &str, history: &[serde_json::Value]) {
    let json = serde_json::to_string(history).unwrap_or_else(|_| "[]".into());
    kv_set(&evolved_skill_usage_key(agent, id), &json);
}

fn patch_suggestion_key(agent: &str, trajectory_id: &str, skill_id: &str) -> String {
    format!(
        "skills:patch_suggestion:{}:{}:{}",
        agent.trim(),
        trajectory_id.trim(),
        skill_id.trim()
    )
}

fn get_evolved_skill_index() -> Vec<(String, String)> {
    kv_get(EVOLVED_SKILLS_INDEX_KEY)
        .and_then(|json| serde_json::from_str::<Vec<(String, String)>>(&json).ok())
        .unwrap_or_default()
}

fn save_evolved_skill_index(index: &[(String, String)]) {
    let json = serde_json::to_string(index).unwrap_or_else(|_| "[]".into());
    kv_set(EVOLVED_SKILLS_INDEX_KEY, &json);
}

fn ensure_evolved_skill_index(agent: &str, id: &str) {
    let mut index = get_evolved_skill_index();
    let entry = (agent.trim().to_string(), id.trim().to_string());
    if !index.iter().any(|item| item == &entry) {
        index.push(entry);
        save_evolved_skill_index(&index);
    }
}

fn save_evolved_skill(agent: &str, skill: &EvolvedSkill) {
    let json = serde_json::to_string(skill).unwrap_or_default();
    kv_set(&evolved_skill_key(agent, &skill.id), &json);
    ensure_evolved_skill_index(agent, &skill.id);
}

fn load_evolved_skill(agent: &str, id: &str) -> Option<EvolvedSkill> {
    kv_get(&evolved_skill_key(agent, id)).and_then(|json| serde_json::from_str(&json).ok())
}

fn skill_is_retrievable(skill: &EvolvedSkill) -> bool {
    !skill.archived && matches!(skill.status.as_str(), "active" | "promoted")
}

fn list_evolved_skills_for_agent(agent: &str) -> Vec<EvolvedSkill> {
    get_evolved_skill_index()
        .into_iter()
        .filter(|(entry_agent, _)| entry_agent == agent.trim())
        .filter_map(|(_, id)| load_evolved_skill(agent, &id))
        .filter(skill_is_retrievable)
        .collect()
}
fn trim_skill_usage_history(mut history: Vec<serde_json::Value>) -> Vec<serde_json::Value> {
    if history.len() > 100 {
        history = history[history.len() - 100..].to_vec();
    }
    history
}

fn scored_evolved_skills_for_query(
    skills: Vec<EvolvedSkill>,
    query: &str,
    limit: usize,
) -> Vec<ScoredEvolvedSkill> {
    let mut scored = skills
        .into_iter()
        .filter_map(|skill| {
            let score = score_evolved_skill(&skill, query);
            if score > 0 {
                Some((score, skill))
            } else {
                None
            }
        })
        .collect::<Vec<(i64, EvolvedSkill)>>();
    scored.sort_by(|left, right| {
        right
            .0
            .cmp(&left.0)
            .then_with(|| right.1.updated_at.cmp(&left.1.updated_at))
            .then_with(|| left.1.id.cmp(&right.1.id))
    });
    scored.truncate(limit);
    scored
        .into_iter()
        .map(|(score, skill)| ScoredEvolvedSkill { score, skill })
        .collect()
}

fn patch_suggestion_validation_error(input: &PatchSuggestionInput) -> Option<&'static str> {
    if input.agent.trim().is_empty()
        || input.skill_id.trim().is_empty()
        || input.trajectory_id.trim().is_empty()
        || input.reason.trim().is_empty()
        || input.patch.trim().is_empty()
    {
        Some("missing agent, skill_id, trajectory_id, reason, or patch")
    } else {
        None
    }
}

fn patch_suggestion_unknown_skill_error(skill_id: &str) -> String {
    format!("unknown evolved skill: {}", skill_id)
}

fn slugify_skill_id(value: &str) -> String {
    let mut out = String::new();
    for ch in value.chars().flat_map(char::to_lowercase) {
        if ch.is_ascii_alphanumeric() {
            out.push(ch);
        } else if (ch.is_whitespace() || ch == '-' || ch == '_') && !out.ends_with('-') {
            out.push('-');
        }
        if out.len() >= 48 {
            break;
        }
    }
    let trimmed = out.trim_matches('-').to_string();
    if trimmed.is_empty() {
        format!("skill-{}", now_ms())
    } else {
        trimmed
    }
}

fn skill_text(skill: &EvolvedSkill) -> String {
    format!(
        "{}\n{}\n{}\n{}\n{}",
        skill.title,
        skill.description,
        skill.triggers.join(" "),
        skill.procedure,
        skill.verification
    )
    .to_lowercase()
}

fn score_evolved_skill(skill: &EvolvedSkill, query: &str) -> i64 {
    if !skill_is_retrievable(skill) {
        return 0;
    }
    let haystack = skill_text(skill);
    let query_lower = query.to_lowercase();
    let mut score = 0_i64;
    if !query_lower.trim().is_empty() && haystack.contains(query_lower.trim()) {
        score += 100;
    }
    for token in query_lower.split_whitespace().filter(|item| item.len() > 1) {
        if haystack.contains(token) {
            score += 10;
        }
    }
    for anti_trigger in &skill.anti_triggers {
        let anti_trigger = anti_trigger.to_lowercase();
        if !anti_trigger.trim().is_empty() && query_lower.contains(anti_trigger.trim()) {
            return 0;
        }
    }
    for trigger in &skill.triggers {
        let trigger = trigger.to_lowercase();
        if !trigger.trim().is_empty() && query_lower.contains(trigger.trim()) {
            score += 25;
        }
    }
    if !skill.required_tools.is_empty()
        && skill
            .required_tools
            .iter()
            .any(|tool| query_lower.contains(&tool.to_lowercase()))
    {
        score += 15;
    }
    let total_uses = skill.successful_uses.saturating_add(skill.failed_uses);
    if total_uses > 0 {
        let success_bonus =
            ((skill.successful_uses as f64 / total_uses as f64) * 20.0).round() as i64;
        score += success_bonus;
        score -= (skill.failed_uses.min(10) as i64) * 2;
    }
    score += (skill.quality_score * 10.0).round() as i64;
    score
}

fn evolved_skill_context_block(skills: &[EvolvedSkill]) -> String {
    skills
        .iter()
        .map(|skill| {
            format!(
                "Skill: {} (id={}, v{})\nWhen to use: {}\nProcedure:\n{}\nVerify by: {}",
                skill.title,
                skill.id,
                skill.version,
                if skill.triggers.is_empty() {
                    skill.description.clone()
                } else {
                    skill.triggers.join(", ")
                },
                skill.procedure,
                skill.verification
            )
        })
        .collect::<Vec<String>>()
        .join("\n\n")
}

fn do_retrieve_applicable(input: RetrieveSkillsInput) -> PackageResult {
    if input.agent.trim().is_empty() {
        return PackageResult::err("missing agent");
    }
    let scored_skills = scored_evolved_skills_for_query(
        list_evolved_skills_for_agent(&input.agent),
        &input.query,
        input.limit,
    );
    let skills = scored_skills
        .iter()
        .map(|entry| entry.skill.clone())
        .collect::<Vec<_>>();
    // evolved 技能块 + 文件式技能块（按 query 命中）合并注入。
    let mut context_block = evolved_skill_context_block(&skills);
    let file_block = applicable_file_skills_block(&input.query);
    if !file_block.is_empty() {
        if context_block.is_empty() {
            context_block = file_block;
        } else {
            context_block.push_str("\n\n");
            context_block.push_str(&file_block);
        }
    }
    PackageResult::ok(serde_json::json!({
        "skills": scored_skills,
        "context_block": context_block,
    }))
}

fn do_list_evolved(agent: &str) -> PackageResult {
    if agent.trim().is_empty() {
        return PackageResult::err("missing agent");
    }
    PackageResult::ok(serde_json::json!({ "skills": list_evolved_skills_for_agent(agent) }))
}

fn skill_success_rate(skill: &EvolvedSkill) -> f64 {
    let total = skill.successful_uses.saturating_add(skill.failed_uses);
    if total == 0 {
        return 0.0;
    }
    skill.successful_uses as f64 / total as f64
}

fn validate_evolved_skill(skill: &EvolvedSkill) -> Vec<String> {
    let mut issues = Vec::new();
    if skill.title.trim().len() < 4 {
        issues.push("title too short".into());
    }
    if skill.procedure.trim().len() < 12 && skill.steps.is_empty() {
        issues.push("procedure or structured steps required".into());
    }
    if skill.verification.trim().len() < 8 && skill.verification_steps.is_empty() {
        issues.push("verification required".into());
    }
    if skill.triggers.is_empty() {
        issues.push("at least one trigger required".into());
    }
    issues
}

fn maintenance_skill_metrics(skill: &EvolvedSkill) -> serde_json::Value {
    serde_json::json!({
        "success_rate": skill_success_rate(skill),
        "successful_uses": skill.successful_uses,
        "failed_uses": skill.failed_uses,
        "quality_score": skill.quality_score,
        "confidence": skill.confidence,
        "last_used_at": skill.last_used_at,
        "last_failure_at": skill.last_failure_at,
    })
}

fn should_promote_skill(skill: &EvolvedSkill, issues: &[String]) -> bool {
    skill.status == "pending_review"
        && issues.is_empty()
        && skill.quality_score >= 0.8
        && skill.risk_level != "high"
        && (skill.successful_uses >= 2 || skill.confidence >= 0.9)
}

fn should_archive_skill(skill: &EvolvedSkill, issues: &[String]) -> bool {
    let total = skill.successful_uses.saturating_add(skill.failed_uses);
    !skill.archived
        && (issues.len() >= 3
            || (total >= 5 && skill_success_rate(skill) < 0.35)
            || skill.status == "rejected")
}

fn do_maintenance() -> PackageResult {
    let now = now_ms();
    let mut checked = 0_u64;
    let mut active = 0_u64;
    let mut pending_review = 0_u64;
    let mut promoted = 0_u64;
    let mut archived = 0_u64;
    let mut invalid = 0_u64;
    let mut reports = Vec::new();

    for (agent, id) in get_evolved_skill_index() {
        let Some(mut skill) = load_evolved_skill(&agent, &id) else {
            invalid = invalid.saturating_add(1);
            reports.push(serde_json::json!({"agent": agent, "skill_id": id, "status": "missing"}));
            continue;
        };
        checked = checked.saturating_add(1);
        let issues = validate_evolved_skill(&skill);
        if !issues.is_empty() {
            invalid = invalid.saturating_add(1);
        }
        if should_promote_skill(&skill, &issues) {
            skill.status = "active".into();
            skill.review_required = false;
            skill.reviewed_at = now;
            skill.promoted_at = now;
            skill.updated_at = now;
            promoted = promoted.saturating_add(1);
            save_evolved_skill(&agent, &skill);
        }
        if should_archive_skill(&skill, &issues) {
            skill.status = "archived".into();
            skill.archived = true;
            skill.archived_at = now;
            skill.updated_at = now;
            archived = archived.saturating_add(1);
            save_evolved_skill(&agent, &skill);
        }
        if skill_is_retrievable(&skill) {
            active = active.saturating_add(1);
        }
        if skill.status == "pending_review" {
            pending_review = pending_review.saturating_add(1);
        }
        reports.push(serde_json::json!({
            "agent": agent,
            "skill_id": skill.id,
            "status": skill.status,
            "issues": issues,
            "metrics": maintenance_skill_metrics(&skill),
        }));
    }

    PackageResult::ok(serde_json::json!({
        "checked": checked,
        "active_evolved_skills": active,
        "pending_review": pending_review,
        "promoted": promoted,
        "archived": archived,
        "invalid": invalid,
        "reports": reports,
    }))
}

fn quality_check(name: &str, passed: bool, reason: impl Into<String>) -> SkillQualityCheck {
    SkillQualityCheck {
        name: name.into(),
        passed,
        reason: reason.into(),
    }
}

fn evaluate_skill_quality(input: &CrystallizeTrajectoryInput) -> SkillQualityReport {
    let checks = vec![
        quality_check(
            "successful_trajectory",
            input.success,
            "trajectory must be marked successful",
        ),
        quality_check(
            "has_reusable_steps",
            !input.steps.is_empty() || !input.tools.is_empty(),
            "skill needs reusable steps or tools",
        ),
        quality_check(
            "has_final_result",
            !input.final_result.trim().is_empty(),
            "final result must be present",
        ),
        quality_check(
            "verification_evidence",
            input.verification_passed && !input.verification_evidence.trim().is_empty(),
            "verification must pass with evidence",
        ),
        quality_check(
            "acceptable_risk",
            matches!(input.risk_level.as_str(), "low" | "medium"),
            "high risk skills require manual review",
        ),
    ];
    let passed_count = checks.iter().filter(|check| check.passed).count() as f64;
    let score = if checks.is_empty() {
        0.0
    } else {
        passed_count / checks.len() as f64
    };
    let passed = checks.iter().all(|check| check.passed);
    SkillQualityReport {
        passed,
        score,
        checks,
    }
}

fn skill_steps_from_input(input: &CrystallizeTrajectoryInput) -> Vec<SkillStep> {
    if input.steps.is_empty() {
        return vec![SkillStep {
            title: "Reuse trajectory approach".into(),
            instruction: format!(
                "Reuse the successful approach from trajectory {} and adapt it to the new request.",
                input.trajectory_id
            ),
            tools: input.tools.clone(),
        }];
    }
    input
        .steps
        .iter()
        .enumerate()
        .map(|(index, step)| SkillStep {
            title: format!("Step {}", index + 1),
            instruction: step.trim().to_string(),
            tools: input.tools.clone(),
        })
        .collect()
}

fn verification_steps_from_input(input: &CrystallizeTrajectoryInput) -> Vec<VerificationStep> {
    vec![VerificationStep {
        title: "Verify reusable outcome".into(),
        expected: if input.verification_evidence.trim().is_empty() {
            "Confirm the result is grounded in the successful trajectory and satisfies the new request.".into()
        } else {
            input.verification_evidence.trim().to_string()
        },
        required: true,
    }]
}

fn build_skill_from_trajectory(input: &CrystallizeTrajectoryInput) -> EvolvedSkill {
    let now = now_ms();
    let id = slugify_skill_id(&format!("{}-{}", input.task, input.trajectory_id));
    let triggers = input
        .task
        .split(|ch: char| ch.is_whitespace() || ch == ',' || ch == ';')
        .map(str::trim)
        .filter(|item| item.chars().count() > 1)
        .take(8)
        .map(ToString::to_string)
        .collect::<Vec<String>>();
    let mut procedure = Vec::new();
    if !input.steps.is_empty() {
        for (index, step) in input.steps.iter().enumerate() {
            procedure.push(format!("{}. {}", index + 1, step.trim()));
        }
    } else {
        procedure.push(format!(
            "1. Reuse the successful approach from trajectory {}.",
            input.trajectory_id
        ));
        if !input.tools.is_empty() {
            procedure.push(format!(
                "2. Prefer these tools when relevant: {}.",
                input.tools.join(", ")
            ));
        }
        procedure.push(
            "3. Produce a concise user-facing result and verify it against the request.".into(),
        );
    }

    let quality = evaluate_skill_quality(input);
    let status = if input.auto_activate && quality.passed && input.risk_level != "high" {
        "active"
    } else {
        "pending_review"
    };
    let steps = skill_steps_from_input(input);
    let verification_steps = verification_steps_from_input(input);

    EvolvedSkill {
        id,
        title: if input.summary.trim().is_empty() {
            format!("Handle similar task: {}", input.task.trim())
        } else {
            input.summary.trim().to_string()
        },
        description: format!(
            "Self-evolved from successful trajectory {}.",
            input.trajectory_id
        ),
        metadata: serde_json::json!({
            "task": input.task.trim(),
            "summary": input.summary.trim(),
            "tools": input.tools,
            "quality_report": quality,
            "final_result": input.final_result.trim(),
        }),
        triggers,
        anti_triggers: Vec::new(),
        required_tools: input.tools.clone(),
        procedure: procedure.join("\n"),
        steps,
        verification: if input.final_result.trim().is_empty() {
            "Confirm that the final response directly satisfies the task and that required tool outputs were used.".into()
        } else {
            "Confirm that the final response is grounded in the recorded successful trajectory and remains relevant to the new request.".into()
        },
        verification_steps,
        version: 1,
        source: "trajectory".into(),
        source_trajectory_id: input.trajectory_id.clone(),
        created_at: now,
        updated_at: now,
        status: status.into(),
        risk_level: input.risk_level.clone(),
        quality_score: quality.score,
        confidence: quality.score,
        review_required: status == "pending_review",
        reviewed_at: 0,
        promoted_at: if status == "active" { now } else { 0 },
        archived_at: 0,
        last_used_at: 0,
        last_failure_at: 0,
        successful_uses: 0,
        failed_uses: 0,
        archived: false,
    }
}

fn do_crystallize_from_trajectory(input: CrystallizeTrajectoryInput) -> PackageResult {
    if input.agent.trim().is_empty()
        || input.trajectory_id.trim().is_empty()
        || input.task.trim().is_empty()
    {
        return PackageResult::err("missing agent, trajectory_id, or task");
    }
    if !input.success {
        return PackageResult::err("only successful trajectories can be crystallized into skills");
    }
    let mut skill = build_skill_from_trajectory(&input);
    if let Some(existing) = load_evolved_skill(&input.agent, &skill.id) {
        skill.created_at = existing.created_at;
        skill.version = existing.version.saturating_add(1);
        skill.successful_uses = existing.successful_uses;
        skill.failed_uses = existing.failed_uses;
    }
    save_evolved_skill(&input.agent, &skill);
    PackageResult::ok(serde_json::json!({ "skill": skill }))
}

fn do_record_skill_usage(input: SkillUsageInput) -> PackageResult {
    if input.agent.trim().is_empty() || input.skill_id.trim().is_empty() {
        return PackageResult::err("missing agent or skill_id");
    }
    let mut skill = match load_evolved_skill(&input.agent, &input.skill_id) {
        Some(skill) => skill,
        None => return PackageResult::err(format!("unknown evolved skill: {}", input.skill_id)),
    };
    let now = now_ms();
    if input.success {
        skill.successful_uses = skill.successful_uses.saturating_add(1);
    } else {
        skill.failed_uses = skill.failed_uses.saturating_add(1);
        skill.last_failure_at = now;
    }
    skill.last_used_at = now;
    skill.updated_at = now;
    let total_uses = skill.successful_uses.saturating_add(skill.failed_uses);
    if total_uses > 0 {
        skill.confidence = skill_success_rate(&skill);
    }
    save_evolved_skill(&input.agent, &skill);

    let usage = serde_json::json!({
        "agent": input.agent.trim(),
        "skill_id": input.skill_id.trim(),
        "success": input.success,
        "note": input.note,
        "timestamp": skill.updated_at,
    });
    let mut history = load_skill_usage_history(&input.agent, &input.skill_id);
    history.push(usage.clone());
    let history = trim_skill_usage_history(history);
    save_skill_usage_history(&input.agent, &input.skill_id, &history);

    PackageResult::ok(serde_json::json!({ "skill": skill, "usage": usage }))
}

fn do_get_evolved(agent: &str, skill_id: &str) -> PackageResult {
    if agent.trim().is_empty() || skill_id.trim().is_empty() {
        return PackageResult::err("missing agent or skill_id");
    }
    match load_evolved_skill(agent, skill_id) {
        Some(skill) => PackageResult::ok(serde_json::json!({ "skill": skill })),
        None => PackageResult::err(format!("unknown evolved skill: {}", skill_id)),
    }
}

fn do_review_evolved(input: ReviewEvolvedInput) -> PackageResult {
    if input.agent.trim().is_empty() || input.skill_id.trim().is_empty() {
        return PackageResult::err("missing agent or skill_id");
    }
    let mut skill = match load_evolved_skill(&input.agent, &input.skill_id) {
        Some(skill) => skill,
        None => return PackageResult::err(format!("unknown evolved skill: {}", input.skill_id)),
    };
    let now = now_ms();
    skill.review_required = false;
    skill.reviewed_at = now;
    skill.updated_at = now;
    if input.approve {
        skill.status = "active".into();
        skill.archived = false;
        skill.promoted_at = now;
    } else {
        skill.status = "rejected".into();
        skill.archived = true;
        skill.archived_at = now;
    }
    skill.metadata["review"] = serde_json::json!({
        "approved": input.approve,
        "notes": input.notes.trim(),
        "timestamp": now,
    });
    save_evolved_skill(&input.agent, &skill);
    PackageResult::ok(serde_json::json!({ "skill": skill }))
}

fn do_record_patch_suggestion(input: PatchSuggestionInput) -> PackageResult {
    if let Some(error) = patch_suggestion_validation_error(&input) {
        return PackageResult::err(error);
    }
    if load_evolved_skill(&input.agent, &input.skill_id).is_none() {
        return PackageResult::err(patch_suggestion_unknown_skill_error(&input.skill_id));
    }
    let suggestion = serde_json::json!({
        "agent": input.agent.trim(),
        "skill_id": input.skill_id.trim(),
        "trajectory_id": input.trajectory_id.trim(),
        "reason": input.reason.trim(),
        "patch": input.patch,
        "timestamp": now_ms(),
        "status": "pending_review",
    });
    kv_set(
        &patch_suggestion_key(
            suggestion["agent"].as_str().unwrap_or(""),
            suggestion["trajectory_id"].as_str().unwrap_or(""),
            suggestion["skill_id"].as_str().unwrap_or(""),
        ),
        &suggestion.to_string(),
    );
    PackageResult::ok(serde_json::json!({ "suggestion": suggestion }))
}

fn external_tool_name(server: &str, tool: &str) -> String {
    format!("mcp::{}::{}", server.trim(), tool.trim())
}

fn parse_external_tool_name(value: &str) -> Option<(String, String)> {
    let parts: Vec<&str> = value.split("::").collect();
    if parts.len() != 3 || parts[0] != "mcp" {
        return None;
    }
    let server = parts[1].trim();
    let tool = parts[2].trim();
    if server.is_empty() || tool.is_empty() {
        return None;
    }
    Some((server.to_string(), tool.to_string()))
}

fn external_tool_specs(agent: &str) -> Vec<serde_json::Value> {
    let response = call_package(
        "mcp-client",
        "get_tools",
        &serde_json::json!({ "agent": agent }).to_string(),
    )
    .unwrap_or_default();
    let tools = serde_json::from_str::<serde_json::Value>(&response)
        .ok()
        .and_then(|value| value.get("data").cloned())
        .or_else(|| serde_json::from_str::<serde_json::Value>(&response).ok())
        .and_then(|value| {
            value.get("tools").cloned().or_else(|| {
                value
                    .get("data")
                    .and_then(|data| data.get("tools"))
                    .cloned()
            })
        })
        .and_then(|value| value.as_array().cloned())
        .unwrap_or_default();

    tools
        .into_iter()
        .filter_map(|tool| serde_json::from_value::<ExternalToolDef>(tool).ok())
        .map(|tool| {
            serde_json::json!({
                "type": "function",
                "function": {
                    "name": external_tool_name(&tool.server, &tool.name),
                    "description": if tool.description.trim().is_empty() {
                        format!("MCP tool '{}' exposed by server '{}'", tool.name, tool.server)
                    } else {
                        format!("[MCP:{}] {}", tool.server, tool.description)
                    },
                    "parameters": if tool.input_schema.is_null() {
                        serde_json::json!({"type": "object", "additionalProperties": true})
                    } else {
                        tool.input_schema
                    },
                }
            })
        })
        .collect()
}

fn builtin_skills() -> Vec<SkillDef> {
    vec![
        SkillDef {
            name: "fs_read".into(),
            description: "Read a regular file. Do not use for directories; use fs_list for directories.".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "Regular file path to read, not a directory"}
                },
                "required": ["path"]
            }),
        },
        SkillDef {
            name: "fs_write".into(),
            description: "Write content to a file. Prefer this over shell redirection for file creation or updates.".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "File path"},
                    "content": {"type": "string", "description": "Content to write"}
                },
                "required": ["path", "content"]
            }),
        },
        SkillDef {
            name: "fs_list".into(),
            description: "List directory contents. Use this for directories; do not use fs_read on directories.".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "Directory path"}
                },
                "required": ["path"]
            }),
        },
        SkillDef {
            name: "shell_exec".into(),
            description: "Execute a host command. Host OS is Windows. To run a Python script use command=\"python\" with args=[script_path, arg1, arg2, ...] and cwd=script_dir — do NOT invoke .py files directly via PowerShell. For other commands use command=\"pwsh\" with args=[\"-NoProfile\",\"-Command\",\"...\"] or command=\"cmd\" with args=[\"/C\",\"...\"]. Prefer fs_* for file operations and git for git commands.".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "command": {"type": "string", "description": "Command or executable to execute"},
                    "args": {"type": "array", "items": {"type": "string"}, "description": "Arguments. If omitted, command is treated as a platform shell script."},
                    "cwd": {"type": "string", "description": "Working directory for the command"},
                    "shell": {"type": "string", "enum": ["auto", "cmd", "powershell", "pwsh", "sh", "none"], "default": "auto"},
                    "timeout_ms": {"type": "integer", "description": "Timeout in milliseconds"}
                },
                "required": ["command"]
            }),
        },
        SkillDef {
            name: "git".into(),
            description: "Run the system git executable. Args exclude the git executable, for example [\"-C\", path, \"status\"].".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "args": {"type": "array", "items": {"type": "string"}, "description": "Git arguments, excluding git itself"}
                },
                "required": ["args"]
            }),
        },
        SkillDef {
            name: "web_fetch".into(),
            description: "Fetch a URL and return the HTTP response body. Use for retrieving web pages, REST API responses, or any HTTP resource. Returns status code and body content.".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "url": {"type": "string", "description": "URL to fetch"},
                    "method": {"type": "string", "description": "HTTP method", "default": "GET"},
                    "body": {"type": "string", "description": "Request body"}
                },
                "required": ["url"]
            }),
        },
        SkillDef {
            name: "web_search".into(),
            description: "Search the public web for current information and concise factual results. If the user explicitly asks to search online, look something up on the web, research current information, or check the internet, call this tool before answering instead of answering from memory.".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "query": {"type": "string", "description": "Search query"},
                    "limit": {"type": "integer", "description": "Maximum number of related results to return", "default": 5}
                },
                "required": ["query"]
            }),
        },
        SkillDef {
            name: "ask_user".into(),
            description: "Pause execution and ask the user a question. Use when you need clarification, a decision, or information that only the user can provide. The agent will wait for the user's reply before continuing.".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "session_id": {"type": "string", "description": "Current session ID"},
                    "question": {"type": "string", "description": "The question to ask the user"},
                    "context": {"type": "string", "description": "Optional context explaining why this question is needed"}
                },
                "required": ["session_id", "question"]
            }),
        },
        SkillDef {
            name: "delegate".into(),
            description: "Delegate a sub-task to another agent session. Use when a task is complex enough to warrant a separate agent turn, or when you want to run work in the background. Returns the agent's reply for sync mode, or a session_id for background mode.".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "session_id": {"type": "string", "description": "Target session ID to send the task to"},
                    "content": {"type": "string", "description": "The task or prompt to send to the agent"},
                    "mode": {"type": "string", "enum": ["sync", "background"], "default": "sync", "description": "sync waits for the reply; background returns immediately"}
                },
                "required": ["session_id", "content"]
            }),
        },
        SkillDef {
            name: "generate_image".into(),
            description: "根据文字描述生成一张图像。用于创作分镜画面、概念图、视觉素材。返回图像的 base64 或保存路径。".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "prompt": {"type": "string", "description": "图像的文字描述"},
                    "model": {"type": "string", "default": "gpt-image-2-vip", "description": "图像模型"},
                    "size": {"type": "string", "default": "1024x1024"}
                },
                "required": ["prompt"]
            }),
        },
        SkillDef {
            name: "render_video".into(),
            description: "把多张图像合成为一段视频（每张图按指定时长显示）。用于把分镜图拼成成片。".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "images": {"type": "array", "items": {"type": "string"}, "description": "图像文件路径列表"},
                    "durations": {"type": "array", "items": {"type": "number"}, "description": "每张图显示秒数"},
                    "output": {"type": "string", "description": "输出视频路径"},
                    "fps": {"type": "integer", "default": 25},
                    "size": {"type": "string", "default": "1024x1024"}
                },
                "required": ["images", "output"]
            }),
        },
    ]
}

fn decode_html_entities(value: &str) -> String {
    value
        .replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&#x27;", "'")
}

fn strip_html_tags(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    let mut in_tag = false;

    for ch in value.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => output.push(ch),
            _ => {}
        }
    }

    output
}

fn collapse_whitespace(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<&str>>()
        .join(" ")
        .trim()
        .to_string()
}

fn trim_query_punctuation(value: &str) -> String {
    value
        .trim()
        .trim_matches(|ch: char| {
            matches!(
                ch,
                '"' | '\''
                    | '`'
                    | ','
                    | '.'
                    | '!'
                    | '?'
                    | ':'
                    | ';'
                    | '('
                    | ')'
                    | '['
                    | ']'
                    | '{'
                    | '}'
                    | '\u{3002}'
                    | '\u{FF01}'
                    | '\u{FF1F}'
                    | '\u{FF0C}'
                    | '\u{FF1A}'
                    | '\u{201C}'
                    | '\u{201D}'
                    | '\u{300C}'
                    | '\u{300D}'
                    | '\u{300E}'
                    | '\u{300F}'
            )
        })
        .trim()
        .to_string()
}

fn strip_known_prefixes(mut value: String) -> String {
    let ascii_prefixes = [
        "please ",
        "can you ",
        "could you ",
        "would you ",
        "help me ",
        "search online for ",
        "search online ",
        "search the web for ",
        "search the web ",
        "search for ",
        "search up ",
        "search ",
        "look up ",
        "research ",
        "find information about ",
        "find out about ",
        "tell me about ",
        "what is the meaning of ",
        "meaning of ",
    ];
    let unicode_prefixes = [
        "\u{4E0A}\u{7F51}\u{641C}\u{7D22}\u{4E00}\u{4E0B}",
        "\u{4E0A}\u{7F51}\u{641C}\u{7D22}",
        "\u{7F51}\u{4E0A}\u{641C}\u{7D22}\u{4E00}\u{4E0B}",
        "\u{7F51}\u{4E0A}\u{641C}\u{7D22}",
        "\u{5E2E}\u{6211}\u{641C}\u{7D22}\u{4E00}\u{4E0B}",
        "\u{5E2E}\u{6211}\u{641C}\u{7D22}",
        "\u{5E2E}\u{6211}\u{67E5}\u{4E00}\u{4E0B}",
        "\u{5E2E}\u{6211}\u{7814}\u{7A76}\u{4E00}\u{4E0B}",
        "\u{641C}\u{7D22}\u{4E00}\u{4E0B}",
        "\u{641C}\u{7D22}",
        "\u{67E5}\u{4E00}\u{4E0B}",
        "\u{67E5}\u{67E5}",
        "\u{7814}\u{7A76}\u{4E00}\u{4E0B}",
        "\u{8C03}\u{7814}\u{4E00}\u{4E0B}",
    ];

    loop {
        let lower = value.to_lowercase();
        let mut changed = false;

        for prefix in ascii_prefixes {
            if lower.starts_with(prefix) {
                value = value[prefix.len()..].trim_start().to_string();
                changed = true;
                break;
            }
        }
        if changed {
            continue;
        }

        for prefix in unicode_prefixes {
            if value.starts_with(prefix) {
                value = value[prefix.len()..].trim_start().to_string();
                changed = true;
                break;
            }
        }

        if !changed {
            break;
        }
    }

    value
}

fn strip_known_suffixes(mut value: String) -> String {
    let ascii_suffixes = [
        " for me",
        " please",
        " thanks",
        " thank you",
        " and tell me",
        " and explain it",
    ];
    let unicode_suffixes = [
        "\u{5427}",
        "\u{5440}",
        "\u{8C22}\u{8C22}",
        "\u{7136}\u{540E}\u{544A}\u{8BC9}\u{6211}",
        "\u{518D}\u{544A}\u{8BC9}\u{6211}",
        "\u{544A}\u{8BC9}\u{6211}",
    ];

    loop {
        let lower = value.to_lowercase();
        let mut changed = false;

        for suffix in ascii_suffixes {
            if lower.ends_with(suffix) {
                let keep = value.len().saturating_sub(suffix.len());
                value = value[..keep].trim_end().to_string();
                changed = true;
                break;
            }
        }
        if changed {
            continue;
        }

        for suffix in unicode_suffixes {
            if value.ends_with(suffix) {
                let keep = value.len().saturating_sub(suffix.len());
                value = value[..keep].trim_end().to_string();
                changed = true;
                break;
            }
        }

        if !changed {
            break;
        }
    }

    value
}

fn normalize_web_search_query(query: &str) -> String {
    let mut value = trim_query_punctuation(query);
    if value.is_empty() {
        return value;
    }

    value = strip_known_prefixes(value);
    value = strip_known_suffixes(value);

    let lower = value.to_lowercase();
    if let Some(rest) = lower.strip_prefix("the meaning of ") {
        let offset = value.len() - rest.len();
        value = format!("{} meaning", value[offset..].trim());
    } else if let Some(rest) = lower.strip_prefix("meaning of ") {
        let offset = value.len() - rest.len();
        value = format!("{} meaning", value[offset..].trim());
    } else if let Some(rest) = lower.strip_prefix("what does ") {
        if let Some(end) = rest.find(" mean") {
            let start = value.len() - rest.len();
            value = format!("{} meaning", value[start..start + end].trim());
        }
    }

    let chinese_meaning_suffix = "\u{7684}\u{542B}\u{4E49}";
    let chinese_meaning_question = "\u{662F}\u{4EC0}\u{4E48}\u{610F}\u{601D}";
    let chinese_meaning_alt = "\u{7684}\u{610F}\u{601D}";
    if value.ends_with(chinese_meaning_suffix) {
        let keep = value.len().saturating_sub(chinese_meaning_suffix.len());
        value = format!("{} \u{542B}\u{4E49}", value[..keep].trim());
    } else if value.ends_with(chinese_meaning_question) {
        let keep = value.len().saturating_sub(chinese_meaning_question.len());
        value = format!("{} \u{542B}\u{4E49}", value[..keep].trim());
    } else if value.ends_with(chinese_meaning_alt) {
        let keep = value.len().saturating_sub(chinese_meaning_alt.len());
        value = format!("{} \u{542B}\u{4E49}", value[..keep].trim());
    }

    collapse_whitespace(&trim_query_punctuation(&value))
}

fn extract_attr_value(block: &str, attr: &str) -> Option<String> {
    let needle = format!("{}=\"", attr);
    let start = block.find(&needle)? + needle.len();
    let rest = &block[start..];
    let end = rest.find('"')?;
    Some(decode_html_entities(&rest[..end]))
}

fn extract_first_li_text(block: &str) -> Option<String> {
    let li_start = block.find("<li")?;
    let li_block = &block[li_start..];
    let content_start = li_block.find('>')? + 1;
    let content = &li_block[content_start..];
    let content_end = content.find("</li>")?;
    let text = collapse_whitespace(&decode_html_entities(&strip_html_tags(
        &content[..content_end],
    )));
    if text.is_empty() {
        None
    } else {
        Some(text)
    }
}

fn parse_brave_definition(html: &str) -> Option<String> {
    let start = html.find("id=\"rh\"")?;
    let block = &html[start..html.len().min(start + 12000)];
    let heading = block.find("<h5").and_then(|idx| {
        let section = &block[idx..];
        let content_start = section.find('>')? + 1;
        let content = &section[content_start..];
        let content_end = content.find("</h5>")?;
        let text = collapse_whitespace(&decode_html_entities(&strip_html_tags(
            &content[..content_end],
        )));
        if text.is_empty() {
            None
        } else {
            Some(text)
        }
    });
    let definition = extract_first_li_text(block)?;

    Some(match heading {
        Some(word) => format!("{}: {}", word, definition),
        None => definition,
    })
}

fn parse_brave_results(html: &str, limit: usize) -> Vec<serde_json::Value> {
    let mut results = Vec::new();
    let mut cursor = html;

    while results.len() < limit {
        let Some(result_start) = cursor.find("class=\"result-content") else {
            break;
        };
        cursor = &cursor[result_start..];

        let Some(anchor_start) = cursor.find("<a href=\"") else {
            cursor = &cursor["class=\"result-content".len()..];
            continue;
        };
        let anchor = &cursor[anchor_start + "<a href=\"".len()..];
        let Some(url_end) = anchor.find('"') else {
            break;
        };
        let url = decode_html_entities(&anchor[..url_end]);
        let after_anchor = &anchor[url_end..];

        let title = after_anchor
            .find("class=\"title ")
            .and_then(|idx| {
                let section = &after_anchor[idx..];
                let title_attr = extract_attr_value(section, "title").unwrap_or_default();
                if !title_attr.is_empty() {
                    return Some(title_attr);
                }
                let start = section.find('>')? + 1;
                let inner = &section[start..];
                let end = inner.find("</div>")?;
                let text =
                    collapse_whitespace(&decode_html_entities(&strip_html_tags(&inner[..end])));
                if text.is_empty() {
                    None
                } else {
                    Some(text)
                }
            })
            .unwrap_or_default();

        let text = after_anchor
            .find("class=\"generic-snippet")
            .and_then(|idx| {
                let section = &after_anchor[idx..];
                let content_idx = section.find("class=\"content ")?;
                let content_section = &section[content_idx..];
                let start = content_section.find('>')? + 1;
                let inner = &content_section[start..];
                let end = inner.find("</div>")?;
                let text =
                    collapse_whitespace(&decode_html_entities(&strip_html_tags(&inner[..end])));
                if text.is_empty() {
                    None
                } else {
                    Some(text)
                }
            })
            .unwrap_or_default();

        if !url.is_empty() && (!title.is_empty() || !text.is_empty()) {
            results.push(serde_json::json!({
                "title": title,
                "url": url,
                "text": text,
            }));
        }

        cursor = after_anchor;
    }

    results
}

const SESSION_EVENTS_CAPABILITY: &str = "session.events";
const AGENT_RUNTIME_CAPABILITY: &str = "agent.runtime";

fn do_delegate(args: &serde_json::Value) -> PackageResult {
    let session_id = args.get("session_id").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
    let content = args.get("content").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
    let mode = args.get("mode").and_then(|v| v.as_str()).unwrap_or("sync");

    if session_id.is_empty() {
        return PackageResult::err("delegate requires session_id");
    }
    if content.is_empty() {
        return PackageResult::err("delegate requires content");
    }

    let payload = serde_json::json!({
        "session_id": session_id,
        "content": content,
    });

    if mode == "background" {
        // Fire-and-forget: spawn via call_capability_action without waiting.
        let _ = call_capability_action(AGENT_RUNTIME_CAPABILITY, "send_session_message", &payload);
        return PackageResult::ok(serde_json::json!({
            "mode": "background",
            "session_id": session_id,
            "status": "dispatched",
        }));
    }

    // Sync: call and wait for reply.
    match call_capability_action(AGENT_RUNTIME_CAPABILITY, "send_session_message", &payload) {
        Ok(raw) => {
            let result: serde_json::Value = serde_json::from_str(&raw).unwrap_or(serde_json::json!({"reply": raw}));
            PackageResult::ok(serde_json::json!({
                "mode": "sync",
                "session_id": session_id,
                "result": result,
            }))
        }
        Err(e) => PackageResult::err(format!("delegate failed: {}", e)),
    }
}

fn ask_user_pending_key(session_id: &str) -> String {
    format!("ask_user:{}:pending", session_id)
}

fn ask_user_response_key(session_id: &str) -> String {
    format!("ask_user:{}:response", session_id)
}

fn do_ask_user(args: &serde_json::Value) -> PackageResult {
    let session_id = args.get("session_id").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
    let question = args.get("question").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
    let context = args.get("context").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();

    if session_id.is_empty() {
        return PackageResult::err("ask_user requires session_id");
    }
    if question.is_empty() {
        return PackageResult::err("ask_user requires question");
    }

    // Clear any stale response from a previous ask.
    let _ = kv_delete(&ask_user_response_key(&session_id));

    // Write pending state so the frontend knows a question is waiting.
    let pending = serde_json::json!({
        "question": question,
        "context": context,
        "asked_at": now_ms(),
    });
    kv_set(&ask_user_pending_key(&session_id), &pending.to_string());

    // Emit session event so the UI can surface the question.
    let event_payload = serde_json::json!({
        "session_id": session_id,
        "type": "user_input_request",
        "payload": {
            "question": question,
            "context": context,
        },
    });
    let _ = call_capability_action(SESSION_EVENTS_CAPABILITY, "append_event", &event_payload);

    // Poll for the user's response (max 120 s, 500 ms interval).
    const MAX_WAIT_MS: u64 = 120_000;
    const POLL_INTERVAL_MS: u64 = 500;
    let start = now_ms();
    loop {
        if let Some(raw) = kv_get(&ask_user_response_key(&session_id)) {
            let _ = kv_delete(&ask_user_response_key(&session_id));
            let _ = kv_delete(&ask_user_pending_key(&session_id));
            let answer = serde_json::from_str::<serde_json::Value>(&raw)
                .ok()
                .and_then(|v| v.get("answer").and_then(|a| a.as_str()).map(str::to_string))
                .unwrap_or(raw);
            return PackageResult::ok(serde_json::json!({ "answer": answer }));
        }
        let elapsed = now_ms().saturating_sub(start);
        if elapsed >= MAX_WAIT_MS {
            let _ = kv_delete(&ask_user_pending_key(&session_id));
            return PackageResult::err("ask_user timed out waiting for user response");
        }
        // Busy-wait: WASM has no async sleep, use a spin loop approximation.
        let spin_until = now_ms() + POLL_INTERVAL_MS;
        while now_ms() < spin_until {}
    }
}

fn do_web_search(args: &serde_json::Value) -> PackageResult {
    let query = args
        .get("query_b64")
        .and_then(|value| value.as_str())
        .and_then(|value| base64::engine::general_purpose::STANDARD.decode(value).ok())
        .and_then(|value| String::from_utf8(value).ok())
        .or_else(|| {
            args.get("query")
                .and_then(|value| value.as_str())
                .or_else(|| args.get("q").and_then(|value| value.as_str()))
                .map(|value| value.to_string())
        })
        .unwrap_or_default();
    let raw_query = query.trim();

    if raw_query.is_empty() {
        return PackageResult::err("missing query");
    }

    let limit = args
        .get("limit")
        .and_then(|value| value.as_u64())
        .unwrap_or(5)
        .clamp(1, 8) as usize;

    let query = normalize_web_search_query(raw_query);
    let query = if query.is_empty() {
        raw_query.to_string()
    } else {
        query
    };

    // Prefer tool-web (js-extension-runtime/Exa) — more reliable than HTML scraping.
    let tool_web_payload = serde_json::json!({
        "action": "web_search",
        "data": { "query": query, "agent": "skills" }
    });
    if let Ok(raw) = call_package("tool-web", "handle_ws_message", &tool_web_payload.to_string()) {
        let parsed: serde_json::Value = serde_json::from_str(&raw).unwrap_or(serde_json::Value::Null);
        let is_ok = parsed.get("status").and_then(|v| v.as_str()) == Some("ok")
            || parsed.get("ok").and_then(|v| v.as_bool()) == Some(true);
        if is_ok {
            let data = parsed.get("data").cloned().unwrap_or(parsed.clone());
            return PackageResult::ok(serde_json::json!({
                "query": query,
                "results": data.get("results").cloned().unwrap_or(serde_json::Value::Array(vec![])),
                "summary": data.get("summary").or_else(|| data.get("text")).cloned()
                    .and_then(|v| v.as_str().map(|s| s.to_string()))
                    .unwrap_or_else(|| format!("Search results for '{}'.", query)),
                "source": "tool-web",
                "raw": data,
            }));
        }
    }

    // Fallback: Brave Search HTML scraping.
    let url = format!(
        "https://search.brave.com/search?q={}&source=web",
        urlencoding::encode(&query),
    );
    let exec = match exec_command("curl.exe", &["-L", "-sS", "--max-time", "8", &url]) {
        Ok(result) => result,
        Err(error) => return PackageResult::err(format!("web search request failed: {}", error)),
    };

    let stdout_text = exec.stdout.trim();
    let stderr_text = exec.stderr.trim();
    let response_text = if stdout_text.is_empty() {
        stderr_text
    } else {
        stdout_text
    };
    if exec.status != 0 && response_text.is_empty() {
        let message = if exec.stderr.trim().is_empty() {
            exec.stdout.trim().to_string()
        } else {
            exec.stderr.trim().to_string()
        };
        return PackageResult::err(format!("web search request failed: {}", message));
    }

    let definition = parse_brave_definition(response_text);
    let related = parse_brave_results(response_text, limit);

    let mut summary_parts = Vec::new();
    if let Some(definition) = definition.as_ref() {
        summary_parts.push(format!("Definition: {}", definition));
    } else if let Some(first) = related.first() {
        let title = first
            .get("title")
            .and_then(|value| value.as_str())
            .unwrap_or("")
            .trim();
        let text = first
            .get("text")
            .and_then(|value| value.as_str())
            .unwrap_or("")
            .trim();
        if !title.is_empty() && !text.is_empty() {
            summary_parts.push(format!("Top result: {}. {}", title, text));
        } else if !text.is_empty() {
            summary_parts.push(format!("Top result: {}", text));
        } else if !title.is_empty() {
            summary_parts.push(format!("Top result: {}", title));
        }
    }
    if !related.is_empty() {
        let bullets = related
            .iter()
            .take(limit)
            .map(|entry| {
                let title = entry
                    .get("title")
                    .and_then(|value| value.as_str())
                    .unwrap_or("")
                    .trim();
                let text = entry
                    .get("text")
                    .and_then(|value| value.as_str())
                    .unwrap_or("")
                    .trim();
                if title.is_empty() {
                    format!("- {}", text)
                } else if text.is_empty() {
                    format!("- {}", title)
                } else {
                    format!("- {}: {}", title, text)
                }
            })
            .collect::<Vec<String>>()
            .join("\n");
        if !bullets.trim().is_empty() {
            summary_parts.push(format!("Related results:\n{}", bullets));
        }
    }

    let summary = if summary_parts.is_empty() {
        format!("No concise web result found for '{}'.", query)
    } else {
        summary_parts.join("\n")
    };

    PackageResult::ok(serde_json::json!({
        "query": query,
        "heading": definition.unwrap_or_default(),
        "results": related,
        "summary": summary,
        "source": "Brave Search HTML",
    }))
}

fn build_web_fetch_curl_args(method: &str, url: &str, body: Option<&str>) -> Vec<String> {
    let mut args = vec![
        "-L".to_string(),
        "-sS".to_string(),
        "-X".to_string(),
        method.to_string(),
        url.to_string(),
        "-w".to_string(),
        "\n__WEFT_STATUS__:%{http_code}".to_string(),
    ];

    if let Some(body) = body {
        args.push("--data".to_string());
        args.push(body.to_string());
    }

    args
}

fn parse_web_fetch_curl_output(stdout: &str) -> Option<(u16, String)> {
    let marker = "\n__WEFT_STATUS__:";
    let index = stdout.rfind(marker)?;
    let body = stdout[..index].to_string();
    let status_text = stdout[index + marker.len()..].trim();
    let status = status_text.parse::<u16>().ok()?;
    Some((status, body))
}

fn do_web_fetch(args: &serde_json::Value) -> PackageResult {
    let url = args["url"].as_str().unwrap_or("").trim();
    if url.is_empty() {
        return PackageResult::err("missing url");
    }

    let method = args["method"].as_str().unwrap_or("GET").trim();
    let method = if method.is_empty() { "GET" } else { method };
    let body = args["body"].as_str();

    let curl_args = build_web_fetch_curl_args(method, url, body);
    let curl_arg_refs = curl_args
        .iter()
        .map(|value| value.as_str())
        .collect::<Vec<_>>();
    let exec = match exec_command("curl.exe", &curl_arg_refs) {
        Ok(result) => result,
        Err(error) => {
            return PackageResult::err(format!("web fetch transport failed: {}", error.trim()))
        }
    };

    if let Some((status, response_body)) = parse_web_fetch_curl_output(&exec.stdout) {
        if exec.status == 0 {
            return PackageResult::ok(serde_json::json!({
                "status": status,
                "body": response_body,
            }));
        }

        let stderr_text = exec.stderr.trim();
        let detail = if stderr_text.is_empty() {
            format!("curl exited with status {}", exec.status)
        } else {
            stderr_text.to_string()
        };
        return PackageResult::err(format!(
            "web fetch request failed with status {}: {}",
            status, detail
        ));
    }

    let stderr_text = exec.stderr.trim();
    if !stderr_text.is_empty() {
        return PackageResult::err(format!("web fetch transport failed: {}", stderr_text));
    }

    PackageResult::err("web fetch transport failed: missing HTTP status marker in curl output")
}

fn capability_result(raw: String) -> PackageResult {
    let result: serde_json::Value = serde_json::from_str(&raw)
        .unwrap_or_else(|_| serde_json::json!({"result": raw}));
    PackageResult::ok(result)
}

fn do_generate_image(args: &serde_json::Value) -> PackageResult {
    let api_key = env_get("WEFT_IMAGE_API_KEY")
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let Some(api_key) = api_key else {
        return PackageResult::err("generate_image requires WEFT_IMAGE_API_KEY environment variable");
    };
    let base_url = env_get("WEFT_IMAGE_BASE_URL")
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "https://api.apiyi.com".to_string());

    let mut payload = args.as_object().cloned().unwrap_or_default();
    payload.insert("api_key".to_string(), serde_json::Value::String(api_key));
    payload.insert("base_url".to_string(), serde_json::Value::String(base_url));

    match call_capability_action("image.generate", "generate", &serde_json::Value::Object(payload)) {
        Ok(raw) => capability_result(raw),
        Err(error) => PackageResult::err(format!("generate_image failed: {}", error)),
    }
}

fn do_render_video(args: &serde_json::Value) -> PackageResult {
    match call_capability_action("video.render", "slideshow", args) {
        Ok(raw) => capability_result(raw),
        Err(error) => PackageResult::err(format!("render_video failed: {}", error)),
    }
}

fn agent_skills_key(agent: &str) -> String {
    format!("skills:agent:{}", agent)
}

fn get_agent_skills(agent: &str) -> Vec<String> {
    let defaults = || {
        builtin_skills()
            .into_iter()
            .map(|skill| skill.name)
            .collect()
    };
    match kv_get(&agent_skills_key(agent)) {
        Some(json) => {
            let skills: Vec<String> = serde_json::from_str(&json).unwrap_or_default();
            if skills.is_empty() {
                defaults()
            } else {
                skills
            }
        }
        None => defaults(),
    }
}

/// 我们自己的文件式技能根目录 = 本包安装目录下的 `skills/`。
/// 通过 core 注入的 WEFT_PACKAGE_DIR 定位（资源跟包走，不写死仓库路径，
/// 不管包从 official/ 还是 installed/ 加载都正确）。
/// 回退：拿不到包目录时用开发态仓库路径兜底。
fn weft_skills_dir() -> String {
    match env_get("WEFT_PACKAGE_DIR") {
        Some(dir) if !dir.trim().is_empty() => format!("{}/skills", dir.trim_end_matches('/')),
        _ => "packages/official/skills/skills".to_string(),
    }
}

/// 一个文件式技能（来自 <包目录>/skills/<slug>/SKILL.md）。
struct FileSkill {
    name: String,
    description: String,
    content: String,
}

/// 解析 SKILL.md 的 YAML frontmatter（--- name: ... / description: ... ---）。
/// 返回 (name, description)；解析不出则用 slug 兜底。
fn parse_skill_frontmatter(slug: &str, md: &str) -> (String, String) {
    let mut name = slug.to_string();
    let mut description = String::new();
    let trimmed = md.trim_start();
    if let Some(rest) = trimmed.strip_prefix("---") {
        if let Some(end) = rest.find("\n---") {
            let front = &rest[..end];
            for line in front.lines() {
                let line = line.trim();
                if let Some(v) = line.strip_prefix("name:") {
                    name = v.trim().trim_matches('"').to_string();
                } else if let Some(v) = line.strip_prefix("description:") {
                    description = v.trim().trim_matches('"').to_string();
                }
            }
        }
    }
    (name, description)
}

/// 扫描 weft-skills/ 目录，加载我们自己的文件式技能。
fn weft_file_skills() -> Vec<FileSkill> {
    let mut out = Vec::new();
    let root = weft_skills_dir();
    let entries = match list_dir(&root) {
        Ok(e) => e,
        Err(_) => return out, // 目录不存在/不可读 → 无文件技能，仅 builtin。
    };
    for entry in entries {
        if !entry.is_dir {
            continue;
        }
        let skill_md = format!("{}/{}/SKILL.md", root, entry.name);
        if let Ok(content) = read_file(&skill_md) {
            if content.trim().is_empty() {
                continue;
            }
            let (name, description) = parse_skill_frontmatter(&entry.name, &content);
            out.push(FileSkill {
                name,
                description,
                content,
            });
        }
    }
    out
}

/// 提取 SKILL.md 正文（剥离 `--- ... ---` frontmatter），用于注入 prompt。
fn skill_body(md: &str) -> &str {
    let trimmed = md.trim_start();
    if let Some(rest) = trimmed.strip_prefix("---") {
        if let Some(end) = rest.find("\n---") {
            // 跳过第二个 `---` 行后的内容。
            let after = &rest[end + 4..];
            return after.trim_start_matches(['-', '\n', '\r']).trim_start();
        }
    }
    trimmed
}

/// 按 query 选出适用的文件技能：query 命中技能 name/description 中的任一关键词即适用。
/// 命中后把技能正文拼成 prompt 注入块（供 agent-core retrieve_applicable 使用），
/// 使 AI 写前端等场景无需在 system_prompt 写死整个模板，按需注入、节省 token。
fn applicable_file_skills_block(query: &str) -> String {
    let q = query.to_lowercase();
    if q.trim().is_empty() {
        return String::new();
    }
    let mut blocks = Vec::new();
    for fs in weft_file_skills() {
        // 关键词来源：技能 name + description 分词（>=2 字符的词）。
        let hay = format!("{} {}", fs.name, fs.description).to_lowercase();
        let hit = hay
            .split(|c: char| !c.is_alphanumeric())
            .filter(|w| w.len() >= 2)
            .any(|w| q.contains(w));
        if hit {
            blocks.push(format!(
                "Skill: {}\nWhen to use: {}\n{}",
                fs.name,
                fs.description,
                skill_body(&fs.content)
            ));
        }
    }
    blocks.join("\n\n")
}

fn do_list_skills() -> PackageResult {
    let mut skills: Vec<serde_json::Value> = builtin_skills()
        .iter()
        .map(|s| serde_json::json!({"name": s.name, "description": s.description}))
        .collect();
    // 合并我们自己的文件式技能。
    for fs in weft_file_skills() {
        skills.push(serde_json::json!({"name": fs.name, "description": fs.description}));
    }
    PackageResult::ok(serde_json::json!({"skills": skills}))
}

fn do_read_skill(name: &str) -> PackageResult {
    let normalized = name.trim();
    // 先查 builtin 工具。
    if let Some(skill) = builtin_skills()
        .into_iter()
        .find(|skill| skill.name == normalized)
    {
        return PackageResult::ok(serde_json::json!({
            "name": skill.name,
            "description": skill.description,
            "source": "builtin",
            "skill_root": "builtin://skills",
            "content": format!(
                "# {}\n\n{}\n\nParameters:\n{}\n",
                skill.name, skill.description, skill.parameters
            ),
        }));
    }
    // 再查我们自己的文件式技能，返回完整 SKILL.md 内容。
    if let Some(fs) = weft_file_skills()
        .into_iter()
        .find(|fs| fs.name == normalized)
    {
        return PackageResult::ok(serde_json::json!({
            "name": fs.name,
            "description": fs.description,
            "source": "file",
            "skill_root": weft_skills_dir(),
            "content": fs.content,
        }));
    }
    PackageResult::err(format!("unknown skill: {}", normalized))
}

fn save_agent_skills(agent: &str, skills: &[String]) {
    let json = serde_json::to_string(skills).unwrap_or_else(|_| "[]".into());
    kv_set(&agent_skills_key(agent), &json);
}

#[plugin_fn]
pub fn init(_input: String) -> FnResult<String> {
    log_info("skills package initialized");
    Ok(PackageResult::ok_empty().to_json())
}

#[plugin_fn]
pub fn handle_ws_message(input: String) -> FnResult<String> {
    let req: WsRequest = serde_json::from_str(&input).unwrap_or(WsRequest {
        action: String::new(),
        data: serde_json::Value::Null,
    });

    let result = match req.action.as_str() {
        "list_available" | "list_skills" => do_list_skills(),
        "read_skill" => {
            let name = req.data["name"].as_str().unwrap_or("");
            do_read_skill(name)
        }
        "get_tool_specs" => {
            let agent = req.data["agent"].as_str().unwrap_or("");
            do_get_tool_specs(agent)
        }
        "retrieve_applicable" => match serde_json::from_value::<RetrieveSkillsInput>(req.data) {
            Ok(input) => do_retrieve_applicable(input),
            Err(_) => PackageResult::err("invalid retrieve_applicable payload"),
        },
        "crystallize_from_trajectory" => {
            match serde_json::from_value::<CrystallizeTrajectoryInput>(req.data) {
                Ok(input) => do_crystallize_from_trajectory(input),
                Err(_) => PackageResult::err("invalid crystallize_from_trajectory payload"),
            }
        }
        "record_skill_usage" => match serde_json::from_value::<SkillUsageInput>(req.data) {
            Ok(input) => do_record_skill_usage(input),
            Err(_) => PackageResult::err("invalid record_skill_usage payload"),
        },
        "record_patch_suggestion" => match serde_json::from_value::<PatchSuggestionInput>(req.data)
        {
            Ok(input) => do_record_patch_suggestion(input),
            Err(_) => PackageResult::err("invalid record_patch_suggestion payload"),
        },
        "get_evolved" => {
            let agent = req.data["agent"].as_str().unwrap_or("");
            let skill_id = req.data["skill_id"].as_str().unwrap_or("");
            do_get_evolved(agent, skill_id)
        }
        "review_evolved" => match serde_json::from_value::<ReviewEvolvedInput>(req.data) {
            Ok(input) => do_review_evolved(input),
            Err(_) => PackageResult::err("invalid review_evolved payload"),
        },
        "list_evolved" => {
            let agent = req.data["agent"].as_str().unwrap_or("");
            do_list_evolved(agent)
        }
        "maintenance" => do_maintenance(),
        "execute_tool" => {
            let agent = req.data["agent"].as_str().unwrap_or("");
            let tool = req.data["tool"].as_str().unwrap_or("");
            let args = &req.data["args"];
            do_execute_tool(agent, tool, args)
        }
        "enable" => {
            let agent = req.data["agent"].as_str().unwrap_or("");
            let skill = req.data["skill"].as_str().unwrap_or("");
            do_enable(agent, skill)
        }
        "disable" => {
            let agent = req.data["agent"].as_str().unwrap_or("");
            let skill = req.data["skill"].as_str().unwrap_or("");
            do_disable(agent, skill)
        }
        "set_for_agent" => {
            let agent = req.data["agent"].as_str().unwrap_or("");
            let skills = req.data["skills"]
                .as_array()
                .map(|items| {
                    items
                        .iter()
                        .filter_map(|value| value.as_str())
                        .map(|value| value.to_string())
                        .collect::<Vec<String>>()
                })
                .unwrap_or_default();
            do_set_for_agent(agent, &skills)
        }
        _ => PackageResult::err(format!("unknown action: {}", req.action)),
    };

    Ok(result.to_json())
}

#[plugin_fn]
pub fn get_tool_specs(input: String) -> FnResult<String> {
    #[derive(Deserialize)]
    struct Input {
        agent: String,
    }
    let p: Input = serde_json::from_str(&input)
        .map_err(|e| extism_pdk::Error::msg(format!("parse error: {}", e)))?;
    Ok(do_get_tool_specs(&p.agent).to_json())
}

fn do_get_tool_specs(agent: &str) -> PackageResult {
    let enabled = get_agent_skills(agent);
    let all = builtin_skills();
    let mut specs: Vec<serde_json::Value> = all
        .iter()
        .filter(|s| enabled.contains(&s.name))
        .map(|s| {
            serde_json::json!({
                "type": "function",
                "function": {
                    "name": s.name,
                    "description": s.description,
                    "parameters": s.parameters,
                }
            })
        })
        .collect();
    specs.extend(external_tool_specs(agent));
    PackageResult::ok(serde_json::json!({"tools": specs}))
}

#[plugin_fn]
pub fn execute_tool(input: String) -> FnResult<String> {
    #[derive(Deserialize)]
    struct Input {
        agent: String,
        tool: String,
        args: serde_json::Value,
    }
    let p: Input = serde_json::from_str(&input)
        .map_err(|e| extism_pdk::Error::msg(format!("parse error: {}", e)))?;
    Ok(do_execute_tool(&p.agent, &p.tool, &p.args).to_json())
}

fn do_execute_tool(agent: &str, tool: &str, args: &serde_json::Value) -> PackageResult {
    match tool {
        "fs_read" | "fs_write" | "fs_list" | "shell_exec" | "git" | "web_fetch" => {
            let request = serde_json::json!({
                "agent": agent,
                "tool": tool,
                "args": args,
            });
            return match call_package_ws_action(
                "tool-runtime-core",
                "execute_tool",
                &request,
            ) {
                Ok(output) => serde_json::from_str::<PackageResult>(&output).unwrap_or_else(|_| {
                    PackageResult::err(format!("invalid tool-runtime-core response: {}", output))
                }),
                Err(error) => PackageResult::err(error),
            };
        }
        "web_search" => do_web_search(args),
        "ask_user" => do_ask_user(args),
        "delegate" => do_delegate(args),
        "generate_image" => do_generate_image(args),
        "render_video" => do_render_video(args),
        _ => {
            if let Some((server, tool_name)) = parse_external_tool_name(tool) {
                match call_package(
                    "mcp-client",
                    "call_tool",
                    &serde_json::json!({
                        "agent": agent,
                        "server": server,
                        "tool": tool_name,
                        "args": args,
                    })
                    .to_string(),
                ) {
                    Ok(output) => serde_json::from_str::<PackageResult>(&output)
                        .unwrap_or_else(|_| PackageResult::err("invalid MCP tool response")),
                    Err(error) => PackageResult::err(error),
                }
            } else {
                PackageResult::err(format!("unknown tool: {}", tool))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_web_fetch_curl_args_includes_method_url_and_status_marker() {
        let args = build_web_fetch_curl_args("GET", "https://example.com", None);

        assert_eq!(
            args,
            vec![
                "-L",
                "-sS",
                "-X",
                "GET",
                "https://example.com",
                "-w",
                "\n__WEFT_STATUS__:%{http_code}",
            ]
        );
    }

    #[test]
    fn build_web_fetch_curl_args_appends_body_when_present() {
        let args = build_web_fetch_curl_args("POST", "https://example.com", Some("hello=1"));

        assert_eq!(
            args,
            vec![
                "-L",
                "-sS",
                "-X",
                "POST",
                "https://example.com",
                "-w",
                "\n__WEFT_STATUS__:%{http_code}",
                "--data",
                "hello=1",
            ]
        );
    }

    #[test]
    fn parse_web_fetch_curl_output_extracts_status_and_body() {
        let parsed = parse_web_fetch_curl_output("Example Domain\n__WEFT_STATUS__:200")
            .expect("expected parsed curl output");

        assert_eq!(parsed.0, 200);
        assert_eq!(parsed.1, "Example Domain");
    }

    #[test]
    fn parse_web_fetch_curl_output_requires_marker() {
        assert!(parse_web_fetch_curl_output("Example Domain").is_none());
    }

    #[test]
    fn parse_external_tool_name_reads_mcp_namespace() {
        let parsed =
            parse_external_tool_name("mcp::filesystem::read_file").expect("tool should parse");
        assert_eq!(parsed.0, "filesystem");
        assert_eq!(parsed.1, "read_file");
    }

    #[test]
    fn external_tool_name_builds_expected_namespace() {
        assert_eq!(
            external_tool_name("filesystem", "read_file"),
            "mcp::filesystem::read_file"
        );
    }

    fn test_evolved_skill(id: &str, updated_at: u64) -> EvolvedSkill {
        EvolvedSkill {
            id: id.into(),
            title: "Reusable Rust formatting workflow".into(),
            description: "Use when a Rust crate needs formatting and verification.".into(),
            metadata: serde_json::json!({}),
            triggers: vec!["rust fmt".into(), "cargo fmt".into()],
            anti_triggers: Vec::new(),
            required_tools: vec!["cargo".into()],
            procedure: "1. Run cargo fmt.\n2. Run the focused cargo test command.".into(),
            steps: vec![SkillStep {
                title: "Format".into(),
                instruction: "Run cargo fmt.".into(),
                tools: vec!["cargo".into()],
            }],
            verification: "Confirm formatting and focused tests pass.".into(),
            verification_steps: vec![VerificationStep {
                title: "Verify".into(),
                expected: "Focused tests pass.".into(),
                required: true,
            }],
            version: 1,
            source: "trajectory".into(),
            source_trajectory_id: "traj-1".into(),
            created_at: 1,
            updated_at,
            status: "active".into(),
            risk_level: "low".into(),
            quality_score: 1.0,
            confidence: 1.0,
            review_required: false,
            reviewed_at: 0,
            promoted_at: 0,
            archived_at: 0,
            last_used_at: 0,
            last_failure_at: 0,
            successful_uses: 0,
            failed_uses: 0,
            archived: false,
        }
    }

    fn verified_trajectory_input(
        auto_activate: bool,
        risk_level: &str,
    ) -> CrystallizeTrajectoryInput {
        CrystallizeTrajectoryInput {
            agent: "agent-a".into(),
            trajectory_id: "traj-quality".into(),
            task: "format rust crate".into(),
            summary: "Reusable formatting skill".into(),
            steps: vec!["Run cargo fmt".into(), "Run focused tests".into()],
            tools: vec!["cargo".into()],
            final_result: "Formatted crate and tests passed".into(),
            success: true,
            verification_passed: true,
            verification_evidence: "cargo fmt and focused tests passed".into(),
            auto_activate,
            risk_level: risk_level.into(),
        }
    }

    #[test]
    fn quality_gate_builds_active_or_pending_review_skills() {
        let active = build_skill_from_trajectory(&verified_trajectory_input(true, "low"));
        assert_eq!(active.status, "active");
        assert!(!active.review_required);
        assert_eq!(active.quality_score, 1.0);
        assert_eq!(active.confidence, 1.0);
        assert!(active.promoted_at > 0);
        assert_eq!(active.required_tools, vec!["cargo".to_string()]);
        assert!(!active.steps.is_empty());
        assert!(!active.verification_steps.is_empty());

        let mut pending_input = verified_trajectory_input(false, "high");
        pending_input.verification_passed = false;
        pending_input.verification_evidence.clear();
        let pending = build_skill_from_trajectory(&pending_input);
        assert_eq!(pending.status, "pending_review");
        assert!(pending.review_required);
        assert!(pending.quality_score < 1.0);
        assert_eq!(pending.promoted_at, 0);
        assert_eq!(pending.metadata["quality_report"]["passed"], false);
    }

    #[test]
    fn retrieval_filters_pending_rejected_archived_and_anti_triggered_skills() {
        let active = test_evolved_skill("active", 30);
        let mut pending = test_evolved_skill("pending", 40);
        pending.status = "pending_review".into();
        let mut rejected = test_evolved_skill("rejected", 50);
        rejected.status = "rejected".into();
        let mut archived = test_evolved_skill("archived", 60);
        archived.archived = true;
        let mut anti = test_evolved_skill("anti", 70);
        anti.anti_triggers = vec!["skip generated".into()];

        let scored = scored_evolved_skills_for_query(
            vec![active, pending, rejected, archived, anti],
            "please run cargo fmt but skip generated files",
            10,
        );

        assert_eq!(scored.len(), 1);
        assert_eq!(scored[0].skill.id, "active");
        assert!(scored
            .iter()
            .all(|entry| skill_is_retrievable(&entry.skill)));
    }

    #[test]
    fn maintenance_rules_promote_and_archive_by_quality() {
        let mut promotable = test_evolved_skill("promotable", 10);
        promotable.status = "pending_review".into();
        promotable.successful_uses = 2;
        promotable.quality_score = 0.9;
        promotable.confidence = 0.95;
        let issues = validate_evolved_skill(&promotable);
        assert!(issues.is_empty());
        assert!(should_promote_skill(&promotable, &issues));

        let mut high_risk = promotable.clone();
        high_risk.risk_level = "high".into();
        assert!(!should_promote_skill(&high_risk, &issues));

        let mut rejected = test_evolved_skill("rejected-for-archive", 10);
        rejected.status = "rejected".into();
        assert!(should_archive_skill(&rejected, &[]));

        let mut low_success = test_evolved_skill("low-success", 10);
        low_success.successful_uses = 1;
        low_success.failed_uses = 4;
        assert!(should_archive_skill(&low_success, &[]));

        let mut invalid = test_evolved_skill("invalid", 10);
        invalid.title.clear();
        invalid.procedure.clear();
        invalid.verification.clear();
        invalid.steps.clear();
        invalid.verification_steps.clear();
        invalid.triggers.clear();
        let invalid_issues = validate_evolved_skill(&invalid);
        assert!(invalid_issues.len() >= 3);
        assert!(should_archive_skill(&invalid, &invalid_issues));
    }
    #[test]
    fn score_retrieve_returns_scored_evolved_skill_and_context_block() {
        let mut older = test_evolved_skill("older", 10);
        older.title = "General cleanup workflow".into();
        older.triggers = vec!["cleanup".into()];
        older.procedure = "1. Inspect cleanup scope.".into();
        older.verification = "Confirm cleanup scope is safe.".into();
        older.steps = Vec::new();
        older.verification_steps = Vec::new();
        older.quality_score = 0.0;
        older.confidence = 0.0;
        older.status = "pending_review".into();
        let newer = test_evolved_skill("newer", 20);

        let scored = scored_evolved_skills_for_query(vec![older, newer], "please run cargo fmt", 3);

        assert_eq!(scored.len(), 1);
        assert!(scored[0].score > 0);
        assert_eq!(scored[0].skill.id, "newer");

        let context_block = evolved_skill_context_block(
            &scored
                .iter()
                .map(|entry| entry.skill.clone())
                .collect::<Vec<_>>(),
        );
        assert!(context_block.contains("Skill: Reusable Rust formatting workflow (id=newer, v1)"));
        assert!(context_block.contains("When to use: rust fmt, cargo fmt"));
        assert!(context_block.contains("Procedure:\n1. Run cargo fmt."));
        assert!(context_block.contains("Verify by: Confirm formatting and focused tests pass."));
    }

    #[test]
    fn retrieve_limit_zero_returns_no_scored_results() {
        let scored = scored_evolved_skills_for_query(
            vec![test_evolved_skill("formatting", 10)],
            "cargo fmt",
            0,
        );

        assert!(scored.is_empty());
        assert_eq!(evolved_skill_context_block(&[]), "");
    }

    #[test]
    fn patch_suggestion_validation_rejects_empty_patch_or_reason_and_unknown_skill() {
        let valid = PatchSuggestionInput {
            agent: "agent-a".into(),
            skill_id: "skill-a".into(),
            trajectory_id: "traj-a".into(),
            reason: "Improve verification.".into(),
            patch: "Add a verification step.".into(),
        };
        assert!(patch_suggestion_validation_error(&valid).is_none());

        let empty_patch = PatchSuggestionInput {
            patch: "   ".into(),
            ..valid
        };
        assert_eq!(
            patch_suggestion_validation_error(&empty_patch),
            Some("missing agent, skill_id, trajectory_id, reason, or patch")
        );

        let empty_reason = PatchSuggestionInput {
            reason: "".into(),
            patch: "Add a verification step.".into(),
            ..empty_patch
        };
        assert_eq!(
            patch_suggestion_validation_error(&empty_reason),
            Some("missing agent, skill_id, trajectory_id, reason, or patch")
        );

        assert_eq!(
            patch_suggestion_unknown_skill_error("missing-skill"),
            "unknown evolved skill: missing-skill"
        );
    }

    #[test]
    fn usage_history_trim_keeps_latest_100_entries() {
        let history = (0..105)
            .map(|index| serde_json::json!({ "index": index }))
            .collect::<Vec<_>>();

        let trimmed = trim_skill_usage_history(history);

        assert_eq!(trimmed.len(), 100);
        assert_eq!(trimmed.first().unwrap()["index"], 5);
        assert_eq!(trimmed.last().unwrap()["index"], 104);
    }
}

#[plugin_fn]
pub fn enable_for_agent(input: String) -> FnResult<String> {
    #[derive(Deserialize)]
    struct Input {
        agent: String,
        skill: String,
    }
    let p: Input = serde_json::from_str(&input)
        .map_err(|e| extism_pdk::Error::msg(format!("parse error: {}", e)))?;
    Ok(do_enable(&p.agent, &p.skill).to_json())
}

fn do_enable(agent: &str, skill: &str) -> PackageResult {
    let mut skills = get_agent_skills(agent);
    if !skills.contains(&skill.to_string()) {
        skills.push(skill.to_string());
        save_agent_skills(agent, &skills);
    }
    PackageResult::ok_empty()
}

#[plugin_fn]
pub fn disable_for_agent(input: String) -> FnResult<String> {
    #[derive(Deserialize)]
    struct Input {
        agent: String,
        skill: String,
    }
    let p: Input = serde_json::from_str(&input)
        .map_err(|e| extism_pdk::Error::msg(format!("parse error: {}", e)))?;
    Ok(do_disable(&p.agent, &p.skill).to_json())
}

fn do_disable(agent: &str, skill: &str) -> PackageResult {
    let mut skills = get_agent_skills(agent);
    skills.retain(|s| s != skill);
    save_agent_skills(agent, &skills);
    PackageResult::ok_empty()
}

fn do_set_for_agent(agent: &str, skills: &[String]) -> PackageResult {
    save_agent_skills(agent, skills);
    PackageResult::ok(serde_json::json!({ "skills": skills }))
}

#[plugin_fn]
pub fn list_available(_input: String) -> FnResult<String> {
    Ok(do_list_skills().to_json())
}

#[plugin_fn]
pub fn list_skills(_input: String) -> FnResult<String> {
    Ok(do_list_skills().to_json())
}

#[plugin_fn]
pub fn read_skill(input: String) -> FnResult<String> {
    #[derive(Deserialize)]
    struct Input {
        name: String,
    }
    let p: Input = serde_json::from_str(&input)
        .map_err(|e| extism_pdk::Error::msg(format!("parse error: {}", e)))?;
    Ok(do_read_skill(&p.name).to_json())
}

#[plugin_fn]
pub fn set_for_agent(input: String) -> FnResult<String> {
    #[derive(Deserialize)]
    struct Input {
        agent: String,
        #[serde(default)]
        skills: Vec<String>,
    }

    let p: Input = serde_json::from_str(&input)
        .map_err(|e| extism_pdk::Error::msg(format!("parse error: {}", e)))?;
    Ok(do_set_for_agent(&p.agent, &p.skills).to_json())
}

#[plugin_fn]
pub fn retrieve_applicable(input: String) -> FnResult<String> {
    let payload: RetrieveSkillsInput = serde_json::from_str(&input)
        .map_err(|e| extism_pdk::Error::msg(format!("parse error: {}", e)))?;
    Ok(do_retrieve_applicable(payload).to_json())
}

#[plugin_fn]
pub fn crystallize_from_trajectory(input: String) -> FnResult<String> {
    let payload: CrystallizeTrajectoryInput = serde_json::from_str(&input)
        .map_err(|e| extism_pdk::Error::msg(format!("parse error: {}", e)))?;
    Ok(do_crystallize_from_trajectory(payload).to_json())
}

#[plugin_fn]
pub fn record_skill_usage(input: String) -> FnResult<String> {
    let payload: SkillUsageInput = serde_json::from_str(&input)
        .map_err(|e| extism_pdk::Error::msg(format!("parse error: {}", e)))?;
    Ok(do_record_skill_usage(payload).to_json())
}

#[plugin_fn]
pub fn record_patch_suggestion(input: String) -> FnResult<String> {
    let payload: PatchSuggestionInput = serde_json::from_str(&input)
        .map_err(|e| extism_pdk::Error::msg(format!("parse error: {}", e)))?;
    Ok(do_record_patch_suggestion(payload).to_json())
}

#[plugin_fn]
pub fn get_evolved(input: String) -> FnResult<String> {
    #[derive(Deserialize)]
    struct Input {
        agent: String,
        skill_id: String,
    }
    let payload: Input = serde_json::from_str(&input)
        .map_err(|e| extism_pdk::Error::msg(format!("parse error: {}", e)))?;
    Ok(do_get_evolved(&payload.agent, &payload.skill_id).to_json())
}

#[plugin_fn]
pub fn review_evolved(input: String) -> FnResult<String> {
    let payload: ReviewEvolvedInput = serde_json::from_str(&input)
        .map_err(|e| extism_pdk::Error::msg(format!("parse error: {}", e)))?;
    Ok(do_review_evolved(payload).to_json())
}

#[plugin_fn]
pub fn list_evolved(input: String) -> FnResult<String> {
    #[derive(Deserialize)]
    struct Input {
        agent: String,
    }
    let payload: Input = serde_json::from_str(&input)
        .map_err(|e| extism_pdk::Error::msg(format!("parse error: {}", e)))?;
    Ok(do_list_evolved(&payload.agent).to_json())
}

#[plugin_fn]
pub fn maintenance(_input: String) -> FnResult<String> {
    Ok(do_maintenance().to_json())
}
