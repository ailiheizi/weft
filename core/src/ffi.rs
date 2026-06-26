//! Flutter Rust Bridge 桥接层:暴露给 Flutter 客户端的公开 API。
//! flutter_rust_bridge_codegen 扫描此文件生成 Dart 绑定。
//!
//! core 直接在 DLL 进程内运行(真正的 FFI,零子进程,无黑框)。

use std::path::PathBuf;
use std::sync::OnceLock;
use tokio::runtime::Runtime;

static RUNTIME: OnceLock<Runtime> = OnceLock::new();
static PROJECT_ROOT: OnceLock<PathBuf> = OnceLock::new();

fn rt() -> &'static Runtime {
    RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("Failed to create tokio runtime")
    })
}

/// 启动 weft-core:进程内直接运行(无子进程,无黑框)。
/// project_root: 项目根目录绝对路径(含 config/config.toml + packages/ + data/)。
/// 返回空字符串=成功(server 在后台跑),非空=错误信息。
pub fn start_core(project_root: String) -> String {
    let root = PathBuf::from(&project_root);
    if !root.join("config").join("config.toml").exists() {
        return format!("config/config.toml not found in {}", project_root);
    }

    // 保存 project root 绝对路径(rpc_call 用来读 runtime-token,不依赖 cwd)。
    let _ = PROJECT_ROOT.set(root.clone());

    // 设置 cwd 为项目根(core 依赖 cwd 解析相对路径)。
    if let Err(e) = std::env::set_current_dir(&root) {
        return format!("failed to set cwd to {}: {}", project_root, e);
    }

    // 在后台线程启动 tokio runtime + run_server(进程内,不阻塞 Dart)。
    std::thread::spawn(move || {
        rt().block_on(async {
            if let Err(e) = crate::runtime::run_server().await {
                eprintln!("[weft-ffi] run_server error: {e}");
            }
        });
    });

    // 等 health 就绪(最多 90s,debug 模式下 WASM 加载可能较慢)。
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(90);
    while std::time::Instant::now() < deadline {
        if let Ok(resp) = reqwest::blocking::get("http://127.0.0.1:17830/api/health") {
            if resp.status().is_success() {
                return String::new(); // 成功
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(500));
    }
    "core started but health check timed out after 30s".to_string()
}

/// 统一 RPC 调用:直接内部 dispatch(不走网络,真正的 FFI)。
pub fn rpc_call(request_json: String) -> String {
    let envelope: crate::api::rpc::RequestEnvelope = match serde_json::from_str(&request_json) {
        Ok(e) => e,
        Err(e) => {
            return serde_json::json!({
                "id": "",
                "status": 400,
                "body": { "error": format!("parse error: {e}") }
            }).to_string();
        }
    };

    // 用绝对路径读 runtime-token(不依赖 cwd,Windows GUI 下 cwd 不稳定)。
    let token_path = PROJECT_ROOT
        .get()
        .map(|r| r.join("data").join("runtime-token"))
        .unwrap_or_else(|| PathBuf::from("./data/runtime-token"));
    let token = std::fs::read_to_string(&token_path)
        .unwrap_or_default()
        .trim()
        .to_string();

    // 直接内部 dispatch(router.oneshot),不走 HTTP 回环。
    let response = rt().block_on(async {
        crate::api::rpc::dispatch_internal(envelope, &token).await
    });

    serde_json::to_string(&response).unwrap_or_else(|_| {
        r#"{"id":"","status":500,"body":{"error":"serialize failed"}}"#.to_string()
    })
}

/// 停止 core。
pub fn stop_core() {
    let token_path = PROJECT_ROOT
        .get()
        .map(|r| r.join("data").join("runtime-token"))
        .unwrap_or_else(|| PathBuf::from("./data/runtime-token"));
    let token = std::fs::read_to_string(&token_path)
        .unwrap_or_default();
    let _ = reqwest::blocking::Client::new()
        .post("http://127.0.0.1:17830/api/shutdown")
        .bearer_auth(token.trim())
        .send();
}

/// 检查 core 是否在跑。
pub fn is_core_running() -> bool {
    reqwest::blocking::get("http://127.0.0.1:17830/api/health")
        .map(|r| r.status().is_success())
        .unwrap_or(false)
}
