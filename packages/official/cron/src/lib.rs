//! Cron package — scheduled task management.

use weft_package_sdk::*;
use serde::{Deserialize, Serialize};

#[cfg(test)]
mod native_test_extism_stubs {
    #[no_mangle]
    pub extern "C" fn input_length() -> u64 {
        0
    }

    #[no_mangle]
    pub extern "C" fn input_load_u64(_offset: u64) -> u64 {
        0
    }

    #[no_mangle]
    pub extern "C" fn input_load_u8(_offset: u64) -> u8 {
        0
    }

    #[no_mangle]
    pub extern "C" fn load_u8(_offset: u64) -> u8 {
        0
    }

    #[no_mangle]
    pub extern "C" fn load_u64(_offset: u64) -> u64 {
        0
    }

    #[no_mangle]
    pub extern "C" fn store_u8(_offset: u64, _value: u8) {}

    #[no_mangle]
    pub extern "C" fn store_u64(_offset: u64, _value: u64) {}

    #[no_mangle]
    pub extern "C" fn output_set(_offset: u64, _length: u64) {}

    #[no_mangle]
    pub extern "C" fn error_set(_offset: u64) {}

    #[no_mangle]
    pub extern "C" fn alloc(_length: u64) -> u64 {
        0
    }

    #[no_mangle]
    pub extern "C" fn length(_offset: u64) -> u64 {
        0
    }

    #[no_mangle]
    pub extern "C" fn length_unsafe(_offset: u64) -> u64 {
        0
    }

    macro_rules! extism_log_stub {
        ($($name:ident),* $(,)?) => {
            $(
                #[no_mangle]
                pub extern "C" fn $name(_offset: u64) {}
            )*
        };
    }

    extism_log_stub!(log_trace, log_debug, log_info, log_warn, log_error);

    macro_rules! host_string_stub {
        ($($name:ident),* $(,)?) => {
            $(
                #[no_mangle]
                pub extern "C" fn $name(_offset: u64) -> u64 {
                    0
                }
            )*
        };
    }

    host_string_stub!(
        host_log,
        host_kv_get,
        host_env_get,
        host_kv_list,
        host_read_file,
        host_list_dir,
        host_exec,
        host_exec_advanced,
        host_chat_completion,
        host_call_package,
        host_call_package_ws,
        host_process_spawn,
        host_process_stop,
        host_process_status,
        host_process_write_stdin,
        host_process_read_stdout,
        host_sqlite_query,
        host_sqlite_execute,
        host_sqlite_batch,
    );

    macro_rules! host_void_stub {
        ($($name:ident),* $(,)?) => {
            $(
                #[no_mangle]
                pub extern "C" fn $name(_offset: u64) {}
            )*
        };
    }

    host_void_stub!(host_kv_set, host_kv_delete, host_write_file);
}

#[derive(Serialize, Deserialize, Clone)]
struct CronJob {
    name: String,
    agent: String,
    interval_ms: u64,
    prompt: String,
    #[serde(default)]
    last_run: u64,
}

const JOBS_INDEX_KEY: &str = "cron:jobs:__index";

fn current_time_ms() -> u64 {
    now_ms()
}

fn job_is_due(job: &CronJob, now_ms: u64) -> bool {
    if job.last_run == 0 {
        return true;
    }

    now_ms >= job.last_run.saturating_add(job.interval_ms)
}

fn get_jobs() -> Vec<CronJob> {
    match kv_get(JOBS_INDEX_KEY) {
        Some(json) => serde_json::from_str(&json).unwrap_or_default(),
        None => vec![],
    }
}

fn save_jobs(jobs: &[CronJob]) {
    let json = serde_json::to_string(jobs).unwrap_or_else(|_| "[]".into());
    kv_set(JOBS_INDEX_KEY, &json);
}

