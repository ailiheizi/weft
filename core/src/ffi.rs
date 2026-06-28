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

/// 探测 17830 是否已被占用(残留 core 进程或其他程序)。
fn port_in_use() -> bool {
    std::net::TcpStream::connect_timeout(
        &"127.0.0.1:17830".parse().unwrap(),
        std::time::Duration::from_millis(300),
    )
    .is_ok()
}

/// 启动前清理残留 core:若 17830 被占,尝试优雅关闭占用者(残留 core 暴露 /api/shutdown),
/// 然后等端口释放。最多等 ~5s。返回 true=端口已空闲可用。
fn ensure_port_free() -> bool {
    if !port_in_use() {
        return true;
    }
    // 端口被占:尝试用 runtime-token 优雅关闭残留 core。
    let token = PROJECT_ROOT
        .get()
        .map(|r| r.join("data").join("runtime-token"))
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

    // 启动前清残留:若 17830 被上次没退干净的 core 占着,先优雅关闭它,
    // 让本进程的 HTTP listener 也能起来(非必须 — FFI dispatch 不依赖 HTTP,
    // 但清掉残留可避免端口告警和僵尸进程堆积)。
    if !ensure_port_free() {
        eprintln!("[weft-ffi] port 17830 still occupied; continuing in FFI-only mode (in-process dispatch unaffected)");
    }

    // 在后台线程启动 tokio runtime + run_server(进程内,不阻塞 Dart)。
    std::thread::spawn(move || {
        rt().block_on(async {
            if let Err(e) = crate::runtime::run_server().await {
                eprintln!("[weft-ffi] run_server error: {e}");
            }
        });
    });

    // 就绪检查走【进程内】信号(ROUTER 注册完成即可用),不依赖 HTTP health。
    // 这彻底消除了"HTTP listener bind 慢/失败/被占 → 误判启动超时 → 降级 HTTP →
    // 连接被拒"这一整类时序脆弱性。ROUTER 在 build_router 后、HTTP bind 前注册,
    // 所以即使端口被占、HTTP 起不来,FFI dispatch 依然就绪。
    // 最多等 90s(debug 模式 WASM 加载较慢)。
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
/// 优先使用进程内 dispatch 触发 shutdown(不依赖 HTTP listener 是否存活),
/// 回退到 HTTP 请求(兼容独立进程部署场景)。
pub fn stop_core() {
    let token_path = PROJECT_ROOT
        .get()
        .map(|r| r.join("data").join("runtime-token"))
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

/// 检查 core 是否在跑。
pub fn is_core_running() -> bool {
    reqwest::blocking::get("http://127.0.0.1:17830/api/health")
        .map(|r| r.status().is_success())
        .unwrap_or(false)
}
