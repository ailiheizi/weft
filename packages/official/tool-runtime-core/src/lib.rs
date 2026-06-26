use base64::Engine;
use weft_package_sdk::*;
use serde::Deserialize;
use std::path::PathBuf;

const WINDOWS_PYTHON_CANDIDATES: &[(&str, &[&str])] = &[
    ("python", &[]),
    ("py", &["-3"]),
    ("py", &[]),
    (
        "C:\\Users\\26617\\scoop\\apps\\python\\current\\python.exe",
        &[],
    ),
];

#[cfg(all(test, not(target_arch = "wasm32")))]
#[allow(dead_code)]
mod native_test_host_stubs {
    macro_rules! extism_stub {
        ($name:ident () -> $ret:ty, $value:expr) => {
            #[no_mangle]
            pub extern "C" fn $name() -> $ret {
                $value
            }
        };
        ($name:ident ($($arg:ident: $ty:ty),*) -> $ret:ty, $value:expr) => {
            #[no_mangle]
            pub extern "C" fn $name($($arg: $ty),*) -> $ret {
                $(let _ = $arg;)*
                $value
            }
        };
    }

    macro_rules! weft_host_stub {
        ($name:ident) => {
            #[no_mangle]
            pub extern "C" fn $name(_input: u64) -> u64 {
                0
            }
        };
    }

    extism_stub!(input_length() -> u64, 0);
    extism_stub!(input_load_u8(_offs: u64) -> u8, 0);
    extism_stub!(input_load_u64(_offs: u64) -> u64, 0);
    extism_stub!(length(_offs: u64) -> u64, 0);
    extism_stub!(length_unsafe(_offs: u64) -> u64, 0);
    extism_stub!(alloc(_length: u64) -> u64, 0);
    extism_stub!(free(_offs: u64) -> (), ());
    extism_stub!(output_set(_offs: u64, _length: u64) -> (), ());
    extism_stub!(error_set(_offs: u64) -> (), ());
    extism_stub!(store_u8(_offs: u64, _data: u8) -> (), ());
    extism_stub!(load_u8(_offs: u64) -> u8, 0);
    extism_stub!(store_u64(_offs: u64, _data: u64) -> (), ());
    extism_stub!(load_u64(_offs: u64) -> u64, 0);
    extism_stub!(config_get(_offs: u64) -> u64, 0);
    extism_stub!(var_get(_offs: u64) -> u64, 0);
    extism_stub!(var_set(_offs: u64, _offs1: u64) -> (), ());
    extism_stub!(http_request(_req: u64, _body: u64) -> u64, 0);
    extism_stub!(http_status_code() -> i32, 500);
    extism_stub!(http_headers() -> u64, 0);
    extism_stub!(log_info(_offs: u64) -> (), ());
    extism_stub!(log_debug(_offs: u64) -> (), ());
    extism_stub!(log_warn(_offs: u64) -> (), ());
    extism_stub!(log_error(_offs: u64) -> (), ());
    extism_stub!(log_trace(_offs: u64) -> (), ());
    extism_stub!(get_log_level() -> i32, 0);

    weft_host_stub!(host_log);
    weft_host_stub!(host_kv_get);
    weft_host_stub!(host_kv_set);
    weft_host_stub!(host_kv_delete);
    weft_host_stub!(host_kv_list);
    weft_host_stub!(host_kv_compare_and_swap);
    weft_host_stub!(host_kv_increment);
    weft_host_stub!(host_read_file);
    weft_host_stub!(host_read_bytes);
    weft_host_stub!(host_read_byte_range);
    weft_host_stub!(host_write_file);
    weft_host_stub!(host_write_bytes);
    weft_host_stub!(host_write_byte_range);
    weft_host_stub!(host_append_file);
    weft_host_stub!(host_truncate_file);
    weft_host_stub!(host_list_dir);
    weft_host_stub!(host_walk_dir);
    weft_host_stub!(host_glob_paths);
    weft_host_stub!(host_watch_path);
    weft_host_stub!(host_watch_poll);
    weft_host_stub!(host_watch_stop);
    weft_host_stub!(host_stat_path);
    weft_host_stub!(host_canonicalize_path);
    weft_host_stub!(host_create_dir);
    weft_host_stub!(host_make_temp_dir);
    weft_host_stub!(host_make_temp_file);
    weft_host_stub!(host_get_current_dir);
    weft_host_stub!(host_set_current_dir);
    weft_host_stub!(host_touch_path);
    weft_host_stub!(host_set_file_times);
    weft_host_stub!(host_atomic_write_file);
    weft_host_stub!(host_fsync_path);
    weft_host_stub!(host_fsync_data_path);
    weft_host_stub!(host_lock_file);
    weft_host_stub!(host_unlock_file);
    weft_host_stub!(host_set_path_readonly);
    weft_host_stub!(host_hard_link_path);
    weft_host_stub!(host_read_link_path);
    weft_host_stub!(host_symlink_path);
    weft_host_stub!(host_same_file_path);
    weft_host_stub!(host_remove_path);
    weft_host_stub!(host_remove_path_advanced);
    weft_host_stub!(host_copy_path);
    weft_host_stub!(host_copy_path_advanced);
    weft_host_stub!(host_move_path);
    weft_host_stub!(host_move_path_advanced);
    weft_host_stub!(host_exec);
    weft_host_stub!(host_exec_advanced);
    weft_host_stub!(host_http_request);
    weft_host_stub!(host_sqlite_execute);
    weft_host_stub!(host_sqlite_query);
    weft_host_stub!(host_sqlite_batch);
    weft_host_stub!(host_sqlite_tables);
    weft_host_stub!(host_sqlite_table_info);
    weft_host_stub!(host_sqlite_pragma_get);
    weft_host_stub!(host_sqlite_pragma_set);
    weft_host_stub!(host_sqlite_backup);
    weft_host_stub!(host_sqlite_restore);
    weft_host_stub!(host_sqlite_vacuum);
    weft_host_stub!(host_sqlite_integrity_check);
    weft_host_stub!(host_sqlite_wal_checkpoint);
    weft_host_stub!(host_sqlite_index_list);
    weft_host_stub!(host_sqlite_foreign_key_check);
    weft_host_stub!(host_sqlite_analyze);
    weft_host_stub!(host_sqlite_optimize);
    weft_host_stub!(host_sqlite_database_list);
    weft_host_stub!(host_sqlite_foreign_key_list);
    weft_host_stub!(host_sqlite_index_info);
    weft_host_stub!(host_sqlite_table_xinfo);
    weft_host_stub!(host_sqlite_index_xinfo);
    weft_host_stub!(host_sqlite_compile_options);
    weft_host_stub!(host_sqlite_collation_list);
    weft_host_stub!(host_sqlite_quick_check);
    weft_host_stub!(host_sqlite_table_list);
    weft_host_stub!(host_sqlite_data_version);
    weft_host_stub!(host_sqlite_db_stats);
    weft_host_stub!(host_sqlite_function_list);
    weft_host_stub!(host_sqlite_module_list);
    weft_host_stub!(host_download_file);
    weft_host_stub!(host_create_archive);
    weft_host_stub!(host_extract_archive);
    weft_host_stub!(host_digest);
    weft_host_stub!(host_compress);
    weft_host_stub!(host_decompress);
    weft_host_stub!(host_tcp_request);
    weft_host_stub!(host_tcp_probe);
    weft_host_stub!(host_tcp_reserve_port);
    weft_host_stub!(host_tcp_release_port);
    weft_host_stub!(host_tcp_list_reserved_ports);
    weft_host_stub!(host_tcp_listen_once);
    weft_host_stub!(host_udp_request);
    weft_host_stub!(host_udp_bind_recv);
    weft_host_stub!(host_dns_lookup);
    weft_host_stub!(host_named_pipe_request);
    weft_host_stub!(host_named_pipe_listen_once);
    weft_host_stub!(host_unix_socket_request);
    weft_host_stub!(host_unix_socket_listen_once);
    weft_host_stub!(host_websocket_connect);
    weft_host_stub!(host_websocket_send);
    weft_host_stub!(host_websocket_recv);
    weft_host_stub!(host_websocket_close);
    weft_host_stub!(host_env_get);
    weft_host_stub!(host_env_set);
    weft_host_stub!(host_env_unset);
    weft_host_stub!(host_env_list);
    weft_host_stub!(host_secret_set);
    weft_host_stub!(host_secret_get);
    weft_host_stub!(host_secret_delete);
    weft_host_stub!(host_secret_list);
    weft_host_stub!(host_event_publish);
    weft_host_stub!(host_event_poll);
    weft_host_stub!(host_event_topics);
    weft_host_stub!(host_event_clear);
    weft_host_stub!(host_now_ms);
    weft_host_stub!(host_sleep_ms);
    weft_host_stub!(host_random_hex);
    weft_host_stub!(host_uuid_v4);
    weft_host_stub!(host_os_info);
    weft_host_stub!(host_home_dir);
    weft_host_stub!(host_hostname);
    weft_host_stub!(host_process_id);
    weft_host_stub!(host_available_parallelism);
    weft_host_stub!(host_which);
    weft_host_stub!(host_call_package);
    weft_host_stub!(host_chat_completion);
    weft_host_stub!(host_process_register);
    weft_host_stub!(host_process_start);
    weft_host_stub!(host_process_spawn);
    weft_host_stub!(host_process_stop);
    weft_host_stub!(host_process_status);
    weft_host_stub!(host_process_restart);
    weft_host_stub!(host_process_wait);
    weft_host_stub!(host_process_inspect);
    weft_host_stub!(host_process_unregister);
    weft_host_stub!(host_process_list);
    weft_host_stub!(host_process_list_info);
    weft_host_stub!(host_process_write_stdin);
    weft_host_stub!(host_process_read_output);
    weft_host_stub!(host_system_process_list);
    weft_host_stub!(host_system_process_inspect);
    weft_host_stub!(host_system_process_kill);
    weft_host_stub!(host_system_identity);
    weft_host_stub!(host_system_cpu_info);
    weft_host_stub!(host_system_memory_info);
    weft_host_stub!(host_system_disk_list);
    weft_host_stub!(host_system_network_list);
    weft_host_stub!(host_system_uptime);
}

const PACKAGE_NAME: &str = "tool-runtime-core";
const CAPABILITY_NAME: &str = "tool.runtime";
const BLINK_ONCE_TIMEOUT_MS: u64 = 90_000;
const SCREEN_OCR_TIMEOUT_MS: u64 = 90_000;

const WEFT_WIKI_CONTRACT: &str = r#"# Weft Wiki Writing Contract

Use this contract whenever you create or revise a durable wiki page.

## Authoring units
- `article`: durable explanatory page, topic page, comparison, or synthesized reference
- `memo`: small but important operational note, preference, decision, or reminder
- `artifact`: media-heavy or file-backed unit such as diagrams, assets, or linked evidence packs

Wiki is the durable authoring surface. Palace is the compiled retrieval index.

## Required shape
1. Start with a short lead paragraph directly under the title
2. Use natural `##` sections for major ideas
3. Include at least one visual element when the page benefits from it
4. Include evidence, citations, or source links
5. Put sources in a normal references area instead of flattening labels into prose
6. Cross-link related wiki pages when useful
"#;

#[derive(Deserialize)]
struct ExecuteToolInput {
    #[serde(default)]
    tool: String,
    #[serde(default)]
    args: serde_json::Value,
}

struct ToolDef {
    name: &'static str,
    description: &'static str,
    parameters: serde_json::Value,
}

struct ToolValidationIssue {
    path: String,
    expected: String,
}

#[derive(Debug)]
struct RepairedToolArgs {
    args: serde_json::Value,
    notes: Vec<String>,
}

#[plugin_fn]
pub fn init(_input: String) -> FnResult<String> {
    log_info("tool-runtime-core initialized");
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
            "actions": ["describe", "health", "dispatch", "get_tool_specs", "execute_tool"],
        })),
        "health" => {
            PackageResult::ok(serde_json::json!({"healthy": true, "package": PACKAGE_NAME}))
        }
        "get_tool_specs" => do_get_tool_specs(),
        "execute_tool" => do_execute_tool(&req.data),
        "dispatch" | "call" => PackageResult::ok(serde_json::json!({
            "package": PACKAGE_NAME,
            "dispatched": true,
            "request": req.data,
        })),
        _ => PackageResult::err(format!("unknown action: {}", req.action)),
    };

    Ok(ascii_safe_json(&result))
}

#[plugin_fn]
pub fn get_tool_specs(_input: String) -> FnResult<String> {
    Ok(do_get_tool_specs().to_json())
}

#[plugin_fn]
pub fn execute_tool(input: String) -> FnResult<String> {
    let data = serde_json::from_str(&input).unwrap_or(serde_json::Value::Null);
    Ok(ascii_safe_json(&do_execute_tool(&data)))
}

fn do_get_tool_specs() -> PackageResult {
    let specs = builtin_tool_defs()
        .into_iter()
        .map(|tool| {
            serde_json::json!({
                "type": "function",
                "function": {
                    "name": tool.name,
                    "description": tool.description,
                    "parameters": tool.parameters,
                }
            })
        })
        .collect::<Vec<_>>();
    PackageResult::ok(serde_json::json!({"tools": specs}))
}

