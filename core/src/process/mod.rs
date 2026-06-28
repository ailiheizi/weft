pub mod health;

use crate::config::ServiceConfig;
use anyhow::{bail, Context, Result};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};

const AUTO_START_READY_TIMEOUT: Duration = Duration::from_secs(30);
const AUTO_START_READY_RECOVERY_TIMEOUT: Duration = Duration::from_secs(300);

// ---------- Windows Job Object: kill all child processes when core exits ----------
#[cfg(windows)]
mod job {
    use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
    use windows_sys::Win32::System::JobObjects::{
        AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation,
        SetInformationJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
        JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
    };
    use windows_sys::Win32::System::Threading::OpenProcess;
    use windows_sys::Win32::System::Threading::PROCESS_ALL_ACCESS;

    /// A Win32 Job Object that kills all assigned processes when dropped.
    pub struct JobObject {
        handle: HANDLE,
    }

    unsafe impl Send for JobObject {}
    unsafe impl Sync for JobObject {}

    impl JobObject {
        pub fn new() -> Option<Self> {
            unsafe {
                let handle = CreateJobObjectW(std::ptr::null(), std::ptr::null());
                if handle.is_null() {
                    return None;
                }
                // Configure: kill all processes in the job when the handle is closed.
                let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
                info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
                let ok = SetInformationJobObject(
                    handle,
                    JobObjectExtendedLimitInformation,
                    &info as *const _ as *const _,
                    std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
                );
                if ok == 0 {
                    CloseHandle(handle);
                    return None;
                }
                Some(Self { handle })
            }
        }

        /// Assign a child process (by pid) to this job.
        pub fn assign_process(&self, pid: u32) -> bool {
            unsafe {
                let proc_handle = OpenProcess(PROCESS_ALL_ACCESS, 0, pid);
                if proc_handle.is_null() {
                    return false;
                }
                let ok = AssignProcessToJobObject(self.handle, proc_handle);
                CloseHandle(proc_handle);
                ok != 0
            }
        }
    }

    impl Drop for JobObject {
        fn drop(&mut self) {
            unsafe {
                CloseHandle(self.handle);
            }
        }
    }
}

#[cfg(not(windows))]
mod job {
    /// No-op on non-Windows platforms (Unix uses process groups / PR_SET_PDEATHSIG).
    pub struct JobObject;
    impl JobObject {
        pub fn new() -> Option<Self> { Some(Self) }
        pub fn assign_process(&self, _pid: u32) -> bool { true }
    }
}
// ---------- End Job Object ----------

#[derive(Debug, Clone, PartialEq)]
pub enum ServiceStatus {
    Stopped,
    Starting,
    Running,
    Unhealthy,
    Crashed,
}

impl std::fmt::Display for ServiceStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Stopped => write!(f, "stopped"),
            Self::Starting => write!(f, "starting"),
            Self::Running => write!(f, "running"),
            Self::Unhealthy => write!(f, "unhealthy"),
            Self::Crashed => write!(f, "crashed"),
        }
    }
}

impl serde::Serialize for ServiceStatus {
    fn serialize<S: serde::Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}

struct ManagedService {
    config: ServiceConfig,
    child: Option<Child>,
    status: ServiceStatus,
    stdin: Option<std::process::ChildStdin>,
    stdout_buffer: Arc<Mutex<Vec<u8>>>,
    stdout_base_offset: Arc<Mutex<usize>>,
}

pub struct ProcessManager {
    services: RwLock<HashMap<String, ManagedService>>,
    /// Job Object: all child processes are assigned here; when core exits
    /// (or this is dropped), the OS kills them automatically.
    _job: Option<job::JobObject>,
}

impl Default for ProcessManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ProcessManager {
    pub fn new() -> Self {
        let _job = job::JobObject::new();
        if _job.is_none() {
            tracing::warn!("Failed to create Job Object — child processes may outlive core");
        }
        Self {
            services: RwLock::new(HashMap::new()),
            _job,
        }
    }

