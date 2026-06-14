use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum ResolvedAppStatus {
    #[default]
    Resolved,
    Unresolved,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ResolvedAppSources {
    pub manifest_path: String,
    pub config_path: Option<String>,
    pub lock_path: Option<String>,
}

pub type ResolvedInstanceSources = ResolvedAppSources;

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct AppBindingResolution {
    pub capability: String,
    pub provider: String,
    pub mutable: bool,
    #[serde(default)]
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ResolvedApp {
    pub name: String,
    pub version: String,
    pub display_name: String,
    pub description: String,
    pub capabilities: Vec<String>,
    #[serde(default)]
    pub enabled_features: Vec<String>,
    pub bindings: Vec<AppBindingResolution>,
    pub validation_checks: Vec<String>,
    pub config_path: Option<String>,
    pub status: ResolvedAppStatus,
    pub errors: Vec<String>,
    pub sources: ResolvedAppSources,
}

pub type ResolvedInstance = ResolvedApp;

pub type ResolvedAppMap = HashMap<String, ResolvedApp>;
pub type ResolvedInstanceMap = HashMap<String, ResolvedInstance>;