fn maintenance_package_candidates(package: &str) -> Vec<String> {
    match package.trim() {
        "skills" | "skills-runtime" => vec!["skills".into(), "skills-runtime".into()],
        other => vec![other.to_string()],
    }
}
fn maintenance_call(package: &str, action: &str, input: serde_json::Value) -> serde_json::Value {
    let payload = input.to_string();
    let mut last_error = String::new();
    for candidate in maintenance_package_candidates(package) {
        match call_package(&candidate, action, &payload) {
            Ok(response) => {
                let parsed = serde_json::from_str::<serde_json::Value>(&response)
                    .unwrap_or_else(|_| serde_json::json!({ "raw": response }));
                if parsed.get("status").and_then(|value| value.as_str()) == Some("error") {
                    last_error = parsed
                        .get("error")
                        .and_then(|value| value.as_str())
                        .unwrap_or("unknown error")
                        .to_string();
                    continue;
                }
                log_info(&format!(
                    "cron: maintenance {}.{} completed",
                    candidate, action
                ));
                return serde_json::json!({
                    "package": candidate,
                    "action": action,
                    "response": parsed,
                });
            }
            Err(error) => {
                last_error = error.to_string();
            }
        }
    }
    log_warn(&format!(
        "cron: maintenance {}.{} failed: {}",
        package, action, last_error
    ));
    serde_json::json!({
        "package": package,
        "action": action,
        "error": last_error,
    })
}

fn maintenance_targets() -> Vec<(&'static str, &'static str, serde_json::Value)> {
    vec![
        (
            "memory",
            "cleanup_expired",
            serde_json::json!({ "agent": "*" }),
        ),
        ("skills", "list_available", serde_json::json!({})),
        (
            "skills",
            "maintenance",
            serde_json::json!({
                "validate": true,
                "promote": true,
                "archive": true,
                "cleanup": true,
                "metrics": true,
            }),
        ),
    ]
}

fn run_maintenance() -> Vec<serde_json::Value> {
    maintenance_targets()
        .into_iter()
        .map(|(package, action, input)| maintenance_call(package, action, input))
        .collect()
}

#[plugin_fn]
pub fn init(_input: String) -> FnResult<String> {
    log_info("cron package initialized");
    Ok(PackageResult::ok_empty().to_json())
}

#[plugin_fn]
pub fn handle_ws_message(input: String) -> FnResult<String> {
    let req: WsRequest = serde_json::from_str(&input).unwrap_or(WsRequest {
        action: String::new(),
        data: serde_json::Value::Null,
    });

    let result = match req.action.as_str() {
        "add_job" => {
            let job: CronJob = serde_json::from_value(req.data).unwrap_or(CronJob {
                name: String::new(),
                agent: String::new(),
                interval_ms: 0,
                prompt: String::new(),
                last_run: 0,
            });
            do_add_job(&job)
        }
        "remove_job" => {
            let name = req.data["name"].as_str().unwrap_or("");
            do_remove_job(name)
        }
        "list_jobs" => {
            let agent = req.data["agent"].as_str().unwrap_or("");
            do_list_jobs(agent)
        }
        _ => PackageResult::err(format!("unknown action: {}", req.action)),
    };

    Ok(result.to_json())
}

