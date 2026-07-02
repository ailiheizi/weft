//! Flutter Rust Bridge 桥接层:暴露给 Flutter 客户端的公开 API。
//! flutter_rust_bridge_codegen 扫描此文件生成 Dart 绑定。
//!
//! core 直接在 DLL 进程内运行(真正的 FFI,零子进程,无黑框)。

use std::path::PathBuf;
use std::sync::OnceLock;
use tokio::runtime::Runtime;

static RUNTIME: OnceLock<Runtime> = OnceLock::new();
static PROJECT_ROOT: OnceLock<PathBuf> = OnceLock::new();
static DATA_DIR: OnceLock<PathBuf> = OnceLock::new();

fn rt() -> &'static Runtime {
    RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("Failed to create tokio runtime")
    })
}

/// 探测 17830 是否已被占用(残留 core 进程或其他程序)。
fn port_in_use() -> bool {
    std::net::TcpStream::connect_timeout(
        &"127.0.0.1:17830".parse().unwrap(),
        std::time::Duration::from_millis(300),
    )
    .is_ok()
}

/// 解析 runtime-token 文件路径(优先 DATA_DIR,回退 PROJECT_ROOT/data)。
fn resolve_token_path() -> Option<PathBuf> {
    DATA_DIR
        .get()
        .map(|d| d.join("runtime-token"))
        .or_else(|| PROJECT_ROOT.get().map(|r| r.join("data").join("runtime-token")))
}

/// 启动前清理残留 core:若 17830 被占,尝试优雅关闭占用者(残留 core 暴露 /api/shutdown),
/// 然后等端口释放。最多等 ~5s。返回 true=端口已空闲可用。
fn ensure_port_free() -> bool {
    if !port_in_use() {
        return true;
    }
    // 端口被占:尝试用 runtime-token 优雅关闭残留 core。
    let token = resolve_token_path()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .unwrap_or_default();
    let _ = reqwest::blocking::Client::new()
        .post("http://127.0.0.1:17830/api/shutdown")
        .bearer_auth(token.trim())
        .timeout(std::time::Duration::from_secs(3))
        .send();
    // 等端口释放(最多 ~5s)。
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while std::time::Instant::now() < deadline {
        if !port_in_use() {
            return true;
        }
        std::thread::sleep(std::time::Duration::from_millis(250));
    }
    !port_in_use()
}

/// 启动 weft-core:进程内直接运行(无子进程,无黑框)。
/// project_root: 项目根目录绝对路径(含 packages/)。
///               config/config.toml 不再强制要求 — 不存在时用默认配置。
/// 返回空字符串=成功(server 在后台跑),非空=错误信息。
pub fn start_core(project_root: String) -> String {
    start_core_with_options(project_root, String::new(), String::new())
}