fn builtin_tool_defs() -> Vec<ToolDef> {
    vec![
        ToolDef { name: "fs_read", description: "Read a regular file. Do not use for directories; use fs_list for directories.", parameters: serde_json::json!({"type":"object","properties":{"path":{"type":"string","x-weft-kind":"pathString","description":"Regular file path to read, not a directory"}},"required":["path"]}) },
        ToolDef { name: "fs_write", description: "Write content to a file. Prefer this over shell redirection for file creation or updates.", parameters: serde_json::json!({"type":"object","properties":{"path":{"type":"string","x-weft-kind":"pathString","description":"File path"},"content":{"type":"string","description":"Content to write"}},"required":["path","content"]}) },
        ToolDef { name: "fs_list", description: "List directory contents. Use this for directories; do not use fs_read on directories.", parameters: serde_json::json!({"type":"object","properties":{"path":{"type":"string","x-weft-kind":"pathString","description":"Directory path"}},"required":["path"]}) },
        ToolDef { name: "shell_exec", description: "Execute a shell command on the host. Host OS is Windows; use PowerShell/cmd-compatible commands unless args executes a known program directly. Prefer filesystem tools for file operations.", parameters: serde_json::json!({"type":"object","properties":{"command":{"type":"string","description":"Command or executable to execute"},"args":{"type":"array","items":{"type":"string"},"description":"Arguments. If omitted, command is treated as a platform shell script."},"cwd":{"type":"string","x-weft-kind":"pathString","description":"Working directory for the command"},"shell":{"type":"string","enum":["auto","cmd","powershell","pwsh","sh","none"],"default":"auto","description":"Shell used when args is omitted; none executes command directly"},"timeout_ms":{"type":"integer","description":"Timeout in milliseconds"}},"required":["command"]}) },
        ToolDef { name: "git", description: "Execute a git command", parameters: serde_json::json!({"type":"object","properties":{"args":{"type":"array","items":{"type":"string"},"description":"Git arguments, excluding the git executable"}},"required":["args"]}) },
        ToolDef { name: "web_fetch", description: "Make an HTTP request", parameters: serde_json::json!({"type":"object","properties":{"url":{"type":"string","x-weft-kind":"urlString","description":"URL to fetch"},"method":{"type":"string","description":"HTTP method","default":"GET"},"body":{"type":"string","description":"Request body"}},"required":["url"]}) },
        ToolDef { name: "fetch_url", description: "Fetch and simplify a webpage.", parameters: serde_json::json!({"type":"object","properties":{"url":{"type":"string","x-weft-kind":"urlString","description":"URL to fetch"},"title":{"type":"string","description":"Optional page title"}},"required":["url"]}) },
        ToolDef { name: "search_web", description: "Search the web.", parameters: serde_json::json!({"type":"object","properties":{"query":{"type":"string","description":"Search query"},"max_results":{"type":"integer","description":"Maximum search results","default":5}},"required":["query"]}) },
        ToolDef { name: "read_file", description: "Read a local plain-text file directly. Use dedicated document skills for PDF/Office formats.", parameters: serde_json::json!({"type":"object","properties":{"path":{"type":"string","x-weft-kind":"pathString","description":"Full local file path"}},"required":["path"]}) },
        ToolDef { name: "read_document", description: "Read the current WPS/Office document through the desktop active-window reader.", parameters: serde_json::json!({"type":"object","properties":{"window_title":{"type":"string","description":"Optional current WPS/Office window title"}},"required":[]}) },
        ToolDef { name: "blink_once", description: "Capture and inspect the current screen using screenshot plus vision analysis.", parameters: serde_json::json!({"type":"object","properties":{"task":{"type":"string","description":"What to inspect on the current screen"},"screen_index":{"type":"integer","description":"Current blink page index","default":1},"force_system":{"type":"boolean","description":"Force full system screenshot mode","default":true},"prev_summaries":{"type":"array","items":{"type":"string"},"description":"Previous blink summaries for continuation context"}},"required":["task"]}) },
        ToolDef { name: "screen_ocr", description: "Capture the full screen and extract visible text locally with PaddleOCR. No LLM or paid API call.", parameters: serde_json::json!({"type":"object","properties":{},"required":[]}) },
        ToolDef { name: "meeting_record", description: "Start, stop, or inspect the meeting recording/transcription flow.", parameters: serde_json::json!({"type":"object","properties":{"action":{"type":"string","enum":["start","stop","status"],"description":"Meeting recording action"},"output_dir":{"type":"string","description":"Optional directory for recording output"}},"required":["action"]}) },
        ToolDef { name: "meeting_notes", description: "Prepare meeting-note structure, action items, and summary support for an active or completed meeting.", parameters: serde_json::json!({"type":"object","properties":{"meeting_title":{"type":"string","description":"Meeting title or app context"},"transcript":{"type":"string","description":"Optional transcript text"},"request":{"type":"string","description":"User confirmation or requested meeting help"}},"required":[]}) },
        ToolDef { name: "wiki_contract", description: "Return the required Weft wiki authoring contract. Use before creating durable wiki memory pages.", parameters: serde_json::json!({"type":"object","properties":{},"required":[]}) },
        ToolDef { name: "wiki_write", description: "Create a durable wiki page in the current workspace through the real workspace wiki API.", parameters: wiki_write_parameters() },
        ToolDef { name: "mode_tool", description: "Resolve the companion mode for a proactive context event and return mode state for shared runtime coordination.", parameters: serde_json::json!({"type":"object","properties":{"event_type":{"type":"string","description":"Context event type, such as game_context_detected or video_context_detected"},"payload":{"type":"object","description":"Raw context event payload"}},"required":["event_type"]}) },
        ToolDef { name: "local_scout", description: "Scan the local system to discover installed apps, games, browsers, dev tools, AI configs, and shell history.", parameters: serde_json::json!({"type":"object","properties":{"root":{"type":"string","description":"Root path to scan, defaults to home directory","default":"~"},"max_depth":{"type":"integer","description":"Max filesystem traversal depth","default":4},"output":{"type":"string","x-weft-kind":"pathString","description":"Optional path to write the JSON report"},"timeout_ms":{"type":"integer","description":"Timeout in milliseconds","default":60000},"include":{"type":"array","items":{"type":"string"},"description":"Optional include filters"}},"required":[]}) },
    ]
}

fn wiki_write_parameters() -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "properties": {
            "title": {"type": "string", "description": "Wiki page title"},
            "content": {"type": "string", "description": "Markdown content"},
            "workspace_id": {"type": "string", "description": "Workspace id"},
            "authoring_unit": {"type": "string", "enum": ["article", "memo", "artifact"], "description": "Wiki authoring unit"},
            "filename": {"type": "string", "x-weft-kind": "pathString", "description": "Optional markdown filename"},
            "assets": {"type": "array", "items": {"type": "object"}, "description": "Optional source assets"}
        },
        "required": ["title", "content", "workspace_id"]
    })
}

fn do_execute_tool(data: &serde_json::Value) -> PackageResult {
    let input: ExecuteToolInput =
        serde_json::from_value(data.clone()).unwrap_or(ExecuteToolInput {
            tool: String::new(),
            args: serde_json::Value::Null,
        });
    let tool = input.tool.trim();
    if tool.is_empty() {
        return PackageResult::err("missing tool".to_string());
    }

    let schema = builtin_tool_schema(tool);
    let repaired = repair_tool_args(tool, input.args, schema.as_ref());

    let result = match tool {
        "fs_read" | "read_file" => do_fs_read(&repaired.args),
        "fs_write" => do_fs_write(&repaired.args),
        "fs_list" => do_fs_list(&repaired.args),
        "shell_exec" => do_shell_exec(&repaired.args),
        "git" => do_git(&repaired.args),
        "web_fetch" => do_web_fetch(&repaired.args),
        "fetch_url" => do_fetch_url(&repaired.args),
        "search_web" => do_search_web(&repaired.args),
        "wiki_contract" => do_wiki_contract(&repaired.args),
        "wiki_write" => do_wiki_write(&repaired.args),
        "read_document" => run_desktop_script(
            "read-document.js",
            serde_json::json!({"windowTitle": decode_arg_text(&repaired.args, "window_title")}),
            None,
            false,
        ),
        "blink_once" => run_desktop_script(
            "blink-once.js",
            build_blink_payload(&repaired.args),
            Some(BLINK_ONCE_TIMEOUT_MS),
            true,
        ),
        "screen_ocr" => run_desktop_script(
            "screen-ocr.js",
            serde_json::json!({}),
            Some(SCREEN_OCR_TIMEOUT_MS),
            false,
        ),
        "meeting_record" => run_meeting_record(&repaired.args),
        "meeting_notes" => do_meeting_notes(&repaired.args),
        "mode_tool" => do_mode_tool(&repaired.args),
        "local_scout" => do_local_scout(&repaired.args),
        _ => PackageResult::err(format!("unknown tool: {}", tool)),
    };
    with_repair_notes(result, &repaired)
}

fn with_repair_notes(mut result: PackageResult, repaired: &RepairedToolArgs) -> PackageResult {
    if repaired.notes.is_empty() {
        return result;
    }
    let repair_note = serde_json::json!({
        "repaired": true,
        "notes": repaired.notes,
    });
    match result.data.take() {
        Some(serde_json::Value::Object(mut map)) => {
            map.insert("tool_input_repair".to_string(), repair_note);
            result.data = Some(serde_json::Value::Object(map));
        }
        Some(data) => {
            result.data = Some(serde_json::json!({
                "result": data,
                "tool_input_repair": repair_note,
            }));
        }
        None if result.status == "ok" => {
            result.data = Some(serde_json::json!({
                "tool_input_repair": repair_note,
            }));
        }
        None => {
            result.data = None;
        }
    }
    result
}

fn builtin_tool_schema(tool: &str) -> Option<serde_json::Value> {
    builtin_tool_defs()
        .into_iter()
        .find(|entry| entry.name == tool)
        .map(|entry| entry.parameters)
}

fn repair_tool_args(
    tool: &str,
    args: serde_json::Value,
    schema: Option<&serde_json::Value>,
) -> RepairedToolArgs {
    let mut value = match args {
        serde_json::Value::Null => serde_json::json!({}),
        other => other,
    };
    let mut notes = Vec::new();

    if let serde_json::Value::Object(ref mut map) = value {
        if let Some(schema) = schema {
            let issues = validate_tool_args_shape(&serde_json::Value::Object(map.clone()), schema);
            for issue in issues {
                if let Some(slot) = map.get_mut(&issue.path) {
                    if issue.expected == "array" {
                        repair_array_value(slot, &issue.path, &mut notes);
                    }
                    if issue.expected == "pathString" || issue.expected == "urlString" {
                        repair_path_value(slot, &issue.path, &mut notes);
                    }
                }
            }
            remove_null_optional_fields(map, schema, &mut notes);
        } else {
            let array_keys = repair_array_keys_for_tool(tool);
            for key in array_keys.iter().copied() {
                if let Some(slot) = map.get_mut(key) {
                    repair_array_value(slot, key, &mut notes);
                }
            }

            let path_keys = repair_path_keys_for_tool(tool);
            for key in path_keys.iter().copied() {
                if let Some(slot) = map.get_mut(key) {
                    repair_path_value(slot, key, &mut notes);
                }
            }
        }

        apply_relational_defaults(tool, map, &mut notes);
    }

    if notes.is_empty() {
        log_info(&format!("tool_input_valid:{}", tool));
    } else {
        log_info(&format!("tool_input_repaired:{}", tool));
    }

    RepairedToolArgs { args: value, notes }
}

fn validate_tool_args_shape(
    args: &serde_json::Value,
    schema: &serde_json::Value,
) -> Vec<ToolValidationIssue> {
    let mut issues = Vec::new();
    let Some(properties) = schema.get("properties").and_then(|value| value.as_object()) else {
        return issues;
    };
    let Some(args_object) = args.as_object() else {
        return issues;
    };

    for (key, property) in properties {
        let Some(value) = args_object.get(key) else {
            continue;
        };
        let expected_kind = property.get("x-weft-kind").and_then(|item| item.as_str());
        if matches!(expected_kind, Some("pathString" | "urlString")) && value.is_string() {
            issues.push(ToolValidationIssue {
                path: key.to_string(),
                expected: expected_kind.unwrap_or_default().to_string(),
            });
        }
        if property.get("type").and_then(|item| item.as_str()) == Some("array") && !value.is_array()
        {
            issues.push(ToolValidationIssue {
                path: key.to_string(),
                expected: "array".to_string(),
            });
        }
    }

    issues
}