#[plugin_fn]
pub fn tick(_input: String) -> FnResult<String> {
    // Called periodically by the host.
    // Check each job and execute if interval has elapsed.
    // Note: WASM doesn't have real time access, so we use a simple counter.
    // The host should pass current timestamp as input in production.
    let now_ms = current_time_ms();
    let mut jobs = get_jobs();
    let mut executed = Vec::new();

    for job in &mut jobs {
        if job.interval_ms == 0 {
            log_warn(&format!(
                "cron: job '{}' has invalid interval_ms=0 and was skipped",
                job.name
            ));
            continue;
        }

        if !job_is_due(job, now_ms) {
            continue;
        }

        let input = serde_json::json!({
            "agent": job.agent,
            "content": job.prompt,
        })
        .to_string();

        match call_package("agent-runtime", "send_message", &input) {
            Ok(_) => {
                executed.push(job.name.clone());
                job.last_run = now_ms;
                log_info(&format!(
                    "cron: executed job '{}' for agent '{}'",
                    job.name, job.agent
                ));
            }
            Err(e) => {
                log_warn(&format!("cron: job '{}' failed: {}", job.name, e));
            }
        }
    }

    let _ = call_package(
        "workflow-orchestrator",
        "tick",
        &format!(r#"{{"now_ms":{}}}"#, now_ms),
    );
    let maintenance = run_maintenance();

    save_jobs(&jobs);
    Ok(PackageResult::ok(serde_json::json!({
        "executed": executed,
        "maintenance": maintenance,
    }))
    .to_json())
}

#[plugin_fn]
pub fn add_job(input: String) -> FnResult<String> {
    let job: CronJob = serde_json::from_str(&input)
        .map_err(|e| extism_pdk::Error::msg(format!("parse error: {}", e)))?;
    Ok(do_add_job(&job).to_json())
}

fn do_add_job(job: &CronJob) -> PackageResult {
    if job.name.is_empty() || job.agent.is_empty() || job.prompt.is_empty() {
        return PackageResult::err("missing name, agent, or prompt");
    }
    if job.interval_ms == 0 {
        return PackageResult::err("invalid interval_ms: must be greater than zero");
    }

    let mut jobs = get_jobs();
    jobs.retain(|j| j.name != job.name);
    jobs.push(job.clone());
    save_jobs(&jobs);

    log_info(&format!(
        "cron: added job '{}' (interval: {}ms)",
        job.name, job.interval_ms
    ));
    PackageResult::ok_empty()
}

#[plugin_fn]
pub fn remove_job(input: String) -> FnResult<String> {
    #[derive(Deserialize)]
    struct Input {
        name: String,
    }
    let p: Input = serde_json::from_str(&input)
        .map_err(|e| extism_pdk::Error::msg(format!("parse error: {}", e)))?;
    Ok(do_remove_job(&p.name).to_json())
}

fn do_remove_job(name: &str) -> PackageResult {
    let mut jobs = get_jobs();
    jobs.retain(|j| j.name != name);
    save_jobs(&jobs);
    PackageResult::ok_empty()
}

#[plugin_fn]
pub fn list_jobs(input: String) -> FnResult<String> {
    #[derive(Deserialize)]
    struct Input {
        #[serde(default)]
        agent: String,
    }
    let p: Input = serde_json::from_str(&input).unwrap_or(Input {
        agent: String::new(),
    });
    Ok(do_list_jobs(&p.agent).to_json())
}

fn do_list_jobs(agent: &str) -> PackageResult {
    let jobs = get_jobs();
    let filtered: Vec<&CronJob> = if agent.is_empty() {
        jobs.iter().collect()
    } else {
        jobs.iter().filter(|j| j.agent == agent).collect()
    };

    let list: Vec<serde_json::Value> = filtered
        .iter()
        .map(|j| {
            serde_json::json!({
                "name": j.name,
                "agent": j.agent,
                "interval_ms": j.interval_ms,
                "prompt": j.prompt,
            })
        })
        .collect();

    PackageResult::ok(serde_json::json!({"jobs": list}))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maintenance_targets_include_memory_and_skills_maintenance_calls_in_tick_order() {
        let targets = maintenance_targets();
        let shaped: Vec<_> = targets
            .iter()
            .map(|(package, action, input)| (*package, *action, input.clone()))
            .collect();

        assert_eq!(
            shaped,
            vec![
                (
                    "memory",
                    "cleanup_expired",
                    serde_json::json!({ "agent": "*" })
                ),
                ("skills", "list_available", serde_json::json!({})),
                (
                    "skills",
                    "maintenance",
                    serde_json::json!({
                        "validate": true,
                        "promote": true,
                        "archive": true,
                        "cleanup": true,
                        "metrics": true,
                    })
                ),
            ]
        );
    }

    #[test]
    fn job_is_due_returns_true_for_first_run_and_after_interval() {
        let mut job = CronJob {
            name: "demo".into(),
            agent: "agent-a".into(),
            interval_ms: 1_000,
            prompt: "hello".into(),
            last_run: 0,
        };

        assert!(job_is_due(&job, 10));
        job.last_run = 10;
        assert!(!job_is_due(&job, 500));
        assert!(job_is_due(&job, 1_010));
    }

    #[test]
    fn do_add_job_rejects_zero_interval() {
        let job = CronJob {
            name: "demo".into(),
            agent: "agent-a".into(),
            interval_ms: 0,
            prompt: "hello".into(),
            last_run: 0,
        };

        let result = do_add_job(&job);
        assert_eq!(result.status, "error");
    }
}

