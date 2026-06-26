use weft_core::runtime::run_server;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    run_server().await
}


#[cfg(test)]
mod tests {
    use weft_core::runtime::{build_generation_store_map, discover_product_roots};
    use weft_core::app::{
        inspect_startup_generation_store, save_generation_index, write_active_generation_pointer,
        write_previous_generation_pointer, AppGenerationIndex, GenerationStatus, ResolvedApp,
        ResolvedAppMap, ResolvedAppSources,
    };

    fn temp_root(name: &str) -> std::path::PathBuf {
        let unique = format!(
            "weft-main-{}-{}",
            name,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system time after epoch")
                .as_nanos()
        );
        std::env::temp_dir().join(unique)
    }

    #[test]
    fn discover_product_roots_returns_package_roots_only() {
        let root = temp_root("discover-roots");
        let package_dir = root.join("packages").join("weft-claw");
        let secondary_dir = root.join("packages").join("secondary-product");
        let legacy_app_dir = root.join("apps").join("legacy-product");
        let instance_dir = root.join(".weft").join("weft-claw");
        std::fs::create_dir_all(&package_dir).expect("package dir created");
        std::fs::create_dir_all(&secondary_dir).expect("secondary dir created");
        std::fs::create_dir_all(&legacy_app_dir).expect("legacy app dir created");
        std::fs::create_dir_all(&instance_dir).expect("instance dir created");
        std::fs::write(
            package_dir.join("package.toml"),
            "[identity]\nname='weft-claw'\n",
        )
        .expect("package declaration written");
        std::fs::write(
            secondary_dir.join("package.toml"),
            "[identity]\nname='secondary-product'\n",
        )
        .expect("secondary package declaration written");
        std::fs::write(
            legacy_app_dir.join("app.toml"),
            "[app]\nname='legacy-product'\n",
        )
        .expect("legacy app declaration written");
        std::fs::write(instance_dir.join("config.toml"), "schema_version = 2\n")
            .expect("instance config written");

        let roots = discover_product_roots(&root);

        assert_eq!(roots.len(), 1);
        assert_eq!(roots[0], package_dir);
        assert!(!roots.iter().any(|path| path == &legacy_app_dir));
        assert!(!roots.iter().any(|path| path == &secondary_dir));

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn generation_store_restores_from_explicit_instance_lock_path() {
        let root = temp_root("generation-store");
        let instance_dir = root.join(".weft").join("weft-claw");
        std::fs::create_dir_all(&instance_dir).expect("instance dir created");
        std::fs::write(
            instance_dir.join("lock.toml"),
            "lock_version = 2\napp='weft-claw'\ngeneration=7\nstatus='active'\nprofile='developer'\n[assembly]\nenabled_features=[]\nselected_packages=[]\n[[bindings]]\ncapability='core.execution'\nprovider='core'\nmutable=false\n",
        )
        .expect("instance lock written");

        let mut resolved_apps = ResolvedAppMap::new();
        resolved_apps.insert(
            "weft-claw".into(),
            ResolvedApp {
                name: "weft-claw".into(),
                version: "0.1.0".into(),
                capabilities: vec!["core.execution".into()],
                bindings: vec![weft_core::app::AppBindingResolution {
                    capability: "core.execution".into(),
                    provider: "core".into(),
                    mutable: false,
                    source: "lock".into(),
                }],
                sources: ResolvedAppSources {
                    manifest_path: String::new(),
                    config_path: None,
                    lock_path: Some(instance_dir.join("lock.toml").display().to_string()),
                },
                ..ResolvedApp::default()
            },
        );

        let store = build_generation_store_map(&resolved_apps);
        let generation = store
            .get("weft-claw")
            .and_then(|entry| entry.active.as_ref())
            .expect("active generation restored from explicit lock path");
        assert_eq!(generation.id, 7);
        assert_eq!(generation.bindings[0].capability, "core.execution");
        assert_eq!(generation.bindings[0].provider, "core");

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn generation_store_prefers_lock_bindings_over_resolved_app_bindings() {
        let root = temp_root("generation-store-lock-bindings");
        let instance_dir = root.join(".weft").join("weft-claw");
        std::fs::create_dir_all(&instance_dir).expect("instance dir created");
        std::fs::write(
            instance_dir.join("lock.toml"),
            "lock_version = 2\napp='weft-claw'\ngeneration=7\nstatus='active'\nprofile='developer'\n[assembly]\nenabled_features=[]\nselected_packages=[]\n[[bindings]]\ncapability='weft_claw.turn'\nprovider='weft-claw'\nmutable=false\nbinding_source='declaration-default'\n[[bindings]]\ncapability='core.execution'\nprovider='core'\nmutable=false\n",
        )
        .expect("instance lock written");

        let mut resolved_apps = ResolvedAppMap::new();
        resolved_apps.insert(
            "weft-claw".into(),
            ResolvedApp {
                name: "weft-claw".into(),
                version: "0.1.0".into(),
                capabilities: vec!["weft_claw.turn".into(), "core.execution".into()],
                bindings: vec![weft_core::app::AppBindingResolution {
                    capability: "core.execution".into(),
                    provider: "core".into(),
                    mutable: false,
                    source: "declaration-default".into(),
                }],
                sources: ResolvedAppSources {
                    manifest_path: String::new(),
                    config_path: None,
                    lock_path: Some(instance_dir.join("lock.toml").display().to_string()),
                },
                ..ResolvedApp::default()
            },
        );

        let store = build_generation_store_map(&resolved_apps);
        let generation = store
            .get("weft-claw")
            .and_then(|entry| entry.active.as_ref())
            .expect("active generation restored from explicit lock path");
        assert!(generation
            .bindings
            .iter()
            .any(|binding| binding.capability == "weft_claw.turn"));
        assert_eq!(generation.bindings.len(), 2);

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn generation_store_restores_runtime_only_instance_lock_using_resolved_app_bindings() {
        let root = temp_root("generation-store-runtime-only");
        let instance_dir = root.join(".weft").join("weft-claw");
        std::fs::create_dir_all(&instance_dir).expect("instance dir created");
        std::fs::write(
            instance_dir.join("lock.toml"),
            "lock_version = 2\napp='weft-claw'\ngeneration=10\nstatus='active'\nprofile='developer'\n[assembly]\nenabled_features=[]\nselected_packages=['agent-runtime']\n",
        )
        .expect("runtime-only instance lock written");

        let mut resolved_apps = ResolvedAppMap::new();
        resolved_apps.insert(
            "weft-claw".into(),
            ResolvedApp {
                name: "weft-claw".into(),
                version: "0.1.0".into(),
                capabilities: vec!["agent.runtime".into(), "core.execution".into()],
                bindings: vec![
                    weft_core::app::AppBindingResolution {
                        capability: "agent.runtime".into(),
                        provider: "agent-runtime".into(),
                        mutable: false,
                        source: "declaration-default".into(),
                    },
                    weft_core::app::AppBindingResolution {
                        capability: "core.execution".into(),
                        provider: "core".into(),
                        mutable: false,
                        source: "declaration-default".into(),
                    },
                ],
                sources: ResolvedAppSources {
                    manifest_path: String::new(),
                    config_path: None,
                    lock_path: Some(instance_dir.join("lock.toml").display().to_string()),
                },
                ..ResolvedApp::default()
            },
        );

        let store = build_generation_store_map(&resolved_apps);
        let generation = store
            .get("weft-claw")
            .and_then(|entry| entry.active.as_ref())
            .expect("active generation restored from runtime-only instance lock");
        assert_eq!(generation.id, 10);
        assert_eq!(generation.version, "0.1.0");
        assert_eq!(generation.bindings.len(), 2);
        assert_eq!(generation.capabilities.len(), 2);
        assert!(generation.enabled_features.is_empty());
        assert_eq!(generation.profile, "developer");

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn startup_generation_store_ignores_missing_pointer_and_index_files() {
        let root = temp_root("startup-generation-missing-index");
        let instance_dir = root.join(".weft").join("weft-claw");
        std::fs::create_dir_all(&instance_dir).expect("instance dir created");
        std::fs::write(
            instance_dir.join("lock.toml"),
            "lock_version = 2\napp='weft-claw'\ngeneration=7\nstatus='active'\nprofile='developer'\nscene='team'\nbinding_set_id='binding-set:sha256:7'\nclosure_id='closure:sha256:7'\n[assembly]\nenabled_features=[]\nselected_packages=[]\n[[bindings]]\ncapability='core.execution'\nprovider='core'\nmutable=false\n",
        )
        .expect("instance lock written");

        let mut resolved_apps = ResolvedAppMap::new();
        resolved_apps.insert(
            "weft-claw".into(),
            ResolvedApp {
                name: "weft-claw".into(),
                version: "0.1.0".into(),
                capabilities: vec!["core.execution".into()],
                bindings: vec![weft_core::app::AppBindingResolution {
                    capability: "core.execution".into(),
                    provider: "core".into(),
                    mutable: false,
                    source: "lock".into(),
                }],
                sources: ResolvedAppSources {
                    manifest_path: String::new(),
                    config_path: None,
                    lock_path: Some(instance_dir.join("lock.toml").display().to_string()),
                },
                ..ResolvedApp::default()
            },
        );

        let store = build_generation_store_map(&resolved_apps);
        let app_store = store.get("weft-claw").expect("store entry present");
        let diagnostics = inspect_startup_generation_store(&instance_dir, app_store);

        assert_eq!(app_store.active.as_ref().expect("active generation").id, 7);
        assert_eq!(app_store.next_id, 8);
        assert!(diagnostics.is_clean());
        assert!(diagnostics.diagnostics.is_empty());

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn startup_generation_store_reports_pointer_and_index_mismatch_without_changing_store() {
        let root = temp_root("startup-generation-mismatch");
        let instance_dir = root.join(".weft").join("weft-claw");
        std::fs::create_dir_all(&instance_dir).expect("instance dir created");
        std::fs::write(
            instance_dir.join("lock.toml"),
            "lock_version = 2\napp='weft-claw'\ngeneration=7\nstatus='active'\nprofile='developer'\nscene='team'\nbinding_set_id='binding-set:sha256:7'\nclosure_id='closure:sha256:7'\n[assembly]\nenabled_features=[]\nselected_packages=[]\n[[bindings]]\ncapability='core.execution'\nprovider='core'\nmutable=false\n",
        )
        .expect("instance lock written");
        write_active_generation_pointer(&instance_dir, Some(9)).expect("active pointer written");
        write_previous_generation_pointer(&instance_dir, Some(6))
            .expect("previous pointer written");
        save_generation_index(
            &instance_dir,
            &AppGenerationIndex {
                schema_version: weft_core::app::GENERATION_INDEX_SCHEMA_VERSION,
                active: Some(9),
                previous: Some(6),
                candidate: None,
                next_id: 10,
                generations: vec![
                    weft_core::app::AppGeneration {
                        id: 6,
                        app_name: "weft-claw".into(),
                        version: "0.1.0".into(),
                        bindings: vec![],
                        capabilities: vec!["core.execution".into()],
                        enabled_features: vec![],
                        scene: "team".into(),
                        profile: "developer".into(),
                        binding_set_id: "binding-set:sha256:6".into(),
                        closure_id: "closure:sha256:6".into(),
                        lock_digest: "sha256:lock-6".into(),
                        lock_path: "generations/6.lock.toml".into(),
                        parent_generation: Some(5),
                        created_by: "cli".into(),
                        status: GenerationStatus::Rollback,
                        validation_results: vec![],
                        created_at: 6,
                    },
                    weft_core::app::AppGeneration {
                        id: 9,
                        app_name: "weft-claw".into(),
                        version: "0.1.0".into(),
                        bindings: vec![],
                        capabilities: vec!["core.execution".into()],
                        enabled_features: vec![],
                        scene: "stale".into(),
                        profile: "developer".into(),
                        binding_set_id: "binding-set:sha256:9".into(),
                        closure_id: "closure:sha256:9".into(),
                        lock_digest: "sha256:lock-9".into(),
                        lock_path: "generations/9.lock.toml".into(),
                        parent_generation: Some(6),
                        created_by: "cli".into(),
                        status: GenerationStatus::Active,
                        validation_results: vec![],
                        created_at: 9,
                    },
                ],
            },
        )
        .expect("index saved");

        let mut resolved_apps = ResolvedAppMap::new();
        resolved_apps.insert(
            "weft-claw".into(),
            ResolvedApp {
                name: "weft-claw".into(),
                version: "0.1.0".into(),
                capabilities: vec!["core.execution".into()],
                bindings: vec![weft_core::app::AppBindingResolution {
                    capability: "core.execution".into(),
                    provider: "core".into(),
                    mutable: false,
                    source: "lock".into(),
                }],
                sources: ResolvedAppSources {
                    manifest_path: String::new(),
                    config_path: None,
                    lock_path: Some(instance_dir.join("lock.toml").display().to_string()),
                },
                ..ResolvedApp::default()
            },
        );

        let store = build_generation_store_map(&resolved_apps);
        let app_store = store.get("weft-claw").expect("store entry present");
        let diagnostics = inspect_startup_generation_store(&instance_dir, app_store);

        assert_eq!(app_store.active.as_ref().expect("active generation").id, 7);
        assert_eq!(
            app_store.active.as_ref().expect("active generation").scene,
            "team"
        );
        assert!(diagnostics
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "startup_active_pointer_mismatch"));
        assert!(diagnostics
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "startup_previous_pointer_mismatch"));
        assert!(diagnostics
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "startup_generation_index_active_mismatch"));

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn startup_generation_store_accepts_matching_pointer_and_index_diagnostics() {
        let root = temp_root("startup-generation-clean");
        let instance_dir = root.join(".weft").join("weft-claw");
        std::fs::create_dir_all(&instance_dir).expect("instance dir created");
        std::fs::write(
            instance_dir.join("lock.toml"),
            "lock_version = 2\napp='weft-claw'\ngeneration=7\nstatus='active'\nprofile='developer'\nscene='team'\nbinding_set_id='binding-set:sha256:7'\nclosure_id='closure:sha256:7'\n[assembly]\nenabled_features=[]\nselected_packages=[]\n[[bindings]]\ncapability='core.execution'\nprovider='core'\nmutable=false\n",
        )
        .expect("instance lock written");
        write_active_generation_pointer(&instance_dir, Some(7)).expect("active pointer written");
        save_generation_index(
            &instance_dir,
            &AppGenerationIndex {
                schema_version: weft_core::app::GENERATION_INDEX_SCHEMA_VERSION,
                active: Some(7),
                previous: None,
                candidate: None,
                next_id: 8,
                generations: vec![weft_core::app::AppGeneration {
                    id: 7,
                    app_name: "weft-claw".into(),
                    version: "0.1.0".into(),
                    bindings: vec![weft_core::app::AppBindingResolution {
                        capability: "core.execution".into(),
                        provider: "core".into(),
                        mutable: false,
                        source: "lock".into(),
                    }],
                    capabilities: vec!["core.execution".into()],
                    enabled_features: vec![],
                    scene: "team".into(),
                    profile: "developer".into(),
                    binding_set_id: "binding-set:sha256:7".into(),
                    closure_id: "closure:sha256:7".into(),
                    lock_digest: String::new(),
                    lock_path: instance_dir.join("lock.toml").display().to_string(),
                    parent_generation: None,
                    created_by: String::new(),
                    status: GenerationStatus::Active,
                    validation_results: vec![],
                    created_at: 0,
                }],
            },
        )
        .expect("index saved");

        let mut resolved_apps = ResolvedAppMap::new();
        resolved_apps.insert(
            "weft-claw".into(),
            ResolvedApp {
                name: "weft-claw".into(),
                version: "0.1.0".into(),
                capabilities: vec!["core.execution".into()],
                bindings: vec![weft_core::app::AppBindingResolution {
                    capability: "core.execution".into(),
                    provider: "core".into(),
                    mutable: false,
                    source: "lock".into(),
                }],
                sources: ResolvedAppSources {
                    manifest_path: String::new(),
                    config_path: None,
                    lock_path: Some(instance_dir.join("lock.toml").display().to_string()),
                },
                ..ResolvedApp::default()
            },
        );

        let store = build_generation_store_map(&resolved_apps);
        let app_store = store.get("weft-claw").expect("store entry present");
        let diagnostics = inspect_startup_generation_store(&instance_dir, app_store);

        assert_eq!(app_store.active.as_ref().expect("active generation").id, 7);
        assert!(diagnostics.is_clean());
        assert!(diagnostics.diagnostics.is_empty());

        let _ = std::fs::remove_dir_all(root);
    }
}
