use super::{PlatformServiceManager, ServiceInstallOptions, ServiceStatus};
use std::path::PathBuf;
use std::process::Command;

// System-level daemon (requires sudo). Use LaunchDaemons for auto-start on boot.
const LAUNCH_DAEMONS_DIR: &str = "/Library/LaunchDaemons";

pub struct MacosServiceManager;

impl PlatformServiceManager for MacosServiceManager {
    fn install(&self, opts: &ServiceInstallOptions) -> anyhow::Result<()> {
        let label = plist_label(&opts.service_name);
        let plist_path = plist_path(&opts.service_name);
        let binary = opts.binary_path.to_string_lossy();
        let config_dir = opts.config_dir.to_string_lossy();
        let data_dir = opts.data_dir.to_string_lossy();

        let plist = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>{label}</string>
    <key>ProgramArguments</key>
    <array>
        <string>{binary}</string>
        <string>--config-dir</string>
        <string>{config_dir}</string>
        <string>--data-dir</string>
        <string>{data_dir}</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
    <key>StandardOutPath</key>
    <string>/var/log/weft-core.log</string>
    <key>StandardErrorPath</key>
    <string>/var/log/weft-core.err.log</string>
</dict>
</plist>
"#
        );

        std::fs::create_dir_all(LAUNCH_DAEMONS_DIR)
            .map_err(|e| anyhow::anyhow!("cannot create LaunchDaemons dir: {}", e))?;
        std::fs::write(&plist_path, plist)
            .map_err(|e| anyhow::anyhow!("cannot write plist {:?}: {}", plist_path, e))?;
        println!("Wrote plist: {}", plist_path.display());

        // Set correct ownership (root:wheel) and permissions
        let _ = Command::new("chown")
            .args(["root:wheel", &plist_path.to_string_lossy()])
            .output();
        let _ = Command::new("chmod")
            .args(["644", &plist_path.to_string_lossy()])
            .output();

        run_cmd("launchctl", &["load", "-w", &plist_path.to_string_lossy()])?;
        println!("Service '{}' installed and started.", opts.service_name);
        Ok(())
    }

    fn uninstall(&self, service_name: &str) -> anyhow::Result<()> {
        let plist_path = plist_path(service_name);
        if plist_path.exists() {
            let _ = run_cmd("launchctl", &["unload", "-w", &plist_path.to_string_lossy()]);
            std::fs::remove_file(&plist_path)
                .map_err(|e| anyhow::anyhow!("cannot remove plist: {}", e))?;
        }
        println!("Service '{}' uninstalled.", service_name);
        Ok(())
    }

    fn start(&self, service_name: &str) -> anyhow::Result<()> {
        let label = plist_label(service_name);
        run_cmd("launchctl", &["start", &label])?;
        println!("Service '{}' started.", service_name);
        Ok(())
    }

    fn stop(&self, service_name: &str) -> anyhow::Result<()> {
        let label = plist_label(service_name);
        run_cmd("launchctl", &["stop", &label])?;
        println!("Service '{}' stopped.", service_name);
        Ok(())
    }

    fn status(&self, service_name: &str) -> anyhow::Result<ServiceStatus> {
        let plist_path = plist_path(service_name);
        let installed = plist_path.exists();
        if !installed {
            return Ok(ServiceStatus {
                installed: false,
                running: false,
                pid: None,
                binary_path: None,
            });
        }

        let label = plist_label(service_name);
        let out = Command::new("launchctl")
            .args(["list", &label])
            .output()
            .map_err(|e| anyhow::anyhow!("launchctl list failed: {}", e))?;

        let stdout = String::from_utf8_lossy(&out.stdout);
        // launchctl list output: PID  Status  Label
        // If PID is "-" the service is not running
        let running = out.status.success()
            && stdout
                .lines()
                .nth(1)
                .and_then(|l| l.split_whitespace().next())
                .map(|pid| pid != "-")
                .unwrap_or(false);

        let pid = if running {
            stdout
                .lines()
                .nth(1)
                .and_then(|l| l.split_whitespace().next())
                .and_then(|s| s.parse::<u32>().ok())
        } else {
            None
        };

        // Parse binary path from plist
        let binary_path = std::fs::read_to_string(&plist_path).ok().and_then(|content| {
            // Find first <string> after <key>ProgramArguments</key>
            let after = content.split("<key>ProgramArguments</key>").nth(1)?;
            let start = after.find("<string>")? + "<string>".len();
            let end = after[start..].find("</string>")?;
            Some(PathBuf::from(&after[start..start + end]))
        });

        Ok(ServiceStatus {
            installed: true,
            running,
            pid,
            binary_path,
        })
    }
}

fn plist_label(service_name: &str) -> String {
    format!("org.weft.{}", service_name)
}

fn plist_path(service_name: &str) -> PathBuf {
    PathBuf::from(LAUNCH_DAEMONS_DIR).join(format!("org.weft.{}.plist", service_name))
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
