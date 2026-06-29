pub mod bridge;
pub mod circuit_breaker;
pub mod config;
pub mod fallback;
pub mod native;
pub mod permissions;
pub mod validate;

use crate::config::ServiceConfig;
use anyhow::{bail, Context, Result};
use bridge::WasmStartupMode;
use config::{load_manifest, PackageManifest};
pub use native::{
    native_library_candidates, NativeHandle, NativePackageHost, NativePackageLoadInfo,
};
use std::cmp::Ordering;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Metadata for a loaded package (exposed via API).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PackageInfo {
    pub name: String,
    pub version: Option<String>,
    #[serde(default)]
    pub overrides: Vec<String>,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default)]
    pub has_ui: bool,
    #[serde(default)]
    pub description: Option<String>,
}

fn default_enabled() -> bool {
    true
}

fn normalize_existing_path(path: &Path) -> PathBuf {
    if let Ok(canonical) = std::fs::canonicalize(path) {
        return canonical;
    }

    if path.is_absolute() {
        return path.to_path_buf();
    }

    std::env::current_dir()
        .map(|cwd| cwd.join(path))
        .unwrap_or_else(|_| path.to_path_buf())
}

/// Manages package registration and listing.
pub struct PackageManager {
    packages: HashMap<String, PackageInfo>,
}

impl Default for PackageManager {
    fn default() -> Self {
        Self::new()
    }
}

impl PackageManager {
    pub fn new() -> Self {
        Self {
            packages: HashMap::new(),
        }
    }

    /// Register a plugin's metadata (for listing).
    pub fn register(&mut self, info: PackageInfo) {
        self.packages.insert(info.name.clone(), info);
    }

    /// List all registered plugins.
    pub fn list(&self) -> Vec<&PackageInfo> {
        self.packages.values().collect()
    }

    /// Unregister a package by name.
    pub fn unregister(&mut self, name: &str) -> Option<PackageInfo> {
        self.packages.remove(name)
    }

    /// Get a package by name.
    pub fn get(&self, name: &str) -> Option<&PackageInfo> {
        self.packages.get(name)
    }

