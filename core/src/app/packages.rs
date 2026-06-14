use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PackageSource {
    pub name: String,
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub package_kind: String,
    #[serde(default)]
    pub runtime_provider: String,
    #[serde(default)]
    pub current_source: String,
    #[serde(default)]
    pub trusted: bool,
    #[serde(default)]
    pub signature: String,
    #[serde(default)]
    pub source_authority: String,
    #[serde(default)]
    pub source_public_keys: Vec<String>,
    #[serde(default)]
    pub provides: Vec<String>,
    #[serde(default)]
    pub requires: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PackageSourceDto {
    pub name: String,
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub package_kind: String,
    #[serde(default)]
    pub runtime_provider: String,
    #[serde(default)]
    pub current_source: String,
    #[serde(default)]
    pub trusted: bool,
    #[serde(default)]
    pub signature: String,
    #[serde(default)]
    pub source_authority: String,
    #[serde(default)]
    pub source_public_keys: Vec<String>,
    #[serde(default)]
    pub provides: Vec<String>,
    #[serde(default)]
    pub requires: Vec<String>,
}

impl From<PackageSourceDto> for PackageSource {
    fn from(dto: PackageSourceDto) -> Self {
        Self {
            name: dto.name,
            kind: dto.kind,
            package_kind: dto.package_kind,
            runtime_provider: dto.runtime_provider,
            current_source: dto.current_source,
            trusted: dto.trusted,
            signature: dto.signature,
            source_authority: dto.source_authority,
            source_public_keys: dto.source_public_keys,
            provides: dto.provides,
            requires: dto.requires,
        }
    }
}

impl PackageSource {
    pub fn runtime_provider_name(&self) -> String {
        if !self.runtime_provider.trim().is_empty() {
            return self.runtime_provider.trim().to_string();
        }

        std::path::Path::new(&self.current_source)
            .file_name()
            .and_then(|name| name.to_str())
            .map(|name| name.to_string())
            .filter(|name| !name.trim().is_empty())
            .unwrap_or_else(|| self.name.clone())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PackageIndex {
    #[serde(default)]
    pub version: u32,
    #[serde(default)]
    pub revision: String,
    #[serde(default)]
    pub source_url: String,
    #[serde(default)]
    pub package_sources: Vec<PackageSource>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PackageIndexDto {
    #[serde(default)]
    pub version: u32,
    #[serde(default)]
    pub revision: String,
    #[serde(default)]
    pub source_url: String,
    #[serde(default)]
    pub package_sources: Vec<PackageSourceDto>,
}

impl From<PackageIndexDto> for PackageIndex {
    fn from(dto: PackageIndexDto) -> Self {
        Self {
            version: dto.version,
            revision: dto.revision,
            source_url: dto.source_url,
            package_sources: dto
                .package_sources
                .into_iter()
                .map(PackageSource::from)
                .collect(),
        }
    }
}

#[async_trait]
pub trait PackageServiceClient: Send + Sync {
    async fn fetch_package_index(&self, source_url: &str) -> Result<PackageIndex>;
}

#[derive(Default, Debug, Clone)]
pub struct ReqwestPackageServiceClient {
    http_client: reqwest::Client,
}

impl ReqwestPackageServiceClient {
    pub fn new() -> Self {
        Self {
            http_client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl PackageServiceClient for ReqwestPackageServiceClient {
    async fn fetch_package_index(&self, source_url: &str) -> Result<PackageIndex> {
        let response = self
            .http_client
            .get(source_url)
            .send()
            .await?
            .error_for_status()?;
        let text = response.text().await?;
        parse_package_index_response(&text)
            .with_context(|| format!("Failed to parse package index response from {}", source_url))
    }
}

impl PackageIndex {
    pub fn get(&self, name: &str) -> Option<&PackageSource> {
        self.package_sources.iter().find(|p| {
            p.name == name || (!p.runtime_provider.trim().is_empty() && p.runtime_provider == name)
        })
    }

    pub fn names(&self) -> Vec<&str> {
        self.package_sources
            .iter()
            .map(|p| p.name.as_str())
            .collect()
    }

    /// B3: capability-level requirement check. Returns every `requires` entry
    /// declared by a package source for which no source in the index `provides`
    /// the capability. An empty result means every declared requirement has at
    /// least one provider registered (it does not guarantee that provider is
    /// loadable — that is the runtime/validate concern).
    ///
    /// WEFT's dependency model is capability-based (`requires = ["session.events"]`),
    /// not package-version-based, so this is the meaningful integrity check.
    pub fn unmet_requirements(&self) -> Vec<UnmetRequirement> {
        let provided: std::collections::HashSet<&str> = self
            .package_sources
            .iter()
            .flat_map(|p| p.provides.iter().map(|s| s.as_str()))
            .collect();

        let mut unmet = Vec::new();
        for source in &self.package_sources {
            for capability in &source.requires {
                let cap = capability.trim();
                if cap.is_empty() {
                    continue;
                }
                if !provided.contains(cap) {
                    unmet.push(UnmetRequirement {
                        package: source.name.clone(),
                        capability: cap.to_string(),
                    });
                }
            }
        }
        unmet
    }
}

/// A capability a package requires that no registered source provides (B3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnmetRequirement {
    pub package: String,
    pub capability: String,
}

pub fn load_package_index(packages_dir: &Path) -> Result<PackageIndex> {
    let path = packages_dir.join("index.toml");
    let content = std::fs::read_to_string(&path)
        .with_context(|| format!("Failed to read {}", path.display()))?;
    toml::from_str(&content).with_context(|| format!("Failed to parse {}", path.display()))
}

pub async fn fetch_package_index_from_url(source_url: &str) -> Result<PackageIndex> {
    ReqwestPackageServiceClient::new()
        .fetch_package_index(source_url)
        .await
}

fn parse_package_index_response(text: &str) -> Result<PackageIndex> {
    let dto: PackageIndexDto = serde_json::from_str(text)?;
    Ok(dto.into())
}

fn load_cached_json<T>(path: &Path) -> Option<T>
where
    T: DeserializeOwned,
{
    std::fs::read_to_string(path)
        .ok()
        .and_then(|content| serde_json::from_str(&content).ok())
}

fn save_cached_json<T>(path: &Path, value: &T)
where
    T: serde::Serialize,
{
    let Some(parent) = path.parent() else {
        return;
    };

    if let Err(error) = std::fs::create_dir_all(parent) {
        tracing::warn!(
            "Failed to create cache directory '{}': {}",
            parent.display(),
            error
        );
        return;
    }

    match serde_json::to_string_pretty(value) {
        Ok(content) => {
            if let Err(error) = std::fs::write(path, content) {
                tracing::warn!("Failed to write cache file '{}': {}", path.display(), error);
            }
        }
        Err(error) => {
            tracing::warn!(
                "Failed to serialize cache file '{}': {}",
                path.display(),
                error
            );
        }
    }
}

pub async fn resolve_package_index(
    repo_root: &Path,
    data_dir: &str,
    configured_source_url: Option<&str>,
) -> PackageIndex {
    resolve_package_index_with_fallback(repo_root, data_dir, configured_source_url).await
}

pub async fn resolve_package_index_with_fallback(
    repo_root: &Path,
    data_dir: &str,
    configured_source_url: Option<&str>,
) -> PackageIndex {
    let client = ReqwestPackageServiceClient::new();
    resolve_package_index_with_client(repo_root, data_dir, configured_source_url, &client).await
}

pub async fn resolve_package_index_with_client<C: PackageServiceClient + ?Sized>(
    repo_root: &Path,
    data_dir: &str,
    configured_source_url: Option<&str>,
    package_service_client: &C,
) -> PackageIndex {
    let packages_dir = repo_root.join("packages");
    let package_cache_path = repo_root
        .join(data_dir)
        .join("source-cache")
        .join("package-index.json");

    let local_index = load_package_index(&packages_dir).unwrap_or_else(|e| {
        tracing::warn!("Failed to load package index: {}", e);
        PackageIndex::default()
    });

    let source_url = configured_source_url
        .map(str::to_string)
        .unwrap_or_else(|| local_index.source_url.clone());

    if source_url.starts_with("http://") || source_url.starts_with("https://") {
        match package_service_client
            .fetch_package_index(&source_url)
            .await
        {
            Ok(remote_index) => {
                save_cached_json(&package_cache_path, &remote_index);
                remote_index
            }
            Err(error) => {
                if let Some(cached_index) = load_cached_json::<PackageIndex>(&package_cache_path) {
                    tracing::warn!(
                        "Failed to fetch package index from '{}': {}. Falling back to cached snapshot at '{}'.",
                        source_url,
                        error,
                        package_cache_path.display()
                    );
                    cached_index
                } else {
                    tracing::warn!(
                        "Failed to fetch package index from '{}': {}. Falling back to local index.",
                        source_url,
                        error
                    );
                    local_index
                }
            }
        }
    } else {
        local_index
    }
}

#[cfg(test)]
mod tests {
    use super::{
        fetch_package_index_from_url, load_package_index, parse_package_index_response,
        PackageIndex, PackageSource,
    };
    use crate::app::resolve_package_index;
    use axum::routing::post;
    use axum::{routing::get, Json, Router};
    use serde_json::json;
    use tokio::net::TcpListener;

    fn src(name: &str, provides: &[&str], requires: &[&str]) -> PackageSource {
        PackageSource {
            name: name.to_string(),
            provides: provides.iter().map(|s| s.to_string()).collect(),
            requires: requires.iter().map(|s| s.to_string()).collect(),
            ..Default::default()
        }
    }

    #[test]
    fn unmet_requirements_empty_when_all_satisfied() {
        let index = PackageIndex {
            package_sources: vec![
                src("a", &["session.events"], &[]),
                src("b", &["agent.runtime"], &["session.events"]),
            ],
            ..Default::default()
        };
        assert!(index.unmet_requirements().is_empty());
    }

    #[test]
    fn unmet_requirements_flags_missing_provider() {
        let index = PackageIndex {
            package_sources: vec![
                // requires ext.mcp but nothing provides it
                src("weft-claw", &["weft_claw.turn"], &["ext.mcp", "agent.runtime"]),
                src("agent", &["agent.runtime"], &[]),
            ],
            ..Default::default()
        };
        let unmet = index.unmet_requirements();
        assert_eq!(unmet.len(), 1);
        assert_eq!(unmet[0].package, "weft-claw");
        assert_eq!(unmet[0].capability, "ext.mcp");
    }

    #[test]
    fn unmet_requirements_repo_index_is_satisfied() {
        // The shipped packages/index.toml must have no unmet capability requirements.
        let repo_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
        let index = load_package_index(&repo_root.join("packages")).expect("index loads");
        let unmet = index.unmet_requirements();
        assert!(
            unmet.is_empty(),
            "shipped index has unmet requirements: {unmet:?}"
        );
    }

    const TRUSTED_INDEX_PACKAGES: &[&str] = &[
        "agent-runtime",
        "memory-store",
        "skills-runtime",
        "channel-core",
        "cron",
        "mcp-client",
    ];
    #[test]
    fn repository_package_index_includes_current_trusted_entries() {
        let repo_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
        let index = load_package_index(&repo_root.join("packages")).expect("package index loads");

        assert_eq!(index.revision, "packages-index-v1");

        for package_name in TRUSTED_INDEX_PACKAGES {
            let pkg = index
                .get(package_name)
                .unwrap_or_else(|| panic!("{package_name} package exists in index"));

            assert!(pkg.trusted, "{package_name} should remain trusted");
            assert!(
                !pkg.current_source.trim().is_empty(),
                "{package_name} should declare a current source"
            );
            assert!(
                !pkg.signature.trim().is_empty(),
                "{package_name} should declare a signature or builtin trust marker"
            );
            assert!(
                !pkg.source_authority.trim().is_empty(),
                "{package_name} should declare a source authority"
            );
        }
    }

    #[tokio::test]
    async fn fetch_package_index_from_url_preserves_revision_metadata() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let app = Router::new().route(
            "/packages",
            get(|| async {
                Json(json!({
                    "version": 1,
                    "revision": "packages-remote-r7",
                    "source_url": "https://registry.example/api/sources/packages",
                    "package_sources": [
                        {
                            "name": "agent-runtime",
                            "kind": "wasm",
                            "current_source": "packages/official/agent-core",
                            "trusted": true,
                            "signature": "builtin:official",
                            "source_authority": "official",
                            "source_public_keys": []
                        }
                    ]
                }))
            }),
        );
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        let index = fetch_package_index_from_url(&format!("http://{addr}/packages"))
            .await
            .expect("remote package index loads");

        assert_eq!(index.version, 1);
        assert_eq!(index.revision, "packages-remote-r7");
        assert_eq!(index.package_sources[0].source_authority, "official");
    }

    #[test]
    fn parse_package_index_response_adapts_dto() {
        let text = r#"
            {
              "version": 1,
              "revision": "dto-v1",
              "source_url": "https://registry.example/api/sources/packages",
              "package_sources": [
                {
                  "name": "agent-runtime",
                  "kind": "wasm",
                  "current_source": "packages/official/agent-core",
                  "trusted": true,
                  "signature": "builtin:official",
                  "source_authority": "official",
                  "source_public_keys": []
                }
              ]
            }
        "#;

        let index = parse_package_index_response(text).expect("dto parse");
        assert_eq!(index.version, 1);
        assert_eq!(index.revision, "dto-v1");
        assert_eq!(
            index.source_url,
            "https://registry.example/api/sources/packages"
        );

        let pkg = &index.package_sources[0];
        assert_eq!(pkg.name, "agent-runtime");
        assert_eq!(pkg.kind, "wasm");
        assert_eq!(pkg.current_source, "packages/official/agent-core");
        assert_eq!(pkg.signature, "builtin:official");
    }

    fn write_package_index(path: &std::path::Path, index: &PackageIndex) {
        let content = toml::to_string_pretty(index).expect("index serializes");
        std::fs::write(path, content).expect("index written");
    }

    fn write_cached_index(path: &std::path::Path, index: &PackageIndex) {
        let content = serde_json::to_string_pretty(index).expect("cache serializes");
        std::fs::create_dir_all(path.parent().unwrap()).expect("cache parent exists");
        std::fs::write(path, content).expect("cache written");
    }

    #[tokio::test]
    async fn resolve_package_index_prefers_remote_and_caches_result() {
        let root = tempfile::tempdir().expect("temp dir");
        let packages_dir = root.path().join("packages");
        std::fs::create_dir_all(&packages_dir).expect("packages dir");

        let local_index = PackageIndex {
            version: 1,
            revision: "local-v1".into(),
            source_url: String::new(),
            package_sources: vec![],
        };
        write_package_index(&packages_dir.join("index.toml"), &local_index);

        let remote_index = PackageIndex {
            version: 2,
            revision: "remote-v2".into(),
            source_url: "https://registry.example/api/sources/packages".into(),
            package_sources: vec![],
        };

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let app = Router::new().route(
            "/packages",
            get(move || {
                let remote_index = remote_index.clone();
                async move { Json(remote_index) }
            }),
        );
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        let package_index = resolve_package_index(
            root.path(),
            "./data",
            Some(&format!("http://{addr}/packages")),
        )
        .await;

        assert_eq!(package_index.version, 2);
        assert_eq!(package_index.revision, "remote-v2");

        let cached_path = root.path().join("./data/source-cache/package-index.json");
        let cached: PackageIndex =
            serde_json::from_str(&std::fs::read_to_string(&cached_path).expect("cache exists"))
                .expect("cache parse");
        assert_eq!(cached.revision, "remote-v2");
    }

    #[tokio::test]
    async fn resolve_package_index_falls_back_to_cached_when_remote_fails() {
        let root = tempfile::tempdir().expect("temp dir");
        let packages_dir = root.path().join("packages");
        std::fs::create_dir_all(&packages_dir).expect("packages dir");

        let local_index = PackageIndex {
            version: 1,
            revision: "local-v1".into(),
            source_url: "https://registry.example/api/sources/packages".into(),
            package_sources: vec![],
        };
        write_package_index(&packages_dir.join("index.toml"), &local_index);

        let cached_index = PackageIndex {
            version: 3,
            revision: "cached-v3".into(),
            source_url: String::new(),
            package_sources: vec![],
        };

        let cache_path = root.path().join("./data/source-cache/package-index.json");
        write_cached_index(&cache_path, &cached_index);

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let app = Router::new().route(
            "/packages",
            post(|| async { axum::http::StatusCode::INTERNAL_SERVER_ERROR }),
        );
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        let package_index = resolve_package_index(
            root.path(),
            "./data",
            Some(&format!("http://{addr}/packages")),
        )
        .await;

        assert_eq!(package_index.version, 3);
        assert_eq!(package_index.revision, "cached-v3");
    }
}
