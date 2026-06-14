use super::{PlatformServiceManager, ServiceInstallOptions, ServiceStatus};
use std::path::PathBuf;
use std::process::Command;

const SYSTEMD_DIR: &str = "/etc/systemd/system";

pub struct LinuxServiceManager;

impl PlatformServiceManager for LinuxServiceManager {
    fn install(&self, opts: &ServiceInstallOptions) -> anyhow::Result<()> {
        let unit_path = unit_path(&opts.service_name);
        let binary = opts.binary_path.to_string_lossy();
        let config_dir = opts.config_dir.to_string_lossy();
        let data_dir = opts.data_dir.to_string_lossy();
        let working_dir = opts
            .binary_path
            .parent()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| "/".to_string());

        let unit = format!(
            "[Unit]\n\
             Description=WEFT Core AI agent runtime service\n\
             After=network.target\n\
             \n\
             [Service]\n\
             Type=simple\n\
             ExecStart={binary} --config-dir {config_dir} --data-dir {data_dir}\n\
             WorkingDirectory={working_dir}\n\
             Restart=on-failure\n\
             RestartSec=5\n\
             StandardOutput=journal\n\
             StandardError=journal\n\
             \n\
             [Install]\n\
             WantedBy=multi-user.target\n"
        );

        std::fs::create_dir_all(SYSTEMD_DIR)
            .map_err(|e| anyhow::anyhow!("cannot create systemd dir: {}", e))?;
        std::fs::write(&unit_path, unit)
            .map_err(|e| anyhow::anyhow!("cannot write unit file {:?}: {}", unit_path, e))?;
        println!("Wrote unit file: {}", unit_path.display());

        run_cmd("systemctl", &["daemon-reload"])?;
        run_cmd("systemctl", &["enable", &opts.service_name])?;
        self.start(&opts.service_name)?;
        println!("Service '{}' installed and started.", opts.service_name);
        Ok(())
    }

    fn uninstall(&self, service_name: &str) -> anyhow::Result<()> {
        let _ = self.stop(service_name);
        let _ = run_cmd("systemctl", &["disable", service_name]);
        let unit = unit_path(service_name);
        if unit.exists() {
            std::fs::remove_file(&unit)
                .map_err(|e| anyhow::anyhow!("cannot remove unit file: {}", e))?;
        }
        let _ = run_cmd("systemctl", &["daemon-reload"]);
        println!("Service '{}' uninstalled.", service_name);
        Ok(())
    }

    fn start(&self, service_name: &str) -> anyhow::Result<()> {
        run_cmd("systemctl", &["start", service_name])?;
        println!("Service '{}' started.", service_name);
        Ok(())
    }

    fn stop(&self, service_name: &str) -> anyhow::Result<()> {
        run_cmd("systemctl", &["stop", service_name])?;
        println!("Service '{}' stopped.", service_name);
        Ok(())
    }

    fn status(&self, service_name: &str) -> anyhow::Result<ServiceStatus> {
        let unit = unit_path(service_name);
        let installed = unit.exists();
        if !installed {
            return Ok(ServiceStatus {
                installed: false,
                running: false,
                pid: None,
                binary_path: None,
            });
        }

        let out = Command::new("systemctl")
            .args(["is-active", service_name])
            .output()
            .map_err(|e| anyhow::anyhow!("systemctl is-active failed: {}", e))?;
        let running = String::from_utf8_lossy(&out.stdout).trim() == "active";

        // Parse ExecStart from unit file to get binary path
        let binary_path = std::fs::read_to_string(&unit).ok().and_then(|content| {
            content
                .lines()
                .find(|l| l.starts_with("ExecStart="))
                .and_then(|l| l.strip_prefix("ExecStart="))
                .and_then(|l| l.split_whitespace().next())
                .map(PathBuf::from)
        });

        // Get PID if running
        let pid = if running {
            Command::new("systemctl")
                .args(["show", "-p", "MainPID", "--value", service_name])
                .output()
                .ok()
                .and_then(|o| String::from_utf8_lossy(&o.stdout).trim().parse::<u32>().ok())
                .filter(|&p| p > 0)
        } else {
            None
        };

        Ok(ServiceStatus {
            installed: true,
            running,
            pid,
            binary_path,
        })
    }
}

fn unit_path(service_name: &str) -> PathBuf {
    PathBuf::from(SYSTEMD_DIR).join(format!("{}.service", service_name))
}

fn run_cmd(cmd: &str, args: &[&str]) -> anyhow::Result<()> {
    let out = Command::new(cmd)
        .args(args)
        .output()
        .map_err(|e| anyhow::anyhow!("{} failed: {}", cmd, e))?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        let stdout = String::from_utf8_lossy(&out.stdout);
        anyhow::bail!(
            "{} {} failed:\n{}\n{}",
            cmd,
            args.join(" "),
            stdout.trim(),
            stderr.trim()
        );
    }
    Ok(())
}