    /// Get a mutable reference to a package by name.
    pub fn get_mut(&mut self, name: &str) -> Option<&mut PackageInfo> {
        self.packages.get_mut(name)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PackageRuntime {
    Wasm,
    Native,
    Service,
    Remote,
    Unknown(String),
}

impl PackageRuntime {
    pub fn from_manifest(manifest: &PackageManifest) -> Self {
        match manifest.runtime_kind().as_str() {
            "wasm" => Self::Wasm,
            "native" => Self::Native,
            "service" => Self::Service,
            "remote" => Self::Remote,
            other => Self::Unknown(other.to_string()),
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            Self::Wasm => "wasm",
            Self::Native => "native",
            Self::Service => "service",
            Self::Remote => "remote",
            Self::Unknown(value) => value.as_str(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct DiscoveredPackage {
    pub manifest: PackageManifest,
    pub dir: PathBuf,
    pub entry_path: Option<PathBuf>,
    pub runtime: PackageRuntime,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PackageSourcePrecedence {
    Official,
    Installed,
    Other,
}

impl PackageSourcePrecedence {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Installed => "installed",
            Self::Official => "official",
            Self::Other => "other",
        }
    }
}

pub fn packages_dir(repo_root: &Path) -> PathBuf {
    repo_root.join("packages")
}

pub fn is_managed_runtime_root(repo_root: &Path) -> bool {
    if std::env::var("WEFT_CORE_MANAGED")
        .map(|value| value == "1")
        .unwrap_or(false)
    {
        return true;
    }

    let config_path = repo_root.join("config").join("config.toml");
    let installed_dir = repo_root.join("plugins").join("installed");
    let suite_state_dir = repo_root.join("suite-state");

    config_path.is_file() && (installed_dir.is_dir() || suite_state_dir.is_dir())
}

pub fn installed_packages_dir(repo_root: &Path) -> PathBuf {
    if is_managed_runtime_root(repo_root) {
        repo_root.join("plugins").join("installed")
    } else {
        packages_dir(repo_root).join("installed")
    }
}

pub fn official_packages_dir(repo_root: &Path) -> PathBuf {
    if is_managed_runtime_root(repo_root) {
        repo_root.join("plugins").join("official")
    } else {
        packages_dir(repo_root).join("official")
    }
}

pub fn package_source_precedence(repo_root: &Path, package_dir: &Path) -> PackageSourcePrecedence {
    let normalized_dir = normalize_existing_path(package_dir);
    let official_dir = normalize_existing_path(&official_packages_dir(repo_root));
    let installed_dir = normalize_existing_path(&installed_packages_dir(repo_root));

    if normalized_dir.starts_with(&official_dir) {
        PackageSourcePrecedence::Official
    } else if normalized_dir.starts_with(&installed_dir) {
        PackageSourcePrecedence::Installed
    } else {
        PackageSourcePrecedence::Other
    }
}

pub fn runtime_package_aliases(
    package_index: &crate::app::PackageIndex,
) -> HashMap<String, String> {
    package_index
        .package_sources
        .iter()
        .filter_map(|source| {
            let runtime_provider = source.runtime_provider_name();
            let package_name = source.name.trim();
            if runtime_provider.is_empty()
                || package_name.is_empty()
                || runtime_provider == package_name
            {
                None
            } else {
                Some((runtime_provider, package_name.to_string()))
            }
        })
        .collect()
}

pub fn merged_package_aliases(
    package_index: &crate::app::PackageIndex,
    configured_aliases: &HashMap<String, String>,
) -> HashMap<String, String> {
    let mut aliases = runtime_package_aliases(package_index);
    aliases.extend(configured_aliases.clone());
    aliases
}

fn compare_package_precedence(
    repo_root: &Path,
    left: &DiscoveredPackage,
    right: &DiscoveredPackage,
) -> Ordering {
    package_source_precedence(repo_root, &left.dir)
        .cmp(&package_source_precedence(repo_root, &right.dir))
        .then_with(|| left.dir.cmp(&right.dir))
}

fn runtime_package_identity(
    repo_root: &Path,
    package_index: &crate::app::PackageIndex,
    package: &DiscoveredPackage,
) -> String {
    let normalized_dir = normalize_existing_path(&package.dir);
    let manifest_name = package.manifest.package_info.name.trim();

    for source in &package_index.package_sources {
        let package_name = source.name.trim();
        if package_name.is_empty() {
            continue;
        }

        let current_source = source.current_source.trim();
        if !current_source.is_empty()
            && normalized_dir == normalize_existing_path(&repo_root.join(current_source))
        {
            return package_name.to_string();
        }

        if manifest_name == package_name || manifest_name == source.runtime_provider_name() {
            return package_name.to_string();
        }
    }

    manifest_name.to_string()
}

fn is_declared_current_runtime_source(
    repo_root: &Path,
    package_index: &crate::app::PackageIndex,
    package: &DiscoveredPackage,
) -> bool {
    let identity = runtime_package_identity(repo_root, package_index, package);
    let normalized_dir = normalize_existing_path(&package.dir);

    package_index
        .get(&identity)
        .map(|source| {
            normalize_existing_path(&repo_root.join(&source.current_source)) == normalized_dir
        })
        .unwrap_or(false)
}

fn compare_runtime_plugin_package_precedence(
    repo_root: &Path,
    package_index: &crate::app::PackageIndex,
    left: &DiscoveredPackage,
    right: &DiscoveredPackage,
) -> Ordering {
    let left_is_current = is_declared_current_runtime_source(repo_root, package_index, left);
    let right_is_current = is_declared_current_runtime_source(repo_root, package_index, right);

    right_is_current
        .cmp(&left_is_current)
        .then_with(|| compare_package_precedence(repo_root, left, right))
}

pub fn select_runtime_packages(
    repo_root: &Path,
    package_index: &crate::app::PackageIndex,
    mut packages: Vec<DiscoveredPackage>,
) -> Vec<DiscoveredPackage> {
    packages.sort_by(|left, right| {
        runtime_package_identity(repo_root, package_index, left)
            .cmp(&runtime_package_identity(repo_root, package_index, right))
            .then_with(|| {
                compare_runtime_plugin_package_precedence(repo_root, package_index, left, right)
            })
    });

    let mut selected = Vec::new();

    for package in packages {
        let package_identity = runtime_package_identity(repo_root, package_index, &package);
        let duplicate = selected.iter().position(|existing: &DiscoveredPackage| {
            runtime_package_identity(repo_root, package_index, existing) == package_identity
        });

        if let Some(existing_index) = duplicate {
            let existing = &selected[existing_index];
            let comparison = compare_runtime_plugin_package_precedence(
                repo_root,
                package_index,
                &package,
                existing,
            );
            if comparison == Ordering::Less {
                tracing::info!(
                    "Runtime package '{}' selected manifest '{}' from {} at {} (replacing manifest '{}' from {} at {})",
                    package_identity,
                    package.manifest.package_info.name,
                    package_source_precedence(repo_root, &package.dir).as_str(),
                    package.dir.display(),
                    existing.manifest.package_info.name,
                    package_source_precedence(repo_root, &existing.dir).as_str(),
                    existing.dir.display()
                );
                selected[existing_index] = package;
            } else if existing.dir != package.dir {
                tracing::info!(
                    "Runtime package '{}' ignored manifest '{}' from {} at {}; keeping manifest '{}' from {} at {}",
                    package_identity,
                    package.manifest.package_info.name,
                    package_source_precedence(repo_root, &package.dir).as_str(),
                    package.dir.display(),
                    existing.manifest.package_info.name,
                    package_source_precedence(repo_root, &existing.dir).as_str(),
                    existing.dir.display()
                );
            }
            continue;
        }

        selected.push(package);
    }

    selected
}

pub fn discover_runtime_packages(
    repo_root: &Path,
    package_index: &crate::app::PackageIndex,
) -> Vec<DiscoveredPackage> {
    let installed_dir = installed_packages_dir(repo_root);
    let official_dir = official_packages_dir(repo_root);

    let mut discovered_packages = discover_packages(&installed_dir);
    discovered_packages.extend(discover_packages(&official_dir));

    for source in &package_index.package_sources {
        let package_dir = repo_root.join(&source.current_source);
        if let Some(package) = discover_package(&package_dir) {
            discovered_packages.push(package);
        }
    }

    select_runtime_packages(repo_root, package_index, discovered_packages)
}

pub fn discover_runtime_package(
    repo_root: &Path,
    package_index: &crate::app::PackageIndex,
    name: &str,
) -> Option<DiscoveredPackage> {
    let canonical_name = canonical_runtime_package_name(package_index, name);

    discover_runtime_packages(repo_root, package_index)
        .into_iter()
        .find(|package| {
            package.manifest.package_info.name == canonical_name
                || package.manifest.package_info.name == name
        })
}

pub fn canonical_runtime_package_name(
    package_index: &crate::app::PackageIndex,
    requested_name: &str,
) -> String {
    let requested_name = requested_name.trim();
    if requested_name.is_empty() {
        return String::new();
    }

    package_index
        .get(requested_name)
        .map(|source| source.name.trim())
        .filter(|name| !name.is_empty())
        .unwrap_or(requested_name)
        .to_string()
}

pub fn resolve_runtime_package(
    repo_root: &Path,
    package_index: &crate::app::PackageIndex,
    requested_name: &str,
) -> Option<DiscoveredPackage> {
    let canonical_name = canonical_runtime_package_name(package_index, requested_name);

    if let Some(source) = package_index.get(&canonical_name) {
        let package_dir = repo_root.join(&source.current_source);
        if let Some(package) = discover_package(&package_dir) {
            return Some(package);
        }
    }

    discover_runtime_package(repo_root, package_index, &canonical_name)
}

/// A missing dependency descriptor.
#[derive(Debug, Clone, serde::Serialize)]
pub struct MissingDep {
    pub name: String,
    pub required_version: String,
}

/// Check if all dependencies of a manifest are satisfied by installed plugins.
pub fn check_dependencies(
    manifest: &PackageManifest,
    installed: &[PackageInfo],
) -> Vec<MissingDep> {
    let installed_names: std::collections::HashSet<&str> =
        installed.iter().map(|p| p.name.as_str()).collect();

    manifest
        .dependencies
        .iter()
        .filter(|(name, _)| !installed_names.contains(name.as_str()))
        .map(|(name, version)| MissingDep {
            name: name.clone(),
            required_version: version.clone(),
        })
        .collect()
}

/// Check if any other package depends on the given package name.
/// Returns the names of plugins that depend on it.
pub fn check_uninstall_safety(name: &str, plugins_dir: &Path) -> Vec<String> {
    let manifests = discover_manifests(plugins_dir);
    manifests
        .iter()
        .filter(|m| m.package_info.name != name && m.dependencies.contains_key(name))
        .map(|m| m.package_info.name.clone())
        .collect()
}

fn resolve_entry_path(package_dir: &Path, manifest: &PackageManifest) -> Option<PathBuf> {
    let declared_entry = manifest
        .resolved_entry()
        .map(|entry| normalize_existing_path(&package_dir.join(entry)));

    if let Some(path) = declared_entry.as_ref().filter(|path| path.exists()) {
        return Some(path.clone());
    }

    if manifest.runtime_kind() == "wasm" {
        if let Some(path) = locate_built_wasm(package_dir) {
            return Some(path);
        }

        if package_dir.join("Cargo.toml").exists() {
            if let Err(error) =
                build_wasm_package_artifact(package_dir, &manifest.package_info.name)
            {
                tracing::warn!(
                    "Package '{}': failed to build missing wasm artifact: {:#}",
                    manifest.package_info.name,
                    error
                );
            } else if let Some(path) = locate_built_wasm(package_dir) {
                return Some(path);
            }
        }
    }

    declared_entry
}

fn locate_built_wasm(package_dir: &Path) -> Option<PathBuf> {
    ["debug", "release"].into_iter().find_map(|profile| {
        let target_dir = package_dir
            .join("target")
            .join("wasm32-wasip1")
            .join(profile);
        let entries = std::fs::read_dir(&target_dir).ok()?;
        let mut wasm_files = entries
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("wasm"))
            .collect::<Vec<_>>();
        wasm_files.sort();
        wasm_files.into_iter().next()
    })
}

fn build_wasm_package_artifact(package_dir: &Path, package_name: &str) -> Result<()> {
    let output = std::process::Command::new("cargo")
        .arg("build")
        .arg("--target")
        .arg("wasm32-wasip1")
        .current_dir(package_dir)
        .output()
        .with_context(|| format!("failed to spawn cargo build for '{}'", package_name))?;

    if output.status.success() {
        tracing::info!(
            "Package '{}': built missing wasm artifact from {}",
            package_name,
            package_dir.display()
        );
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    bail!(
        "cargo build --target wasm32-wasip1 failed in '{}': {}{}{}",
        package_dir.display(),
        stderr.trim(),
        if stderr.trim().is_empty() || stdout.trim().is_empty() {
            ""
        } else {
            " | stdout: "
        },
        if stderr.trim().is_empty() {
            stdout.trim()
        } else {
            ""
        }
    )
}

pub fn discover_package(package_dir: &Path) -> Option<DiscoveredPackage> {
    let path = normalize_existing_path(package_dir);
    if !path.is_dir() {
        return None;
    }

    let manifest_path = path.join("package.toml");
    if !manifest_path.exists() {
        return None;
    }

    match load_manifest(&path) {
        Ok(manifest) => {
            let runtime = PackageRuntime::from_manifest(&manifest);
            let entry_path = resolve_entry_path(&path, &manifest);

            // B1: surface manifest inconsistencies as explicit findings.
            // load_manifest is lenient (empty name, kind/runtime mismatch still
            // parse); validate_manifest turns that into severity-rated warnings.
            let entry_exists = entry_path.as_ref().map(|p| p.exists());
            for issue in validate::validate_manifest(&manifest, entry_exists) {
                match issue.severity {
                    validate::Severity::Error => tracing::warn!(
                        "manifest validation [{}] at {}: {}",
                        issue.code,
                        path.display(),
                        issue.message
                    ),
                    validate::Severity::Warning => tracing::debug!(
                        "manifest validation [{}] at {}: {}",
                        issue.code,
                        path.display(),
                        issue.message
                    ),
                }
            }

            let requires_local_entry = matches!(
                runtime,
                PackageRuntime::Wasm | PackageRuntime::Service
            );
            if requires_local_entry {
                match entry_path.as_ref() {
                    Some(candidate) if candidate.exists() => {}
                    Some(candidate) => {
                        tracing::warn!(
                            "Package '{}': entry file '{}' not found, skipping",
                            manifest.package_info.name,
                            candidate.display()
                        );
                        return None;
                    }
                    None => {
                        tracing::warn!(
                            "Package '{}': no entry declared for runtime '{}', skipping",
                            manifest.package_info.name,
                            runtime.as_str()
                        );
                        return None;
                    }
                }
            }

            tracing::info!(
                "Discovered package '{}' v{} at {} (runtime={})",
                manifest.package_info.name,
                manifest.package_info.version,
                path.display(),
                runtime.as_str()
            );
            Some(DiscoveredPackage {
                manifest,
                dir: path,
                entry_path,
                runtime,
            })
        }
        Err(e) => {
            tracing::warn!("Failed to load package at {}: {}", path.display(), e);
            None
        }
    }
}

pub fn discover_packages(plugins_dir: &Path) -> Vec<DiscoveredPackage> {
    let mut packages = Vec::new();

    let entries = match std::fs::read_dir(plugins_dir) {
        Ok(e) => e,
        Err(e) => {
            tracing::warn!(
                "Cannot read plugins directory {}: {}",
                plugins_dir.display(),
                e
            );
            return packages;
        }
    };

    for entry in entries.flatten() {
        if let Some(package) = discover_package(&entry.path()) {
            packages.push(package);
        }
    }

    packages
}

pub fn discover_manifests(plugins_dir: &Path) -> Vec<PackageManifest> {
    discover_packages(plugins_dir)
        .into_iter()
        .map(|package| package.manifest)
        .collect()
}

fn resolve_powershell_command() -> String {
    // Windows: 优先找 pwsh.exe 真实路径(绕过 scoop .cmd shim,避免弹 cmd 窗口)。
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // 常见 pwsh 真实路径
        let candidates = [
            r"C:\Program Files\PowerShell\7\pwsh.exe",
            r"C:\Program Files (x86)\PowerShell\7\pwsh.exe",
        ];
        for candidate in &candidates {
            if std::path::Path::new(candidate).exists() {
                return candidate.to_string();
            }
        }
        // 用 where.exe 找真实路径(不弹窗)
        let mut cmd = std::process::Command::new("where.exe");
        cmd.arg("pwsh.exe");
        cmd.creation_flags(0x08000000);
        if let Ok(output) = cmd.output() {
            if output.status.success() {
                let path = String::from_utf8_lossy(&output.stdout);
                if let Some(line) = path.lines().find(|l| l.ends_with(".exe")) {
                    return line.trim().to_string();
                }
            }
        }
    }
    #[cfg(not(windows))]
    {
        let mut cmd = std::process::Command::new("pwsh");
        cmd.arg("-NoProfile").arg("-Command").arg("echo ok");
        if cmd.output().map(|o| o.status.success()).unwrap_or(false) {
            return "pwsh".into();
        }
    }
    "powershell".into()
}

fn resolve_service_command(entry_path: &Path) -> (String, Vec<String>) {
    let mut entry = entry_path.to_string_lossy().to_string();
    if cfg!(windows) {
        if let Some(stripped) = entry.strip_prefix(r#"\\?\UNC\"#) {
            entry = format!(r#"\\{}"#, stripped);
        } else if let Some(stripped) = entry.strip_prefix(r#"\\?\"#) {
            entry = stripped.to_string();
        }
    }

    let extension = entry_path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    match extension.as_str() {
        "js" | "mjs" | "cjs" => ("node".into(), vec![entry]),
        "py" => ("python".into(), vec![entry]),
        "ps1" => (
            resolve_powershell_command(),
            vec![
                "-NoProfile".into(),
                "-ExecutionPolicy".into(),
                "Bypass".into(),
                "-File".into(),
                entry,
            ],
        ),
        _ => (entry, vec![]),
    }
}

pub fn build_service_config(package: &DiscoveredPackage) -> Result<ServiceConfig> {
    if package.runtime != PackageRuntime::Service {
        bail!(
            "Package '{}' is runtime '{}', not a service runtime",
            package.manifest.package_info.name,
            package.runtime.as_str()
        );
    }

    let entry_path = package.entry_path.as_ref().ok_or_else(|| {
        anyhow::anyhow!(
            "Service package '{}' has no entry",
            package.manifest.package_info.name
        )
    })?;
    let absolute_entry_path =
        std::fs::canonicalize(entry_path).unwrap_or_else(|_| entry_path.clone());

    let (command, args) = resolve_service_command(&absolute_entry_path);

    let startup_mode = package
        .manifest
        .runtime_contract
        .as_ref()
        .and_then(|contract| contract.startup_mode.as_deref())
        .unwrap_or("on_demand");
    let restart_policy = package
        .manifest
        .runtime_contract
        .as_ref()
        .and_then(|contract| contract.restart_policy.as_deref())
        .unwrap_or("host_managed");

    let mut env = HashMap::new();
    env.insert(
        "WEFT_PACKAGE_NAME".into(),
        package.manifest.package_info.name.clone(),
    );
    env.insert(
        "WEFT_PACKAGE_DIR".into(),
        package.dir.to_string_lossy().to_string(),
    );
    env.insert("WEFT_PACKAGE_RUNTIME".into(), "service".into());

    Ok(ServiceConfig {
        name: package.manifest.package_info.name.clone(),
        command,
        args,
        workdir: Some(package.dir.to_string_lossy().to_string()),
        env,
        health_url: package.manifest.resolved_healthcheck(),
        health_interval: 10,
        auto_start: startup_mode == "persistent",
        restart_on_crash: !matches!(restart_policy, "never" | "manual"),
    })
}

pub fn resolve_wasm_startup_mode(manifest: &PackageManifest) -> WasmStartupMode {
    match manifest
        .runtime_contract
        .as_ref()
        .and_then(|contract| contract.startup_mode.as_deref())
        .map(str::trim)
    {
        Some("on_demand") => WasmStartupMode::OnDemand,
        _ => WasmStartupMode::Persistent,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn runtime_plugin_aliases_maps_runtime_provider_to_package_name() {
        let package_index = crate::app::PackageIndex {
            version: 1,
            revision: "test".into(),
            source_url: "local://packages".into(),
            package_sources: vec![
                crate::app::PackageSource {
                    name: "agent-runtime".into(),
                    kind: "wasm".into(),
                    package_kind: "foundation".into(),
                    runtime_provider: "agent-core".into(),
                    current_source: "packages/official/agent-core".into(),
                    trusted: true,
                    signature: "builtin:official".into(),
                    source_authority: "official".into(),
                    source_public_keys: vec![],
                    provides: vec!["agent.runtime".into()],
                    requires: vec![],
                },
                crate::app::PackageSource {
                    name: "skills-runtime".into(),
                    kind: "wasm".into(),
                    package_kind: "provider".into(),
                    runtime_provider: "skills".into(),
                    current_source: "packages/official/skills".into(),
                    trusted: true,
                    signature: "builtin:official".into(),
                    source_authority: "official".into(),
                    source_public_keys: vec![],
                    provides: vec!["ext.skills".into()],
                    requires: vec![],
                },
            ],
        };

        let aliases = runtime_package_aliases(&package_index);

        assert_eq!(
            aliases.get("agent-core").map(String::as_str),
            Some("agent-runtime")
        );
        assert_eq!(
            aliases.get("skills").map(String::as_str),
            Some("skills-runtime")
        );
    }

    #[test]
    fn canonical_runtime_package_name_prefers_package_index_name() {
        let package_index = crate::app::PackageIndex {
            version: 1,
            revision: "test".into(),
            source_url: "local://packages".into(),
            package_sources: vec![crate::app::PackageSource {
                name: "agent-runtime".into(),
                kind: "wasm".into(),
                package_kind: "foundation".into(),
                runtime_provider: "agent-core".into(),
                current_source: "packages/official/agent-core".into(),
                trusted: true,
                signature: "builtin:official".into(),
                source_authority: "official".into(),
                source_public_keys: vec![],
                provides: vec!["agent.runtime".into()],
                requires: vec![],
            }],
        };

        assert_eq!(
            canonical_runtime_package_name(&package_index, "agent-core"),
            "agent-runtime"
        );
        assert_eq!(
            canonical_runtime_package_name(&package_index, "agent-runtime"),
            "agent-runtime"
        );
        assert_eq!(
            canonical_runtime_package_name(&package_index, "missing-package"),
            "missing-package"
        );
    }

    #[test]
    fn test_discover_plugins() {
        let dir = TempDir::new().unwrap();

        // Create a valid plugin
        let package_dir = dir.path().join("test-package");
        fs::create_dir_all(&package_dir).unwrap();
        fs::write(
            package_dir.join("package.toml"),
            r#"
[package_info]
name = "test-package"
version = "0.1.0"
description = "Test"
entry = "test.wasm"
"#,
        )
        .unwrap();
        fs::write(package_dir.join("test.wasm"), b"fake wasm").unwrap();

        // Create an invalid dir (no package.toml)
        let invalid_dir = dir.path().join("not-a-plugin");
        fs::create_dir_all(&invalid_dir).unwrap();

        let discovered = discover_manifests(dir.path());
        assert_eq!(discovered.len(), 1);
        assert_eq!(discovered[0].package_info.name, "test-package");
    }

    #[test]
    fn test_discover_skips_missing_entry() {
        let dir = TempDir::new().unwrap();

        let package_dir = dir.path().join("broken-package");
        fs::create_dir_all(&package_dir).unwrap();
        fs::write(
            package_dir.join("package.toml"),
            r#"
[package_info]
name = "broken"
version = "0.1.0"
description = "Missing entry"
"#,
        )
        .unwrap();

        let discovered = discover_manifests(dir.path());
        assert_eq!(discovered.len(), 0);
    }

    #[test]
    fn test_discover_uses_built_wasm_artifact_when_entry_is_not_materialized() {
        let dir = TempDir::new().unwrap();

        let package_dir = dir.path().join("source-only-package");
        fs::create_dir_all(package_dir.join("target/wasm32-wasip1/release")).unwrap();
        fs::write(
            package_dir.join("package.toml"),
            r#"
[package_info]
name = "source-only"
version = "0.1.0"
description = "Built wasm lives under target"
"#,
        )
        .unwrap();

        let built_wasm = package_dir
            .join("target")
            .join("wasm32-wasip1")
            .join("release")
            .join("plugin_source_only.wasm");
        fs::write(&built_wasm, b"fake wasm").unwrap();

        let package = discover_package(&package_dir).expect("package should discover built wasm");
        assert_eq!(package.manifest.package_info.name, "source-only");
        assert_eq!(
            package
                .entry_path
                .as_ref()
                .map(|path| normalize_existing_path(path)),
            Some(normalize_existing_path(&built_wasm))
        );
    }

    #[test]
    fn test_discover_empty_dir() {
        let dir = TempDir::new().unwrap();
        let discovered = discover_manifests(dir.path());
        assert_eq!(discovered.len(), 0);
    }

    #[test]
    fn resolve_runtime_plugin_package_prefers_declared_current_source_over_legacy_alias() {
        let repo = TempDir::new().unwrap();
        let packages_dir = repo.path().join("packages");
        let official_dir = packages_dir.join("official").join("agent-core");
        let installed_dir = packages_dir.join("installed").join("agent-core");

        fs::create_dir_all(&official_dir).unwrap();
        fs::create_dir_all(&installed_dir).unwrap();

        fs::write(
            official_dir.join("package.toml"),
            r#"
[identity]
                name = "agent-runtime"
                version = "0.1.0"
                description = "Official agent runtime"

[package]
entry = "package.wasm"
runtime = "wasm"
"#,
        )
        .unwrap();
        fs::write(official_dir.join("package.wasm"), b"official wasm").unwrap();

        fs::write(
            installed_dir.join("package.toml"),
            r#"
[package_info]
name = "agent-core"
version = "0.1.0"
description = "Installed stale runtime"
entry = "package.wasm"
"#,
        )
        .unwrap();
        fs::write(installed_dir.join("package.wasm"), b"installed wasm").unwrap();

        let package_index = crate::app::PackageIndex {
            version: 1,
            revision: "test".into(),
            source_url: "local://packages".into(),
            package_sources: vec![crate::app::PackageSource {
                name: "agent-runtime".into(),
                kind: "wasm".into(),
                package_kind: "foundation".into(),
                runtime_provider: "agent-core".into(),
                current_source: "packages/official/agent-core".into(),
                trusted: true,
                signature: "builtin:official".into(),
                source_authority: "official".into(),
                source_public_keys: vec![],
                provides: vec!["agent.runtime".into()],
                requires: vec![],
            }],
        };

        let resolved = resolve_runtime_package(repo.path(), &package_index, "agent-core")
            .expect("runtime provider alias should resolve");

        assert_eq!(resolved.manifest.package_info.name, "agent-runtime");
        assert_eq!(
            normalize_existing_path(&resolved.dir),
            normalize_existing_path(&official_dir)
        );
    }
}
