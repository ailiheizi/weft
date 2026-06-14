use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use weft_package_sdk::*;

const PACKAGE_NAME: &str = "ffmpeg-runtime";
const CAPABILITY_NAME: &str = "video.render";

#[plugin_fn]
pub fn init(_input: String) -> FnResult<String> {
    log_info("ffmpeg-runtime initialized");
    Ok(PackageResult::ok_empty().to_json())
}

#[plugin_fn]
pub fn handle_ws_message(input: String) -> FnResult<String> {
    let req: WsRequest = serde_json::from_str(&input).unwrap_or(WsRequest {
        action: String::new(),
        data: Value::Null,
    });

    let result = match req.action.as_str() {
        "describe" => PackageResult::ok(json!({
            "package": PACKAGE_NAME,
            "capability": CAPABILITY_NAME,
            "actions": ["describe", "health", "probe", "concat", "export"],
        })),
        "health" => do_health(),
        "probe" => do_probe(&req.data),
        "concat" => do_concat(&req.data),
        "export" => do_export(&req.data),
        _ => PackageResult::err(format!("unknown action: {}", req.action)),
    };

    Ok(result.to_json())
}

fn do_health() -> PackageResult {
    match exec_command("ffmpeg", &["-version"]) {
        Ok(exec) if exec.status == 0 => PackageResult::ok(json!({
            "healthy": true,
            "package": PACKAGE_NAME,
            "ffmpeg_available": true,
            "version": first_line(&exec.stdout_text()),
        })),
        Ok(exec) => PackageResult::err(format!(
            "ffmpeg health check failed (status {}): {}",
            exec.status,
            exec_error_text(&exec)
        )),
        Err(error) => PackageResult::err(format!("ffmpeg health check failed: {}", error)),
    }
}

fn do_probe(data: &Value) -> PackageResult {
    let Some(path) = get_required_string(data, "path") else {
        return PackageResult::err("probe requires data.path");
    };

    let ffprobe_args = [
        "-v",
        "error",
        "-print_format",
        "json",
        "-show_format",
        "-show_streams",
        path.as_str(),
    ];
    if let Ok(exec) = exec_command("ffprobe", &ffprobe_args) {
        if exec.status == 0 {
            let stdout = exec.stdout_text();
            let parsed = serde_json::from_str::<Value>(&stdout).unwrap_or_else(|_| json!({ "raw": stdout }));
            return PackageResult::ok(json!({
                "tool": "ffprobe",
                "path": path,
                "probe": parsed,
            }));
        }
    }

    let fallback_args = ["-i", path.as_str()];
    match exec_command("ffmpeg", &fallback_args) {
        Ok(exec) => {
            let stderr = exec.stderr_text();
            PackageResult::ok(json!({
                "tool": "ffmpeg",
                "path": path,
                "status": exec.status,
                "duration": extract_between(&stderr, "Duration: ", ","),
                "resolution": extract_resolution(&stderr),
                "raw": stderr,
            }))
        }
        Err(error) => PackageResult::err(format!("probe failed: {}", error)),
    }
}

fn do_export(data: &Value) -> PackageResult {
    let Some(input) = get_required_string(data, "input") else {
        return PackageResult::err("export requires data.input");
    };
    let Some(output) = get_required_string(data, "output") else {
        return PackageResult::err("export requires data.output");
    };

    let preset = get_optional_string(data, "preset").unwrap_or_else(|| "medium".to_string());
    let format = get_optional_string(data, "format");

    let mut args = vec!["-y".to_string(), "-i".to_string(), input.clone()];
    if let Some(format) = format.clone() {
        args.push("-f".to_string());
        args.push(format);
    }
    args.push("-preset".to_string());
    args.push(preset.clone());
    args.push(output.clone());

    match run_ffmpeg(&args) {
        Ok(exec) => PackageResult::ok(json!({
            "input": input,
            "output": output,
            "format": format,
            "preset": preset,
            "status": exec.status,
            "stdout": exec.stdout_text(),
            "stderr": exec.stderr_text(),
        })),
        Err(error) => PackageResult::err(error),
    }
}

