use weft_package_sdk::*;

const PACKAGE_NAME: &str = "tool-files";
const CAPABILITY_NAME: &str = "tool.files";

#[plugin_fn]
pub fn init(_input: String) -> FnResult<String> {
    log_info("tool-files initialized");
    Ok(PackageResult::ok_empty().to_json())
}

#[plugin_fn]
pub fn handle_ws_message(input: String) -> FnResult<String> {
    let req: WsRequest = serde_json::from_str(&input).unwrap_or(WsRequest {
        action: String::new(),
        data: serde_json::Value::Null,
    });

    let result = match req.action.as_str() {
        "describe" => PackageResult::ok(serde_json::json!({
            "package": PACKAGE_NAME,
            "capability": CAPABILITY_NAME,
            "runtime": "wasm",
            "actions": ["describe", "health", "list", "read", "write"],
        })),
        "health" => PackageResult::ok(serde_json::json!({"healthy": true, "package": PACKAGE_NAME})),
        "list" => {
            let path = req
                .data
                .get("path")
                .and_then(|value| value.as_str())
                .unwrap_or(".");
            match list_dir(path) {
                Ok(entries) => PackageResult::ok(serde_json::json!({"entries": entries})),
                Err(error) => PackageResult::err(error),
            }
        }
        "read" => {
            let path = req
                .data
                .get("path")
                .and_then(|value| value.as_str())
                .unwrap_or("");
            match read_file(path) {
                Ok(content) => PackageResult::ok(serde_json::json!({"content": content})),
                Err(error) => PackageResult::err(error),
            }
        }
        "write" => {
            let path = req
                .data
                .get("path")
                .and_then(|value| value.as_str())
                .unwrap_or("");
            let content = req
                .data
                .get("content")
                .and_then(|value| value.as_str())
                .unwrap_or("");
            write_file(path, content);
            PackageResult::ok(serde_json::json!({"written": true, "path": path}))
        }
        "call" => PackageResult::ok(req.data),
        _ => PackageResult::err(format!("unknown action: {}", req.action)),
    };

    Ok(result.to_json())
}

