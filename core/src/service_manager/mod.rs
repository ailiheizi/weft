use std::path::{Path, PathBuf};

pub use platform::platform_service_manager;

#[cfg(target_os = "windows")]
mod platform {
    pub use super::windows::WindowsServiceManager as PlatformImpl;
    pub fn platform_service_manager() -> PlatformImpl {
        PlatformImpl
    }
}

#[cfg(target_os = "linux")]
mod platform {
    pub use super::linux::LinuxServiceManager as PlatformImpl;
    pub fn platform_service_manager() -> PlatformImpl {
        PlatformImpl
    }
}

#[cfg(target_os = "macos")]
mod platform {
    pub use super::macos::MacosServiceManager as PlatformImpl;
    pub fn platform_service_manager() -> PlatformImpl {
        PlatformImpl
    }
}

#[cfg(windows)]
pub mod windows;
#[cfg(target_os = "linux")]
pub mod linux;
#[cfg(target_os = "macos")]
pub mod macos;

/// Options resolved before calling install().
#[derive(Debug, Clone)]
pub struct ServiceInstallOptions {
    /// Path to the weft-core binary that the service will execute.
    /// Inplace mode: current executable path.
    /// --path mode: destination after copy.
    pub binary_path: PathBuf,
    /// Directory containing config.toml.
    pub config_dir: PathBuf,
    /// Directory for runtime data (KV store, etc.).
    pub data_dir: PathBuf,
    /// OS service name (default: "weft-core").
    pub service_name: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ServiceStatus {
    pub installed: bool,
    pub running: bool,
    pub pid: Option<u32>,
    pub binary_path: Option<PathBuf>,
}

pub trait PlatformServiceManager {
    fn install(&self, opts: &ServiceInstallOptions) -> anyhow::Result<()>;
    fn uninstall(&self, service_name: &str) -> anyhow::Result<()>;
    fn start(&self, service_name: &str) -> anyhow::Result<()>;
    fn stop(&self, service_name: &str) -> anyhow::Result<()>;
    fn status(&self, service_name: &str) -> anyhow::Result<ServiceStatus>;
}

/// Parse CLI args into ServiceInstallOptions.
/// Handles: --path, --config, --data, --service-name, --mode=system
pub fn resolve_install_options(args: &[String]) -> anyhow::Result<ServiceInstallOptions> {
    let current_exe = std::env::current_exe()
        .map_err(|e| anyhow::anyhow!("cannot determine current executable: {}", e))?;

    let mut dest_dir: Option<PathBuf> = None;
    let mut config_dir: Option<PathBuf> = None;
    let mut data_dir: Option<PathBuf> = None;
    let mut service_name = "weft-core".to_string();
    let mut mode_system = false;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--path" => {
                dest_dir = Some(PathBuf::from(next_arg(args, i, "--path")?));
                i += 2;
            }
            "--config" => {
                config_dir = Some(PathBuf::from(next_arg(args, i, "--config")?));
                i += 2;
            }
            "--data" => {
                data_dir = Some(PathBuf::from(next_arg(args, i, "--data")?));
                i += 2;
            }
            "--service-name" => {
                service_name = next_arg(args, i, "--service-name")?.to_string();
                i += 2;
            }
            "--mode=system" | "--mode" if args.get(i + 1).map(|s| s.as_str()) == Some("system") => {
                mode_system = true;
                i += if args[i] == "--mode" { 2 } else { 1 };
            }
            other => {
                anyhow::bail!("unknown flag: {}", other);
            }
        }
    }

    if mode_system && dest_dir.is_none() {
        dest_dir = Some(system_binary_dir());
    }
    if mode_system && config_dir.is_none() {
        config_dir = Some(system_config_dir());
    }
    if mode_system && data_dir.is_none() {
        data_dir = Some(system_data_dir());
    }

    // Resolve binary path: copy if --path given, otherwise use current exe in-place.
    let binary_path = if let Some(dir) = dest_dir {
        let bin_name = current_exe
            .file_name()
            .ok_or_else(|| anyhow::anyhow!("cannot determine binary filename"))?;
        let dest = dir.join(bin_name);
        std::fs::create_dir_all(&dir)
            .map_err(|e| anyhow::anyhow!("cannot create destination dir {:?}: {}", dir, e))?;
        std::fs::copy(&current_exe, &dest)
            .map_err(|e| anyhow::anyhow!("cannot copy binary to {:?}: {}", dest, e))?;
        println!("Copied binary to {}", dest.display());
        dest
    } else {
        current_exe.clone()
    };

    // Default config/data dirs: sibling of binary.
    let binary_dir = binary_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf();

    let config_dir = config_dir.unwrap_or_else(|| binary_dir.join("config"));
    let data_dir = data_dir.unwrap_or_else(|| binary_dir.join("data"));

    Ok(ServiceInstallOptions {
        binary_path,
        config_dir,
        data_dir,
        service_name,
    })
}

fn next_arg<'a>(args: &'a [String], i: usize, flag: &str) -> anyhow::Result<&'a str> {
    args.get(i + 1)
        .map(|s| s.as_str())
        .ok_or_else(|| anyhow::anyhow!("{} requires a value", flag))
}

#[cfg(target_os = "windows")]
fn system_binary_dir() -> PathBuf {
    PathBuf::from(std::env::var("ProgramData").unwrap_or_else(|_| "C:\\ProgramData".into()))
        .join("WEFT")
}
#[cfg(target_os = "windows")]
fn system_config_dir() -> PathBuf {
    system_binary_dir().join("config")
}
#[cfg(target_os = "windows")]
fn system_data_dir() -> PathBuf {
    system_binary_dir().join("data")
}

#[cfg(target_os = "linux")]
fn system_binary_dir() -> PathBuf {
    PathBuf::from("/usr/local/bin")
}
#[cfg(target_os = "linux")]
fn system_config_dir() -> PathBuf {
    PathBuf::from("/etc/weft")
}
#[cfg(target_os = "linux")]
fn system_data_dir() -> PathBuf {
    PathBuf::from("/var/lib/weft")
}

#[cfg(target_os = "macos")]
fn system_binary_dir() -> PathBuf {
    PathBuf::from("/usr/local/bin")
}
#[cfg(target_os = "macos")]
fn system_config_dir() -> PathBuf {
    PathBuf::from("/Library/Application Support/WEFT")
}
#[cfg(target_os = "macos")]
fn system_data_dir() -> PathBuf {
    PathBuf::from("/Library/Application Support/WEFT/data")
}
