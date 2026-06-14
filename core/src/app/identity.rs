use crate::app::config::{
    AppLockBinding, AppLockEvidence, AppLockPackage, AppSceneBindingPin, AppSceneConfig,
    AppScenePackagePin,
};
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};

const SHA256_PREFIX: &str = "sha256:";
const BINDING_SET_PREFIX: &str = "binding-set:sha256:";
const CLOSURE_PREFIX: &str = "closure:sha256:";

pub fn canonical_json_string(value: &Value) -> String {
    let mut output = String::new();
    write_canonical_json(value, &mut output);
    output
}

pub fn canonical_sha256_digest(value: &Value) -> String {
    let digest = Sha256::digest(canonical_json_string(value).as_bytes());
    format!("{SHA256_PREFIX}{digest:x}")
}

pub fn scene_digest(scene: &AppSceneConfig) -> String {
    canonical_sha256_digest(&normalized_scene_value(scene))
}

pub fn binding_set_id_from_scene_binding_pins(binding_pins: &[AppSceneBindingPin]) -> String {
    prefixed_digest(
        BINDING_SET_PREFIX,
        &normalized_scene_binding_set_value(binding_pins),
    )
}

pub fn binding_set_id_from_lock_bindings(bindings: &[AppLockBinding]) -> String {
    prefixed_digest(
        BINDING_SET_PREFIX,
        &normalized_lock_binding_set_value(bindings),
    )
}

pub fn closure_digest_from_lock_packages(packages: &[AppLockPackage]) -> String {
    canonical_sha256_digest(&normalized_closure_value(packages))
}

pub fn closure_id_from_lock_packages(packages: &[AppLockPackage]) -> String {
    prefixed_digest(CLOSURE_PREFIX, &normalized_closure_value(packages))
}

fn prefixed_digest(prefix: &str, value: &Value) -> String {
    let digest = canonical_sha256_digest(value);
    let hex = digest.trim_start_matches(SHA256_PREFIX);
    format!("{prefix}{hex}")
}

fn normalized_scene_value(scene: &AppSceneConfig) -> Value {
    json!({
        "schema_version": scene.schema_version,
        "name": scene.name,
        "profile": scene.profile,
        "base_generation": scene.base_generation,
        "features": {
            "enabled": sorted_strings(&scene.enabled_features),
            "disabled": sorted_strings(&scene.disabled_features),
        },
        "bindings": sorted_values(
            scene.binding_pins.iter().map(normalized_scene_binding_pin_value),
        ),
        "packages": sorted_values(
            scene.package_pins.iter().map(normalized_scene_package_pin_value),
        ),
    })
}

fn normalized_scene_binding_set_value(binding_pins: &[AppSceneBindingPin]) -> Value {
    Value::Array(sorted_values(
        binding_pins.iter().map(normalized_scene_binding_pin_value),
    ))
}

fn normalized_lock_binding_set_value(bindings: &[AppLockBinding]) -> Value {
    Value::Array(sorted_values(
        bindings.iter().map(normalized_lock_binding_value),
    ))
}

fn normalized_closure_value(packages: &[AppLockPackage]) -> Value {
    Value::Array(sorted_values(
        packages.iter().map(normalized_lock_package_value),
    ))
}

fn normalized_scene_binding_pin_value(binding_pin: &AppSceneBindingPin) -> Value {
    json!({
        "capability": binding_pin.capability,
        "package": binding_pin.package,
        "provider": binding_pin.provider,
        "version": binding_pin.version,
        "sha512": binding_pin.sha512,
        "source": binding_pin.source,
    })
}

fn normalized_scene_package_pin_value(package_pin: &AppScenePackagePin) -> Value {
    json!({
        "package": package_pin.package,
        "version": package_pin.version,
        "sha512": package_pin.sha512,
        "source": package_pin.source,
    })
}

fn normalized_lock_binding_value(binding: &AppLockBinding) -> Value {
    json!({
        "capability": binding.capability,
        "provider": binding.provider,
        "package": binding.package,
        "mutable": binding.mutable,
        "package_version": binding.package_version,
        "package_sha512": binding.package_sha512,
        "binding_source": binding.binding_source,
    })
}

