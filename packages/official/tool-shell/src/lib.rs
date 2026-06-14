use weft_package_sdk::*;

const PACKAGE_NAME: &str = "tool-shell";
const CAPABILITY_NAME: &str = "tool.shell";

#[plugin_fn]
pub fn init(_input: String) -> FnResult<String> {
    log_info("tool-shell initialized");
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
            "actions": ["describe", "health", "run"],
        })),
        "health" => PackageResult::ok(serde_json::json!({"healthy": true, "package": PACKAGE_NAME})),
        "run" | "call" => {
            let command = req
                .data
                .get("command")
                .and_then(|value| value.as_str())
                .unwrap_or("");
            let args: Vec<String> = req
                .data
                .get("args")
                .and_then(|value| value.as_array())
                .map(|items| {
                    items
                        .iter()
                        .filter_map(|item| item.as_str().map(str::to_string))
                        .collect()
                })
                .unwrap_or_default();
            let arg_refs = args.iter().map(|item| item.as_str()).collect::<Vec<_>>();
            match exec_command(command, &arg_refs) {
                Ok(output) => PackageResult::ok(serde_json::json!({
                    "status": output.status,
                    "stdout": output.stdout,
                    "stderr": output.stderr,
                })),
                Err(error) => PackageResult::err(error),
            }
        }
        _ => PackageResult::err(format!("unknown action: {}", req.action)),
    };

    Ok(result.to_json())
}