    /// Register a service from config (does not start it).
    pub async fn register(&self, config: ServiceConfig) {
        self.register_sync(config);
    }

    /// Sync version of register, callable from non-async contexts.
    pub fn register_sync(&self, config: ServiceConfig) {
        let name = config.name.clone();
        let mut services = self.services.write().unwrap();
        services.insert(
            name,
            ManagedService {
                config,
                child: None,
                status: ServiceStatus::Stopped,
                stdin: None,
                stdout_buffer: Arc::new(Mutex::new(Vec::new())),
                stdout_base_offset: Arc::new(Mutex::new(0)),
            },
        );
    }

    /// Start a registered service.
    pub async fn start(&self, name: &str) -> Result<()> {
        self.start_sync(name)
    }

    /// Sync version of start, callable from non-async contexts (e.g. JS ops).
    pub fn start_sync(&self, name: &str) -> Result<()> {
        let mut services = self.services.write().unwrap();
        let svc = services
            .get_mut(name)
            .ok_or_else(|| anyhow::anyhow!("Service '{}' not registered", name))?;

        if svc.child.is_some() {
            bail!("Service '{}' is already running", name);
        }

        svc.status = ServiceStatus::Starting;

        let mut cmd = Command::new(&svc.config.command);
        cmd.args(&svc.config.args);

        if let Some(ref workdir) = svc.config.workdir {
            cmd.current_dir(workdir);
        }

        for (k, v) in &svc.config.env {
            cmd.env(k, v);
        }

        cmd.stdin(Stdio::piped());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        // Windows: 隐藏子进程 console 窗口(FFI 嵌入模式下 parent 是 GUI app,
        // 不加这个每个 service 都弹黑框)。
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x08000000;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }

        tracing::info!(
            "Starting service '{}' with command='{}' args={:?} cwd={:?}",
            name,
            svc.config.command,
            svc.config.args,
            svc.config.workdir
        );

        let child = cmd
            .spawn()
            .with_context(|| format!("Failed to start service '{}'", name))?;

        // Assign to Job Object so the child dies when core exits.
        if let Some(ref job) = self._job {
            let pid = child.id();
            if !job.assign_process(pid) {
                tracing::warn!("Failed to assign service '{}' (pid {}) to Job Object", name, pid);
            }
        }

        tracing::info!("Started service '{}' (pid: {:?})", name, child.id());
        let mut child = child;
        svc.stdin = child.stdin.take();
        if let Ok(mut buffer) = svc.stdout_buffer.lock() {
            buffer.clear();
        }
        if let Ok(mut base_offset) = svc.stdout_base_offset.lock() {
            *base_offset = 0;
        }

        if let Some(stdout) = child.stdout.take() {
            let service_name = name.to_string();
            let stdout_buffer = svc.stdout_buffer.clone();
            let stdout_base_offset = svc.stdout_base_offset.clone();
            std::thread::spawn(move || {
                let mut reader = BufReader::new(stdout);
                let mut chunk = [0u8; 4096];
                loop {
                    let read = match reader.read(&mut chunk) {
                        Ok(0) => break,
                        Ok(size) => size,
                        Err(_) => break,
                    };
                    if let Ok(mut buffer) = stdout_buffer.lock() {
                        buffer.extend_from_slice(&chunk[..read]);
                        if buffer.len() > 1024 * 1024 {
                            let overflow = buffer.len() - (1024 * 1024);
                            buffer.drain(0..overflow);
                            if let Ok(mut base_offset) = stdout_base_offset.lock() {
                                *base_offset += overflow;
                            }
                        }
                    }
                    let text = String::from_utf8_lossy(&chunk[..read]);
                    for line in text.lines() {
                        tracing::info!("[service:{} stdout] {}", service_name, line);
                    }
                }
            });
        }

