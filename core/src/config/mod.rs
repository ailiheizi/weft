use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub core: CoreConfig,
    #[serde(default)]
    pub providers: Vec<ProviderConfig>,
    #[serde(default)]
    pub routing: RoutingConfig,
    #[serde(default)]
    pub key_strategy: KeyStrategyConfig,
    #[serde(default)]
    pub fallback: FallbackConfig,
    #[serde(default)]
    pub virtual_keys: Vec<VirtualKeyConfig>,
    #[serde(default)]
    pub services: Vec<ServiceConfig>,
    #[serde(default)]
    pub packages: Vec<PackageConfig>,
    #[serde(default)]
    pub registry: RegistryConfig,
    #[serde(default)]
    pub package_aliases: HashMap<String, String>,
    /// 搜索/Web 工具的 API key（透传给 js-extension-runtime 等服务的环境变量）。
    #[serde(default)]
    pub web_search: WebSearchConfig,
    /// 多 agent 编排的角色→模型映射（orchestrator-worker 分层）。
    #[serde(default)]
    pub team: TeamConfig,
}

/// `[team]` 配置段：多 agent 编排的角色→模型映射。
/// 角色(planner/implementer/reviewer/integrator)可各自指定 provider/model；
/// 留空则继承 core 的 `routing.default_model`。core 启动时把 role_routing
/// 序列化写入 KV(key=`team:role_routing`),team-runtime 读取后按角色注入。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TeamConfig {
    /// 角色 id → 模型路由。键如 "planner"/"implementer"/"reviewer"/"integrator"。
    #[serde(default, alias = "roleRouting")]
    pub role_routing: HashMap<String, RoleModel>,
}

/// 单个角色的模型路由：provider 与 model 均可选,缺省继承默认。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RoleModel {
    #[serde(default)]
    pub provider: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
}

/// `[web_search]` 配置段：搜索后端 API key。
/// 启动时这些 key 会被注入进程环境（EXA_API_KEY 等），
/// js-extension-runtime 子进程继承后即可使用。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WebSearchConfig {
    #[serde(default)]
    pub exa_api_key: Option<String>,
    #[serde(default)]
    pub tavily_api_key: Option<String>,
    #[serde(default)]
    pub brave_api_key: Option<String>,
    /// 默认搜索 provider（exa/tavily/brave/duckduckgo/auto）。
    #[serde(default)]
    pub provider: Option<String>,
}

