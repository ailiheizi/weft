use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Top-level package.toml structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageManifest {
    #[serde(default)]
    pub identity: Option<PackageIdentity>,
    #[serde(rename = "package_info")]
    pub package_info: PackageDescriptor,
    #[serde(default)]
    pub capability: Option<PackageCapability>,
    #[serde(default)]
    pub package: Option<PackageMeta>,
    #[serde(default)]
    pub runtime_contract: Option<PackageRuntimeContract>,
    #[serde(default)]
    pub lifecycle: Option<PackageLifecycle>,
    #[serde(default)]
    pub permissions: PackagePermissions,
    #[serde(default)]
    pub dependencies: HashMap<String, String>,
    #[serde(default)]
    pub config: HashMap<String, PackageConfigField>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PackageCapability {
    #[serde(default)]
    pub provides: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PackageIdentity {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub description: String,
}

/// [package_info] section — identity and entry point.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageDescriptor {
    pub name: String,
    pub version: String,
    pub description: String,
    #[serde(default = "default_entry")]
    pub entry: String,
    /// Capabilities this package provides (e.g., ["chat_channel", "tool", "transform"])
    #[serde(default)]
    pub provides: Vec<String>,
    /// Chat endpoint path if this package provides chat_channel capability
    pub chat_endpoint: Option<String>,
}

fn default_entry() -> String {
    "package.wasm".into()
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PackageMeta {
    #[serde(default)]
    pub entry: Option<String>,
    #[serde(default)]
    pub runtime: Option<String>,
    /// Package kind, e.g. "product" (capability composition, no executable entry),
    /// "foundation", "provider". Absent for most leaf packages.
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub api_version: Option<String>,
    #[serde(default)]
    pub native_allowed: bool,
    #[serde(default)]
    pub expected_digest: Option<String>,
    #[serde(default)]
    pub chat_endpoint: Option<String>,
    #[serde(default)]
    pub provides: Vec<String>,
    #[serde(default)]
    pub priority: Option<i32>,
    #[serde(default)]
    pub assets: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PackageRuntimeContract {
    #[serde(default)]
    pub startup_mode: Option<String>,
    #[serde(default)]
    pub communication: Vec<String>,
    #[serde(default)]
    pub config_delivery: Vec<String>,
    #[serde(default)]
    pub readiness_probe: Option<String>,
    #[serde(default)]
    pub liveness_probe: Option<String>,
    #[serde(default)]
    pub stop_mode: Option<String>,
    #[serde(default)]
    pub restart_policy: Option<String>,
}

/// [config.*] section — schema for package configuration fields.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageConfigField {
    #[serde(rename = "type")]
    pub field_type: String, // string, number, boolean, select
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub secret: bool,
    pub default: Option<serde_json::Value>,
    pub description: Option<String>,
    #[serde(default)]
    pub options: Option<Vec<String>>, // for select type
    pub min: Option<f64>,
    pub max: Option<f64>,
}

/// [permissions] section — capability declarations.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PackagePermissions {
    #[serde(default)]
    pub process: bool,
    #[serde(default)]
    pub network: bool,
    #[serde(default)]
    pub auth: bool,
    #[serde(default)]
    pub storage: bool,
    #[serde(default)]
    pub pipeline: bool,
    #[serde(default)]
    pub routes: bool,
    #[serde(default)]
    pub events: bool,
    #[serde(default)]
    pub log: bool,
    #[serde(default)]
    pub ui: bool,
    #[serde(default)]
    pub config: bool,
    #[serde(default)]
    pub scheduler: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PackageLifecycle {
    #[serde(default)]
    pub healthcheck: Option<String>,
}

fn clean_optional_string(value: Option<String>) -> Option<String> {
    value.and_then(|entry| {
        let trimmed = entry.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

impl PackageManifest {
    pub fn resolved_entry(&self) -> Option<String> {
        clean_optional_string(
            self.package
                .as_ref()
                .and_then(|package| package.entry.clone())
                .or_else(|| Some(self.package_info.entry.clone())),
        )
    }

    pub fn runtime_kind(&self) -> String {
        if let Some(runtime) = clean_optional_string(
            self.package
                .as_ref()
                .and_then(|package| package.runtime.clone()),
        ) {
            return runtime;
        }

        match self.resolved_entry() {
            Some(entry) if entry.ends_with(".wasm") => "wasm".into(),
            Some(_) => "service".into(),
            None => "wasm".into(),
        }
    }

    pub fn resolved_chat_endpoint(&self) -> Option<String> {
        clean_optional_string(
            self.package
                .as_ref()
                .and_then(|package| package.chat_endpoint.clone())
                .or_else(|| self.package_info.chat_endpoint.clone()),
        )
    }

    pub fn resolved_provides(&self) -> Vec<String> {
        let mut provides = self.package_info.provides.clone();
        if let Some(capability) = &self.capability {
            provides.extend(capability.provides.clone());
        }
        if let Some(package) = &self.package {
            provides.extend(package.provides.clone());
        }
        provides.retain(|value| !value.trim().is_empty());
        provides.sort();
        provides.dedup();
        provides
    }

    /// True if this is a product package (`[package] kind = "product"`), which
    /// composes capabilities via [requires]/[bindings] and has no executable
    /// entry of its own — so wasm/entry checks do not apply.
    pub fn is_product(&self) -> bool {
        self.package
            .as_ref()
            .and_then(|package| package.kind.as_deref())
            .map(|kind| kind.eq_ignore_ascii_case("product"))
            .unwrap_or(false)
    }

    pub fn resolved_healthcheck(&self) -> Option<String> {
        clean_optional_string(
            self.runtime_contract
                .as_ref()
                .and_then(|contract| contract.readiness_probe.clone())
                .or_else(|| {
                    self.lifecycle
                        .as_ref()
                        .and_then(|lifecycle| lifecycle.healthcheck.clone())
                }),
        )
    }
}

/// Load and parse a package.toml from a package directory.
pub fn load_manifest(package_dir: &Path) -> Result<PackageManifest> {
    let manifest_path = package_dir.join("package.toml");
    let content = std::fs::read_to_string(&manifest_path)
        .with_context(|| format!("Failed to read {}", manifest_path.display()))?;

    let has_package_section = content.lines().any(|line| line.trim() == "[package_info]");
    let mut manifest: PackageManifest = if has_package_section {
        toml::from_str(&content)
            .with_context(|| format!("Failed to parse {}", manifest_path.display()))?
    } else {
        #[derive(Debug, Clone, Default, Serialize, Deserialize)]
        struct IdentityOnlyManifest {
            #[serde(default)]
            identity: Option<PackageIdentity>,
            #[serde(default)]
            capability: Option<PackageCapability>,
            #[serde(default)]
            package: Option<PackageMeta>,
            #[serde(default)]
            runtime_contract: Option<PackageRuntimeContract>,
            #[serde(default)]
            lifecycle: Option<PackageLifecycle>,
            #[serde(default)]
            permissions: PackagePermissions,
            #[serde(default)]
            dependencies: HashMap<String, String>,
            #[serde(default)]
            config: HashMap<String, PackageConfigField>,
        }

        let raw: IdentityOnlyManifest = toml::from_str(&content)
            .with_context(|| format!("Failed to parse {}", manifest_path.display()))?;
        PackageManifest {
            identity: raw.identity,
            package_info: PackageDescriptor {
                name: String::new(),
                version: String::new(),
                description: String::new(),
                entry: default_entry(),
                provides: Vec::new(),
                chat_endpoint: None,
            },
            capability: raw.capability,
            package: raw.package,
            runtime_contract: raw.runtime_contract,
            lifecycle: raw.lifecycle,
            permissions: raw.permissions,
            dependencies: raw.dependencies,
            config: raw.config,
        }
    };

    if manifest.package_info.name.trim().is_empty() {
        if let Some(identity) = &manifest.identity {
            manifest.package_info.name = identity.name.clone();
            manifest.package_info.version = identity.version.clone();
            manifest.package_info.description = identity.description.clone();
            if manifest.package_info.entry.trim().is_empty() {
                manifest.package_info.entry = default_entry();
            }
        }
    }

    Ok(manifest)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_package_manifest() {
        let toml_str = r#"
[package_info]
name = "weft-claw"
version = "0.1.0"
description = "Weft Claw AI agent orchestration"
entry = "backend.ts"

[permissions]
process = true
network = true
auth = true
storage = true
pipeline = false
routes = true
events = true
log = true
ui = true
config = true
scheduler = true

[ui]
title = "Weft Claw"
icon = "robot"
entry = "ui/index.html"
"#;
        let manifest: PackageManifest = toml::from_str(toml_str).unwrap();
        assert_eq!(manifest.package_info.name, "weft-claw");
        assert_eq!(manifest.package_info.version, "0.1.0");
        assert_eq!(manifest.package_info.entry, "backend.ts"); // explicit entry in toml
        assert!(manifest.permissions.process);
        assert!(manifest.permissions.network);
        assert!(!manifest.permissions.pipeline);
    }

    #[test]
    fn test_parse_minimal_manifest() {
        let toml_str = r#"
[package_info]
name = "simple"
version = "0.1.0"
description = "A simple package"
"#;
        let manifest: PackageManifest = toml::from_str(toml_str).unwrap();
        assert_eq!(manifest.package_info.name, "simple");
        assert_eq!(manifest.package_info.entry, "package.wasm"); // default entry
        assert!(!manifest.permissions.process);
    }

    #[test]
    fn test_parse_identity_manifest() {
        let toml_str = r#"
[identity]
name = "companion-core"
version = "0.1.0"
description = "Companion runtime"

[package]
entry = "server.py"
runtime = "service"
"#;

        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("package.toml");
        std::fs::write(&path, toml_str).unwrap();
        let loaded = load_manifest(temp.path()).unwrap();
        assert_eq!(loaded.package_info.name, "companion-core");
        assert_eq!(loaded.package_info.version, "0.1.0");
        assert_eq!(loaded.package_info.description, "Companion runtime");
        assert_eq!(loaded.resolved_entry().as_deref(), Some("server.py"));
    }

    #[test]
    fn test_parse_modern_service_manifest() {
        let toml_str = r#"
[package_info]
name = "ticker-watch"
version = "0.1.0"
description = "Ticker watcher"

[package]
entry = "dist/server.js"
runtime = "service"
chat_endpoint = "/chat"

[runtime_contract]
startup_mode = "persistent"
communication = ["http"]
config_delivery = ["config_file"]
readiness_probe = "http://127.0.0.1:43111/health"
restart_policy = "host_managed"

[lifecycle]
healthcheck = "http://127.0.0.1:43111/health"
"#;
        let manifest: PackageManifest = toml::from_str(toml_str).unwrap();
        assert_eq!(manifest.resolved_entry().as_deref(), Some("dist/server.js"));
        assert_eq!(manifest.runtime_kind(), "service");
        assert_eq!(manifest.resolved_chat_endpoint().as_deref(), Some("/chat"));
        assert_eq!(
            manifest.resolved_healthcheck().as_deref(),
            Some("http://127.0.0.1:43111/health")
        );
    }

    #[test]
    fn test_resolved_provides_includes_package_values() {
        let toml_str = r#"
[package_info]
name = "companion-core"
version = "0.1.0"
description = "Companion"

[package]
entry = "server.py"
runtime = "service"
provides = ["chat_channel", "companion.turn.handle"]
"#;
        let manifest: PackageManifest = toml::from_str(toml_str).unwrap();
        assert!(manifest
            .resolved_provides()
            .contains(&"chat_channel".to_string()));
        assert!(manifest
            .resolved_provides()
            .contains(&"companion.turn.handle".to_string()));
    }

    #[test]
    fn test_resolved_provides_includes_capability_section() {
        let toml_str = r#"
[identity]
name = "companion-core"
version = "0.1.0"
description = "Companion runtime"

[package]
entry = "server.py"
runtime = "service"

[capability]
provides = ["chat_channel", "companion.turn.handle"]
"#;
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("package.toml");
        std::fs::write(&path, toml_str).unwrap();
        let loaded = load_manifest(temp.path()).unwrap();
        assert!(loaded
            .resolved_provides()
            .contains(&"chat_channel".to_string()));
        assert!(loaded
            .resolved_provides()
            .contains(&"companion.turn.handle".to_string()));
    }

    #[test]
    fn test_load_current_official_package_manifests() {
        let repo_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");

        let agent_core = load_manifest(
            &repo_root
                .join("packages")
                .join("official")
                .join("agent-core"),
        )
        .expect("agent-core manifest should load");
        assert_eq!(agent_core.package_info.name, "agent-runtime");
        assert_eq!(agent_core.runtime_kind(), "wasm");
        assert!(agent_core
            .resolved_provides()
            .contains(&"agent.runtime".to_string()));

        let weft_claw = load_manifest(
            &repo_root
                .join("packages")
                .join("official")
                .join("weft-claw"),
        )
        .expect("weft-claw manifest should load");
        assert_eq!(weft_claw.package_info.name, "weft-claw");
        assert_eq!(weft_claw.runtime_kind(), "wasm");
        assert!(weft_claw
            .resolved_provides()
            .contains(&"weft_claw.turn".to_string()));
    }
}