fn remove_null_optional_fields(
    map: &mut serde_json::Map<String, serde_json::Value>,
    schema: &serde_json::Value,
    notes: &mut Vec<String>,
) {
    let required = schema
        .get("required")
        .and_then(|value| value.as_array())
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let null_optional = map
        .iter()
        .filter_map(|(key, value)| {
            if value.is_null() && !required.iter().any(|required_key| required_key == key) {
                Some(key.clone())
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    for key in null_optional {
        map.remove(&key);
        notes.push(format!("omitted null optional field {}", key));
    }
}

fn repair_array_keys_for_tool(tool: &str) -> &'static [&'static str] {
    match tool {
        "git" => &["args"],
        "shell_exec" => &["args"],
        "meeting_record" => &["args"],
        "local_scout" => &["include"],
        "wiki_write" => &["assets"],
        _ => &["args", "assets", "include", "prev_summaries"],
    }
}

fn repair_path_keys_for_tool(tool: &str) -> &'static [&'static str] {
    match tool {
        "fs_read" | "read_file" | "fs_write" | "fs_list" => &["path"],
        "fetch_url" | "web_fetch" => &["url"],
        "read_document" => &["window_title"],
        _ => &[
            "path",
            "filePath",
            "absolutePath",
            "file_path",
            "local_path",
            "target_path",
            "filename",
        ],
    }
}

fn repair_array_value(value: &mut serde_json::Value, key: &str, notes: &mut Vec<String>) {
    if value.is_null() {
        *value = serde_json::json!([]);
        notes.push(format!("defaulted null {} to empty array", key));
        return;
    }
    if let Some(text) = value.as_str() {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            *value = serde_json::json!([]);
            notes.push(format!("defaulted empty string {} to empty array", key));
            return;
        }
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(trimmed) {
            if parsed.is_array() {
                *value = parsed;
                notes.push(format!("parsed stringified JSON array for {}", key));
                return;
            }
        }
        *value = serde_json::json!([trimmed]);
        notes.push(format!("wrapped bare string into array for {}", key));
        return;
    }
    if value.is_object() {
        *value = serde_json::json!([]);
        notes.push(format!(
            "converted placeholder object to empty array for {}",
            key
        ));
    }
}

fn repair_path_value(value: &mut serde_json::Value, key: &str, notes: &mut Vec<String>) {
    let Some(text) = value.as_str() else {
        return;
    };
    let repaired = normalize_markdown_path_autolink(text);
    if repaired != text {
        *value = serde_json::Value::String(repaired);
        notes.push(format!("unwrapped markdown autolink for {}", key));
    }
}

fn normalize_markdown_path_autolink(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    let mut index = 0usize;
    let bytes = value.as_bytes();
    while let Some(open_rel) = value[index..].find('[') {
        let open = index + open_rel;
        let Some(close_rel) = value[open + 1..].find(']') else {
            break;
        };
        let close = open + 1 + close_rel;
        if bytes.get(close + 1) != Some(&b'(') {
            output.push_str(&value[index..close + 1]);
            index = close + 1;
            continue;
        }
        let Some(end_rel) = value[close + 2..].find(')') else {
            break;
        };
        let end = close + 2 + end_rel;
        let label = &value[open + 1..close];
        let url = &value[close + 2..end];
        if let Some(replacement) = unwrap_path_autolink(label, url) {
            output.push_str(&value[index..open]);
            output.push_str(&replacement);
            index = end + 1;
            continue;
        }
        output.push_str(&value[index..end + 1]);
        index = end + 1;
    }
    output.push_str(&value[index..]);
    output
}

fn unwrap_path_autolink(label: &str, url: &str) -> Option<String> {
    let label = label.trim();
    let url = url.trim();
    if label.is_empty() || url.is_empty() {
        return None;
    }
    if url.contains(' ') {
        return Some(label.to_string());
    }
    if url.starts_with("http://") || url.starts_with("https://") {
        let without_protocol = url
            .strip_prefix("http://")
            .or_else(|| url.strip_prefix("https://"))
            .unwrap_or(url)
            .trim_end_matches('/');
        if without_protocol.eq_ignore_ascii_case(label) {
            return Some(label.to_string());
        }
        return None;
    }
    if url.eq_ignore_ascii_case(label) {
        return Some(label.to_string());
    }
    None
}

fn apply_relational_defaults(
    tool: &str,
    map: &mut serde_json::Map<String, serde_json::Value>,
    notes: &mut Vec<String>,
) {
    if tool == "fs_read" || tool == "read_file" {
        if map.get("offset").is_some() && map.get("limit").is_none() {
            map.insert("limit".to_string(), serde_json::json!(2000));
            notes.push("defaulted limit to 2000 when offset was provided".to_string());
        }
        if map.get("limit").is_some() && map.get("offset").is_none() {
            map.insert("offset".to_string(), serde_json::json!(0));
            notes.push("defaulted offset to 0 when limit was provided".to_string());
        }
    }
}

fn decode_arg_text(args: &serde_json::Value, key: &str) -> String {
    args.get(key)
        .and_then(|value| value.as_str())
        .unwrap_or("")
        .to_string()
}

fn first_non_empty_string(args: &serde_json::Value, keys: &[&str]) -> String {
    for key in keys {
        if let Some(value) = args.get(*key).and_then(|value| value.as_str()) {
            if !value.trim().is_empty() {
                return value.trim().to_string();
            }
        }
    }
    String::new()
}

fn ascii_safe_json(result: &PackageResult) -> String {
    let json = result.to_json();
    let mut escaped = String::with_capacity(json.len());
    for ch in json.chars() {
        if ch.is_ascii() {
            escaped.push(ch);
        } else {
            let mut units = [0u16; 2];
            for unit in ch.encode_utf16(&mut units) {
                escaped.push_str(&format!("\\u{:04x}", unit));
            }
        }
    }
    escaped
}

fn decode_exec_utf8(encoded: &Option<String>) -> Option<String> {
    encoded
        .as_deref()
        .and_then(|value| base64::engine::general_purpose::STANDARD.decode(value).ok())
        .and_then(|bytes| String::from_utf8(bytes).ok())
}

fn exec_stdout_text(exec: &ExecResult) -> String {
    decode_exec_utf8(&exec.stdout_base64).unwrap_or_else(|| exec.stdout.clone())
}

fn exec_stderr_text(exec: &ExecResult) -> String {
    decode_exec_utf8(&exec.stderr_base64).unwrap_or_else(|| exec.stderr.clone())
}

fn host_exec_value(
    executable: &str,
    args: &[&str],
    workdir: Option<&str>,
    timeout_ms: Option<u64>,
) -> Result<serde_json::Value, String> {
    let mut input = serde_json::json!({
        "command": executable,
        "args": args,
    });
    if let Some(workdir) = workdir.filter(|value| !value.trim().is_empty()) {
        input["workdir"] = serde_json::json!(workdir);
    }
    if let Some(timeout_ms) = timeout_ms {
        input["timeout_ms"] = serde_json::json!(timeout_ms);
    }
    let input = input.to_string();
    let raw = unsafe { host_exec_advanced(input) }.map_err(|error| format!("{}", error))?;
    let value: serde_json::Value = serde_json::from_str(&raw)
        .map_err(|error| format!("host_exec_advanced parse failed: {}", error))?;
    if let Some(error) = value.get("error").and_then(|item| item.as_str()) {
        return Err(error.to_string());
    }
    Ok(value)
}

fn exec_value_status(value: &serde_json::Value) -> i64 {
    value
        .get("status")
        .and_then(|item| item.as_i64())
        .or_else(|| value.get("exit_code").and_then(|item| item.as_i64()))
        .or_else(|| {
            value
                .get("data")
                .and_then(|data| data.get("status"))
                .and_then(|item| item.as_i64())
        })
        .or_else(|| {
            value
                .get("data")
                .and_then(|data| data.get("exit_code"))
                .and_then(|item| item.as_i64())
        })
        .unwrap_or(-1)
}

fn exec_value_text(value: &serde_json::Value, key: &str) -> String {
    value
        .get(key)
        .and_then(|item| item.as_str())
        .or_else(|| {
            value
                .get("data")
                .and_then(|data| data.get(key))
                .and_then(|item| item.as_str())
        })
        .unwrap_or("")
        .to_string()
}

fn do_fs_list(args: &serde_json::Value) -> PackageResult {
    let raw = args
        .get("path")
        .and_then(|value| value.as_str())
        .unwrap_or(".");
    let ws = workspace_root_of(args);
    // 默认 "." → 工作区根(若已设),否则原样;具体路径走工作区解析。
    let path = if raw.trim() == "." && !ws.is_empty() {
        ws.trim_end_matches(['\\', '/']).to_string()
    } else {
        match resolve_in_workspace(raw, &ws) {
            Ok(p) => p,
            Err(e) => return PackageResult::err(e),
        }
    };
    match list_dir(&path) {
        Ok(entries) => PackageResult::ok(serde_json::json!({"entries": entries})),
        Err(error) => PackageResult::err(error),
    }
}

fn do_fs_read(args: &serde_json::Value) -> PackageResult {
    let path = match resolve_in_workspace(
        args.get("path")
            .and_then(|value| value.as_str())
            .unwrap_or(""),
        &workspace_root_of(args),
    ) {
        Ok(p) => p,
        Err(e) => return PackageResult::err(e),
    };
    match read_file(&path) {
        Ok(content) => PackageResult::ok(serde_json::json!({"content": content})),
        Err(error) => PackageResult::err(error),
    }
}

fn do_fs_write(args: &serde_json::Value) -> PackageResult {
    let raw_path = args
        .get("path")
        .and_then(|value| value.as_str())
        .unwrap_or("");
    let path = match resolve_in_workspace(raw_path, &workspace_root_of(args)) {
        Ok(p) => p,
        Err(e) => return PackageResult::err(e),
    };
    let content = args
        .get("content")
        .and_then(|value| value.as_str())
        .unwrap_or("");
    write_file(&path, content);
    match read_file(&path) {
        Ok(written) if written == content => {}
        Ok(_) => {
            return PackageResult::err(format!(
                "write verification failed: content mismatch at {}",
                path
            ))
        }
        Err(error) => {
            return PackageResult::err(format!("write verification failed for {}: {}", path, error))
        }
    }
    PackageResult::ok(serde_json::json!({"written": true, "path": path}))
}

fn normalize_user_path(path: &str) -> String {
    let trimmed = path.trim();
    if trimmed == "~" {
        return home_dir_string().unwrap_or_else(|| trimmed.to_string());
    }
    if let Some(rest) = trimmed
        .strip_prefix("~/")
        .or_else(|| trimmed.strip_prefix("~\\"))
    {
        if let Some(home) = home_dir_string() {
            return format!(
                "{}\\{}",
                home.trim_end_matches(['\\', '/']),
                rest.replace('/', "\\")
            );
        }
    }
    if let Some(path) = normalize_windows_user_desktop_path(trimmed) {
        return path;
    }
    trimmed.to_string()
}

/// 路径是否绝对(Windows 盘符 `X:\`/`X:/`、UNC `\\`、或 Unix `/` 开头)。
fn is_absolute_path(path: &str) -> bool {
    let p = path.trim();
    if p.starts_with('/') || p.starts_with('\\') {
        return true; // Unix 绝对 或 UNC/根
    }
    // Windows 盘符:形如 C:\ 或 C:/
    let bytes = p.as_bytes();
    bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && (bytes[2] == b'\\' || bytes[2] == b'/')
}

/// 拆分路径为(前缀, 段列表)。前缀指盘符根 `C:\`、UNC 根 `\\server\share` 或
/// Unix 根 `/`;无绝对前缀时前缀为空(纯相对)。段不含分隔符,`.`/`..` 保留待折叠。
fn split_path_prefix(path: &str) -> (String, Vec<String>) {
    let norm = path.replace('/', "\\");
    // UNC: \\server\share\...
    if norm.starts_with("\\\\") {
        let rest = &norm[2..];
        let mut it = rest.splitn(3, '\\');
        let server = it.next().unwrap_or("");
        let share = it.next().unwrap_or("");
        let tail = it.next().unwrap_or("");
        let prefix = format!("\\\\{}\\{}", server, share);
        let segs = tail
            .split('\\')
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect();
        return (prefix, segs);
    }
    // Windows 盘符: C:\... 或 C:...
    let bytes = norm.as_bytes();
    if bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':' {
        let drive = format!("{}:\\", &norm[0..1]);
        let rest = norm[2..].trim_start_matches('\\');
        let segs = rest
            .split('\\')
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect();
        return (drive, segs);
    }
    // Unix 绝对根
    if norm.starts_with('\\') {
        let rest = norm.trim_start_matches('\\');
        let segs = rest
            .split('\\')
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect();
        return ("\\".to_string(), segs);
    }
    // 纯相对
    let segs = norm
        .split('\\')
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect();
    (String::new(), segs)
}

/// 纯词法归一化(不触碰文件系统,适配 WASM 沙箱+文件可能尚不存在)。
/// 折叠 `.`/`..` 段。`..` 越过前缀根时被丢弃(钳到根),不抛出。
/// 返回归一化后的路径段列表(不含前缀)。
fn lexical_normalize_segs(segs: &[String]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for seg in segs {
        match seg.as_str() {
            "." | "" => {}
            ".." => {
                out.pop();
            }
            other => out.push(other.to_string()),
        }
    }
    out
}

/// 判断 candidate 是否被钳制在 workspace 内(含 workspace 根自身)。
/// 纯词法 + Windows 大小写不敏感 + 段边界比较(避免 `C:\ws` 误配 `C:\ws2`)。
/// 在界内返回 Ok(归一化绝对路径),越界返回 Err。
fn contain_in_workspace(candidate_abs: &str, workspace_abs: &str) -> Result<String, String> {
    let (cand_prefix, cand_segs_raw) = split_path_prefix(candidate_abs);
    let (ws_prefix, ws_segs_raw) = split_path_prefix(workspace_abs);
    let cand_segs = lexical_normalize_segs(&cand_segs_raw);
    let ws_segs = lexical_normalize_segs(&ws_segs_raw);

    // 前缀(盘符/UNC根)必须一致(大小写不敏感)。
    if cand_prefix.to_lowercase() != ws_prefix.to_lowercase() {
        return Err(format!(
            "path '{}' escapes workspace '{}' (different root)",
            candidate_abs, workspace_abs
        ));
    }
    // candidate 段数必须 >= workspace 段数,且前 N 段逐一匹配(大小写不敏感)。
    if cand_segs.len() < ws_segs.len() {
        return Err(format!(
            "path '{}' escapes workspace '{}'",
            candidate_abs, workspace_abs
        ));
    }
    for (c, w) in cand_segs.iter().zip(ws_segs.iter()) {
        if c.to_lowercase() != w.to_lowercase() {
            return Err(format!(
                "path '{}' escapes workspace '{}'",
                candidate_abs, workspace_abs
            ));
        }
    }
    // 重建归一化绝对路径。
    let base = cand_prefix.trim_end_matches('\\');
    if cand_segs.is_empty() {
        Ok(format!("{}\\", base))
    } else {
        Ok(format!("{}\\{}", base, cand_segs.join("\\")))
    }
}

