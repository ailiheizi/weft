use crate::api::openai_compat::AppState;
use crate::config::AppConfig;
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::{self, BufRead, Write};

#[derive(Debug, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: Option<Value>,
    pub method: String,
    #[serde(default)]
    pub params: Option<Value>,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct JsonRpcResponse {
    pub jsonrpc: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct JsonRpcError {
    pub code: i64,
    pub message: String,
}

pub async fn handle_request(request: JsonRpcRequest, state: Option<&AppState>) -> JsonRpcResponse {
    if request.jsonrpc != "2.0" {
        return error_response(request.id, -32600, "Invalid Request");
    }

    match request.method.as_str() {
        "health" => success_response(request.id, health_result()),
        "models/list" => {
            if let Some(state) = state {
                let config = state.config.read().await;
                success_response(request.id, model_list_result(&config))
            } else {
                success_response(request.id, model_list_from_providers(&[]))
            }
        }
        _ => error_response(request.id, -32601, "Method not found"),
    }
}

pub fn health_result() -> Value {
    json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION"),
    })
}

pub fn model_list_result(config: &AppConfig) -> Value {
    let providers: Vec<(&str, &[String])> = config
        .providers
        .iter()
        .map(|provider| (provider.name.as_str(), provider.models.as_slice()))
        .collect();
    model_list_from_providers(&providers)
}

fn model_list_from_providers(providers: &[(&str, &[String])]) -> Value {
    let models: Vec<Value> = providers
        .iter()
        .flat_map(|(provider, models)| {
            models.iter().map(move |model| {
                json!({
                    "id": model,
                    "object": "model",
                    "owned_by": provider,
                })
            })
        })
        .collect();

    json!({
        "object": "list",
        "data": models,
    })
}

pub async fn serve_stdio(state: Option<AppState>) -> Result<()> {
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }

        let response = match serde_json::from_str::<JsonRpcRequest>(&line) {
            Ok(request) => handle_request(request, state.as_ref()).await,
            Err(_) => error_response(None, -32700, "Parse error"),
        };
        serde_json::to_writer(&mut stdout, &response)?;
        stdout.write_all(b"\n")?;
        stdout.flush()?;
    }

    Ok(())
}

pub fn parse_single_request(input: &str) -> Result<JsonRpcRequest> {
    serde_json::from_str(input).map_err(|error| anyhow!("invalid JSON-RPC request: {error}"))
}

fn success_response(id: Option<Value>, result: Value) -> JsonRpcResponse {
    JsonRpcResponse {
        jsonrpc: "2.0",
        id,
        result: Some(result),
        error: None,
    }
}

fn error_response(id: Option<Value>, code: i64, message: &str) -> JsonRpcResponse {
    JsonRpcResponse {
        jsonrpc: "2.0",
        id,
        result: None,
        error: Some(JsonRpcError {
            code,
            message: message.to_string(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn handles_health_request() {
        let request = parse_single_request(r#"{"jsonrpc":"2.0","id":1,"method":"health"}"#)
            .expect("valid request");

        let response = handle_request(request, None).await;

        assert_eq!(response.id, Some(json!(1)));
        assert_eq!(response.error, None);
        assert_eq!(response.result.unwrap()["status"], "ok");
    }

    #[tokio::test]
    async fn handles_model_list_without_state() {
        let request =
            parse_single_request(r#"{"jsonrpc":"2.0","id":"models","method":"models/list"}"#)
                .expect("valid request");

        let response = handle_request(request, None).await;

        assert_eq!(response.id, Some(json!("models")));
        assert_eq!(response.error, None);
        assert_eq!(response.result.unwrap(), json!({"object":"list","data":[]}));
    }

    #[tokio::test]
    async fn rejects_unknown_method() {
        let request = parse_single_request(r#"{"jsonrpc":"2.0","id":2,"method":"missing"}"#)
            .expect("valid request");

        let response = handle_request(request, None).await;

        assert_eq!(
            response.error,
            Some(JsonRpcError {
                code: -32601,
                message: "Method not found".to_string(),
            })
        );
    }
}
