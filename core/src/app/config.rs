use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

fn string_is_empty(value: &str) -> bool {
    value.trim().is_empty()
}

fn scene_metadata_is_empty(metadata: &AppSceneMetadata) -> bool {
    metadata.created_by.trim().is_empty()
        && metadata.created_at.is_none()
        && metadata.updated_at.is_none()
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppIdentity {
    pub name: String,
    pub version: String,
    pub display_name: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppRequires {
    #[serde(default)]
    pub capabilities: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppFeatureSet {
    #[serde(default)]
    pub default_enabled: Vec<String>,
    #[serde(default)]
    pub optional: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppFeatureBinding {
    #[serde(default)]
    pub requires: Vec<String>,
    #[serde(default)]
    pub packages: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppBindingConfig {
    pub provider: String,
    #[serde(default)]
    pub mutable: bool,
    #[serde(default)]
    pub allowed: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppValidation {
    #[serde(default)]
    pub checks: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppUpgrade {
    pub strategy: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppManifest {
    #[serde(default)]
    pub schema_version: u32,
    #[serde(default)]
    pub app: AppIdentity,
    #[serde(default)]
    pub requires: AppRequires,
    #[serde(default)]
    pub bindings: HashMap<String, HashMap<String, AppBindingConfig>>,
    #[serde(default)]
    pub features: AppFeatureSet,
    #[serde(default)]
    pub feature_bindings: HashMap<String, AppFeatureBinding>,
    #[serde(default)]
    pub validation: AppValidation,
    #[serde(default)]
    pub upgrade: AppUpgrade,
}

pub type ProductPackageDeclaration = AppManifest;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct RawProductIdentity {
    #[serde(default)]
    name: String,
    #[serde(default)]
    version: String,
    #[serde(default)]
    display_name: String,
    #[serde(default)]
    description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct RawProductPackageMeta {
    #[serde(default)]
    kind: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct RawProductPackageDeclaration {
    #[serde(default)]
    schema_version: u32,
    #[serde(default)]
    identity: RawProductIdentity,
    #[serde(default)]
    package: RawProductPackageMeta,
    #[serde(default)]
    requires: AppRequires,
    #[serde(default)]
    bindings: HashMap<String, AppBindingConfig>,
    #[serde(default)]
    features: AppFeatureSet,
    #[serde(default)]
    feature_bindings: HashMap<String, AppFeatureBinding>,
    #[serde(default)]
    validation: AppValidation,
    #[serde(default)]
    upgrade: AppUpgrade,
}

impl AppManifest {
    pub fn flattened_bindings(&self) -> HashMap<String, AppBindingConfig> {
        let mut result = HashMap::new();

        for (section, entries) in &self.bindings {
            for (name, binding) in entries {
                result.insert(format!("{}.{}", section, name), binding.clone());
            }
        }

        result
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppRuntimeConfig {
    #[serde(default)]
    pub profile: String,
    #[serde(default)]
    pub scene: String,
    #[serde(default)]
    pub workspace: String,
    #[serde(default)]
    pub default_model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppSecretsConfig {
    #[serde(default)]
    pub llm_api_key_ref: Option<String>,
    #[serde(default)]
    pub telegram_bot_token_ref: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppFeaturesConfig {
    #[serde(default)]
    pub enabled: Vec<String>,
    #[serde(default)]
    pub disabled: Vec<String>,
    #[serde(default)]
    pub channels_enabled: bool,
    #[serde(default)]
    pub mcp_enabled: bool,
    #[serde(default)]
    pub skills_enabled: bool,
    #[serde(default)]
    pub enable_chat: bool,
    #[serde(default)]
    pub enable_tools: bool,
    #[serde(default)]
    pub enable_memory: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppBindingOverride {
    #[serde(default)]
    pub provider: String,
    #[serde(default)]
    pub reason: String,
    #[serde(default)]
    pub package: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub sha512: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppSceneBindingPin {
    #[serde(default)]
    pub capability: String,
    #[serde(default)]
    pub package: String,
    #[serde(default)]
    pub provider: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub sha512: String,
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppScenePackagePin {
    #[serde(default)]
    pub package: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub sha512: String,
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppSceneFeatureSelection {
    #[serde(default)]
    pub enabled: Vec<String>,
    #[serde(default)]
    pub disabled: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppSceneMetadata {
    #[serde(default, skip_serializing_if = "string_is_empty")]
    pub created_by: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_at: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<u64>,
}

#[derive(Debug, Clone, Default)]
pub struct AppSceneConfig {
    pub schema_version: u32,
    pub name: String,
    pub description: String,
    pub profile: String,
    pub base_generation: Option<u64>,
    pub enabled_features: Vec<String>,
    pub disabled_features: Vec<String>,
    pub binding_pins: Vec<AppSceneBindingPin>,
    pub package_pins: Vec<AppScenePackagePin>,
    /// 角色→模型路由(planner/implementer/reviewer/integrator)。激活该 scene 时
    /// 写入 KV `team:role_routing`,使切场景即切团队各角色用的模型。空则不改路由。
    pub role_routing: HashMap<String, crate::config::RoleModel>,
    /// 会话默认工作区目录。激活该 scene 时作为新会话的默认 workspace_root。空则用系统默认。
    pub workspace: String,
    pub metadata: AppSceneMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct AppSceneConfigSerde {
    #[serde(default)]
    schema_version: u32,
    #[serde(default)]
    name: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    profile: String,
    #[serde(default)]
    base_generation: Option<u64>,
    #[serde(default)]
    features: AppSceneFeatureSelection,
    #[serde(default)]
    bindings: Vec<AppSceneBindingPin>,
    #[serde(default)]
    packages: Vec<AppScenePackagePin>,
    #[serde(default)]
    enabled_features: Vec<String>,
    #[serde(default)]
    disabled_features: Vec<String>,
    #[serde(default, rename = "binding_pins")]
    binding_pins: Vec<AppSceneBindingPin>,
    #[serde(default, rename = "package_pins")]
    package_pins: Vec<AppScenePackagePin>,
    #[serde(default, skip_serializing_if = "scene_metadata_is_empty")]
    metadata: AppSceneMetadata,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    role_routing: HashMap<String, crate::config::RoleModel>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    workspace: String,
}

impl From<AppSceneConfigSerde> for AppSceneConfig {
    fn from(value: AppSceneConfigSerde) -> Self {
        let enabled_features = if value.features.enabled.is_empty() {
            value.enabled_features
        } else {
            value.features.enabled
        };
        let disabled_features = if value.features.disabled.is_empty() {
            value.disabled_features
        } else {
            value.features.disabled
        };
        let binding_pins = if value.bindings.is_empty() {
            value.binding_pins
        } else {
            value.bindings
        };
        let package_pins = if value.packages.is_empty() {
            value.package_pins
        } else {
            value.packages
        };

        Self {
            schema_version: value.schema_version,
            name: value.name,
            description: value.description,
            profile: value.profile,
            base_generation: value.base_generation,
            enabled_features,
            disabled_features,
            binding_pins,
            package_pins,
            role_routing: value.role_routing,
            workspace: value.workspace,
            metadata: value.metadata,
        }
    }
}

impl From<&AppSceneConfig> for AppSceneConfigSerde {
    fn from(value: &AppSceneConfig) -> Self {
        Self {
            schema_version: value.schema_version,
            name: value.name.clone(),
            description: value.description.clone(),
            profile: value.profile.clone(),
            base_generation: value.base_generation,
            features: AppSceneFeatureSelection {
                enabled: value.enabled_features.clone(),
                disabled: value.disabled_features.clone(),
            },
            bindings: value.binding_pins.clone(),
            packages: value.package_pins.clone(),
            enabled_features: Vec::new(),
            disabled_features: Vec::new(),
            binding_pins: Vec::new(),
            package_pins: Vec::new(),
            metadata: value.metadata.clone(),
            role_routing: value.role_routing.clone(),
            workspace: value.workspace.clone(),
        }
    }
}

impl Serialize for AppSceneConfig {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        AppSceneConfigSerde::from(self).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for AppSceneConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        AppSceneConfigSerde::deserialize(deserializer).map(Into::into)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppPackageOverride {
    #[serde(default)]
    pub enabled: Vec<String>,
    #[serde(default)]
    pub disabled: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppConfigFile {
    #[serde(default)]
    pub schema_version: u32,
    #[serde(default)]
    pub app_runtime: AppRuntimeConfig,
    #[serde(default)]
    pub secrets: AppSecretsConfig,
    #[serde(default)]
    pub features: AppFeaturesConfig,
    #[serde(default)]
    pub packages: AppPackageOverride,
    #[serde(default)]
    pub binding_overrides: HashMap<String, AppBindingOverride>,
    #[serde(default)]
    pub active_scene: String,
    #[serde(default)]
    pub scenes: HashMap<String, AppSceneConfig>,
}

pub type InstanceConfig = AppConfigFile;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppLockEvidence {
    #[serde(default)]
    pub digest: String,
    #[serde(default)]
    pub signature: String,
    #[serde(default)]
    pub source_authority: String,
    #[serde(default)]
    pub source_public_keys: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppLockPackageReason {
    #[serde(default)]
    pub layer: String,
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppLockNotes {
    #[serde(default)]
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppLockPackage {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub runtime: String,
    #[serde(default)]
    pub sha512: String,
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub trusted: bool,
    #[serde(default)]
    pub signature: String,
    #[serde(default)]
    pub source_authority: String,
    #[serde(default)]
    pub source_public_keys: Vec<String>,
    #[serde(default)]
    pub package_kind: String,
    #[serde(default)]
    pub manifest_digest: String,
    #[serde(default)]
    pub artifact_digest: String,
    #[serde(default)]
    pub artifact_set_id: String,
    #[serde(default)]
    pub store_object_id: String,
    #[serde(default)]
    pub store_path: String,
    #[serde(default)]
    pub closure_id: String,
    #[serde(default)]
    pub entry_kind: String,
    #[serde(default)]
    pub runtime_provider: String,
    #[serde(default)]
    pub provides: Vec<String>,
    #[serde(default)]
    pub requires: Vec<String>,
    #[serde(default)]
    pub roles: Vec<String>,
    #[serde(default)]
    pub capabilities: Vec<String>,
    #[serde(default)]
    pub features: Vec<String>,
    #[serde(default)]
    pub default_enabled_features: Vec<String>,
    #[serde(default)]
    pub evidence: AppLockEvidence,
    #[serde(default)]
    pub reasons: Vec<AppLockPackageReason>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppLockBinding {
    #[serde(default)]
    pub capability: String,
    #[serde(default)]
    pub provider: String,
    #[serde(default)]
    pub package: String,
    #[serde(default)]
    pub mutable: bool,
    #[serde(default)]
    pub package_version: String,
    #[serde(default)]
    pub package_sha512: String,
    #[serde(default)]
    pub binding_source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppLockFeature {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub resolved: bool,
    #[serde(default)]
    pub packages: Vec<String>,
    #[serde(default)]
    pub capabilities: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppLockInputSnapshot {
    #[serde(default)]
    pub declaration_schema_version: u32,
    #[serde(default)]
    pub config_schema_version: u32,
    #[serde(default)]
    pub declaration_digest: String,
    #[serde(default)]
    pub config_digest: String,
    #[serde(default)]
    pub scene_digest: String,
    #[serde(default)]
    pub package_index_digest: String,
    #[serde(default)]
    pub package_server_snapshot_digest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppLockAssembly {
    #[serde(default)]
    pub enabled_features: Vec<String>,
    #[serde(default)]
    pub selected_packages: Vec<String>,
    #[serde(default)]
    pub scene: String,
    #[serde(default)]
    pub scene_digest: String,
    #[serde(default)]
    pub binding_set_id: String,
    #[serde(default)]
    pub closure_id: String,
    #[serde(default)]
    pub closure_digest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppLockBindingSource {
    #[serde(default)]
    pub capability: String,
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub package: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppLockFile {
    #[serde(default)]
    pub lock_version: u32,
    #[serde(default)]
    pub app: String,
    #[serde(default)]
    pub generation: u32,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub profile: String,
    #[serde(default)]
    pub scene: String,
    #[serde(default)]
    pub scene_digest: String,
    #[serde(default)]
    pub binding_set_id: String,
    #[serde(default)]
    pub closure_id: String,
    #[serde(default)]
    pub closure_digest: String,
    #[serde(default)]
    pub store_generation_id: String,
    #[serde(default)]
    pub trust_level: String,
    #[serde(default)]
    pub dev_unsealed: bool,
    #[serde(default)]
    pub features: Vec<AppLockFeature>,
    #[serde(default)]
    pub inputs: AppLockInputSnapshot,
    #[serde(default)]
    pub assembly: AppLockAssembly,
    #[serde(default)]
    pub packages: Vec<AppLockPackage>,
    #[serde(default)]
    pub bindings: Vec<AppLockBinding>,
    #[serde(default)]
    pub binding_sources: Vec<AppLockBindingSource>,
    #[serde(default)]
    pub notes: AppLockNotes,
}

pub type InstanceLock = AppLockFile;

fn discover_repo_root(base_dir: &Path) -> Option<PathBuf> {
    base_dir.ancestors().find_map(|ancestor| {
        let has_packages = ancestor.join("packages").exists();
        if has_packages {
            Some(ancestor.to_path_buf())
        } else {
            None
        }
    })
}

fn logical_name(base_dir: &Path) -> Option<String> {
    if base_dir.join("package.toml").exists() {
        if let Ok(content) = std::fs::read_to_string(base_dir.join("package.toml")) {
            if let Ok(raw) = toml::from_str::<RawProductPackageDeclaration>(&content) {
                let name = raw.identity.name.trim();
                if !name.is_empty() {
                    return Some(name.to_string());
                }
            }
        }
    }

    match base_dir.file_name().and_then(|name| name.to_str()) {
        Some(".weft") | None => None,
        Some(name) => Some(name.to_string()),
    }
}

fn preferred_product_package_dir(base_dir: &Path) -> Option<PathBuf> {
    if base_dir.join("package.toml").exists() {
        return Some(base_dir.to_path_buf());
    }

    let repo_root = discover_repo_root(base_dir)?;
    let name = logical_name(base_dir)?;
    let package_dir = repo_root.join("packages").join(&name);
    if package_dir.join("package.toml").exists() {
        Some(package_dir)
    } else {
        None
    }
}

fn preferred_instance_dir(base_dir: &Path) -> Option<PathBuf> {
    if base_dir.join("config.toml").exists() || base_dir.join("lock.toml").exists() {
        return Some(base_dir.to_path_buf());
    }

    let repo_root = discover_repo_root(base_dir)?;
    let name = logical_name(base_dir)?;
    let instance_dir = repo_root.join(".weft").join(&name);
    if instance_dir.join("config.toml").exists() || instance_dir.join("lock.toml").exists() {
        Some(instance_dir)
    } else {
        None
    }
}

fn desired_instance_dir(base_dir: &Path) -> Option<PathBuf> {
    preferred_instance_dir(base_dir).or_else(|| {
        let repo_root = discover_repo_root(base_dir)?;
        let name = logical_name(base_dir)?;
        Some(repo_root.join(".weft").join(name))
    })
}

fn parse_product_package_declaration(path: &Path) -> Result<ProductPackageDeclaration> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read {}", path.display()))?;

    if let Ok(raw) = toml::from_str::<RawProductPackageDeclaration>(&content) {
        if !raw.identity.name.trim().is_empty()
            && (raw.package.kind.trim().is_empty() || raw.package.kind == "product")
        {
            return Ok(ProductPackageDeclaration {
                schema_version: raw.schema_version,
                app: AppIdentity {
                    name: raw.identity.name,
                    version: raw.identity.version,
                    display_name: raw.identity.display_name,
                    description: raw.identity.description,
                },
                requires: raw.requires,
                bindings: inflate_flat_bindings(raw.bindings),
                features: raw.features,
                feature_bindings: raw.feature_bindings,
                validation: raw.validation,
                upgrade: raw.upgrade,
            });
        }
    }

    toml::from_str(&content).with_context(|| format!("Failed to parse {}", path.display()))
}

pub fn load_product_package_declaration_from_path(
    path: &Path,
) -> Result<ProductPackageDeclaration> {
    parse_product_package_declaration(path)
}

pub fn load_instance_config_from_path(path: &Path) -> Result<InstanceConfig> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read {}", path.display()))?;
    let mut config: InstanceConfig =
        toml::from_str(&content).with_context(|| format!("Failed to parse {}", path.display()))?;

    if let Some(instance_dir) = path.parent() {
        merge_scene_files(instance_dir, &mut config)?;
    }

    Ok(config)
}

pub fn load_instance_lock_from_path(path: &Path) -> Result<InstanceLock> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read {}", path.display()))?;
    toml::from_str(&content).with_context(|| format!("Failed to parse {}", path.display()))
}

pub fn load_scene_config_from_path(path: &Path) -> Result<AppSceneConfig> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read {}", path.display()))?;
    toml::from_str(&content).with_context(|| format!("Failed to parse {}", path.display()))
}

pub fn save_scene_config_to_path(path: &Path, scene: &AppSceneConfig) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create {}", parent.display()))?;
    }
    let content =
        toml::to_string_pretty(scene).with_context(|| "Failed to serialize scene file")?;
    std::fs::write(path, content).with_context(|| format!("Failed to write {}", path.display()))
}

pub fn save_instance_config_to_path(path: &Path, config: &InstanceConfig) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create {}", parent.display()))?;
    }
    let content =
        toml::to_string_pretty(config).with_context(|| "Failed to serialize config file")?;
    std::fs::write(path, content).with_context(|| format!("Failed to write {}", path.display()))
}

pub fn save_instance_lock_to_path(path: &Path, lock: &InstanceLock) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create {}", parent.display()))?;
    }
    let content = toml::to_string_pretty(lock).with_context(|| "Failed to serialize lock file")?;
    std::fs::write(path, content).with_context(|| format!("Failed to write {}", path.display()))
}

fn inflate_flat_bindings(
    bindings: HashMap<String, AppBindingConfig>,
) -> HashMap<String, HashMap<String, AppBindingConfig>> {
    let mut nested = HashMap::new();

    for (capability, binding) in bindings {
        let (section, name) = capability
            .rsplit_once('.')
            .map(|(section, name)| (section.to_string(), name.to_string()))
            .unwrap_or_else(|| (capability.clone(), String::from("default")));
        nested
            .entry(section)
            .or_insert_with(HashMap::new)
            .insert(name, binding);
    }

    nested
}

fn merge_scene_files(instance_dir: &Path, config: &mut InstanceConfig) -> Result<()> {
    let scenes_dir = instance_dir.join("scenes");
    if !scenes_dir.exists() {
        return Ok(());
    }

    let mut scene_paths = std::fs::read_dir(&scenes_dir)
        .with_context(|| format!("Failed to read {}", scenes_dir.display()))?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("toml"))
        .collect::<Vec<_>>();
    scene_paths.sort();

    for scene_path in scene_paths {
        let mut scene = load_scene_config_from_path(&scene_path)?;
        if scene.name.trim().is_empty() {
            scene.name = scene_path
                .file_stem()
                .and_then(|stem| stem.to_str())
                .unwrap_or_default()
                .to_string();
        }

        if scene.name.trim().is_empty() {
            continue;
        }

        if config.scenes.contains_key(&scene.name) {
            tracing::warn!(
                scene = %scene.name,
                path = %scene_path.display(),
                "scene file overrides embedded config scene"
            );
        }

        config.scenes.insert(scene.name.clone(), scene);
    }

    Ok(())
}

pub fn product_package_declaration_path(base_dir: &Path) -> PathBuf {
    preferred_product_package_dir(base_dir)
        .map(|dir| dir.join("package.toml"))
        .unwrap_or_else(|| base_dir.join("package.toml"))
}

pub fn instance_config_path(base_dir: &Path) -> PathBuf {
    preferred_instance_dir(base_dir)
        .map(|dir| dir.join("config.toml"))
        .filter(|path| path.exists())
        .unwrap_or_else(|| base_dir.join("config.toml"))
}

pub fn instance_lock_path(base_dir: &Path) -> PathBuf {
    preferred_instance_dir(base_dir)
        .map(|dir| dir.join("lock.toml"))
        .filter(|path| path.exists())
        .unwrap_or_else(|| base_dir.join("lock.toml"))
}

impl AppLockPackage {
    pub fn identity(&self) -> String {
        if !self.name.trim().is_empty() {
            self.name.clone()
        } else {
            self.runtime_provider.clone()
        }
    }
}

pub fn load_app_manifest(app_dir: &Path) -> Result<AppManifest> {
    let path = app_dir.join("app.toml");
    let content = std::fs::read_to_string(&path)
        .with_context(|| format!("Failed to read {}", path.display()))?;
    toml::from_str(&content).with_context(|| format!("Failed to parse {}", path.display()))
}

pub fn load_product_package_declaration(app_dir: &Path) -> Result<ProductPackageDeclaration> {
    let path = product_package_declaration_path(app_dir);
    load_product_package_declaration_from_path(&path)
}

pub fn load_app_config(app_dir: &Path) -> Result<AppConfigFile> {
    let path = app_dir.join("app.config.toml");
    let content = std::fs::read_to_string(&path)
        .with_context(|| format!("Failed to read {}", path.display()))?;
    toml::from_str(&content).with_context(|| format!("Failed to parse {}", path.display()))
}

pub fn load_instance_config(app_dir: &Path) -> Result<InstanceConfig> {
    let path = instance_config_path(app_dir);
    load_instance_config_from_path(&path)
}

pub fn load_app_lock(app_dir: &Path) -> Result<AppLockFile> {
    let path = app_dir.join("app.lock");
    let content = std::fs::read_to_string(&path)
        .with_context(|| format!("Failed to read {}", path.display()))?;
    toml::from_str(&content).with_context(|| format!("Failed to parse {}", path.display()))
}

pub fn load_instance_lock(app_dir: &Path) -> Result<InstanceLock> {
    let path = instance_lock_path(app_dir);
    load_instance_lock_from_path(&path)
}

pub fn save_app_lock(app_dir: &Path, lock: &AppLockFile) -> Result<()> {
    let path = app_dir.join("app.lock");
    let content = toml::to_string_pretty(lock).with_context(|| "Failed to serialize lock file")?;
    std::fs::write(&path, content).with_context(|| format!("Failed to write {}", path.display()))
}

pub fn save_instance_lock(app_dir: &Path, lock: &InstanceLock) -> Result<()> {
    let path = desired_instance_dir(app_dir)
        .unwrap_or_else(|| app_dir.to_path_buf())
        .join("lock.toml");
    save_instance_lock_to_path(&path, lock)
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use super::{
        instance_config_path, instance_lock_path, load_instance_config, load_instance_lock,
        load_product_package_declaration, product_package_declaration_path, AppConfigFile,
        AppLockFile, AppLockPackage, AppLockPackageReason, AppSceneBindingPin, AppSceneConfig,
        AppSceneMetadata, AppScenePackagePin,
    };

    fn temp_root(name: &str) -> std::path::PathBuf {
        let unique = format!(
            "weft-phase1-{}-{}",
            name,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system time after epoch")
                .as_nanos()
        );
        std::env::temp_dir().join(unique)
    }

    fn weft_claw_package_dir() -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("packages")
            .join("official")
            .join("weft-claw")
    }

    fn weft_claw_instance_dir() -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join(".weft")
            .join("weft-claw")
    }

    #[test]
    fn product_package_declaration_loader_reads_package_manifest() {
        let declaration = load_product_package_declaration(&weft_claw_package_dir())
            .expect("product package declaration remains readable from package.toml");

        assert_eq!(declaration.app.name, "weft-claw");
        assert_eq!(declaration.app.version, "0.1.0");
    }

    #[test]
    fn weft_code_product_package_manifest_parses_with_repo_loader() {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("packages")
            .join("weft-code")
            .join("package.toml");

        let declaration = super::load_product_package_declaration_from_path(&path)
            .expect("weft-code product package manifest should parse");

        assert_eq!(declaration.app.name, "weft-code");
        assert_eq!(declaration.app.version, "0.1.0");
        assert!(declaration
            .requires
            .capabilities
            .iter()
            .any(|capability| capability == "agent.runtime"));
        assert!(declaration.features.default_enabled.is_empty());
        assert!(declaration
            .flattened_bindings()
            .contains_key("weft_code.runtime"));
    }

    #[test]
    fn instance_config_loader_reads_instance_config_path() {
        let config = load_instance_config(&weft_claw_instance_dir())
            .expect("instance config remains readable from config.toml");

        assert_eq!(config.app_runtime.profile, "developer");
        assert!(config
            .features
            .enabled
            .iter()
            .all(|feature| !feature.starts_with("weft-claw-") || !feature.ends_with("-feature")));
    }

    #[test]
    fn instance_lock_path_defaults_to_instance_lock_location() {
        assert_eq!(
            instance_lock_path(&weft_claw_instance_dir()),
            weft_claw_instance_dir().join("lock.toml")
        );
    }

    #[test]
    fn product_package_loader_reads_package_toml_only() {
        let root = temp_root("product-package");
        let package_dir = root.join("packages").join("demo");
        std::fs::create_dir_all(&package_dir).expect("package dir created");
        std::fs::write(
            package_dir.join("package.toml"),
            "schema_version = 2\n[identity]\nname='demo'\nversion='product'\ndisplay_name='Product'\ndescription='product'\n[package]\nkind='product'\n[requires]\ncapabilities=[]\n",
        )
        .expect("product package declaration written");

        let declaration = load_product_package_declaration(&package_dir)
            .expect("package declaration should load from package.toml");
        assert_eq!(
            product_package_declaration_path(&package_dir),
            package_dir.join("package.toml")
        );
        assert_eq!(declaration.app.version, "product");

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn instance_config_loader_reads_instance_path_only() {
        let root = temp_root("instance-config");
        let instance_dir = root.join(".weft").join("demo");
        std::fs::create_dir_all(&instance_dir).expect("instance dir created");
        std::fs::write(
            instance_dir.join("config.toml"),
            "schema_version = 2\n[app_runtime]\nprofile='developer'\n",
        )
        .expect("instance config written");

        let config = load_instance_config(&instance_dir)
            .expect("instance config should load from config.toml");
        assert_eq!(
            instance_config_path(&instance_dir),
            instance_dir.join("config.toml")
        );
        assert_eq!(config.app_runtime.profile, "developer");

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn instance_lock_loader_reads_instance_path_only() {
        let root = temp_root("instance-lock");
        let instance_dir = root.join(".weft").join("demo");
        std::fs::create_dir_all(&instance_dir).expect("instance dir created");
        std::fs::write(
            instance_dir.join("lock.toml"),
            "lock_version = 2\napp='demo'\ngeneration=2\nstatus='instance'\n",
        )
        .expect("instance lock written");

        let lock =
            load_instance_lock(&instance_dir).expect("instance lock should load from lock.toml");
        assert_eq!(
            instance_lock_path(&instance_dir),
            instance_dir.join("lock.toml")
        );
        assert_eq!(lock.generation, 2);
        assert_eq!(lock.status, "instance");

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn legacy_embedded_scene_config_still_deserializes() {
        let config: AppConfigFile = toml::from_str(
            r#"
schema_version = 2
active_scene = 'team'

[scenes.team]
name = 'team'
description = 'Legacy embedded scene.'
profile = 'developer'
base_generation = 18
enabled_features = ['feature-a']
disabled_features = ['feature-b']

[[scenes.team.binding_pins]]
capability = 'team.delegate'
package = 'agent-runtime'
provider = 'agent-runtime'

[[scenes.team.package_pins]]
package = 'agent-runtime'
version = '0.1.0'
source = 'local-index'
"#,
        )
        .expect("legacy embedded scenes should deserialize");

        let scene = config.scenes.get("team").expect("team scene present");
        assert_eq!(scene.name, "team");
        assert_eq!(scene.schema_version, 0);
        assert_eq!(scene.enabled_features, vec!["feature-a"]);
        assert_eq!(scene.disabled_features, vec!["feature-b"]);
        assert_eq!(scene.binding_pins.len(), 1);
        assert_eq!(scene.package_pins.len(), 1);
        assert!(scene.metadata.created_by.is_empty());
        assert_eq!(scene.metadata.created_at, None);
        assert_eq!(scene.metadata.updated_at, None);
    }

    #[test]
    fn instance_scene_file_overrides_embedded_scene_config() {
        let root = temp_root("scene-merge");
        let instance_dir = root.join(".weft").join("demo");
        let scenes_dir = instance_dir.join("scenes");
        std::fs::create_dir_all(&scenes_dir).expect("scene dir created");
        std::fs::write(
            instance_dir.join("config.toml"),
            r#"
schema_version = 2
active_scene = 'team'

[scenes.team]
name = 'team'
description = 'embedded'
enabled_features = ['embedded-feature']
"#,
        )
        .expect("config written");
        std::fs::write(
            scenes_dir.join("team.toml"),
            r#"
schema_version = 1
name = 'team'
description = 'scene file'
profile = 'developer'

[features]
enabled = ['file-feature']

[metadata]
created_by = 'cli'
created_at = 1710000000
updated_at = 1710001000
"#,
        )
        .expect("scene file written");

        let config = load_instance_config(&instance_dir).expect("instance config should load");
        let scene = config.scenes.get("team").expect("scene file merged");

        assert_eq!(scene.schema_version, 1);
        assert_eq!(scene.description, "scene file");
        assert_eq!(scene.profile, "developer");
        assert_eq!(scene.enabled_features, vec!["file-feature"]);
        assert_eq!(scene.metadata.created_by, "cli");
        assert_eq!(scene.metadata.created_at, Some(1710000000));
        assert_eq!(scene.metadata.updated_at, Some(1710001000));

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn new_scene_schema_round_trips() {
        let scene = AppSceneConfig {
            schema_version: 1,
            name: "team".into(),
            description: "Enable team delegation.".into(),
            profile: "developer".into(),
            base_generation: Some(18),
            enabled_features: vec!["feature-a".into()],
            disabled_features: vec!["feature-b".into()],
            binding_pins: vec![AppSceneBindingPin {
                capability: "team.delegate".into(),
                package: "agent-runtime".into(),
                provider: "agent-runtime".into(),
                version: "0.1.0".into(),
                sha512: "sha512:abc".into(),
                source: "local-index".into(),
                reason: "Pin team delegate provider.".into(),
            }],
            package_pins: vec![AppScenePackagePin {
                package: "agent-runtime".into(),
                version: "0.1.0".into(),
                sha512: "sha512:abc".into(),
                source: "local-index".into(),
                reason: "Required by team scene.".into(),
            }],
            metadata: AppSceneMetadata {
                created_by: "cli".into(),
                created_at: Some(1710000000),
                updated_at: Some(1710001000),
            },
            role_routing: HashMap::new(),
            workspace: String::new(),
        };

        let content = toml::to_string_pretty(&scene).expect("scene should serialize");
        assert!(content.contains("[features]"));
        assert!(content.contains("[[bindings]]"));
        assert!(content.contains("[[packages]]"));
        assert!(content.contains("[metadata]"));

        let parsed: AppSceneConfig = toml::from_str(&content).expect("scene should deserialize");
        assert_eq!(parsed.schema_version, 1);
        assert_eq!(parsed.name, "team");
        assert_eq!(parsed.base_generation, Some(18));
        assert_eq!(parsed.enabled_features, vec!["feature-a"]);
        assert_eq!(parsed.disabled_features, vec!["feature-b"]);
        assert_eq!(parsed.binding_pins.len(), 1);
        assert_eq!(parsed.package_pins.len(), 1);
        assert_eq!(parsed.metadata.created_by, "cli");
        assert_eq!(parsed.metadata.created_at, Some(1710000000));
        assert_eq!(parsed.metadata.updated_at, Some(1710001000));
    }

    #[test]
    fn legacy_lock_without_generation_metadata_still_deserializes() {
        let lock: AppLockFile = toml::from_str(
            r#"
lock_version = 1
app = 'demo'
generation = 7
status = 'active'
profile = 'developer'

[assembly]
enabled_features = []
selected_packages = []

[[packages]]
name = 'agent-runtime'
version = '0.1.0'
"#,
        )
        .expect("legacy lock should deserialize");

        assert_eq!(lock.scene, "");
        assert_eq!(lock.scene_digest, "");
        assert_eq!(lock.binding_set_id, "");
        assert_eq!(lock.closure_id, "");
        assert_eq!(lock.closure_digest, "");
        assert_eq!(lock.store_generation_id, "");
        assert_eq!(lock.trust_level, "");
        assert!(!lock.dev_unsealed);
        assert_eq!(lock.packages.len(), 1);
        assert!(lock.packages[0].reasons.is_empty());
    }

    #[test]
    fn new_lock_generation_metadata_round_trips() {
        let lock = AppLockFile {
            lock_version: 2,
            app: "demo".into(),
            generation: 20,
            status: "verified".into(),
            profile: "developer".into(),
            scene: "team".into(),
            scene_digest: "sha256:scene".into(),
            binding_set_id: "binding-set:sha256:bindings".into(),
            closure_id: "closure:sha256:closure".into(),
            closure_digest: "sha256:closure".into(),
            store_generation_id: "store-gen:sha256:store".into(),
            trust_level: "verified".into(),
            dev_unsealed: false,
            inputs: super::AppLockInputSnapshot {
                declaration_schema_version: 1,
                config_schema_version: 1,
                declaration_digest: "sha256:declaration".into(),
                config_digest: "sha256:config".into(),
                scene_digest: "sha256:scene".into(),
                package_index_digest: "sha256:index".into(),
                package_server_snapshot_digest: "sha256:server".into(),
            },
            assembly: super::AppLockAssembly {
                enabled_features: vec!["feature-a".into()],
                selected_packages: vec!["agent-runtime".into()],
                scene: "team".into(),
                scene_digest: "sha256:scene".into(),
                binding_set_id: "binding-set:sha256:bindings".into(),
                closure_id: "closure:sha256:closure".into(),
                closure_digest: "sha256:closure".into(),
            },
            packages: vec![AppLockPackage {
                name: "agent-runtime".into(),
                version: "0.1.0".into(),
                source: "local-index".into(),
                sha512: "sha512:abc".into(),
                manifest_digest: "sha256:manifest".into(),
                artifact_digest: "sha256:artifact".into(),
                artifact_set_id: "artifact-set:sha256:artifact".into(),
                store_object_id: "store:sha512:artifact".into(),
                store_path: ".weft/store/sha512-abc-agent-runtime-0.1.0".into(),
                closure_id: "closure:sha256:closure".into(),
                runtime_provider: "agent-runtime".into(),
                provides: vec!["team.delegate".into()],
                reasons: vec![AppLockPackageReason {
                    layer: "scene".into(),
                    source: "team".into(),
                    message: "Scene pins team.delegate.".into(),
                }],
                ..Default::default()
            }],
            bindings: vec![super::AppLockBinding {
                capability: "team.delegate".into(),
                provider: "agent-runtime".into(),
                package: "agent-runtime".into(),
                mutable: false,
                package_version: "0.1.0".into(),
                package_sha512: "sha512:abc".into(),
                binding_source: "scene".into(),
            }],
            ..Default::default()
        };

        let content = toml::to_string_pretty(&lock).expect("lock should serialize");
        assert!(content.contains("scene_digest = \"sha256:scene\""));
        assert!(content.contains("[[packages.reasons]]"));

        let parsed: AppLockFile = toml::from_str(&content).expect("lock should deserialize");
        assert_eq!(parsed.scene, "team");
        assert_eq!(parsed.scene_digest, "sha256:scene");
        assert_eq!(parsed.binding_set_id, "binding-set:sha256:bindings");
        assert_eq!(parsed.closure_id, "closure:sha256:closure");
        assert_eq!(parsed.closure_digest, "sha256:closure");
        assert_eq!(parsed.store_generation_id, "store-gen:sha256:store");
        assert_eq!(parsed.trust_level, "verified");
        assert!(!parsed.dev_unsealed);
        assert_eq!(parsed.inputs.scene_digest, "sha256:scene");
        assert_eq!(parsed.assembly.scene_digest, "sha256:scene");
        assert_eq!(parsed.packages[0].reasons.len(), 1);
        assert_eq!(parsed.packages[0].reasons[0].layer, "scene");
    }
}