/// 在会话工作区内解析用户路径并强制沙箱钳制。
/// workspace_root 为空时退回旧行为(相对进程 cwd,不钳制),保证不破坏未设工作区的场景。
/// workspace_root 非空时:相对路径 join 工作区;绝对/`~`/`..` 展开后必须落在工作区内,
/// 越界返回 Err(严格沙箱,拒绝越界写入)。
fn resolve_in_workspace(raw_path: &str, workspace_root: &str) -> Result<String, String> {
    let trimmed = raw_path.trim();
    let ws = workspace_root.trim();

    // 未设工作区:维持旧行为(展开 ~/绝对路径,相对路径原样)。
    if ws.is_empty() {
        if trimmed.starts_with('~') || is_absolute_path(trimmed) {
            return Ok(normalize_user_path(trimmed));
        }
        return Ok(normalize_user_path(trimmed));
    }

    let ws_base = ws.trim_end_matches(['\\', '/']);

    // 计算 candidate 的绝对形式(尚未钳制)。
    let candidate_abs = if trimmed.starts_with('~') {
        // ~ 展开 home 后按绝对路径处理。
        normalize_user_path(trimmed)
    } else if is_absolute_path(trimmed) {
        normalize_user_path(trimmed)
    } else {
        // 相对路径(含 ..) → join 到工作区根,交给词法归一化折叠 ..。
        let rel = trimmed.replace('/', "\\");
        let rel = rel.trim_start_matches('\\');
        format!("{}\\{}", ws_base, rel)
    };

    // 强制钳制:candidate 归一化后必须落在工作区内。
    contain_in_workspace(&candidate_abs, ws_base)
}

/// 从工具 args 取运行时注入的会话工作区根(agent-core 注入 `__workspace_root`)。
fn workspace_root_of(args: &serde_json::Value) -> String {
    args.get("__workspace_root")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim()
        .to_string()
}

fn normalize_windows_user_desktop_path(path: &str) -> Option<String> {
    let normalized = path.replace('/', "\\");
    let parts = normalized.split('\\').collect::<Vec<_>>();
    if parts.len() < 4
        || !parts
            .get(0)
            .is_some_and(|part| part.eq_ignore_ascii_case("C:"))
        || !parts
            .get(1)
            .is_some_and(|part| part.eq_ignore_ascii_case("Users"))
        || !parts
            .get(3)
            .is_some_and(|part| part.eq_ignore_ascii_case("Desktop"))
    {
        return None;
    }
    let requested_user = parts[2];
    if requested_user.eq_ignore_ascii_case("Public") {
        return None;
    }
    let home = home_dir_string()?;
    let current_user = home
        .trim_end_matches(['\\', '/'])
        .rsplit(['\\', '/'])
        .next()
        .unwrap_or("");
    if requested_user.eq_ignore_ascii_case(current_user) {
        return None;
    }
    let mut corrected = format!("{}\\Desktop", home.trim_end_matches(['\\', '/']));
    if parts.len() > 4 {
        corrected.push('\\');
        corrected.push_str(&parts[4..].join("\\"));
    }
    Some(corrected)
}

fn home_dir_string() -> Option<String> {
    env_get("USERPROFILE").or_else(|| env_get("HOME"))
}

/// 尽力而为的 shell 命令越界防护(非硬边界)。
/// 设了工作区时,扫描命令文本+argv,拦截"明显写到工作区外"的常见模式:
///   - 重定向到 `..` 路径(`> ..\x`、`>> ../x`)
///   - 重定向/写入到绝对路径(`> D:\x`、`> /etc/x`、`Out-File C:\x`)
///   - `~` 家目录写入
/// 这些是 AI 被 fs_write 拒绝后最常"自然"改用的逃逸方式。
/// 无法覆盖 python -c/powershell .NET 等任意间接写入(已向用户说明为尽力而为)。
fn shell_escape_guard(command: &str, argv: &[String], workspace_root: &str) -> Result<(), String> {
    let ws = workspace_root.trim();
    if ws.is_empty() {
        return Ok(()); // 未设工作区:不限制。
    }
    // 合并命令与参数为统一扫描文本。
    let mut hay = command.to_string();
    for a in argv {
        hay.push(' ');
        hay.push_str(a);
    }
    let lower = hay.to_lowercase();

    // 1) 绝对路径重定向/写入:盘符 `X:\`/`X:/`、Unix `/`、UNC `\\`。
    //    只在出现写入动词/重定向符附近才拦(避免误伤纯读取的绝对路径)。
    let has_write_redirect = hay.contains('>')
        || lower.contains("out-file")
        || lower.contains("set-content")
        || lower.contains("add-content")
        || lower.contains("tee ")
        || lower.contains("tee-object");

    // 2) `..` 路径穿越(在任意 token 中出现 `..\` 或 `../`)。
    let has_dotdot = hay.contains("..\\") || hay.contains("../");

    // 3) `~` 家目录引用。
    let has_tilde = hay.contains("~/") || hay.contains("~\\") || hay.split_whitespace().any(|t| t == "~");

    // 检测绝对路径 token(粗粒度)。
    let has_abs = hay.split(|c: char| c == ' ' || c == '"' || c == '\'' || c == '>' || c == '|' || c == '\t')
        .any(|tok| {
            let t = tok.trim();
            !t.is_empty() && is_absolute_path(t)
        });

    if has_dotdot {
        return Err(format!(
            "shell command rejected: contains '..' path traversal which may escape the workspace ('{}'). Use fs_write with a workspace-relative path instead.",
            ws
        ));
    }
    if has_tilde {
        return Err(
            "shell command rejected: references '~' home directory outside the workspace. Use fs_write with a workspace-relative path instead.".to_string()
        );
    }
    if has_abs && has_write_redirect {
        return Err(format!(
            "shell command rejected: writes to an absolute path outside the workspace ('{}'). Use fs_write with a workspace-relative path instead.",
            ws
        ));
    }
    Ok(())
}

fn do_shell_exec(args: &serde_json::Value) -> PackageResult {
    let raw_command = args
        .get("command")
        .and_then(|value| value.as_str())
        .unwrap_or("");
    let command = normalize_shell_command_text(raw_command);
    let argv = string_array_arg(args, "args");
    let ws = workspace_root_of(args);
    // 尽力而为的命令越界防护(设了工作区时拦截明显的 ../绝对路径/~ 写入)。
    if let Err(e) = shell_escape_guard(&command, &argv, &ws) {
        return PackageResult::err(e);
    }
    // cwd 解析:已设工作区时,caller 指定的 cwd 必须落在工作区内(越界拒绝);
    // 缺省时回退到工作区根,让脚本默认在工作区内运行(产物不散落项目根)。
    let raw_cwd = args
        .get("cwd")
        .or_else(|| args.get("workdir"))
        .and_then(|value| value.as_str());
    let cwd: Option<String> = match raw_cwd {
        Some(c) if !c.trim().is_empty() => {
            if ws.is_empty() {
                Some(normalize_user_path(c))
            } else {
                match resolve_in_workspace(c, &ws) {
                    Ok(p) => Some(p),
                    Err(e) => return PackageResult::err(e),
                }
            }
        }
        _ => {
            if ws.is_empty() {
                None
            } else {
                Some(ws.trim_end_matches(['\\', '/']).to_string())
            }
        }
    };
    let timeout_ms = args.get("timeout_ms").and_then(|value| value.as_u64());
    let shell = args
        .get("shell")
        .and_then(|value| value.as_str())
        .unwrap_or("auto");
    let (executable, normalized_args): (String, Vec<String>) = if !argv.is_empty() && is_python_executable(&command) {
        let mut candidates = Vec::new();
        if cfg!(windows) {
            for (candidate, prefix) in WINDOWS_PYTHON_CANDIDATES {
                let mut candidate_args = prefix
                    .iter()
                    .map(|item| item.to_string())
                    .collect::<Vec<_>>();
                candidate_args.extend(argv.clone());
                candidates.push((candidate.to_string(), candidate_args));
            }
        } else {
            candidates.push((command.to_string(), argv.clone()));
            let mut python3_args = Vec::new();
            python3_args.extend(argv.clone());
            candidates.push(("python3".to_string(), python3_args));
        }
        return run_exec_candidates(&candidates, cwd.as_deref(), timeout_ms);
    } else if is_python_script_path(&command) {
        // command is a .py file path — prepend it to argv and run via python
        let mut script_args = vec![command.clone()];
        script_args.extend(argv.clone());
        let mut candidates = Vec::new();
        if cfg!(windows) {
            for (candidate, prefix) in WINDOWS_PYTHON_CANDIDATES {
                let mut candidate_args = prefix
                    .iter()
                    .map(|item| item.to_string())
                    .collect::<Vec<_>>();
                candidate_args.extend(script_args.clone());
                candidates.push((candidate.to_string(), candidate_args));
            }
        } else {
            candidates.push(("python3".to_string(), script_args.clone()));
            candidates.push(("python".to_string(), script_args));
        }
        return run_exec_candidates(&candidates, cwd.as_deref(), timeout_ms);
    } else if argv.is_empty() {
        let trimmed = command.trim_start();
        let lower = trimmed.to_ascii_lowercase();
        if shell == "none" {
            (command.to_string(), Vec::new())
        } else if shell == "cmd" {
            ("cmd.exe".to_string(), cmd_command_args(&command))
        } else if shell == "powershell" {
            (
                "powershell.exe".to_string(),
                powershell_command_args(&command),
            )
        } else if shell == "pwsh" {
            ("pwsh".to_string(), powershell_command_args(&command))
        } else if shell == "sh" {
            (
                "sh".to_string(),
                vec!["-c".to_string(), command.to_string()],
            )
        } else if lower.starts_with("powershell ") || lower.starts_with("powershell.exe ") {
            let executable = if lower.starts_with("powershell.exe ") {
                "powershell.exe"
            } else {
                "powershell"
            };
            let rest = trimmed
                .split_once(char::is_whitespace)
                .map(|(_, tail)| tail.trim_start().to_string())
                .unwrap_or_default();
            (executable.to_string(), powershell_command_args(&rest))
        } else if lower.starts_with("cmd ") || lower.starts_with("cmd.exe ") {
            let executable = if lower.starts_with("cmd.exe ") {
                "cmd.exe"
            } else {
                "cmd"
            };
            let rest = trimmed
                .split_once(char::is_whitespace)
                .map(|(_, tail)| tail.trim_start().to_string())
                .unwrap_or_default();
            (executable.to_string(), cmd_command_args(&rest))
        } else if let Some(comspec) = env_get("COMSPEC").filter(|value| !value.trim().is_empty()) {
            (comspec, vec!["/C".to_string(), command.to_string()])
        } else if env_get("OS")
            .map(|value| value.to_ascii_lowercase().contains("windows"))
            .unwrap_or(false)
        {
            (
                "C:\\Windows\\System32\\cmd.exe".to_string(),
                vec!["/C".to_string(), command.to_string()],
            )
        } else {
            (
                "sh".to_string(),
                vec!["-c".to_string(), command.to_string()],
            )
        }
    } else if is_cmd_executable(&command) {
        (command.to_string(), normalize_explicit_cmd_args(argv))
    } else {
        (command.to_string(), argv)
    };
    let refs = normalized_args
        .iter()
        .map(|item: &String| item.as_str())
        .collect::<Vec<_>>();
    match host_exec_value(&executable, &refs, cwd.as_deref(), timeout_ms) {
        Ok(output) => package_result_from_exec_output(&output),
        Err(error) => PackageResult::err(error),
    }
}

fn package_result_from_exec_output(output: &serde_json::Value) -> PackageResult {
    let status = exec_value_status(output);
    let stdout = exec_value_text(output, "stdout");
    let stderr = exec_value_text(output, "stderr");
    if status == 0 {
        PackageResult::ok(serde_json::json!({
            "status": status,
            "stdout": stdout,
            "stderr": stderr,
        }))
    } else {
        let detail = if stderr.trim().is_empty() {
            stdout
        } else {
            stderr
        };
        PackageResult::err(format!(
            "shell command exited with status {}: {}",
            status,
            detail.trim()
        ))
    }
}

