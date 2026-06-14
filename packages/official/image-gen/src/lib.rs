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
            "actions": ["describe", "health", "submit", "poll", "fetch"],
            "note": "async submit/poll/fetch state machine (mock skeleton; real provider API pending)",
        })),
        "health" => PackageResult::ok(serde_json::json!({
            "healthy": true,
            "package": PACKAGE_NAME,
        })),
        "submit" => submit(&req.data),
        "poll" => poll(&req.data),
        "fetch" => fetch(&req.data),
        other => PackageResult::err(format!("unknown action: {other}")),
    };

    Ok(result.to_json())
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