impl WebSearchConfig {
    /// 把已配置的 key 注入当前进程环境，供后续 spawn 的 service 子进程继承。
    /// 仅在环境中尚未设置时写入（已有的环境变量优先，便于临时覆盖）。
    pub fn apply_to_env(&self) {
        fn set_if_absent(key: &str, value: &Option<String>) {
            if let Some(v) = value {
                let v = v.trim();
                if !v.is_empty() && std::env::var(key).is_err() {
                    std::env::set_var(key, v);
                }
            }
        }
        set_if_absent("EXA_API_KEY", &self.exa_api_key);
        set_if_absent("TAVILY_API_KEY", &self.tavily_api_key);
        set_if_absent("BRAVE_API_KEY", &self.brave_api_key);
        set_if_absent("WEFT_WEB_SEARCH_PROVIDER", &self.provider);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreConfig {
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_log_level")]
    pub log_level: String,
    #[serde(default = "default_data_dir")]
    pub data_dir: String,
}

fn default_host() -> String {
    "127.0.0.1".into()
}
fn default_port() -> u16 {
    17830
}
fn default_log_level() -> String {
    "info".into()
}
fn default_data_dir() -> String {
    "./data".into()
}

impl Default for CoreConfig {
    fn default() -> Self {
        Self {
            host: default_host(),
            port: default_port(),
            log_level: default_log_level(),
            data_dir: default_data_dir(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub name: String,
    pub base_url: String,
    #[serde(default = "default_format")]
    pub format: String,
    #[serde(default)]
    pub api: ProviderApi,
    #[serde(default)]
    pub keys: Vec<ApiKeyConfig>,
    #[serde(default)]
    pub models: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderApi {
    ChatCompletions,
    Responses,
}

impl Default for ProviderApi {
    fn default() -> Self {
        Self::ChatCompletions
    }
}

impl ProviderApi {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ChatCompletions => "chat_completions",
            Self::Responses => "responses",
        }
    }
}

fn default_format() -> String {
    "openai".into()
}

pub fn validate_config(config: &AppConfig) -> anyhow::Result<()> {
    for provider in &config.providers {
        validate_provider(provider)?;
    }
    Ok(())
}

pub fn validate_provider(provider: &ProviderConfig) -> anyhow::Result<()> {
    match provider.format.as_str() {
        "openai" | "anthropic" => {}
        other => anyhow::bail!(
            "Invalid provider format '{}'. Must be 'openai' or 'anthropic'",
            other
        ),
    }

    if provider.format == "anthropic" && provider.api != ProviderApi::ChatCompletions {
        anyhow::bail!(
            "Provider '{}' uses format 'anthropic' but api '{}'; only chat_completions is supported for anthropic providers",
            provider.name,
            provider.api.as_str()
        );
    }

    for model in &provider.models {
        if requires_openai_responses(model) && provider.api != ProviderApi::Responses {
            anyhow::bail!(
                "Model '{}' on provider '{}' requires OpenAI Responses API. Set api = 'responses' for this provider.",
                model,
                provider.name
            );
        }
    }

    Ok(())
}

pub fn requires_openai_responses(model: &str) -> bool {
    let model = model.to_ascii_lowercase();
    model.starts_with("gpt-5")
        || model.starts_with("gpt-4o")
        || model.starts_with("o1")
        || model.starts_with("o3")
        || model.starts_with("o4")
        || model.contains("codex")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn provider(name: &str, base_url: &str, models: Vec<&str>) -> ProviderConfig {
        ProviderConfig {
            name: name.into(),
            base_url: base_url.into(),
            format: "openai".into(),
            api: ProviderApi::ChatCompletions,
            keys: vec![ApiKeyConfig {
                value: "sk-test".into(),
                label: None,
                enabled: true,
            }],
            models: models.into_iter().map(String::from).collect(),
        }
    }

    #[test]
    fn legacy_deepseek_config_loads_with_chat_completions_default() {
        let config: AppConfig = toml::from_str(
            r#"
                [core]
                host = "127.0.0.1"
                port = 17830

                [routing]
                default_provider = "deepseek"
                default_model = "deepseek-chat"

                [[providers]]
                name = "deepseek"
                base_url = "https://api.deepseek.com"
                format = "openai"
                models = ["deepseek-chat", "deepseek-reasoner"]

                [[providers.keys]]
                value = "sk-deepseek"
            "#,
        )
        .unwrap();

        assert_eq!(config.providers[0].api, ProviderApi::ChatCompletions);
        validate_config(&config).unwrap();
    }

    #[test]
    fn deepseek_models_do_not_require_openai_responses() {
        let config = AppConfig {
            core: CoreConfig::default(),
            providers: vec![provider(
                "deepseek",
                "https://api.deepseek.com",
                vec!["deepseek-chat", "deepseek-reasoner"],
            )],
            routing: RoutingConfig::default(),
            key_strategy: KeyStrategyConfig::default(),
            fallback: FallbackConfig::default(),
            virtual_keys: vec![],
            services: vec![],
            packages: vec![],
            registry: RegistryConfig::default(),
            package_aliases: HashMap::new(),
            web_search: Default::default(),
            team: Default::default(),
        };

        validate_config(&config).unwrap();
    }

    #[test]
    fn openai_responses_only_models_still_require_responses_api() {
        let config = AppConfig {
            core: CoreConfig::default(),
            providers: vec![provider(
                "openai",
                "https://api.openai.com",
                vec!["gpt-5", "gpt-4o", "o3-mini", "codex-mini-latest"],
            )],
            routing: RoutingConfig::default(),
            key_strategy: KeyStrategyConfig::default(),
            fallback: FallbackConfig::default(),
            virtual_keys: vec![],
            services: vec![],
            packages: vec![],
            registry: RegistryConfig::default(),
            package_aliases: HashMap::new(),
            web_search: Default::default(),
            team: Default::default(),
        };

        let err = validate_config(&config).unwrap_err().to_string();
        assert!(err.contains("requires OpenAI Responses API"));
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyConfig {
    pub value: String,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RoutingConfig {
    #[serde(default)]
    pub default_provider: Option<String>,
    #[serde(default)]
    pub default_model: Option<String>,
    /// 图像生成专用 provider（对应某个 [[providers]].name）。
    /// 出图 capability(image.generate) 调用时,Core 从该 provider 取
    /// base_url+key 注入请求,使 image-gen WASM 无需环境变量即可出图。
    #[serde(default)]
    pub image_provider: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyStrategyConfig {
    #[serde(default = "default_key_mode")]
    pub mode: String,
}

fn default_key_mode() -> String {
    "failover".into()
}

impl Default for KeyStrategyConfig {
    fn default() -> Self {
        Self {
            mode: default_key_mode(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FallbackConfig {
    #[serde(default = "default_retry_count")]
    pub retry_count: u32,
    #[serde(default = "default_true")]
    pub switch_key: bool,
    #[serde(default = "default_true")]
    pub switch_provider: bool,
    #[serde(default)]
    pub priority: Vec<String>,
}

fn default_retry_count() -> u32 {
    2
}

impl Default for FallbackConfig {
    fn default() -> Self {
        Self {
            retry_count: default_retry_count(),
            switch_key: true,
            switch_provider: true,
            priority: vec![],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VirtualKeyConfig {
    pub key: String,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub provider: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub max_rpm: Option<u32>,
    #[serde(default)]
    pub max_budget: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceConfig {
    pub name: String,
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub workdir: Option<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
    #[serde(default)]
    pub health_url: Option<String>,
    #[serde(default = "default_health_interval")]
    pub health_interval: u64,
    #[serde(default)]
    pub auto_start: bool,
    #[serde(default)]
    pub restart_on_crash: bool,
}

fn default_health_interval() -> u64 {
    10
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageConfig {
    pub name: String,
    /// Path to the package directory (e.g. "packages/installed/weft-claw")
    #[serde(default)]
    pub path: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryConfig {
    #[serde(default = "default_gitea_url")]
    pub gitea_url: String,
    #[serde(default)]
    pub gitea_token: Option<String>,
    #[serde(default)]
    pub package_source_url: Option<String>,
    #[serde(default)]
    pub app_source_url: Option<String>,
}

fn default_gitea_url() -> String {
    "https://gitea.alhz.org".into()
}

impl Default for RegistryConfig {
    fn default() -> Self {
        Self {
            gitea_url: default_gitea_url(),
            gitea_token: None,
            package_source_url: None,
            app_source_url: None,
        }
    }
}

pub mod store;