fn run_exec_candidates(
    candidates: &[(String, Vec<String>)],
    cwd: Option<&str>,
    timeout_ms: Option<u64>,
) -> PackageResult {
    let mut last_error = String::new();
    for (executable, args) in candidates {
        let refs = args.iter().map(|item| item.as_str()).collect::<Vec<_>>();
        match host_exec_value(executable, &refs, cwd, timeout_ms) {
            Ok(output) if exec_value_status(&output) == 0 => {
                return package_result_from_exec_output(&output)
            }
            Ok(output) => {
                let stderr = exec_value_text(&output, "stderr");
                let stdout = exec_value_text(&output, "stdout");
                last_error = if stderr.trim().is_empty() {
                    stdout
                } else {
                    stderr
                };
            }
            Err(error) => last_error = error,
        }
    }
    PackageResult::err(if last_error.trim().is_empty() {
        "all command candidates failed".to_string()
    } else {
        last_error
    })
}

fn is_python_executable(command: &str) -> bool {
    command
        .trim()
        .rsplit(['\\', '/'])
        .next()
        .map(|name| {
            name.eq_ignore_ascii_case("python")
                || name.eq_ignore_ascii_case("python.exe")
                || name.eq_ignore_ascii_case("python3")
                || name.eq_ignore_ascii_case("python3.exe")
                || name.eq_ignore_ascii_case("py")
                || name.eq_ignore_ascii_case("py.exe")
        })
        .unwrap_or(false)
}

fn is_python_script_path(command: &str) -> bool {
    command.trim().ends_with(".py") || command.trim().ends_with(".PY")
}

fn cmd_command_args(rest: &str) -> Vec<String> {
    let trimmed = rest.trim_start();
    let lower = trimmed.to_ascii_lowercase();
    if lower.starts_with("/c ") {
        return vec![
            "/C".to_string(),
            normalize_cmd_command_script(trimmed["/c".len()..].trim_start()),
        ];
    }
    if lower.starts_with("/k ") {
        return vec![
            "/K".to_string(),
            normalize_cmd_command_script(trimmed["/k".len()..].trim_start()),
        ];
    }
    vec![normalize_cmd_command_script(trimmed)]
}

fn is_cmd_executable(command: &str) -> bool {
    command
        .trim()
        .rsplit(['\\', '/'])
        .next()
        .map(|name| name.eq_ignore_ascii_case("cmd") || name.eq_ignore_ascii_case("cmd.exe"))
        .unwrap_or(false)
}

fn normalize_explicit_cmd_args(args: Vec<String>) -> Vec<String> {
    let mut normalized = args;
    let mut index = 0usize;
    while index < normalized.len() {
        let lower = normalized[index].to_ascii_lowercase();
        if (lower == "/c" || lower == "/k") && index + 1 < normalized.len() {
            normalized[index + 1] = normalize_cmd_command_script(&normalized[index + 1]);
            index += 2;
            continue;
        }
        index += 1;
    }
    normalized
}

fn normalize_cmd_command_script(script: &str) -> String {
    let expanded = expand_common_windows_env_vars(script);
    remove_unneeded_cmd_redirect_quotes(&expanded)
}

fn expand_common_windows_env_vars(script: &str) -> String {
    let mut expanded = script.to_string();
    if let Some(home) = home_dir_string() {
        expanded = replace_ascii_case_insensitive(&expanded, "%USERPROFILE%", &home);
    }
    if let Some(public) = env_get("PUBLIC") {
        expanded = replace_ascii_case_insensitive(&expanded, "%PUBLIC%", &public);
    }
    expanded
}

fn replace_ascii_case_insensitive(input: &str, needle: &str, replacement: &str) -> String {
    let lower_input = input.to_ascii_lowercase();
    let lower_needle = needle.to_ascii_lowercase();
    let mut result = String::with_capacity(input.len());
    let mut index = 0usize;
    while let Some(relative) = lower_input[index..].find(&lower_needle) {
        let found = index + relative;
        result.push_str(&input[index..found]);
        result.push_str(replacement);
        index = found + needle.len();
    }
    result.push_str(&input[index..]);
    result
}

fn remove_unneeded_cmd_redirect_quotes(script: &str) -> String {
    let mut result = String::with_capacity(script.len());
    let chars = script.chars().collect::<Vec<_>>();
    let mut index = 0usize;
    while index < chars.len() {
        let ch = chars[index];
        if (ch == '>' || ch == '<')
            && chars.get(index + 1) == Some(&' ')
            && chars.get(index + 2) == Some(&'"')
        {
            let mut end = index + 3;
            while end < chars.len() && chars[end] != '"' {
                end += 1;
            }
            if end < chars.len() {
                let target = chars[index + 3..end].iter().collect::<String>();
                if !target.chars().any(char::is_whitespace) {
                    result.push(ch);
                    result.push(' ');
                    result.push_str(&target);
                    index = end + 1;
                    continue;
                }
            }
        }
        result.push(ch);
        index += 1;
    }
    result
}

fn normalize_shell_command_text(command: &str) -> String {
    command
        .replace('\u{0008}', "\\b")
        .replace('\u{000c}', "\\f")
        .replace('\r', "\\r")
        .replace('\n', "\\n")
        .replace('\t', "\\t")
}

fn powershell_command_args(rest: &str) -> Vec<String> {
    let trimmed = rest.trim_start();
    let lower = trimmed.to_ascii_lowercase();
    if lower.starts_with("-command ") {
        let script = trimmed["-Command".len()..].trim_start();
        return vec![
            "-Command".to_string(),
            normalize_powershell_command_script(trim_matching_quotes(script)),
        ];
    }
    if lower.starts_with("-c ") {
        let script = trimmed["-c".len()..].trim_start();
        return vec![
            "-Command".to_string(),
            normalize_powershell_command_script(trim_matching_quotes(script)),
        ];
    }
    vec![trimmed.to_string()]
}

fn normalize_powershell_command_script(script: &str) -> String {
    let mut normalized = String::with_capacity(script.len());
    let mut pending_backslashes = 0usize;
    for ch in script.chars() {
        if ch == '\\' {
            pending_backslashes += 1;
            continue;
        }
        if ch == '"' {
            pending_backslashes = 0;
            normalized.push(ch);
            continue;
        }
        for _ in 0..pending_backslashes {
            normalized.push('\\');
        }
        pending_backslashes = 0;
        normalized.push(ch);
    }
    for _ in 0..pending_backslashes {
        normalized.push('\\');
    }
    normalized
}

fn trim_matching_quotes(value: &str) -> &str {
    let trimmed = value.trim();
    if trimmed.len() >= 2 {
        let bytes = trimmed.as_bytes();
        if (bytes[0] == b'"' && bytes[trimmed.len() - 1] == b'"')
            || (bytes[0] == b'\'' && bytes[trimmed.len() - 1] == b'\'')
        {
            return &trimmed[1..trimmed.len() - 1];
        }
    }
    trimmed
}

fn do_git(args: &serde_json::Value) -> PackageResult {
    let mut argv = string_array_arg(args, "args");
    if argv.first().is_some_and(|item| item == "git") {
        argv.remove(0);
    }
    if let Some(first) = argv.first().cloned() {
        if first.starts_with("git ") {
            let mut split = first
                .split_whitespace()
                .skip(1)
                .map(str::to_string)
                .collect::<Vec<_>>();
            split.extend(argv.into_iter().skip(1));
            argv = split;
        }
    }
    normalize_git_c_flag(&mut argv);
    let refs = argv.iter().map(|item| item.as_str()).collect::<Vec<_>>();
    let candidates: &[&str] = if cfg!(windows) {
        &["git", "git.exe", "C:\\Program Files\\Git\\cmd\\git.exe"]
    } else {
        &["git"]
    };
    let mut last_error = String::new();
    for candidate in candidates {
        match exec_command(candidate, &refs) {
            Ok(output) if output.status == 0 => {
                return PackageResult::ok(
                    serde_json::json!({"status": output.status, "stdout": exec_stdout_text(&output), "stderr": exec_stderr_text(&output)}),
                )
            }
            Ok(output) => {
                last_error = exec_stderr_text(&output);
                if last_error.trim().is_empty() {
                    last_error = exec_stdout_text(&output);
                }
            }
            Err(error) => last_error = error,
        }
    }
    PackageResult::err(last_error)
}

fn normalize_git_c_flag(argv: &mut Vec<String>) {
    if argv.len() >= 3 && argv[0] != "-C" && argv[1] == "-C" {
        let command = argv.remove(0);
        let flag = argv.remove(0);
        let path = argv.remove(0);
        argv.insert(0, flag);
        argv.insert(1, path);
        argv.insert(2, command);
    }
}

fn string_array_arg(args: &serde_json::Value, key: &str) -> Vec<String> {
    args.get(key)
        .and_then(|value| value.as_array())
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tool_input_repair_tests {
    use super::*;

    #[test]
    fn parses_stringified_array_before_wrapping_bare_string() {
        let schema = builtin_tool_schema("shell_exec").expect("shell_exec schema");
        let repaired = repair_tool_args(
            "shell_exec",
            serde_json::json!({
                "command": "git",
                "args": "[\"status\",\"--short\"]"
            }),
            Some(&schema),
        );

        assert_eq!(
            string_array_arg(&repaired.args, "args"),
            vec!["status".to_string(), "--short".to_string()],
        );
        assert!(repaired
            .notes
            .iter()
            .any(|note| note.contains("parsed stringified JSON array")));
    }

    #[test]
    fn wraps_bare_string_for_array_field() {
        let schema = builtin_tool_schema("git").expect("git schema");
        let repaired = repair_tool_args(
            "git",
            serde_json::json!({
                "args": "status"
            }),
            Some(&schema),
        );

        assert_eq!(
            string_array_arg(&repaired.args, "args"),
            vec!["status".to_string()]
        );
    }

    #[test]
    fn removes_null_optional_fields_without_touching_valid_content() {
        let schema = builtin_tool_schema("web_fetch").expect("web_fetch schema");
        let repaired = repair_tool_args(
            "web_fetch",
            serde_json::json!({
                "url": "https://example.com",
                "body": "{\"keep\":null}",
                "method": null
            }),
            Some(&schema),
        );

        assert!(repaired.args.get("method").is_none());
        assert_eq!(
            repaired.args.get("body").and_then(|value| value.as_str()),
            Some("{\"keep\":null}")
        );
    }

    #[test]
    fn unwraps_degenerate_markdown_autolink_for_path_fields() {
        let schema = builtin_tool_schema("read_file").expect("read_file schema");
        let repaired = repair_tool_args(
            "read_file",
            serde_json::json!({
                "path": "/Users/x/proj/[notes.md](http://notes.md)"
            }),
            Some(&schema),
        );

        assert_eq!(
            repaired.args.get("path").and_then(|value| value.as_str()),
            Some("/Users/x/proj/notes.md"),
        );
    }

    #[test]
    fn leaves_real_markdown_links_in_content_alone() {
        let schema = builtin_tool_schema("fs_write").expect("fs_write schema");
        let repaired = repair_tool_args(
            "fs_write",
            serde_json::json!({
                "path": "notes.md",
                "content": "[click](https://example.com)"
            }),
            Some(&schema),
        );

        assert_eq!(
            repaired
                .args
                .get("content")
                .and_then(|value| value.as_str()),
            Some("[click](https://example.com)"),
        );
    }

    #[test]
    fn converts_empty_placeholder_object_to_empty_array() {
        let schema = builtin_tool_schema("local_scout").expect("local_scout schema");
        let repaired = repair_tool_args(
            "local_scout",
            serde_json::json!({
                "root": "~",
                "include": {}
            }),
            Some(&schema),
        );

        assert_eq!(
            repaired
                .args
                .get("include")
                .and_then(|value| value.as_array())
                .map(Vec::len),
            Some(0)
        );
    }

    #[test]
    fn leaves_real_markdown_links_in_path_like_fields_alone() {
        let schema = builtin_tool_schema("read_file").expect("read_file schema");
        let repaired = repair_tool_args(
            "read_file",
            serde_json::json!({
                "path": "[click](https://example.com)"
            }),
            Some(&schema),
        );

        assert_eq!(
            repaired.args.get("path").and_then(|value| value.as_str()),
            Some("[click](https://example.com)"),
        );
    }

    #[test]
    fn relational_defaults_are_reported_as_repair_notes() {
        let schema = builtin_tool_schema("read_file").expect("read_file schema");
        let repaired = repair_tool_args(
            "read_file",
            serde_json::json!({
                "path": "notes.md",
                "limit": 30
            }),
            Some(&schema),
        );

        assert_eq!(
            repaired.args.get("offset").and_then(|value| value.as_i64()),
            Some(0)
        );
        assert!(repaired
            .notes
            .iter()
            .any(|note| note.contains("defaulted offset to 0")));
    }

    #[test]
    fn schema_local_repair_does_not_repair_unknown_stringified_arrays() {
        let schema = builtin_tool_schema("fs_write").expect("fs_write schema");
        let repaired = repair_tool_args(
            "fs_write",
            serde_json::json!({
                "path": "notes.md",
                "content": "[\"keep\",\"as\",\"text\"]"
            }),
            Some(&schema),
        );

        assert_eq!(
            repaired
                .args
                .get("content")
                .and_then(|value| value.as_str()),
            Some("[\"keep\",\"as\",\"text\"]"),
        );
    }
}

fn env_value(args: &serde_json::Value, arg_key: &str, env_key: &str) -> Option<String> {
    let arg_value = decode_arg_text(args, arg_key);
    Some(arg_value)
        .filter(|value| !value.is_empty())
        .or_else(|| env_get(env_key))
}

fn tavily_api_key(args: &serde_json::Value) -> Option<String> {
    env_value(args, "tavily_api_key", "TAVILY_API_KEY")
        .or_else(read_tavily_api_key_from_runtime_config)
}

fn read_tavily_api_key_from_runtime_config() -> Option<String> {
    let raw = read_file("./config/config.toml").ok()?;
    read_toml_env_value(&raw, "TAVILY_API_KEY").or_else(|| read_toml_provider_key(&raw, "tavily"))
}

fn read_toml_env_value(raw: &str, key: &str) -> Option<String> {
    let mut in_env = false;
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            in_env = trimmed.eq_ignore_ascii_case("[env]");
            continue;
        }
        if in_env {
            let Some((entry_key, _)) = trimmed.split_once('=') else {
                continue;
            };
            if entry_key.trim().eq_ignore_ascii_case(key) {
                if let Some(value) = toml_string_value(trimmed) {
                    if !value.trim().is_empty() {
                        return Some(value);
                    }
                }
            }
        }
    }
    None
}