fn normalized_lock_package_value(package: &AppLockPackage) -> Value {
    json!({
        "name": package.name,
        "version": package.version,
        "runtime": package.runtime,
        "sha512": package.sha512,
        "source": package.source,
        "trusted": package.trusted,
        "signature": package.signature,
        "source_authority": package.source_authority,
        "source_public_keys": sorted_strings(&package.source_public_keys),
        "package_kind": package.package_kind,
        "manifest_digest": package.manifest_digest,
        "artifact_digest": package.artifact_digest,
        "artifact_set_id": package.artifact_set_id,
        "store_object_id": package.store_object_id,
        "entry_kind": package.entry_kind,
        "runtime_provider": package.runtime_provider,
        "provides": sorted_strings(&package.provides),
        "requires": sorted_strings(&package.requires),
        "roles": sorted_strings(&package.roles),
        "capabilities": sorted_strings(&package.capabilities),
        "features": sorted_strings(&package.features),
        "default_enabled_features": sorted_strings(&package.default_enabled_features),
        "evidence": normalized_lock_evidence_value(&package.evidence),
    })
}

fn normalized_lock_evidence_value(evidence: &AppLockEvidence) -> Value {
    json!({
        "digest": evidence.digest,
        "signature": evidence.signature,
        "source_authority": evidence.source_authority,
        "source_public_keys": sorted_strings(&evidence.source_public_keys),
    })
}

fn sorted_strings(values: &[String]) -> Vec<String> {
    let mut sorted = values.to_vec();
    sorted.sort();
    sorted
}

fn sorted_values(values: impl IntoIterator<Item = Value>) -> Vec<Value> {
    let mut sorted = values
        .into_iter()
        .map(|value| {
            let key = canonical_json_string(&value);
            (key, value)
        })
        .collect::<Vec<_>>();
    sorted.sort_by(|left, right| left.0.cmp(&right.0));
    sorted.into_iter().map(|(_, value)| value).collect()
}

fn write_canonical_json(value: &Value, output: &mut String) {
    match value {
        Value::Null => output.push_str("null"),
        Value::Bool(boolean) => output.push_str(if *boolean { "true" } else { "false" }),
        Value::Number(number) => output.push_str(&number.to_string()),
        Value::String(string) => {
            let encoded = serde_json::to_string(string).expect("string serialization should work");
            output.push_str(&encoded);
        }
        Value::Array(array) => {
            output.push('[');
            for (index, item) in array.iter().enumerate() {
                if index > 0 {
                    output.push(',');
                }
                write_canonical_json(item, output);
            }
            output.push(']');
        }
        Value::Object(object) => write_canonical_object(object, output),
    }
}

fn write_canonical_object(object: &Map<String, Value>, output: &mut String) {
    output.push('{');

    let mut entries = object.iter().collect::<Vec<_>>();
    entries.sort_by(|left, right| left.0.cmp(right.0));

    for (index, (key, value)) in entries.into_iter().enumerate() {
        if index > 0 {
            output.push(',');
        }

        let encoded_key = serde_json::to_string(key).expect("key serialization should work");
        output.push_str(&encoded_key);
        output.push(':');
        write_canonical_json(value, output);
    }

    output.push('}');
}

#[cfg(test)]
mod tests {
    use super::{
        binding_set_id_from_lock_bindings, binding_set_id_from_scene_binding_pins,
        canonical_json_string, closure_id_from_lock_packages, scene_digest,
    };
    use crate::app::config::{
        AppLockBinding, AppLockEvidence, AppLockPackage, AppLockPackageReason, AppSceneBindingPin,
        AppSceneConfig, AppSceneMetadata, AppScenePackagePin,
    };
    use serde_json::json;

    #[test]
    fn canonical_json_string_sorts_object_keys() {
        let value = json!({
            "z": 1,
            "a": {
                "d": true,
                "b": ["x", "y"],
            },
        });

        assert_eq!(
            canonical_json_string(&value),
            r#"{"a":{"b":["x","y"],"d":true},"z":1}"#
        );
    }

