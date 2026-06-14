//! Channels package — message channels (webhook, inter-agent communication).

use weft_package_sdk::*;
use serde::{Deserialize, Serialize};

fn inbox_key(agent: &str) -> String {
    format!("channels:inbox:{}", agent)
}

fn channels_key(agent: &str) -> String {
    format!("channels:config:{}", agent)
}

fn get_inbox(agent: &str) -> Vec<serde_json::Value> {
    match kv_get(&inbox_key(agent)) {
        Some(json) => serde_json::from_str(&json).unwrap_or_default(),
        None => vec![],
    }
}

fn save_inbox(agent: &str, msgs: &[serde_json::Value]) {
    let json = serde_json::to_string(msgs).unwrap_or_else(|_| "[]".into());
    kv_set(&inbox_key(agent), &json);
}

#[plugin_fn]
pub fn init(_input: String) -> FnResult<String> {
    log_info("channels package initialized");
    Ok(PackageResult::ok_empty().to_json())
}

#[plugin_fn]
pub fn handle_ws_message(input: String) -> FnResult<String> {
    let req: WsRequest = serde_json::from_str(&input).unwrap_or(WsRequest {
        action: String::new(), data: serde_json::Value::Null,
    });

    let result = match req.action.as_str() {
        "send" => {
            let from = req.data["from"].as_str().unwrap_or("");
            let to = req.data["to"].as_str().unwrap_or("");
            let content = req.data["content"].as_str().unwrap_or("");
            do_send(from, to, content)
        }
        "get_inbox" => {
            let agent = req.data["agent"].as_str().unwrap_or("");
            do_get_inbox(agent)
        }
        "register_channel" => {
            let agent = req.data["agent"].as_str().unwrap_or("");
            let config = &req.data["config"];
            do_register_channel(agent, config)
        }
        "list_channels" => {
            let agent = req.data["agent"].as_str().unwrap_or("");
            do_list_channels(agent)
        }
        _ => PackageResult::err(format!("unknown action: {}", req.action)),
    };

    Ok(result.to_json())
}

#[plugin_fn]
pub fn send(input: String) -> FnResult<String> {
    #[derive(Deserialize)]
    struct Input { from: String, to: String, content: String }
    let p: Input = serde_json::from_str(&input)
        .map_err(|e| extism_pdk::Error::msg(format!("parse error: {}", e)))?;
    Ok(do_send(&p.from, &p.to, &p.content).to_json())
}

fn do_send(from: &str, to: &str, content: &str) -> PackageResult {
    if to.is_empty() || content.is_empty() {
        return PackageResult::err("missing 'to' or 'content'");
    }

    let mut inbox = get_inbox(to);
    inbox.push(serde_json::json!({
        "from": from,
        "content": content,
    }));
    save_inbox(to, &inbox);

    log_info(&format!("channel: {} -> {}", from, to));
    PackageResult::ok_empty()
}

#[plugin_fn]
pub fn get_inbox_fn(input: String) -> FnResult<String> {
    #[derive(Deserialize)]
    struct Input { agent: String }
    let p: Input = serde_json::from_str(&input)
        .map_err(|e| extism_pdk::Error::msg(format!("parse error: {}", e)))?;
    Ok(do_get_inbox(&p.agent).to_json())
}

fn do_get_inbox(agent: &str) -> PackageResult {
    let msgs = get_inbox(agent);
    // Clear inbox after reading
    save_inbox(agent, &[]);
    PackageResult::ok(serde_json::json!({"messages": msgs}))
}

fn do_register_channel(agent: &str, config: &serde_json::Value) -> PackageResult {
    let key = channels_key(agent);
    let mut channels: Vec<serde_json::Value> = match kv_get(&key) {
        Some(json) => serde_json::from_str(&json).unwrap_or_default(),
        None => vec![],
    };
    channels.push(config.clone());
    let json = serde_json::to_string(&channels).unwrap_or_else(|_| "[]".into());
    kv_set(&key, &json);
    PackageResult::ok_empty()
}

fn do_list_channels(agent: &str) -> PackageResult {
    let key = channels_key(agent);
    let channels: Vec<serde_json::Value> = match kv_get(&key) {
        Some(json) => serde_json::from_str(&json).unwrap_or_default(),
        None => vec![],
    };
    PackageResult::ok(serde_json::json!({"channels": channels}))
}

#[plugin_fn]
pub fn register_channel(input: String) -> FnResult<String> {
    #[derive(Deserialize)]
    struct Input { agent: String, config: serde_json::Value }
    let p: Input = serde_json::from_str(&input)
        .map_err(|e| extism_pdk::Error::msg(format!("parse error: {}", e)))?;
    Ok(do_register_channel(&p.agent, &p.config).to_json())
}

#[plugin_fn]
pub fn list_channels(input: String) -> FnResult<String> {
    #[derive(Deserialize)]
    struct Input { agent: String }
    let p: Input = serde_json::from_str(&input)
        .map_err(|e| extism_pdk::Error::msg(format!("parse error: {}", e)))?;
    Ok(do_list_channels(&p.agent).to_json())
}