fn read_toml_provider_key(raw: &str, provider_name: &str) -> Option<String> {
    let mut in_provider = false;
    let mut matched_provider = false;
    let mut in_key = false;
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("[[") {
            if trimmed == "[[providers]]" {
                in_provider = true;
                matched_provider = false;
                in_key = false;
            } else if trimmed == "[[providers.keys]]" && in_provider && matched_provider {
                in_key = true;
            } else {
                in_key = false;
            }
            continue;
        }
        if in_provider && trimmed.starts_with("[") {
            in_provider = false;
            matched_provider = false;
            in_key = false;
            continue;
        }
        if in_provider && trimmed.starts_with("name") {
            matched_provider = toml_string_value(trimmed)
                .map(|value| value.eq_ignore_ascii_case(provider_name))
                .unwrap_or(false);
            continue;
        }
        if in_provider && matched_provider && in_key && trimmed.starts_with("value") {
            if let Some(value) = toml_string_value(trimmed) {
                if !value.trim().is_empty() {
                    return Some(value);
                }
            }
        }
    }
    None
}

fn toml_string_value(line: &str) -> Option<String> {
    let (_, value) = line.split_once('=')?;
    let trimmed = value.trim();
    let unquoted = trimmed.strip_prefix('"')?.strip_suffix('"')?;
    Some(unquoted.to_string())
}

fn read_toml_section_value(raw: &str, section: &str, key: &str) -> Option<String> {
    let mut in_section = false;
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            in_section = trimmed.eq_ignore_ascii_case(&format!("[{}]", section));
            continue;
        }
        if in_section {
            let Some((entry_key, _)) = trimmed.split_once('=') else {
                continue;
            };
            if entry_key.trim().eq_ignore_ascii_case(key) {
                return toml_string_value(trimmed).filter(|value| !value.trim().is_empty());
            }
        }
    }
    None
}

fn read_first_provider_model(raw: &str) -> Option<String> {
    for line in raw.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with("models") {
            continue;
        }
        let (_, value) = trimmed.split_once('=')?;
        let inner = value.trim().strip_prefix('[')?.strip_suffix(']')?;
        for item in inner.split(',') {
            let model = item.trim().strip_prefix('"')?.strip_suffix('"')?.trim();
            if !model.is_empty() {
                return Some(model.to_string());
            }
        }
    }
    None
}

#[derive(Clone, Debug)]
struct RuntimeChatProvider {
    name: String,
    base_url: String,
    api_key: String,
    model: String,
}

fn default_chat_provider_from_runtime_config() -> Result<RuntimeChatProvider, String> {
    let raw = read_file("./config/config.toml")
        .map_err(|error| format!("read config failed: {}", error))?;
    let default_provider =
        read_toml_section_value(&raw, "routing", "default_provider").unwrap_or_default();
    let default_model = read_toml_section_value(&raw, "routing", "default_model")
        .or_else(|| read_first_provider_model(&raw))
        .unwrap_or_default();
    if default_model.trim().is_empty() {
        return Err("missing default_model in runtime config".into());
    }

    let mut in_provider = false;
    let mut matched_provider = false;
    let mut provider_name = String::new();
    let mut base_url = String::new();
    let mut api_key = String::new();
    let mut in_key = false;
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("[[") {
            if trimmed == "[[providers]]" {
                if in_provider && matched_provider && !base_url.is_empty() && !api_key.is_empty() {
                    break;
                }
                in_provider = true;
                in_key = false;
                provider_name.clear();
                base_url.clear();
                api_key.clear();
                matched_provider = default_provider.trim().is_empty();
            } else if trimmed == "[[providers.keys]]" && in_provider {
                in_key = true;
            } else {
                in_key = false;
            }
            continue;
        }
        if !in_provider {
            continue;
        }
        if !in_key && trimmed.starts_with("name") {
            if let Some(value) = toml_string_value(trimmed) {
                provider_name = value.clone();
                matched_provider = default_provider.trim().is_empty()
                    || value.eq_ignore_ascii_case(&default_provider);
            }
            continue;
        }
        if !in_key && matched_provider && trimmed.starts_with("base_url") {
            if let Some(value) = toml_string_value(trimmed) {
                base_url = value;
            }
            continue;
        }
        if in_key && matched_provider && trimmed.starts_with("value") && api_key.is_empty() {
            if let Some(value) = toml_string_value(trimmed) {
                api_key = value;
            }
        }
    }

    if base_url.trim().is_empty() || api_key.trim().is_empty() {
        return Err(format!(
            "provider '{}' is missing base_url or key",
            if default_provider.trim().is_empty() {
                provider_name.as_str()
            } else {
                default_provider.as_str()
            }
        ));
    }
    Ok(RuntimeChatProvider {
        name: if provider_name.is_empty() {
            default_provider
        } else {
            provider_name
        },
        base_url,
        api_key,
        model: default_model,
    })
}

fn post_openai_chat_completion(
    provider: &RuntimeChatProvider,
    prompt: &str,
) -> Result<String, String> {
    let url = format!(
        "{}/chat/completions",
        provider.base_url.trim_end_matches('/')
    );
    let body = serde_json::json!({
        "model": provider.model,
        "messages": [{"role": "user", "content": prompt}],
        "temperature": 0.2,
        "max_tokens": 1600,
    })
    .to_string();
    let auth = format!("Authorization: Bearer {}", provider.api_key);
    let curl_args = vec![
        "-L".to_string(),
        "-sS".to_string(),
        "--max-time".to_string(),
        "75".to_string(),
        "-H".to_string(),
        "Content-Type: application/json".to_string(),
        "-H".to_string(),
        auth,
        "-d".to_string(),
        body,
        url,
    ];
    let refs = curl_args
        .iter()
        .map(|value| value.as_str())
        .collect::<Vec<_>>();
    let exec = exec_command("curl.exe", &refs).map_err(|error| format!("{}", error))?;
    if exec.status != 0 {
        return Err(format!(
            "{}{}",
            exec_stderr_text(&exec),
            exec_stdout_text(&exec)
        ));
    }
    let parsed = serde_json::from_str::<serde_json::Value>(exec_stdout_text(&exec).trim())
        .map_err(|error| format!("parse failed: {}", error))?;
    if let Some(error) = parsed.get("error") {
        return Err(error.to_string());
    }
    parsed
        .get("choices")
        .and_then(|value| value.as_array())
        .and_then(|choices| choices.first())
        .and_then(|choice| choice.get("message"))
        .and_then(|message| message.get("content"))
        .and_then(|value| value.as_str())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("provider '{}' returned empty content", provider.name))
}

fn json_string_escape(value: &str) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "\"\"".into())
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
    let url = decode_arg_text(args, "url");
    if url.trim().is_empty() {
        return PackageResult::err("missing url");
    }
    let method = decode_arg_text(args, "method");
    let method = if method.is_empty() {
        "GET"
    } else {
        method.as_str()
    };
    let body = args.get("body").and_then(|value| value.as_str());
    let curl_args = build_web_fetch_curl_args(method, &url, body);
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
    let stdout_text = exec.stdout_text();
    if let Some((status, response_body)) = parse_web_fetch_curl_output(&stdout_text) {
        if exec.status == 0 {
            return PackageResult::ok(serde_json::json!({"status": status, "body": response_body}));
        }
        let stderr_text = exec.stderr_text();
        let detail = if stderr_text.trim().is_empty() {
            format!("curl exited with status {}", exec.status)
        } else {
            stderr_text.trim().to_string()
        };
        return PackageResult::err(format!(
            "web fetch request failed with status {}: {}",
            status, detail
        ));
    }
    let stderr_text = exec.stderr_text();
    if !stderr_text.trim().is_empty() {
        return PackageResult::err(format!(
            "web fetch transport failed: {}",
            stderr_text.trim()
        ));
    }
    PackageResult::err("web fetch transport failed: missing HTTP status marker in curl output")
}

fn do_search_web(args: &serde_json::Value) -> PackageResult {
    let query = decode_arg_text(args, "query");
    if query.trim().is_empty() {
        return PackageResult::err("missing query");
    }
    let limit = args
        .get("limit")
        .and_then(|value| value.as_u64())
        .or_else(|| args.get("max_results").and_then(|value| value.as_u64()))
        .unwrap_or(5)
        .clamp(1, 10);
    let Some(api_key) = tavily_api_key(args) else {
        return PackageResult::err(
            "web_search provider tavily unavailable: missing TAVILY_API_KEY",
        );
    };
    let payload = format!("{{\"api_key\":{},\"query\":{},\"max_results\":{},\"search_depth\":\"basic\",\"include_answer\":false}}", json_string_escape(&api_key), json_string_escape(&query), limit);
    let curl_args = vec![
        "-L".to_string(),
        "-sS".to_string(),
        "--max-time".to_string(),
        "12".to_string(),
        "-H".to_string(),
        "Content-Type: application/json".to_string(),
        "-d".to_string(),
        payload,
        "https://api.tavily.com/search".to_string(),
    ];
    let curl_arg_refs = curl_args
        .iter()
        .map(|value| value.as_str())
        .collect::<Vec<_>>();
    let exec = match exec_command("curl.exe", &curl_arg_refs) {
        Ok(result) => result,
        Err(error) => {
            return PackageResult::err(format!("web_search provider tavily failed: {}", error))
        }
    };
    if exec.status != 0 {
        return PackageResult::err(format!(
            "web_search provider tavily failed: {}{}",
            exec.stderr_text(),
            exec.stdout_text()
        ));
    }
    let parsed = match serde_json::from_str::<serde_json::Value>(exec.stdout_text().trim()) {
        Ok(value) => value,
        Err(error) => {
            return PackageResult::err(format!(
                "web_search provider tavily parse failed: {}",
                error
            ))
        }
    };
    let results = parsed.get("results").and_then(|value| value.as_array()).map(|entries| {
        entries.iter().take(limit as usize).map(|entry| serde_json::json!({"title": entry.get("title").and_then(|value| value.as_str()).unwrap_or(""), "url": entry.get("url").and_then(|value| value.as_str()).unwrap_or(""), "text": entry.get("content").and_then(|value| value.as_str()).unwrap_or(""), "score": entry.get("score").cloned().unwrap_or_default()})).filter(|entry| entry.get("url").and_then(|value| value.as_str()).unwrap_or("").starts_with("http")).collect::<Vec<_>>()
    }).unwrap_or_default();
    let links = results.iter().map(|entry| serde_json::json!({"title": entry.get("title").cloned().unwrap_or_default(), "url": entry.get("url").cloned().unwrap_or_default(), "snippet": entry.get("text").cloned().unwrap_or_default()})).collect::<Vec<_>>();
    if links.is_empty() {
        return PackageResult::err("web_search provider tavily returned no results");
    }
    PackageResult::ok(
        serde_json::json!({"query": query, "provider": "tavily", "heading": "", "results": results, "links": links, "summary": parsed.get("answer").and_then(|value| value.as_str()).unwrap_or(""), "source": "Tavily Search API", "notes": ""}),
    )
}

fn strip_html_to_text(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut in_tag = false;
    let mut last_was_space = false;
    let mut entity = String::new();
    let mut in_entity = false;
    for ch in input.chars() {
        if in_tag {
            if ch == '>' {
                in_tag = false;
                if !last_was_space {
                    output.push(' ');
                    last_was_space = true;
                }
            }
            continue;
        }
        if ch == '<' {
            in_tag = true;
            continue;
        }
        if in_entity {
            if ch == ';' {
                let decoded = match entity.as_str() {
                    "amp" => "&",
                    "lt" => "<",
                    "gt" => ">",
                    "quot" => "\"",
                    "apos" | "#39" => "'",
                    "nbsp" => " ",
                    _ => " ",
                };
                output.push_str(decoded);
                last_was_space = decoded.trim().is_empty();
                entity.clear();
                in_entity = false;
            } else if entity.len() < 12 {
                entity.push(ch);
            } else {
                entity.clear();
                in_entity = false;
            }
            continue;
        }
        if ch == '&' {
            in_entity = true;
            entity.clear();
            continue;
        }
        if ch.is_whitespace() {
            if !last_was_space {
                output.push(' ');
                last_was_space = true;
            }
        } else {
            output.push(ch);
            last_was_space = false;
        }
    }
    output.trim().to_string()
}