        if let Some(stderr) = child.stderr.take() {
            let service_name = name.to_string();
            std::thread::spawn(move || {
                let reader = BufReader::new(stderr);
                for line in reader.lines().map_while(Result::ok) {
                    tracing::warn!("[service:{} stderr] {}", service_name, line);
                }
            });
        }

        svc.child = Some(child);
        svc.status = ServiceStatus::Running;

        Ok(())
    }

    /// Stop a running service.
    pub async fn stop(&self, name: &str) -> Result<()> {
        self.stop_sync(name)
    }

    /// Sync version of stop.
    pub fn stop_sync(&self, name: &str) -> Result<()> {
        let mut services = self.services.write().unwrap();
        let svc = services
            .get_mut(name)
            .ok_or_else(|| anyhow::anyhow!("Service '{}' not registered", name))?;

        if let Some(ref mut child) = svc.child {
            tracing::info!("Stopping service '{}'", name);
            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt;
                const CREATE_NO_WINDOW: u32 = 0x08000000;
                let pid = child.id();
                let _ = Command::new("taskkill")
                    .args(["/PID", &pid.to_string(), "/T", "/F"])
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .creation_flags(CREATE_NO_WINDOW)
                    .status();
            }
            #[cfg(not(windows))]
            {
                child.kill().ok();
            }
            child.wait().ok();
            svc.child = None;
            svc.stdin = None;
            svc.status = ServiceStatus::Stopped;
        }

        Ok(())
    }

    /// Restart a service.
    pub async fn restart(&self, name: &str) -> Result<()> {
        self.stop_sync(name).ok();
        self.start_sync(name)
    }

    /// Get status of all services.
    pub async fn all_statuses(&self) -> HashMap<String, ServiceStatus> {
        self.all_statuses_sync()
    }

    pub fn all_statuses_sync(&self) -> HashMap<String, ServiceStatus> {
        let services = self.services.read().unwrap();
        services
            .iter()
            .map(|(name, svc)| (name.clone(), svc.status.clone()))
            .collect()
    }

    /// Get status of one service.
    pub async fn status(&self, name: &str) -> Option<ServiceStatus> {
        self.status_sync(name)
    }

    pub fn status_sync(&self, name: &str) -> Option<ServiceStatus> {
        let services = self.services.read().unwrap();
        services.get(name).map(|s| s.status.clone())
    }

    pub fn service_config_sync(&self, name: &str) -> Option<ServiceConfig> {
        let services = self.services.read().unwrap();
        services.get(name).map(|svc| svc.config.clone())
    }

    pub async fn service_config(&self, name: &str) -> Option<ServiceConfig> {
        self.service_config_sync(name)
    }

    pub fn write_stdin_sync(&self, name: &str, input: &str) -> Result<()> {
        let mut services = self.services.write().unwrap();
        let svc = services
            .get_mut(name)
            .ok_or_else(|| anyhow::anyhow!("Service '{}' not registered", name))?;
        let stdin = svc
            .stdin
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("Service '{}' has no stdin", name))?;
        stdin.write_all(input.as_bytes())?;
        stdin.flush()?;
        Ok(())
    }

    pub fn read_stdout_since_sync(&self, name: &str, offset: usize) -> Result<(usize, Vec<u8>)> {
        let services = self.services.read().unwrap();
        let svc = services
            .get(name)
            .ok_or_else(|| anyhow::anyhow!("Service '{}' not registered", name))?;
        let base_offset = *svc.stdout_base_offset.lock().unwrap();
        let buffer = svc.stdout_buffer.lock().unwrap();
        if offset < base_offset {
            bail!(
                "stdout buffer for service '{}' truncated before offset {} (current base offset {})",
                name,
                offset,
                base_offset
            );
        }
        let start = (offset - base_offset).min(buffer.len());
        Ok((base_offset + buffer.len(), buffer[start..].to_vec()))
    }

    /// Start all services that have auto_start = true.
    pub async fn start_auto(self: &Arc<Self>) {
        let names: Vec<String> = {
            let services = self.services.read().unwrap();
            services
                .iter()
                .filter(|(_, svc)| svc.config.auto_start)
                .map(|(name, _)| name.clone())
                .collect()
        };

        for name in names {
            if let Err(e) = self.start_sync(&name) {
                tracing::error!("Failed to auto-start service '{}': {}", name, e);
                continue;
            }

            if let Err(error) = self
                .wait_for_service_ready(&name, AUTO_START_READY_TIMEOUT)
                .await
            {
                tracing::error!(
                    "Service '{}' did not become ready after auto-start: {}",
                    name,
                    error
                );

                self.spawn_readiness_recovery(name.clone());
            }
        }
    }

    fn spawn_readiness_recovery(self: &Arc<Self>, name: String) {
        let process_manager = Arc::clone(self);
        tokio::spawn(async move {
            if let Err(error) = process_manager
                .recover_service_readiness(name.clone(), AUTO_START_READY_RECOVERY_TIMEOUT)
                .await
            {
                tracing::warn!(
                    "Service '{}' never recovered readiness after startup timeout: {}",
                    name,
                    error
                );
            }
        });
    }

    async fn wait_for_service_ready(&self, name: &str, timeout: Duration) -> Result<()> {
        let health_url = {
            let services = self.services.read().unwrap();
            services
                .get(name)
                .and_then(|svc| svc.config.health_url.clone())
        };

        let Some(health_url) = health_url else {
            return Ok(());
        };

        let deadline = Instant::now() + timeout;
        loop {
            if health::check_health(&health_url, Duration::from_secs(2)).await? {
                let mut services = self.services.write().unwrap();
                if let Some(svc) = services.get_mut(name) {
                    svc.status = ServiceStatus::Running;
                }
                return Ok(());
            }

            if Instant::now() >= deadline {
                let mut services = self.services.write().unwrap();
                if let Some(svc) = services.get_mut(name) {
                    svc.status = ServiceStatus::Unhealthy;
                }
                bail!("healthcheck never passed for {}", health_url);
            }

            tokio::time::sleep(Duration::from_millis(200)).await;
        }
    }

    async fn recover_service_readiness(&self, name: String, timeout: Duration) -> Result<()> {
        let deadline = Instant::now() + timeout;

        loop {
            match self.status_sync(&name) {
                Some(ServiceStatus::Running) => return Ok(()),
                Some(ServiceStatus::Stopped | ServiceStatus::Crashed) | None => {
                    bail!("service stopped before readiness recovered")
                }
                Some(ServiceStatus::Starting | ServiceStatus::Unhealthy) => {}
            }

            match self
                .wait_for_service_ready(&name, Duration::from_secs(5))
                .await
            {
                Ok(()) => return Ok(()),
                Err(_) if Instant::now() < deadline => {
                    tokio::time::sleep(Duration::from_millis(500)).await;
                }
                Err(error) => return Err(error),
            }
        }
    }

    /// Stop all running services.
    pub async fn stop_all(&self) {
        let names: Vec<String> = {
            let services = self.services.read().unwrap();
            services.keys().cloned().collect()
        };

        for name in names {
            self.stop_sync(&name).ok();
        }
    }

    /// Run health checks for all services with health_url configured.
    pub async fn run_health_checks(&self) {
        let configs: Vec<(String, Option<String>)> = {
            let services = self.services.read().unwrap();
            services
                .iter()
                .filter(|(_, svc)| svc.child.is_some())
                .map(|(name, svc)| (name.clone(), svc.config.health_url.clone()))
                .collect()
        };

        for (name, health_url) in configs {
            if let Some(url) = health_url {
                let healthy = health::check_health(&url, std::time::Duration::from_secs(5))
                    .await
                    .unwrap_or(false);

                let mut services = self.services.write().unwrap();
                if let Some(svc) = services.get_mut(&name) {
                    svc.status = if healthy {
                        ServiceStatus::Running
                    } else {
                        ServiceStatus::Unhealthy
                    };
                }
            }
        }
    }
}