    #[test]
    fn scene_digest_is_order_insensitive_for_scene_sets() {
        let mut first = sample_scene();
        let mut second = sample_scene();

        second.enabled_features.reverse();
        second.disabled_features.reverse();
        second.binding_pins.reverse();
        second.package_pins.reverse();
        second.description = "Updated display text only".into();
        second.metadata = AppSceneMetadata {
            created_by: "gui".into(),
            created_at: Some(200),
            updated_at: Some(300),
        };

        assert_eq!(scene_digest(&first), scene_digest(&second));

        first.base_generation = Some(99);
        assert_ne!(scene_digest(&first), scene_digest(&second));
    }

    #[test]
    fn binding_set_id_ignores_reason_and_order_for_scene_pins() {
        let first = vec![
            AppSceneBindingPin {
                capability: "team.delegate".into(),
                package: "agent-runtime".into(),
                provider: "agent-runtime".into(),
                version: "0.1.0".into(),
                sha512: "sha512:aaa".into(),
                source: "local-index".into(),
                reason: "CLI explanation".into(),
            },
            AppSceneBindingPin {
                capability: "memory.store".into(),
                package: "memory-store".into(),
                provider: "memory-store".into(),
                version: "0.2.0".into(),
                sha512: "sha512:bbb".into(),
                source: "local-index".into(),
                reason: "First reason".into(),
            },
        ];

        let second = vec![
            AppSceneBindingPin {
                reason: "A different display-only reason".into(),
                ..first[1].clone()
            },
            AppSceneBindingPin {
                reason: "Another explanation".into(),
                ..first[0].clone()
            },
        ];

        assert_eq!(
            binding_set_id_from_scene_binding_pins(&first),
            binding_set_id_from_scene_binding_pins(&second)
        );
    }

    #[test]
    fn lock_binding_set_id_is_order_insensitive() {
        let first = vec![
            AppLockBinding {
                capability: "team.delegate".into(),
                provider: "agent-runtime".into(),
                package: "agent-runtime".into(),
                mutable: false,
                package_version: "0.1.0".into(),
                package_sha512: "sha512:aaa".into(),
                binding_source: "scene".into(),
            },
            AppLockBinding {
                capability: "memory.store".into(),
                provider: "memory-store".into(),
                package: "memory-store".into(),
                mutable: false,
                package_version: "0.2.0".into(),
                package_sha512: "sha512:bbb".into(),
                binding_source: "default".into(),
            },
        ];
        let second = vec![first[1].clone(), first[0].clone()];

        assert_eq!(
            binding_set_id_from_lock_bindings(&first),
            binding_set_id_from_lock_bindings(&second)
        );
    }

    #[test]
    fn lock_binding_set_id_changes_with_binding_source_layer() {
        let first = vec![AppLockBinding {
            capability: "team.delegate".into(),
            provider: "agent-runtime".into(),
            package: "agent-runtime".into(),
            mutable: false,
            package_version: "0.1.0".into(),
            package_sha512: "sha512:aaa".into(),
            binding_source: "default".into(),
        }];
        let second = vec![AppLockBinding {
            binding_source: "scene".into(),
            ..first[0].clone()
        }];

        assert_ne!(
            binding_set_id_from_lock_bindings(&first),
            binding_set_id_from_lock_bindings(&second)
        );
    }

    #[test]
    fn closure_id_is_order_insensitive_but_changes_with_package_identity() {
        let first = vec![sample_lock_package("agent-runtime", "0.1.0", "sha512:aaa")];
        let mut reordered = vec![sample_lock_package("agent-runtime", "0.1.0", "sha512:aaa")];
        reordered[0].provides = vec!["cap.b".into(), "cap.a".into()];
        reordered[0].requires = vec!["dep.b".into(), "dep.a".into()];
        reordered[0].features = vec!["feature-b".into(), "feature-a".into()];
        reordered[0].source_public_keys = vec!["key-b".into(), "key-a".into()];
        reordered[0].evidence.source_public_keys = vec!["proof-b".into(), "proof-a".into()];
        reordered[0].reasons = vec![AppLockPackageReason {
            layer: "scene".into(),
            source: "team".into(),
            message: "Different explanation".into(),
        }];

        assert_eq!(
            closure_id_from_lock_packages(&first),
            closure_id_from_lock_packages(&reordered)
        );

        let mut changed_identity = reordered.clone();
        changed_identity[0].artifact_digest = "sha256:new-artifact".into();

        assert_ne!(
            closure_id_from_lock_packages(&reordered),
            closure_id_from_lock_packages(&changed_identity)
        );
    }