fn do_fetch_url(args: &serde_json::Value) -> PackageResult {
    let url = decode_arg_text(args, "url");
    if url.trim().is_empty() {
        return PackageResult::err("missing url");
    }
    let title = decode_arg_text(args, "title");
    let fetch_result = do_web_fetch(&serde_json::json!({"url": url}));
    if fetch_result.status != "ok" {
        return fetch_result;
    }
    let body = fetch_result
        .data
        .as_ref()
        .and_then(|data| data.get("body"))
        .and_then(|value| value.as_str())
        .unwrap_or("");
    let mut content = strip_html_to_text(body);
    if content.is_empty() {
        content = body.trim().to_string();
    }
    if content.len() > 80_000 {
        content.truncate(80_000);
        content.push_str("\n\n[内容已截断]");
    }
    let label = if title.is_empty() {
        url.as_str()
    } else {
        title.as_str()
    };
    let summary_body = if content.len() > 8_000 {
        &content[..8_000]
    } else {
        content.as_str()
    };
    PackageResult::ok(
        serde_json::json!({"ok": true, "url": url, "title": title, "content": content, "summary": format!("[网页内容：{}]\n\n{}", label, summary_body)}),
    )
}

fn plugin_dir() -> Option<PathBuf> {
    env_get("WEFT_PACKAGE_DIR")
        .map(|value| value.strip_prefix(r"\\?\").unwrap_or(&value).to_string())
        .map(PathBuf::from)
}

fn desktop_runtime_script(name: &str) -> Option<String> {
    let dir = plugin_dir()?;
    Some(
        dir.join("desktop-runtime")
            .join("scripts")
            .join(name)
            .to_string_lossy()
            .to_string(),
    )
}

fn local_scout_python_dir() -> Option<String> {
    let dir = plugin_dir()?;
    Some(dir.join("ai-local-scout").to_string_lossy().to_string())
}

fn build_blink_payload(args: &serde_json::Value) -> serde_json::Value {
    serde_json::json!({"task": decode_arg_text(args, "task"), "forceSystem": args.get("force_system").and_then(|value| value.as_bool()).unwrap_or(true), "screenIndex": args.get("screen_index").and_then(|value| value.as_u64()).unwrap_or(1), "prevSummaries": args.get("prev_summaries").cloned().unwrap_or_else(|| serde_json::json!([]))})
}

fn parse_json_or_last_object(raw: &str) -> Result<serde_json::Value, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err("empty output".into());
    }
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(trimmed) {
        return Ok(value);
    }
    parse_last_balanced_json_object(trimmed).ok_or_else(|| "missing JSON object".into())
}

fn parse_last_balanced_json_object(raw: &str) -> Option<serde_json::Value> {
    let bytes = raw.as_bytes();
    for start in raw.match_indices('{').map(|(index, _)| index).rev() {
        let mut depth = 0i32;
        let mut in_string = false;
        let mut escaped = false;
        for offset in start..bytes.len() {
            let byte = bytes[offset];
            if in_string {
                if escaped {
                    escaped = false;
                } else if byte == b'\\' {
                    escaped = true;
                } else if byte == b'"' {
                    in_string = false;
                }
                continue;
            }
            match byte {
                b'"' => in_string = true,
                b'{' => depth += 1,
                b'}' => {
                    depth -= 1;
                    if depth == 0 {
                        if let Ok(value) =
                            serde_json::from_str::<serde_json::Value>(&raw[start..=offset])
                        {
                            return Some(value);
                        }
                        break;
                    }
                }
                _ => {}
            }
        }
    }
    None
}

fn parse_blink_stdout(stdout: &str) -> Result<serde_json::Value, String> {
    let outer = parse_json_or_last_object(stdout)?;
    if let Some(encoded) = outer.get("payload_b64").and_then(|value| value.as_str()) {
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(encoded)
            .map_err(|error| error.to_string())?;
        return serde_json::from_slice::<serde_json::Value>(&bytes)
            .map_err(|error| error.to_string());
    }
    Ok(outer)
}

fn run_desktop_script(
    script_name: &str,
    payload: serde_json::Value,
    timeout_ms: Option<u64>,
    parse_blink: bool,
) -> PackageResult {
    let Some(script) = desktop_runtime_script(script_name) else {
        return PackageResult::err(format!(
            "desktop tool failed: missing WEFT_PACKAGE_DIR for {}",
            script_name
        ));
    };
    let payload_text = serde_json::to_string(&payload).unwrap_or_else(|_| "{}".into());
    run_desktop_command(&[script, payload_text], timeout_ms, parse_blink)
}

fn run_meeting_record(args: &serde_json::Value) -> PackageResult {
    let Some(script) = desktop_runtime_script("meeting-control.js") else {
        return PackageResult::err(
            "desktop tool failed: missing WEFT_PACKAGE_DIR for meeting-control.js",
        );
    };
    run_desktop_command(
        &[
            script,
            decode_arg_text(args, "action"),
            decode_arg_text(args, "output_dir"),
        ],
        None,
        false,
    )
}

fn run_desktop_command(
    command: &[String],
    timeout_ms: Option<u64>,
    parse_blink: bool,
) -> PackageResult {
    let command_args = command
        .iter()
        .filter(|value| !value.trim().is_empty())
        .map(|value| value.as_str())
        .collect::<Vec<_>>();
    let exec = if let Some(timeout_ms) = timeout_ms {
        exec_command_advanced_with_options(
            "node",
            &command_args,
            &ExecAdvancedOptions {
                timeout_ms: Some(timeout_ms),
                ..Default::default()
            },
        )
    } else {
        exec_command("node", &command_args)
    };
    let exec = match exec {
        Ok(result) => result,
        Err(error) => return PackageResult::err(format!("desktop tool failed: {}", error)),
    };
    if exec.status != 0 {
        return PackageResult::err(format!(
            "desktop tool failed: {}{}",
            exec_stderr_text(&exec),
            exec_stdout_text(&exec)
        ));
    }
    let stdout_text = exec_stdout_text(&exec);
    let parsed = if parse_blink {
        parse_blink_stdout(stdout_text.trim())
    } else {
        parse_json_or_last_object(stdout_text.trim())
    };
    match parsed {
        Ok(value) => PackageResult::ok(value),
        Err(error) => PackageResult::err(format!("desktop tool parse failed: {}", error)),
    }
}

fn do_meeting_notes(args: &serde_json::Value) -> PackageResult {
    let meeting_title = decode_arg_text(args, "meeting_title");
    let transcript = decode_arg_text(args, "transcript");
    let request = decode_arg_text(args, "request");
    let mut final_prompt = String::new();
    if !meeting_title.trim().is_empty() {
        final_prompt.push_str(&format!("Meeting title: {}\n\n", meeting_title.trim()));
    }
    if !transcript.trim().is_empty() {
        final_prompt
            .push_str("Create professional meeting notes from this transcript.\n\nTranscript:\n");
        final_prompt.push_str(transcript.trim());
    } else if !request.trim().is_empty() {
        final_prompt.push_str(request.trim());
    } else {
        final_prompt
            .push_str("Create concise structured meeting notes for the current meeting context.");
    }
    let provider = match default_chat_provider_from_runtime_config() {
        Ok(provider) => provider,
        Err(error) => {
            return PackageResult::err(format!("meeting notes completion failed: {}", error))
        }
    };
    let prompt = format!("Return meeting notes in markdown with sections: Summary, Decisions, Action Items, Follow-ups.\n\n{}", final_prompt);
    let content = match post_openai_chat_completion(&provider, &prompt) {
        Ok(content) => content,
        Err(error) => {
            return PackageResult::err(format!("meeting notes completion failed: {}", error))
        }
    };
    PackageResult::ok(
        serde_json::json!({"ok": true, "meeting_title": meeting_title, "transcript": transcript, "content": content, "files": []}),
    )
}

fn do_mode_tool(args: &serde_json::Value) -> PackageResult {
    let event_type = first_non_empty_string(args, &["event_type", "eventType"]);
    let payload = args
        .get("payload")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));
    let title = payload
        .get("title")
        .and_then(|value| value.as_str())
        .unwrap_or("")
        .trim()
        .to_string();
    let result = match event_type.as_str() {
        "game_context_detected"
        | "video_context_detected"
        | "active_url_changed"
        | "reading_page_detected" => {
            let content_kind = if event_type == "game_context_detected" {
                "game"
            } else {
                "video"
            };
            let skill_refs = if content_kind == "video" {
                serde_json::json!(["video-watcher"])
            } else {
                serde_json::json!([])
            };
            serde_json::json!({"mode_id": "content_mode", "purpose": "Maintain lightweight awareness while the user is consuming or interacting with visual content.", "description": "The user is consuming or interacting with visual content, such as a game, video, stream, page, or other screen-first media. This mode only tells context-engine which lightweight enhancers may run; it does not decide whether the companion should speak, remember, or act.", "status": "content", "enhancers": {"probes": [{"tool": "blink_once", "cooldown_ms": 10000, "max_calls_per_window": 1}]}, "tool_refs": ["blink_once"], "skill_refs": skill_refs, "context_engine": {"vision": true}, "context": {"event_type": event_type, "content_kind": content_kind, "title": title}})
        }
        _ => {
            serde_json::json!({"mode_id": "none", "purpose": "No proactive mode guidance for this context.", "status": "none", "enhancers": {"probes": []}, "tool_refs": [], "skill_refs": [], "context_engine": {}, "context": {"event_type": event_type, "title": title}})
        }
    };
    PackageResult::ok(result)
}

fn do_local_scout(args: &serde_json::Value) -> PackageResult {
    let Some(scout_dir) = local_scout_python_dir() else {
        return PackageResult::err("local_scout failed: missing WEFT_PACKAGE_DIR");
    };
    let root = args.get("root").and_then(|v| v.as_str()).unwrap_or("~");
    let max_depth = args
        .get("max_depth")
        .and_then(|v| v.as_u64())
        .unwrap_or(4)
        .to_string();
    let timeout_ms = args
        .get("timeout_ms")
        .and_then(|v| v.as_u64())
        .unwrap_or(60_000);
    let mut cmd_args = vec![
        "-m",
        "ai_local_scout.cli",
        "run",
        "--root",
        root,
        "--max-depth",
        &max_depth,
    ];
    let output_arg;
    if let Some(output) = args.get("output").and_then(|v| v.as_str()) {
        output_arg = output.to_string();
        cmd_args.extend_from_slice(&["--output", &output_arg]);
    }
    let exec = exec_command_advanced_with_options(
        "python",
        &cmd_args,
        &ExecAdvancedOptions {
            workdir: Some(scout_dir),
            timeout_ms: Some(timeout_ms),
            ..Default::default()
        },
    );
    let exec = match exec {
        Ok(r) => r,
        Err(e) => return PackageResult::err(format!("local_scout exec failed: {}", e)),
    };
    if exec.status != 0 {
        return PackageResult::err(format!(
            "local_scout failed: {}{}",
            exec_stderr_text(&exec),
            exec_stdout_text(&exec)
        ));
    }
    let stdout = exec_stdout_text(&exec);
    match parse_json_or_last_object(stdout.trim()) {
        Ok(v) => PackageResult::ok(v),
        Err(_) => PackageResult::ok(serde_json::json!({"raw": stdout})),
    }
}

fn do_wiki_contract(_args: &serde_json::Value) -> PackageResult {
    PackageResult::ok(
        serde_json::json!({"contract": WEFT_WIKI_CONTRACT, "tools": ["wiki_contract", "wiki_write"], "durable_path": "agent -> wiki_write -> wiki document -> palace sync"}),
    )
}

fn slugify_wiki_filename(value: &str) -> String {
    let mut out = String::new();
    let mut last_dash = false;
    for ch in value.trim().to_lowercase().chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch);
            last_dash = false;
        } else if !last_dash {
            out.push('-');
            last_dash = true;
        }
    }
    let stem = out.trim_matches('-');
    let stem = if stem.is_empty() { "wiki-page" } else { stem };
    if stem.ends_with(".md") {
        stem.to_string()
    } else {
        format!("{}.md", stem)
    }
}

