use weft_package_sdk::*;

const PACKAGE_NAME: &str = "image-gen";
const CAPABILITY_NAME: &str = "image.generate";

// KV key prefix for async generation jobs.
fn job_key(job_id: &str) -> String {
    format!("image-gen:job:{job_id}")
}

#[plugin_fn]
pub fn init(_input: String) -> FnResult<String> {
    log_info("image-gen initialized");
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
            "actions": ["describe", "health", "generate", "submit", "poll", "fetch"],
            "note": "generate = synchronous real image gen (OpenAI-compatible /v1/images/generations); submit/poll/fetch = async state machine skeleton",
        })),
        "health" => PackageResult::ok(serde_json::json!({
            "healthy": true,
            "package": PACKAGE_NAME,
        })),
        "generate" => generate(&req.data),
        "submit" => submit(&req.data),
        "poll" => poll(&req.data),
        "fetch" => fetch(&req.data),
        other => PackageResult::err(format!("unknown action: {other}")),
    };

    Ok(result.to_json())
}

/// Synchronous real image generation via an OpenAI-compatible
/// `/v1/images/generations` endpoint (e.g. apiyi gateway).
/// data: {prompt, model?, size?, base_url, api_key}.
/// Returns {b64_json, model} on success (caller decides how to persist the image).
fn generate(data: &serde_json::Value) -> PackageResult {
    let prompt = data.get("prompt").and_then(|v| v.as_str()).unwrap_or("");
    if prompt.trim().is_empty() {
        return PackageResult::err("generate requires 'prompt'");
    }
    let base_url = data
        .get("base_url")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .filter(|s| !s.trim().is_empty())
        .or_else(|| env_get("WEFT_IMAGE_BASE_URL").filter(|s| !s.trim().is_empty()))
        .unwrap_or_else(|| "https://api.apiyi.com".to_string());
    let base_url = base_url.trim_end_matches('/');
    // api_key 优先用参数，缺省回退到 Core 环境变量（前端不持有密钥）。
    let api_key = data
        .get("api_key")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .filter(|s| !s.trim().is_empty())
        .or_else(|| env_get("WEFT_IMAGE_API_KEY").filter(|s| !s.trim().is_empty()))
        .unwrap_or_default();
    if api_key.trim().is_empty() {
        return PackageResult::err(
            "generate requires 'api_key' (传参或设置 WEFT_IMAGE_API_KEY 环境变量)",
        );
    }
    let model = data
        .get("model")
        .and_then(|v| v.as_str())
        .unwrap_or("gpt-image-2-vip");
    let size = data.get("size").and_then(|v| v.as_str()).unwrap_or("1024x1024");

    let req_body = serde_json::json!({
        "model": model,
        "prompt": prompt,
        "n": 1,
        "size": size,
    })
    .to_string();

    let url = format!("{base_url}/v1/images/generations");
    let auth = format!("Bearer {api_key}");
    let resp = match http_request(
        "POST",
        &url,
        &[("Authorization", auth.as_str()), ("Content-Type", "application/json")],
        &req_body,
    ) {
        Ok(body) => body,
        Err(error) => return PackageResult::err(format!("image API call failed: {error}")),
    };

    // Parse OpenAI image response: {data:[{b64_json | url}]}
    let parsed: serde_json::Value = match serde_json::from_str(&resp) {
        Ok(v) => v,
        Err(error) => return PackageResult::err(format!("invalid image API response: {error}")),
    };
    let first = parsed.get("data").and_then(|d| d.as_array()).and_then(|a| a.first());
    match first {
        Some(item) => {
            let img_url = item.get("url").and_then(|v| v.as_str());
            if let Some(b64) = item.get("b64_json").and_then(|v| v.as_str()) {
                // Persist the image to a file and return its PATH (not the 2MB
                // base64) — large media must not cross the WASM/LLM boundary.
                let out_path = data
                    .get("output_path")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| {
                        format!("./workspace/image-gen/img-{}.png", prompt.chars().count())
                    });
                match write_file_base64(&out_path, b64) {
                    Ok(saved) => PackageResult::ok(serde_json::json!({
                        "model": model,
                        "prompt": prompt,
                        "output_path": saved,
                    })),
                    Err(error) => PackageResult::err(format!("failed to save image: {error}")),
                }
            } else if let Some(u) = img_url {
                // Provider returned a URL instead of base64 — pass it through.
                PackageResult::ok(serde_json::json!({
                    "model": model,
                    "prompt": prompt,
                    "url": u,
                }))
            } else {
                PackageResult::err("image API response had neither b64_json nor url")
            }
        }
        None => PackageResult::err(format!("image API returned no data: {}", resp.chars().take(200).collect::<String>())),
    }
}