    fn sample_scene() -> AppSceneConfig {
        AppSceneConfig {
            schema_version: 1,
            name: "team".into(),
            description: "Human-friendly text".into(),
            profile: "developer".into(),
            base_generation: Some(18),
            enabled_features: vec!["feature-b".into(), "feature-a".into()],
            disabled_features: vec!["feature-d".into(), "feature-c".into()],
            binding_pins: vec![
                AppSceneBindingPin {
                    capability: "memory.store".into(),
                    package: "memory-store".into(),
                    provider: "memory-store".into(),
                    version: "0.2.0".into(),
                    sha512: "sha512:bbb".into(),
                    source: "local-index".into(),
                    reason: "Display-only reason".into(),
                },
                AppSceneBindingPin {
                    capability: "team.delegate".into(),
                    package: "agent-runtime".into(),
                    provider: "agent-runtime".into(),
                    version: "0.1.0".into(),
                    sha512: "sha512:aaa".into(),
                    source: "local-index".into(),
                    reason: "Another note".into(),
                },
            ],
            package_pins: vec![
                AppScenePackagePin {
                    package: "agent-runtime".into(),
                    version: "0.1.0".into(),
                    sha512: "sha512:aaa".into(),
                    source: "local-index".into(),
                    reason: "Display-only package note".into(),
                },
                AppScenePackagePin {
                    package: "memory-store".into(),
                    version: "0.2.0".into(),
                    sha512: "sha512:bbb".into(),
                    source: "local-index".into(),
                    reason: "Another display note".into(),
                },
            ],
            metadata: AppSceneMetadata {
                created_by: "cli".into(),
                created_at: Some(100),
                updated_at: Some(150),
            },
        }
    }

    fn sample_lock_package(name: &str, version: &str, sha512: &str) -> AppLockPackage {
        AppLockPackage {
            name: name.into(),
            version: version.into(),
            runtime: format!("{name}-runtime"),
            sha512: sha512.into(),
            source: "local-index".into(),
            trusted: true,
            signature: "sig-1".into(),
            source_authority: "weft".into(),
            source_public_keys: vec!["key-a".into(), "key-b".into()],
            package_kind: "runtime".into(),
            manifest_digest: format!("sha256:{name}-manifest"),
            artifact_digest: format!("sha256:{name}-artifact"),
            artifact_set_id: format!("artifact-set:sha256:{name}-artifact"),
            store_object_id: format!("store:{sha512}"),
            store_path: format!(".weft/store/{name}-{version}"),
            closure_id: "closure:sha256:placeholder".into(),
            entry_kind: "direct".into(),
            runtime_provider: name.into(),
            provides: vec!["cap.a".into(), "cap.b".into()],
            requires: vec!["dep.a".into(), "dep.b".into()],
            roles: vec!["role-a".into(), "role-b".into()],
            capabilities: vec!["capability-a".into(), "capability-b".into()],
            features: vec!["feature-a".into(), "feature-b".into()],
            default_enabled_features: vec!["feature-a".into(), "feature-b".into()],
            evidence: AppLockEvidence {
                digest: format!("sha256:{name}-evidence"),
                signature: "sig-evidence".into(),
                source_authority: "weft".into(),
                source_public_keys: vec!["proof-a".into(), "proof-b".into()],
            },
            reasons: vec![AppLockPackageReason {
                layer: "scene".into(),
                source: "team".into(),
                message: "Initial explanation".into(),
            }],
        }
    }
}
