use super::{PlatformServiceManager, ServiceInstallOptions, ServiceStatus};
use std::process::Command;

pub struct WindowsServiceManager;

impl PlatformServiceManager for WindowsServiceManager {
    fn install(&self, opts: &ServiceInstallOptions) -> anyhow::Result<()> {
        let bin = opts.binary_path.to_string_lossy();
        let config_dir = opts.config_dir.to_string_lossy();
        let data_dir = opts.data_dir.to_string_lossy();
        // sc.exe requires the full binPath including arguments in one quoted string.
        let bin_path = format!(
            "\"{}\" --config-dir \"{}\" --data-dir \"{}\"",
            bin, config_dir, data_dir
        );
        let out = Command::new("sc")
            .args([
                "create",
                &opts.service_name,
                &format!("binPath={}", bin_path),
                "start=auto",
                "DisplayName=WEFT Core",
            ])
            .output()
            .map_err(|e| anyhow::anyhow!("sc create failed: {}", e))?;
        check_sc_output(&out, "install")?;
        // Set description
        let _ = Command::new("sc")
            .args([
                "description",
                &opts.service_name,
                "WEFT Core AI agent runtime service",
            ])
            .output();
        // Start immediately after install
        self.start(&opts.service_name)?;
        println!("Service '{}' installed and started.", opts.service_name);
        Ok(())
    }

    fn uninstall(&self, service_name: &str) -> anyhow::Result<()> {
        // Stop first (ignore error if already stopped)
        let _ = self.stop(service_name);
        let out = Command::new("sc")
            .args(["delete", service_name])
            .output()
            .map_err(|e| anyhow::anyhow!("sc delete failed: {}", e))?;
        check_sc_output(&out, "uninstall")?;
        println!("Service '{}' uninstalled.", service_name);
        Ok(())
    }

    fn start(&self, service_name: &str) -> anyhow::Result<()> {
        let out = Command::new("sc")
            .args(["start", service_name])
            .output()
            .map_err(|e| anyhow::anyhow!("sc start failed: {}", e))?;
        // Exit code 1056 = already running, treat as success
        let stdout = String::from_utf8_lossy(&out.stdout);
        if out.status.success() || stdout.contains("1056") || stdout.contains("already") {
            println!("Service '{}' started.", service_name);
            return Ok(());
        }
        check_sc_output(&out, "start")
    }

    fn stop(&self, service_name: &str) -> anyhow::Result<()> {
        let out = Command::new("sc")
            .args(["stop", service_name])
            .output()
            .map_err(|e| anyhow::anyhow!("sc stop failed: {}", e))?;
        // Exit code 1062 = not started, treat as success
        let stdout = String::from_utf8_lossy(&out.stdout);
        if out.status.success() || stdout.contains("1062") || stdout.contains("not started") {
            println!("Service '{}' stopped.", service_name);
            return Ok(());
        }
        check_sc_output(&out, "stop")
    }

    fn status(&self, service_name: &str) -> anyhow::Result<ServiceStatus> {
        let out = Command::new("sc")
            .args(["query", service_name])
            .output()
            .map_err(|e| anyhow::anyhow!("sc query failed: {}", e))?;

        if !out.status.success() {
            // Service not installed
            return Ok(ServiceStatus {
                installed: false,
                running: false,
                pid: None,
                binary_path: None,
            });
        }

        let stdout = String::from_utf8_lossy(&out.stdout);
        let running = stdout.contains("RUNNING");

        // Get binary path from sc qc
        let qc = Command::new("sc")
            .args(["qc", service_name])
            .output()
            .ok();
        let binary_path = qc.and_then(|o| {
            let text = String::from_utf8_lossy(&o.stdout).to_string();
            // BINARY_PATH_NAME : "C:\path\weft-core.exe" --config-dir ...
            text.lines()
                .find(|l| l.contains("BINARY_PATH_NAME"))
                .and_then(|l| l.split(':').nth(1))
                .map(|s| s.trim().trim_matches('"').to_string())
                .and_then(|s| {
                    // Extract just the exe path (before first space after closing quote)
                    let s = s.trim_start_matches('"');
                    let end = s.find("\" ").map(|i| i + 1).unwrap_or(s.len());
                    Some(std::path::PathBuf::from(&s[..end].trim_matches('"')))
                })
        });

        Ok(ServiceStatus {
            installed: true,
            running,
            pid: None,
            binary_path,
        })
    }
}

fn check_sc_output(out: &std::process::Output, op: &str) -> anyhow::Result<()> {
    if out.status.success() {
        return Ok(());
    }
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    anyhow::bail!(
        "sc {} failed (exit {:?}):\n{}\n{}",
        op,
        out.status.code(),
        stdout.trim(),
        stderr.trim()
    )
}