fn normalize_wiki_match_text(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                ' '
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn wiki_existing_document_score(
    title: &str,
    filename: &str,
    candidate: &serde_json::Value,
) -> usize {
    let title_key = normalize_wiki_match_text(title);
    let filename_key = normalize_wiki_match_text(filename.trim_end_matches(".md"));
    let candidate_title = normalize_wiki_match_text(
        candidate
            .get("title")
            .and_then(|value| value.as_str())
            .unwrap_or(""),
    );
    let candidate_filename = normalize_wiki_match_text(
        candidate
            .get("filename")
            .and_then(|value| value.as_str())
            .unwrap_or("")
            .trim_end_matches(".md"),
    );
    if candidate_filename.is_empty() && candidate_title.is_empty() {
        return 0;
    }
    if !filename_key.is_empty() && candidate_filename == filename_key {
        return 100;
    }
    if !title_key.is_empty() && candidate_title == title_key {
        return 90;
    }
    if !filename_key.is_empty()
        && (candidate_filename.contains(&filename_key)
            || filename_key.contains(&candidate_filename))
    {
        return 70;
    }
    if !title_key.is_empty()
        && (candidate_title.contains(&title_key) || title_key.contains(&candidate_title))
    {
        return 60;
    }
    0
}

fn resolve_wiki_write_existing_target(
    api_root: &str,
    token: &str,
    workspace_id: &str,
    path: &str,
    title: &str,
    filename: &str,
) -> Option<(String, String)> {
    if workspace_id.trim().is_empty() || path.trim() != "/wiki/" {
        return None;
    }
    let query_workspace = workspace_id.trim().replace(' ', "%20");
    let query_path = path.trim().replace('/', "%2F");
    let route = format!(
        "/v1/workspace-wiki/documents?workspace_id={}&path={}",
        query_workspace, query_path
    );
    let documents = get_wiki_json(api_root, token, &route).ok()?;
    let rows = documents.as_array()?;
    rows.iter()
        .filter_map(|row| {
            let score = wiki_existing_document_score(title, filename, row);
            if score == 0 {
                return None;
            }
            let existing_filename = row
                .get("filename")
                .and_then(|value| value.as_str())
                .unwrap_or("")
                .trim();
            let existing_path = row
                .get("path")
                .and_then(|value| value.as_str())
                .unwrap_or("")
                .trim();
            if existing_filename.is_empty() || existing_path.is_empty() {
                return None;
            }
            Some((
                score,
                existing_filename.to_string(),
                existing_path.to_string(),
            ))
        })
        .max_by_key(|(score, _, _)| *score)
        .map(|(_, existing_filename, existing_path)| (existing_filename, existing_path))
}

fn normalize_wiki_authoring_unit(value: &str) -> String {
    match value.trim().to_ascii_lowercase().as_str() {
        "article" => "article".into(),
        "memo" => "memo".into(),
        "artifact" => "artifact".into(),
        _ => "article".into(),
    }
}

fn ensure_wiki_title(content: &str, title: &str) -> String {
    let trimmed = content.trim();
    if trimmed.starts_with("# ") {
        trimmed.to_string()
    } else {
        format!("# {}\n\n{}", title.trim(), trimmed)
    }
}

fn wiki_content_has_contract_shape(content: &str) -> bool {
    let lower = content.to_ascii_lowercase();
    let has_media_section = [
        "\n## media",
        "\n## visual",
        "\n## diagram",
        "\n## image",
        "\n## sources",
        "\n## evidence",
        "\n## 媒体",
        "\n## 图片",
        "\n## 图表",
        "\n## 证据",
    ]
    .iter()
    .any(|marker| lower.contains(marker));
    let has_reference_section = [
        "\n## references",
        "\n## reference",
        "\n## sources",
        "\n## source",
        "\n## evidence",
        "\n## citations",
        "\n## 参考",
        "\n## 资料来源",
        "\n## 来源",
        "\n## 证据",
    ]
    .iter()
    .any(|marker| lower.contains(marker));
    let has_visual_element = content.contains("![")
        || lower.contains("```mermaid")
        || lower.contains(".excalidraw")
        || lower.contains("|---")
        || lower.contains("youtube.com")
        || lower.contains("bilibili.com");
    content.trim_start().starts_with("# ")
        && lower.contains("\n## ")
        && (has_media_section || has_visual_element)
        && has_reference_section
        && has_visual_element
        && (content.contains("http://") || content.contains("https://") || content.contains("[^"))
}

fn wiki_write_api_root(args: &serde_json::Value) -> String {
    let from_args = first_non_empty_string(args, &["api_root"]);
    if !from_args.trim().is_empty() {
        return from_args.trim().trim_end_matches('/').to_string();
    }
    env_get("WEFT_WIKI_API_ROOT")
        .unwrap_or_else(|| "http://127.0.0.1:18000".into())
        .trim()
        .trim_end_matches('/')
        .to_string()
}

fn wiki_write_auth_token(args: &serde_json::Value) -> String {
    let from_args = first_non_empty_string(args, &["auth_token", "token"]);
    if !from_args.trim().is_empty() {
        return from_args.trim().to_string();
    }
    env_get("WEFT_WIKI_AUTH_TOKEN").unwrap_or_else(|| "local-dev-token".into())
}

fn wiki_write_assets_array(args: &serde_json::Value) -> Vec<serde_json::Value> {
    if let Some(assets) = args.get("assets").and_then(|value| value.as_array()) {
        return assets.clone();
    }
    if let Some(raw_assets) = args.get("assets").and_then(|value| value.as_str()) {
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(raw_assets) {
            if let Some(assets) = parsed.as_array() {
                return assets.clone();
            }
        }
    }
    Vec::new()
}

fn wiki_asset_target(asset: &serde_json::Value) -> String {
    asset
        .get("url")
        .or_else(|| asset.get("source_url"))
        .or_else(|| asset.get("href"))
        .or_else(|| asset.get("path"))
        .or_else(|| asset.get("file_path"))
        .or_else(|| asset.get("local_path"))
        .and_then(|value| value.as_str())
        .unwrap_or("")
        .trim()
        .to_string()
}

fn wiki_asset_markdown_line(asset: &serde_json::Value) -> Option<String> {
    let target = wiki_asset_target(asset);
    if target.is_empty() {
        return None;
    }
    let caption = asset
        .get("caption")
        .or_else(|| asset.get("name"))
        .and_then(|value| value.as_str())
        .unwrap_or("Source asset")
        .replace(['[', ']'], "");
    let lower = target.to_ascii_lowercase();
    if [".png", ".jpg", ".jpeg", ".webp", ".gif", ".svg"]
        .iter()
        .any(|suffix| lower.ends_with(suffix))
    {
        Some(format!("![{}]({})", caption, target))
    } else {
        Some(format!("[{}]({})", caption, target))
    }
}

fn ensure_wiki_assets_referenced_in_content(content: &str, assets: &[serde_json::Value]) -> String {
    let missing_lines = assets
        .iter()
        .filter_map(|asset| {
            let target = wiki_asset_target(asset);
            if target.is_empty() || content.contains(&target) {
                None
            } else {
                wiki_asset_markdown_line(asset)
            }
        })
        .collect::<Vec<_>>();
    if missing_lines.is_empty() {
        return content.to_string();
    }
    format!(
        "{}\n\n## Media\n\n{}\n",
        content.trim_end(),
        missing_lines.join("\n\n")
    )
}

fn wiki_write_asset_requests(
    args: &serde_json::Value,
    parent_document_id: &str,
    workspace_id: &str,
) -> Vec<serde_json::Value> {
    wiki_write_assets_array(args).iter().filter_map(|asset| {
        let object = asset.as_object()?;
        let path = object.get("path").or_else(|| object.get("file_path")).or_else(|| object.get("local_path")).and_then(|value| value.as_str()).unwrap_or("").trim();
        let url = object.get("url").or_else(|| object.get("source_url")).or_else(|| object.get("href")).and_then(|value| value.as_str()).unwrap_or("").trim();
        if path.is_empty() && url.is_empty() { return None; }
        Some(serde_json::json!({"path": path, "url": url, "caption": object.get("caption").or_else(|| object.get("name")).and_then(|value| value.as_str()).unwrap_or(""), "note": object.get("note").and_then(|value| value.as_str()).unwrap_or(""), "summary": object.get("summary").and_then(|value| value.as_str()).unwrap_or(""), "transcript": object.get("transcript").and_then(|value| value.as_str()).unwrap_or(""), "ocr_text": object.get("ocr_text").and_then(|value| value.as_str()).unwrap_or(""), "extracted_text": object.get("extracted_text").and_then(|value| value.as_str()).unwrap_or(""), "kind": object.get("kind").or_else(|| object.get("type")).and_then(|value| value.as_str()).unwrap_or(""), "parent_document_id": parent_document_id, "workspace_id": workspace_id}))
    }).collect()
}

fn post_wiki_json(
    api_root: &str,
    token: &str,
    path: &str,
    body: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let url = format!("{}{}", api_root, path);
    let body_text = body.to_string();
    let curl_args = vec![
        "-L".to_string(),
        "-sS".to_string(),
        "--max-time".to_string(),
        "15".to_string(),
        "-H".to_string(),
        "Content-Type: application/json".to_string(),
        "-H".to_string(),
        format!("Authorization: Bearer {}", token),
        "-d".to_string(),
        body_text,
        url,
    ];
    let curl_arg_refs = curl_args
        .iter()
        .map(|value| value.as_str())
        .collect::<Vec<_>>();
    let exec = exec_command("curl.exe", &curl_arg_refs)
        .map_err(|error| format!("transport failed: {}", error))?;
    if exec.status != 0 {
        return Err(format!(
            "transport failed: {}{}",
            exec.stderr_text(),
            exec.stdout_text()
        ));
    }
    serde_json::from_str::<serde_json::Value>(exec.stdout_text().trim())
        .map_err(|error| format!("parse failed: {}", error))
}

fn get_wiki_json(api_root: &str, token: &str, path: &str) -> Result<serde_json::Value, String> {
    let url = format!("{}{}", api_root, path);
    let curl_args = vec![
        "-L".to_string(),
        "-sS".to_string(),
        "--max-time".to_string(),
        "8".to_string(),
        "-H".to_string(),
        format!("Authorization: Bearer {}", token),
        url,
    ];
    let curl_arg_refs = curl_args
        .iter()
        .map(|value| value.as_str())
        .collect::<Vec<_>>();
    let exec = exec_command("curl.exe", &curl_arg_refs)
        .map_err(|error| format!("transport failed: {}", error))?;
    if exec.status != 0 {
        return Err(format!(
            "transport failed: {}{}",
            exec.stderr_text(),
            exec.stdout_text()
        ));
    }
    serde_json::from_str::<serde_json::Value>(exec.stdout_text().trim())
        .map_err(|error| format!("parse failed: {}", error))
}

fn do_wiki_write(args: &serde_json::Value) -> PackageResult {
    let api_root = wiki_write_api_root(args);
    let token = wiki_write_auth_token(args);
    let title = first_non_empty_string(args, &["title"]);
    if title.trim().is_empty() {
        return PackageResult::err("wiki_write requires title");
    }
    let raw_content = decode_arg_text(args, "content");
    if raw_content.trim().is_empty() {
        return PackageResult::err("wiki_write requires content");
    }
    let assets = wiki_write_assets_array(args);
    let content =
        ensure_wiki_assets_referenced_in_content(&ensure_wiki_title(&raw_content, &title), &assets);
    if !wiki_content_has_contract_shape(&content) {
        return PackageResult::err("wiki_write rejected content that does not satisfy the wiki contract: include title, lead, sections, media/visual, and references/evidence");
    }
    let authoring_unit =
        normalize_wiki_authoring_unit(&first_non_empty_string(args, &["authoring_unit"]));
    let filename = first_non_empty_string(args, &["filename"]);
    let mut filename = if filename.trim().is_empty() {
        slugify_wiki_filename(&title)
    } else if filename.trim().ends_with(".md") {
        filename.trim().to_string()
    } else {
        format!("{}.md", filename.trim())
    };
    let path = first_non_empty_string(args, &["path"]);
    let mut path = if path.trim().is_empty() {
        "/wiki/".into()
    } else {
        path
    };
    let workspace_id = first_non_empty_string(args, &["workspace_id"]);
    if workspace_id.trim().is_empty() {
        return PackageResult::err("wiki_write requires workspace_id");
    }
    if let Some((existing_filename, existing_path)) = resolve_wiki_write_existing_target(
        &api_root,
        &token,
        workspace_id.trim(),
        &path,
        &title,
        &filename,
    ) {
        filename = existing_filename;
        path = existing_path;
    }
    let page_family = first_non_empty_string(args, &["page_family"]);
    let cluster_id = first_non_empty_string(args, &["cluster_id"]);
    let mut metadata = args
        .get("metadata")
        .and_then(|value| value.as_object())
        .cloned()
        .unwrap_or_else(serde_json::Map::new);
    metadata.insert("authoring_unit".into(), serde_json::json!(authoring_unit));
    metadata.insert(
        "page_family".into(),
        serde_json::json!(if page_family.trim().is_empty() {
            "topic"
        } else {
            page_family.trim()
        }),
    );
    metadata.insert("authored_by".into(), serde_json::json!("weft-agent"));
    metadata.insert(
        "authoring_path".into(),
        serde_json::json!("agent->wiki_write->wiki->palace"),
    );
    metadata.insert(
        "workspace_id".into(),
        serde_json::json!(workspace_id.trim()),
    );
    if !cluster_id.trim().is_empty() {
        metadata.insert("cluster_id".into(), serde_json::json!(cluster_id.trim()));
    }
    let body = serde_json::json!({"filename": filename, "path": path, "content": content, "metadata": metadata});
    let parsed = match post_wiki_json(&api_root, &token, "/v1/workspace-wiki/documents/note", body)
    {
        Ok(value) => value,
        Err(error) => return PackageResult::err(format!("wiki_write failed: {}", error)),
    };
    let parent_document_id = parsed
        .get("id")
        .and_then(|value| value.as_str())
        .unwrap_or("");
    let mut registered_assets = Vec::new();
    for asset_body in wiki_write_asset_requests(args, parent_document_id, workspace_id.trim()) {
        match post_wiki_json(
            &api_root,
            &token,
            "/v1/workspace-wiki/documents/local-asset",
            asset_body,
        ) {
            Ok(value) => registered_assets.push(value),
            Err(error) => {
                return PackageResult::err(format!(
                    "wiki_write asset registration failed: {}",
                    error
                ))
            }
        }
    }
    PackageResult::ok(
        serde_json::json!({"created": true, "document": parsed, "assets": registered_assets, "contract": {"authoring_unit": metadata.get("authoring_unit").cloned().unwrap_or(serde_json::Value::Null), "path": path, "filename": filename, "palace_sync": "wiki API syncs the created document into palace"}}),
    )
}
