use crate::app::state::ResolvedAppMap;
use crate::package::DiscoveredPackage;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CapabilityProviderRecord {
    pub provider: String,
    pub runtime: String,
    #[serde(default)]
    pub priority: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CapabilityBindingRecord {
    pub app: String,
    pub provider: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CapabilityRegistryEntry {
    pub capability: String,
    pub providers: Vec<CapabilityProviderRecord>,
    pub bindings: Vec<CapabilityBindingRecord>,
}

pub type CapabilityRegistry = HashMap<String, CapabilityRegistryEntry>;

pub fn build_capability_registry(
    packages: &[DiscoveredPackage],
    apps: &ResolvedAppMap,
) -> CapabilityRegistry {
    let mut registry: CapabilityRegistry = HashMap::new();

    for package in packages {
        for capability in package.manifest.resolved_provides() {
            let entry =
                registry
                    .entry(capability.clone())
                    .or_insert_with(|| CapabilityRegistryEntry {
                        capability: capability.clone(),
                        ..Default::default()
                    });

            entry.providers.push(CapabilityProviderRecord {
                provider: package.manifest.package_info.name.clone(),
                runtime: package.runtime.as_str().to_string(),
                priority: package
                    .manifest
                    .package
                    .as_ref()
                    .and_then(|package| package.priority)
                    .unwrap_or(0),
            });
        }
    }

    for (app_name, app) in apps {
        for binding in &app.bindings {
            let entry = registry
                .entry(binding.capability.clone())
                .or_insert_with(|| CapabilityRegistryEntry {
                    capability: binding.capability.clone(),
                    ..Default::default()
                });

            entry.bindings.push(CapabilityBindingRecord {
                app: app_name.clone(),
                provider: binding.provider.clone(),
            });
        }
    }

    for entry in registry.values_mut() {
        entry.providers.sort_by(|left, right| {
            right
                .priority
                .cmp(&left.priority)
                .then(left.provider.cmp(&right.provider))
        });
        entry.providers.dedup_by(|left, right| {
            left.provider == right.provider && left.runtime == right.runtime
        });

        entry.bindings.sort_by(|left, right| {
            left.app
                .cmp(&right.app)
                .then(left.provider.cmp(&right.provider))
        });
        entry
            .bindings
            .dedup_by(|left, right| left.app == right.app && left.provider == right.provider);
    }

    registry
}
