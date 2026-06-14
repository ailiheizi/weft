use weft_package_sdk::*;

const PACKAGE_NAME: &str = "prompt-system";
const CAPABILITY_NAME: &str = "prompt.system";

fn describe() -> PackageResult {
    PackageResult::ok(serde_json::json!({
        "package": PACKAGE_NAME,
        "capability": CAPABILITY_NAME,
        "runtime": "wasm",
        "actions": ["describe", "health", "render"],
    }))
}

#[plugin_fn]
pub fn init(_input: String) -> FnResult<String> {
    log_info("prompt-system initialized");
    Ok(PackageResult::ok_empty().to_json())
}

#[plugin_fn]
pub fn handle_ws_message(input: String) -> FnResult<String> {
    let req: WsRequest = serde_json::from_str(&input).unwrap_or(WsRequest {
        action: String::new(),
        data: serde_json::Value::Null,
    });

    let result = match req.action.as_str() {
        "describe" => describe(),
        "health" => PackageResult::ok(serde_json::json!({"healthy": true, "package": PACKAGE_NAME})),
        "render" | "call" => PackageResult::ok(serde_json::json!({
            "package": PACKAGE_NAME,
            "capability": CAPABILITY_NAME,
            "system_prompt": req.data.get("system_prompt").cloned().unwrap_or(serde_json::Value::String(String::new())),
        })),
        _ => PackageResult::err(format!("unknown action: {}", req.action)),
    };

    Ok(result.to_json())
}