fn do_concat(data: &Value) -> PackageResult {
    let Some(output) = get_required_string(data, "output") else {
        return PackageResult::err("concat requires data.output");
    };
    let Some(segments) = data.get("segments").and_then(|value| value.as_array()) else {
        return PackageResult::err("concat requires data.segments");
    };
    if segments.is_empty() {
        return PackageResult::err("concat requires at least one segment");
    }

    let temp_root = temp_artifact_root(&output);
    let mut temp_files = Vec::new();

    for (index, segment) in segments.iter().enumerate() {
        let Some(input) = get_required_string(segment, "input") else {
            return PackageResult::err(format!("segment {} requires input", index));
        };
        let start = get_optional_string(segment, "start");
        let end = get_optional_string(segment, "end");
        let clip_path = format!("{}-segment-{}.mp4", temp_root, index);

        let mut clip_args = vec!["-y".to_string()];
        if let Some(start) = start.clone() {
            clip_args.push("-ss".to_string());
            clip_args.push(start);
        }
        clip_args.push("-i".to_string());
        clip_args.push(input);
        if let Some(end) = end.clone() {
            clip_args.push("-to".to_string());
            clip_args.push(end);
        }
        clip_args.extend([
            "-c:v".to_string(),
            "libx264".to_string(),
            "-c:a".to_string(),
            "aac".to_string(),
            clip_path.clone(),
        ]);

        if let Err(error) = run_ffmpeg(&clip_args) {
            return PackageResult::err(format!("concat segment {} failed: {}", index, error));
        }
        temp_files.push(clip_path);
    }

    // TODO: switch to stream-copy or filter_complex for better performance once the runtime flow is proven.
    let list_path = format!("{}-concat.txt", temp_root);
    let list_content = temp_files
        .iter()
        .map(|path| format!("file '{}'", path.replace('\\', "/")))
        .collect::<Vec<_>>()
        .join("\n");
    write_file(&list_path, &list_content);

    let concat_args = vec![
        "-y".to_string(),
        "-f".to_string(),
        "concat".to_string(),
        "-safe".to_string(),
        "0".to_string(),
        "-i".to_string(),
        list_path.clone(),
        "-c".to_string(),
        "copy".to_string(),
        output.clone(),
    ];

    match run_ffmpeg(&concat_args) {
        Ok(exec) => PackageResult::ok(json!({
            "output": output,
            "segment_count": segments.len(),
            "temp_files": temp_files,
            "concat_list": list_path,
            "status": exec.status,
            "stdout": exec.stdout_text(),
            "stderr": exec.stderr_text(),
        })),
        Err(error) => PackageResult::err(format!("concat failed: {}", error)),
    }
}

fn run_ffmpeg(args: &[String]) -> Result<ExecResult, String> {
    let refs = args.iter().map(|value| value.as_str()).collect::<Vec<_>>();
    match exec_command("ffmpeg", &refs) {
        Ok(exec) if exec.status == 0 => Ok(exec),
        Ok(exec) => Err(format!(
            "ffmpeg exited with status {}: {}",
            exec.status,
            exec_error_text(&exec)
        )),
        Err(error) => Err(format!("ffmpeg exec failed: {}", error)),
    }
}

fn exec_error_text(exec: &ExecResult) -> String {
    let stderr = exec.stderr_text();
    if !stderr.trim().is_empty() {
        stderr
    } else {
        exec.stdout_text()
    }
}

fn get_required_string(data: &Value, key: &str) -> Option<String> {
    data.get(key)
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn get_optional_string(data: &Value, key: &str) -> Option<String> {
    get_required_string(data, key)
}

fn first_line(value: &str) -> String {
    value.lines().next().unwrap_or("").trim().to_string()
}

fn extract_between(input: &str, start: &str, end: &str) -> Option<String> {
    let (_, tail) = input.split_once(start)?;
    let (value, _) = tail.split_once(end)?;
    Some(value.trim().to_string())
}

fn extract_resolution(stderr: &str) -> Option<String> {
    for line in stderr.lines() {
        if let Some(stream_part) = line.split("Video:").nth(1) {
            for token in stream_part.split(',') {
                let trimmed = token.trim();
                let mut parts = trimmed.split('x');
                let left = parts.next().unwrap_or("");
                let right = parts.next().unwrap_or("");
                if !left.is_empty()
                    && !right.is_empty()
                    && left.chars().all(|ch| ch.is_ascii_digit())
                    && right.chars().all(|ch| ch.is_ascii_digit())
                {
                    return Some(format!("{}x{}", left, right));
                }
            }
        }
    }
    None
}

fn temp_artifact_root(output: &str) -> String {
    let output_path = Path::new(output);
    let parent = output_path.parent().unwrap_or_else(|| Path::new("."));
    let stem = output_path
        .file_stem()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .unwrap_or("ffmpeg-runtime");
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0);

    let mut path = PathBuf::from(parent);
    path.push(format!("{}.tmp-{}", stem, nonce));
    path.to_string_lossy().into_owned()
}
