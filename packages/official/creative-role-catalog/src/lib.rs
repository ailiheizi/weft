use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use weft_package_sdk::*;

const PACKAGE_NAME: &str = "creative-role-catalog";
const ROLE_CATALOG_CAPABILITY: &str = "team.role.catalog";
const SCHEMA_VERSION: u32 = 1;
const DEFAULT_ROLE_SET_ID: &str = "creative-v1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct TeamRoleDefinition {
    role_id: String,
    title: String,
    summary: String,
    responsibilities: Vec<String>,
    default_phase_ids: Vec<String>,
    capability_hints: Vec<String>,
    delegate_targets: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct TeamRoleCatalog {
    schema_version: u32,
    catalog_id: String,
    title: String,
    roles: Vec<TeamRoleDefinition>,
}

fn role_catalog() -> TeamRoleCatalog {
    TeamRoleCatalog {
        schema_version: SCHEMA_VERSION,
        catalog_id: DEFAULT_ROLE_SET_ID.to_string(),
        title: "AI 导演创作角色目录".to_string(),
        roles: vec![
            TeamRoleDefinition {
                role_id: "director".to_string(),
                title: "总导演".to_string(),
                summary: "Owns the full creative arc, aligns goals, and routes work across the creative team.".to_string(),
                responsibilities: vec![
                    "梳理创作目标与约束".to_string(),
                    "在创作阶段间分派任务".to_string(),
                    "汇总脚本、分镜与剪辑方案".to_string(),
                ],
                default_phase_ids: vec!["intake".to_string(), "direct".to_string(), "done".to_string()],
                capability_hints: vec![
                    "workflow.template.creative".to_string(),
                    "team.taskboard".to_string(),
                    "director.plan".to_string(),
                ],
                delegate_targets: vec![
                    "copywriter".to_string(),
                    "storyboard".to_string(),
                    "editor".to_string(),
                ],
            },
            TeamRoleDefinition {
                role_id: "copywriter".to_string(),
                title: "文案".to_string(),
                summary: "Turns briefs into scripts, messaging, and narrative copy for the production.".to_string(),
                responsibilities: vec![
                    "产出脚本与文案草稿".to_string(),
                    "统一叙事口吻与信息层级".to_string(),
                    "为后续分镜提供结构化文本".to_string(),
                ],
                default_phase_ids: vec!["script".to_string()],
                capability_hints: vec![
                    "workflow.template.creative".to_string(),
                    "team.context.shared".to_string(),
                ],
                delegate_targets: vec!["voice".to_string(), "storyboard".to_string()],
            },
            TeamRoleDefinition {
                role_id: "storyboard".to_string(),
                title: "分镜".to_string(),
                summary: "Converts script intent into shot structure, scene rhythm, and visual sequencing.".to_string(),
                responsibilities: vec![
                    "拆解镜头与场景节奏".to_string(),
                    "明确画面衔接与镜头意图".to_string(),
                    "为导演与剪辑提供视觉蓝图".to_string(),
                ],
                default_phase_ids: vec!["storyboard".to_string()],
                capability_hints: vec![
                    "workflow.template.creative".to_string(),
                    "team.context.shared".to_string(),
                ],
                delegate_targets: vec!["director".to_string(), "editor".to_string()],
            },
            TeamRoleDefinition {
                role_id: "voice".to_string(),
                title: "配音".to_string(),
                summary: "Develops spoken delivery, audio direction, and supporting voice assets.".to_string(),
                responsibilities: vec![
                    "整理旁白与配音要求".to_string(),
                    "匹配语气、节奏与情绪".to_string(),
                    "补充音频执行备注".to_string(),
                ],
                default_phase_ids: vec![],
                capability_hints: vec![
                    "team.context.shared".to_string(),
                    "session.events".to_string(),
                ],
                delegate_targets: vec!["editor".to_string()],
            },
            TeamRoleDefinition {
                role_id: "editor".to_string(),
                title: "剪辑".to_string(),
                summary: "Assembles the final cut from the approved direction, sequence, and creative assets.".to_string(),
                responsibilities: vec![
                    "执行剪辑与素材整合".to_string(),
                    "落实导演方案与节奏控制".to_string(),
                    "输出可交付的成片说明".to_string(),
                ],
                default_phase_ids: vec!["assemble".to_string()],
                capability_hints: vec![
                    "director.plan".to_string(),
                    "workflow.template.creative".to_string(),
                    "team.taskboard".to_string(),
                ],
                delegate_targets: vec!["voice".to_string()],
            },
        ],
    }
}

fn describe_result() -> PackageResult {
    PackageResult::ok(json!({
        "package": PACKAGE_NAME,
        "runtime": "wasm",
        "capabilities": [ROLE_CATALOG_CAPABILITY],
        "actions": {
            ROLE_CATALOG_CAPABILITY: ["describe", "health", "list_roles", "get_catalog"]
        },
    }))
}

#[plugin_fn]
pub fn init(_input: String) -> FnResult<String> {
    log_info("creative-role-catalog initialized");
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
        "health" => PackageResult::ok(json!({
            "healthy": true,
            "package": PACKAGE_NAME,
            "capabilities": [ROLE_CATALOG_CAPABILITY],
        })),
        "list_roles" | "get_catalog" | "call" => {
            PackageResult::ok(json!({ "catalog": role_catalog() }))
        }
        _ => PackageResult::err(format!("unknown action: {}", req.action)),
    };
    Ok(result.to_json())
}