/// 完整 FFI 启动入口:接受显式的 data_dir 和 config_dir。
/// - project_root: 运行时根目录(含 packages/ 子目录)。
/// - data_dir: 可写数据目录(存 runtime-token / KV / wasm-cache)。
///             空字符串 = 默认(project_root/data)。
/// - config_dir: 配置目录(含 config.toml)。
///               空字符串 = 默认(project_root/config)。
/// 返回空字符串=成功,非空=错误信息。
pub fn start_core_with_options(project_root: String, data_dir: String, config_dir: String) -> String {
    let root = PathBuf::from(&project_root);

    // 保存 project root 绝对路径。
    let _ = PROJECT_ROOT.set(root.clone());

    // 解析 data_dir(优先显式参数,否则 root/data)。
    let effective_data_dir = if data_dir.is_empty() {
        root.join("data")
    } else {
        PathBuf::from(&data_dir)
    };
    // 确保 data_dir 存在。
    let _ = std::fs::create_dir_all(&effective_data_dir);
    let _ = DATA_DIR.set(effective_data_dir.clone());

    // 解析 config_dir(优先显式参数,否则 root/config)。
    let effective_config_dir = if config_dir.is_empty() {
        root.join("config")
    } else {
        PathBuf::from(&config_dir)
    };

    // 设置 cwd 为项目根(run_server 依赖 cwd 解析 repo_root)。
    if let Err(e) = std::env::set_current_dir(&root) {
        return format!("failed to set cwd to {}: {}", project_root, e);
    }

    // 注入环境变量:run_server 会在 args 解析后检查这些变量做 FFI 模式覆盖。
    std::env::set_var("WEFT_FFI_DATA_DIR", effective_data_dir.to_string_lossy().as_ref());
    std::env::set_var("WEFT_FFI_CONFIG_DIR", effective_config_dir.to_string_lossy().as_ref());

    // 启动前清残留。
    if !ensure_port_free() {
        eprintln!("[weft-ffi] port 17830 still occupied; continuing in FFI-only mode (in-process dispatch unaffected)");
    }

    // 在后台线程启动 tokio runtime + run_server(进程内,不阻塞 Dart)。
    let debug_log_path = effective_data_dir.join("ffi-debug.log");
    std::thread::spawn(move || {
        use std::io::Write;
        let mut log = std::fs::OpenOptions::new()
            .create(true).write(true).truncate(true)
            .open(&debug_log_path).ok();
        if let Some(ref mut f) = log {
            let _ = writeln!(f, "[ffi] run_server starting, cwd={:?}", std::env::current_dir());
        }
        rt().block_on(async {
            if let Err(e) = crate::runtime::run_server().await {
                eprintln!("[weft-ffi] run_server error: {e}");
                if let Some(ref mut f) = log {
                    let _ = writeln!(f, "[ffi] run_server ERROR: {e}");
                }
            } else {
                if let Some(ref mut f) = log {
                    let _ = writeln!(f, "[ffi] run_server exited normally");
                }
            }
        });
    });

    // 就绪检查走【进程内】信号(ROUTER 注册完成即可用),不依赖 HTTP health。
    // ROUTER 在 build_router 后、HTTP bind 前注册,所以即使端口被占、HTTP 起不来,
    // FFI dispatch 依然就绪。最多等 90s(debug 模式 WASM 加载较慢)。
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(90);
    while std::time::Instant::now() < deadline {
        if crate::api::rpc::router_ready() {
            return String::new(); // 成功:FFI dispatch 就绪
        }
        std::thread::sleep(std::time::Duration::from_millis(200));
    }
    "core start timed out: router not ready after 90s".to_string()
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
    let token_path = resolve_token_path()
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
/// 优先使用进程内 dispatch 触发 shutdown(不依赖 HTTP listener 是否存活),
/// 回退到 HTTP 请求(兼容独立进程部署场景)。
pub fn stop_core() {
    let token_path = resolve_token_path()
        .unwrap_or_else(|| PathBuf::from("./data/runtime-token"));
    let token = std::fs::read_to_string(&token_path)
        .unwrap_or_default()
        .trim()
        .to_string();

    // 优先:进程内 dispatch(FFI-only 模式下 HTTP 不可用,但 router 已注册)。
    if crate::api::rpc::router_ready() {
        let envelope = crate::api::rpc::RequestEnvelope {
            id: "ffi-shutdown".to_string(),
            method: "POST".to_string(),
            path: "/api/shutdown".to_string(),
            headers: std::collections::HashMap::new(),
            body: serde_json::Value::Null,
        };
        rt().block_on(async {
            crate::api::rpc::dispatch_internal(envelope, &token).await;
        });
        return;
    }

    // 回退:HTTP 请求(独立部署或 router 尚未就绪的极端情况)。
    let _ = reqwest::blocking::Client::new()
        .post("http://127.0.0.1:17830/api/shutdown")
        .bearer_auth(&token)
        .send();
}

/// 检查 core 是否在跑(优先进程内检查,回退 HTTP)。
pub fn is_core_running() -> bool {
    if crate::api::rpc::router_ready() {
        return true;
    }
    reqwest::blocking::get("http://127.0.0.1:17830/api/health")
        .map(|r| r.status().is_success())
        .unwrap_or(false)
}
