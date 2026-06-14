use super::{validate_config, AppConfig};
use anyhow::{Context, Result};
use std::path::Path;

pub fn load_config(path: &Path) -> Result<AppConfig> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read config: {}", path.display()))?;
    let config: AppConfig =
        toml::from_str(&content).with_context(|| "Failed to parse config TOML")?;
    validate_config(&config).with_context(|| "Invalid config")?;
    Ok(config)
}

pub fn load_config_or_default(path: &Path) -> AppConfig {
    match load_config(path) {
        Ok(cfg) => cfg,
        Err(e) => {
            tracing::warn!("Config load failed ({:#}), using defaults", e);
            default_config()
        }
    }
}

pub fn save_config(path: &Path, config: &AppConfig) -> Result<()> {
    let content = toml::to_string_pretty(config).context("Failed to serialize config")?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, content)
        .with_context(|| format!("Failed to write config: {}", path.display()))?;
    Ok(())
}

fn default_config() -> AppConfig {
    AppConfig {
        core: Default::default(),
        providers: vec![],
        routing: Default::default(),
        key_strategy: Default::default(),
        fallback: Default::default(),
        virtual_keys: vec![],
        services: vec![],
        packages: vec![],
        registry: Default::default(),
        package_aliases: Default::default(),
        web_search: Default::default(),
        team: Default::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_load_minimal_config() {
        let mut f = NamedTempFile::new().unwrap();
        write!(
            f,
            r#"
[core]
host = "0.0.0.0"
port = 8080
"#
        )
        .unwrap();
        let cfg = load_config(f.path()).unwrap();
        assert_eq!(cfg.core.host, "0.0.0.0");
        assert_eq!(cfg.core.port, 8080);
        assert_eq!(cfg.core.log_level, "info");
    }

    #[test]
    fn test_load_with_providers() {
        let mut f = NamedTempFile::new().unwrap();
        write!(
            f,
            r#"
[core]
host = "127.0.0.1"
port = 3000

[[providers]]
name = "openrouter"
base_url = "https://openrouter.ai/api/v1"
models = ["claude-sonnet-4"]

[[providers.keys]]
value = "sk-test-123"
label = "test key"
"#
        )
        .unwrap();
        let cfg = load_config(f.path()).unwrap();
        assert_eq!(cfg.providers.len(), 1);
        assert_eq!(cfg.providers[0].name, "openrouter");
        assert_eq!(cfg.providers[0].keys.len(), 1);
        assert_eq!(cfg.providers[0].keys[0].value, "sk-test-123");
    }

    #[test]
    fn test_roundtrip() {
        let mut f = NamedTempFile::new().unwrap();
        write!(
            f,
            r#"
[core]
host = "127.0.0.1"
port = 3000
log_level = "debug"
data_dir = "./data"

[routing]
default_provider = "openrouter"

[key_strategy]
mode = "round_robin"
"#
        )
        .unwrap();
        let cfg = load_config(f.path()).unwrap();
        let tmp = NamedTempFile::new().unwrap();
        save_config(tmp.path(), &cfg).unwrap();
        let cfg2 = load_config(tmp.path()).unwrap();
        assert_eq!(cfg2.core.port, 3000);
        assert_eq!(cfg2.key_strategy.mode, "round_robin");
    }

    #[test]
    fn test_load_registry_source_urls() {
        let mut f = NamedTempFile::new().unwrap();
        write!(
            f,
            r#"
[core]
host = "127.0.0.1"
port = 3000

[registry]
gitea_url = "https://gitea.alhz.org"
package_source_url = "http://127.0.0.1:4011/packages"
app_source_url = "http://127.0.0.1:4012/apps"
"#
        )
        .unwrap();
        let cfg = load_config(f.path()).unwrap();
        assert_eq!(
            cfg.registry.package_source_url.as_deref(),
            Some("http://127.0.0.1:4011/packages")
        );
        assert_eq!(
            cfg.registry.app_source_url.as_deref(),
            Some("http://127.0.0.1:4012/apps")
        );
    }

    #[test]
    fn test_legacy_provider_config_defaults_to_chat_completions_api() {
        let mut f = NamedTempFile::new().unwrap();
        write!(
            f,
            r#"
[core]
host = "127.0.0.1"
port = 3000

[[providers]]
name = "legacy-openai"
base_url = "https://api.openai.com/v1"
format = "openai"
models = ["gpt-3.5-turbo"]
"#
        )
        .unwrap();

        let cfg = load_config(f.path()).unwrap();
        assert_eq!(
            cfg.providers[0].api,
            crate::config::ProviderApi::ChatCompletions
        );
    }

    #[test]
    fn test_gpt_5_with_default_api_fails_config_load_with_responses_message() {
        let mut f = NamedTempFile::new().unwrap();
        write!(
            f,
            r#"
[core]
host = "127.0.0.1"
port = 3000

[[providers]]
name = "openai"
base_url = "https://api.openai.com/v1"
format = "openai"
models = ["gpt-5"]
"#
        )
        .unwrap();

        let err = load_config(f.path()).unwrap_err();
        let message = format!("{err:#}");
        assert!(message.contains("Invalid config"));
        assert!(message.contains("requires OpenAI Responses API"));
        assert!(message.contains("api = 'responses'"));
    }

    #[test]
    fn test_gpt_5_with_responses_api_loads_successfully() {
        let mut f = NamedTempFile::new().unwrap();
        write!(
            f,
            r#"
[core]
host = "127.0.0.1"
port = 3000

[[providers]]
name = "openai"
base_url = "https://api.openai.com/v1"
format = "openai"
api = "responses"
models = ["gpt-5"]
"#
        )
        .unwrap();

        let cfg = load_config(f.path()).unwrap();
        assert_eq!(cfg.providers[0].api, crate::config::ProviderApi::Responses);
    }
}
