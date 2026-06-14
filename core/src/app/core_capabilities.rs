use crate::app::{CapabilityProviderRecord, CapabilityRegistry, CapabilityRegistryEntry};

pub fn merge_core_capabilities(registry: &mut CapabilityRegistry) {
    register_core_capability(registry, "core.files");
    register_core_capability(registry, "core.execution");
}

fn register_core_capability(registry: &mut CapabilityRegistry, capability: &str) {
    let entry = registry
        .entry(capability.to_string())
        .or_insert_with(|| CapabilityRegistryEntry {
            capability: capability.to_string(),
            ..Default::default()
        });

    if !entry
        .providers
        .iter()
        .any(|provider| provider.provider == "core" && provider.runtime == "core")
    {
        entry.providers.push(CapabilityProviderRecord {
            provider: "core".into(),
            runtime: "core".into(),
            priority: 0,
        });
    }

    entry.providers.sort_by(|left, right| {
        left.provider
            .cmp(&right.provider)
            .then(left.runtime.cmp(&right.runtime))
    });
}

#[cfg(test)]
mod tests {
    use super::merge_core_capabilities;
    use crate::app::resolve::resolve_app_manifest_with_policy;
    use crate::app::{build_capability_registry, AppManifest, ResolvedAppStatus};

    #[test]
    fn core_capabilities_are_added_to_registry() {
        let mut registry = build_capability_registry(&[], &Default::default());
        merge_core_capabilities(&mut registry);

        let execution = registry
            .get("core.execution")
            .expect("core.execution should exist");
        assert!(execution
            .providers
            .iter()
            .any(|provider| provider.provider == "core" && provider.runtime == "core"));
    }

    #[test]
    fn resolve_app_accepts_core_execution_binding() {
        let manifest: AppManifest = toml::from_str(
            r#"
[app]
name = "weft-claw"
version = "0.1.0"
display_name = "Weft Claw"
description = "test"

[requires]
capabilities = ["core.execution"]

[bindings.core.execution]
provider = "core"
mutable = false
"#,
        )
        .expect("manifest parses");

        let mut registry = build_capability_registry(&[], &Default::default());
        merge_core_capabilities(&mut registry);

        let resolved =
            resolve_app_manifest_with_policy(&manifest, &[], None, None, Some(&registry), None)
                .expect("core execution binding should resolve");

        assert_eq!(resolved.status, ResolvedAppStatus::Resolved);
        assert_eq!(resolved.bindings.len(), 1);
        assert_eq!(resolved.bindings[0].capability, "core.execution");
        assert_eq!(resolved.bindings[0].provider, "core");
    }
}