/// Submit an image generation job. Returns a job_id.
/// MOCK: stores a job record in host_kv with status "done".
/// TODO: call host_http_request to submit to a real image provider
/// (Seedance / Kling / SD-WebUI / etc), store the returned remote job id.
fn submit(data: &serde_json::Value) -> PackageResult {
    let prompt = data.get("prompt").and_then(|v| v.as_str()).unwrap_or("");
    if prompt.trim().is_empty() {
        return PackageResult::err("submit requires 'prompt'");
    }

    // Deterministic mock id from prompt length + a fixed tag (no RNG in wasm).
    let job_id = format!("imgjob-{}", prompt.chars().count());

    // Persist job state. Real impl would store the remote provider job id +
    // status "pending" until poll confirms completion.
    let record = serde_json::json!({
        "job_id": job_id,
        "prompt": prompt,
        "status": "done", // MOCK: instantly done. Real: "pending".
        // MOCK output path; real impl writes downloaded image here on fetch.
        "output_path": format!("./workspace/image-gen/{job_id}.png"),
    });
    kv_set(&job_key(&job_id), &record.to_string());

    PackageResult::ok(serde_json::json!({ "job_id": job_id, "status": "submitted" }))
}

/// Poll a job's status.
/// TODO: real impl queries provider API for job status.
fn poll(data: &serde_json::Value) -> PackageResult {
    let job_id = data.get("job_id").and_then(|v| v.as_str()).unwrap_or("");
    if job_id.trim().is_empty() {
        return PackageResult::err("poll requires 'job_id'");
    }

    match kv_get(&job_key(job_id)) {
        Some(raw) => match serde_json::from_str::<serde_json::Value>(&raw) {
            Ok(record) => PackageResult::ok(serde_json::json!({
                "job_id": job_id,
                "status": record.get("status").and_then(|v| v.as_str()).unwrap_or("unknown"),
            })),
            Err(error) => PackageResult::err(format!("corrupt job record: {error}")),
        },
        None => PackageResult::err(format!("unknown job_id: {job_id}")),
    }
}

/// Fetch a completed job's output (image file path in workspace).
/// TODO: real impl downloads the image via host_http_request to workspace
/// and returns the local path.
fn fetch(data: &serde_json::Value) -> PackageResult {
    let job_id = data.get("job_id").and_then(|v| v.as_str()).unwrap_or("");
    if job_id.trim().is_empty() {
        return PackageResult::err("fetch requires 'job_id'");
    }

    match kv_get(&job_key(job_id)) {
        Some(raw) => match serde_json::from_str::<serde_json::Value>(&raw) {
            Ok(record) => {
                let status = record.get("status").and_then(|v| v.as_str()).unwrap_or("");
                if status != "done" {
                    return PackageResult::err(format!("job not done (status={status})"));
                }
                PackageResult::ok(serde_json::json!({
                    "job_id": job_id,
                    "output_path": record.get("output_path").cloned().unwrap_or_default(),
                }))
            }
            Err(error) => PackageResult::err(format!("corrupt job record: {error}")),
        },
        None => PackageResult::err(format!("unknown job_id: {job_id}")),
    }
}
