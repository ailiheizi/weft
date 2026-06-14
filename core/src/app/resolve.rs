use crate::app::config::AppManifest;
use crate::app::packages::PackageIndex;
use crate::app::policy::{AppProfile, CorePolicy};
use crate::app::registry::CapabilityRegistry;
use crate::app::state::{
    AppBindingResolution, ResolvedApp, ResolvedAppSources, ResolvedAppStatus,
};
use crate::package::DiscoveredPackage;
use anyhow::{anyhow, Result};
use std::collections::{HashMap, HashSet};

const WEFT_CLAW_REQUIRED_REAL_PACKAGES: &[&str] = &[
    "prompt-system",
    "workflow-orchestrator",
    "tool-runtime-core",
    "tool-shell",
    "tool-files",
    "tool-web",
    "tool-git",
];

fn requires_real_package_source(package_name: &str) -> bool {
    WEFT_CLAW_REQUIRED_REAL_PACKAGES.contains(&package_name)
}

#[derive(Default)]
pub struct ResolveCandidateContext {
    provider_candidates: ProviderCandidateSet,
    candidate_grouping: ResolveCandidateGroupingSet,
}

#[derive(Debug, Clone)]
pub struct ServiceOriginCandidatePayload {
    pub provider: String,
    pub candidates: Vec<ServiceOriginCandidatePayloadEntry>,
    pub closure_metadata: ResolveCandidateGrouping,
}

#[derive(Debug, Clone)]
pub struct ServiceContractCandidatePayload {
    pub provider: String,
    pub candidates: Vec<String>,
    pub closure: ServiceContractCandidateClosure,
    pub provenance: Option<ServiceContractCandidateProvenance>,
}

#[derive(Debug, Clone)]
pub struct ServiceContractCandidateClosure {
    pub id: Option<String>,
    pub candidates: Vec<String>,
    pub rationale: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ServiceContractCandidateProvenance {
    pub package_name: String,
    pub package_kind: String,
    pub runtime_provider: String,
    pub current_source: String,
    pub source_kind: String,
}

#[derive(Debug, Clone)]
pub struct ServiceOriginCandidatePayloadFixture {
    pub provider: String,
    pub candidates: Vec<String>,
    pub closure_candidates: Vec<String>,
    pub closure_id: Option<String>,
    pub closure_rationale: Option<String>,
    pub provenance: Option<PackageCandidateProvenance>,
}

#[derive(Debug, Clone)]
pub struct ServiceOriginCandidatePayloadEntry {
    pub candidate: String,
    pub provenance: Option<PackageCandidateProvenance>,
}

impl ServiceOriginCandidatePayload {
    pub fn from_service_contract_payload(payload: &[ServiceContractCandidatePayload]) -> Vec<Self> {
        payload
            .iter()
            .filter_map(|service_payload| {
                let provider = service_payload.provider.trim();
                if provider.is_empty() {
                    return None;
                }

                let mut closure_candidates: Vec<String> = service_payload
                    .closure
                    .candidates
                    .iter()
                    .filter_map(|candidate| {
                        let candidate = candidate.trim();
                        if candidate.is_empty() {
                            None
                        } else {
                            Some(candidate.to_string())
                        }
                    })
                    .collect();

                if !closure_candidates.contains(&provider.to_string()) {
                    closure_candidates.push(provider.to_string());
                }

                let mut candidates = Vec::new();
                for candidate in &service_payload.candidates {
                    let candidate = candidate.trim();
                    if candidate.is_empty() {
                        continue;
                    }

                    candidates.push(ServiceOriginCandidatePayloadEntry {
                        candidate: candidate.to_string(),
                        provenance: service_payload.provenance.as_ref().map(|provenance| {
                            PackageCandidateProvenance {
                                package_name: provenance.package_name.clone(),
                                package_kind: provenance.package_kind.clone(),
                                runtime_provider: provenance.runtime_provider.clone(),
                                current_source: provenance.current_source.clone(),
                                source_kind: provenance.source_kind.clone(),
                            }
                        }),
                    });
                }

                Some(Self {
                    provider: provider.to_string(),
                    candidates,
                    closure_metadata: ResolveCandidateGrouping {
                        id: service_payload
                            .closure
                            .id
                            .as_ref()
                            .and_then(|id| {
                                let id = id.trim();
                                if id.is_empty() {
                                    None
                                } else {
                                    Some(id.to_string())
                                }
                            })
                            .unwrap_or_else(|| format!("service-contract:{}:closure", provider)),
                        source: ResolveCandidateGroupingSource::Service,
                        kind: ResolveCandidateGroupingKind::Closure,
                        candidates: closure_candidates,
                        rationale: Some(
                            service_payload
                                .closure
                                .rationale
                                .as_ref()
                                .and_then(|rationale| {
                                    let rationale = rationale.trim();
                                    if rationale.is_empty() {
                                        None
                                    } else {
                                        Some(rationale.to_string())
                                    }
                                })
                                .unwrap_or_else(|| "service-candidate-contract".to_string()),
                        ),
                    },
                })
            })
            .collect()
    }

    pub fn from_fixtures(fixtures: &[ServiceOriginCandidatePayloadFixture]) -> Vec<Self> {
        fixtures
            .iter()
            .filter_map(|fixture| {
                let provider = fixture.provider.trim().to_string();
                if provider.is_empty() {
                    return None;
                }

                let mut closure_candidates: Vec<String> = fixture
                    .closure_candidates
                    .iter()
                    .filter_map(|candidate| {
                        let candidate = candidate.trim();
                        if candidate.is_empty() {
                            None
                        } else {
                            Some(candidate.to_string())
                        }
                    })
                    .collect();

                if !closure_candidates
                    .iter()
                    .any(|candidate| candidate == &provider)
                {
                    closure_candidates.push(provider.clone());
                }

                let candidates = fixture
                    .candidates
                    .iter()
                    .filter_map(|candidate| {
                        let candidate = candidate.trim();
                        if candidate.is_empty() {
                            None
                        } else {
                            Some(ServiceOriginCandidatePayloadEntry {
                                candidate: candidate.to_string(),
                                provenance: fixture.provenance.clone(),
                            })
                        }
                    })
                    .collect();

                let closure_id = fixture
                    .closure_id
                    .as_ref()
                    .and_then(|closure_id| {
                        let closure_id = closure_id.trim();
                        if closure_id.is_empty() {
                            None
                        } else {
                            Some(closure_id.to_string())
                        }
                    })
                    .unwrap_or_else(|| format!("service-fixture:{}:closure", provider));

                let closure_rationale = fixture
                    .closure_rationale
                    .as_ref()
                    .and_then(|rationale| {
                        let rationale = rationale.trim();
                        if rationale.is_empty() {
                            None
                        } else {
                            Some(rationale.to_string())
                        }
                    })
                    .unwrap_or_else(|| "service-candidate-fixture".to_string());

                Some(Self {
                    provider,
                    candidates,
                    closure_metadata: ResolveCandidateGrouping {
                        id: closure_id,
                        source: ResolveCandidateGroupingSource::Service,
                        kind: ResolveCandidateGroupingKind::Closure,
                        candidates: closure_candidates,
                        rationale: Some(closure_rationale),
                    },
                })
            })
            .collect()
    }

    pub fn from_package_index(package_index: &PackageIndex) -> Vec<Self> {
        package_index
            .package_sources
            .iter()
            .filter(|package| package.kind == "service")
            .filter_map(|package| {
                let runtime_provider = package.runtime_provider_name();
                if runtime_provider.trim().is_empty() {
                    return None;
                }

                let service_package_name = package.name.trim();
                let mut candidates = Vec::new();
                if !service_package_name.is_empty() && service_package_name != runtime_provider {
                    candidates.push(ServiceOriginCandidatePayloadEntry {
                        candidate: service_package_name.to_string(),
                        provenance: Some(PackageCandidateProvenance {
                            package_name: package.name.clone(),
                            package_kind: package.package_kind.clone(),
                            runtime_provider: package.runtime_provider.clone(),
                            current_source: package.current_source.clone(),
                            source_kind: package.kind.clone(),
                        }),
                    });
                }

                let mut closure_candidates = vec![runtime_provider.clone()];
                if !service_package_name.is_empty()
                    && !closure_candidates.contains(&service_package_name.to_string())
                {
                    closure_candidates.push(service_package_name.to_string());
                }

                Some(Self {
                    provider: runtime_provider,
                    candidates,
                    closure_metadata: ResolveCandidateGrouping {
                        id: format!("service:{}:closure", package.name),
                        source: ResolveCandidateGroupingSource::Service,
                        kind: ResolveCandidateGroupingKind::Closure,
                        candidates: closure_candidates,
                        rationale: Some("service-candidate-synthesis".into()),
                    },
                })
            })
            .collect()
    }
}
pub struct ResolveInputCoordinator<'a> {
    package_index: &'a PackageIndex,
    resolve_candidate_context: ResolveCandidateContext,
}

pub struct ResolveInputCoordinatorBuilder<'a> {
    package_index: &'a PackageIndex,
    resolve_candidate_context: ResolveCandidateContext,
}

impl<'a> ResolveInputCoordinator<'a> {
    pub fn from_package_index(
        package_index: &'a PackageIndex,
    ) -> ResolveInputCoordinatorBuilder<'a> {
        ResolveInputCoordinatorBuilder {
            package_index,
            resolve_candidate_context: ResolveCandidateContext::from_package_index(package_index),
        }
    }

    pub fn package_index(&self) -> &'a PackageIndex {
        self.package_index
    }

    pub fn resolve_candidate_context(&self) -> &ResolveCandidateContext {
        &self.resolve_candidate_context
    }
}

impl<'a> ResolveInputCoordinatorBuilder<'a> {
    pub fn with_service_origin_candidates(
        mut self,
        adapter: &dyn ServiceOriginCandidateAdapter,
        payload: &[ServiceOriginCandidatePayload],
    ) -> Self {
        self.resolve_candidate_context = self
            .resolve_candidate_context
            .with_service_origin_candidates(adapter, payload);
        self
    }

    pub fn with_synthesized_service_origin_candidates(self) -> Self {
        let payload = ServiceOriginCandidatePayload::from_package_index(self.package_index);
        self.with_service_origin_candidates(&SynthesizedServiceOriginCandidateAdapter, &payload)
    }

    pub fn build(self) -> ResolveInputCoordinator<'a> {
        ResolveInputCoordinator {
            package_index: self.package_index,
            resolve_candidate_context: self.resolve_candidate_context,
        }
    }
}

pub trait ServiceOriginCandidateAdapter {
    fn synthesize(
        &self,
        resolve_candidate_context: &mut ResolveCandidateContext,
        payload: &[ServiceOriginCandidatePayload],
    );
}

pub struct SynthesizedServiceOriginCandidateAdapter;

impl ServiceOriginCandidateAdapter for SynthesizedServiceOriginCandidateAdapter {
    fn synthesize(
        &self,
        resolve_candidate_context: &mut ResolveCandidateContext,
        payload: &[ServiceOriginCandidatePayload],
    ) {
        for entry in payload {
            let runtime_provider = entry.provider.trim();
            if runtime_provider.is_empty() {
                continue;
            }

            for candidate in &entry.candidates {
                if candidate.candidate.trim().is_empty() {
                    continue;
                }

                resolve_candidate_context
                    .provider_candidates
                    .add_candidate_entry(
                        runtime_provider,
                        ResolveCandidateEntry {
                            candidate: candidate.candidate.clone(),
                            provenance: candidate
                                .provenance
                                .clone()
                                .map(ResolveCandidateProvenance::PackageIndex)
                                .unwrap_or(ResolveCandidateProvenance::External),
                        },
                    );
            }

            resolve_candidate_context
                .candidate_grouping
                .add_grouping(runtime_provider, entry.closure_metadata.clone());
        }
    }
}

#[derive(Debug, Clone)]
pub enum ResolveCandidateGroupingSource {
    Service,
    Local,
    Index,
}

#[derive(Debug, Clone)]
pub enum ResolveCandidateGroupingKind {
    Closure,
    RecommendationSet,
}

#[derive(Debug, Clone)]
pub struct ResolveCandidateGrouping {
    pub id: String,
    pub source: ResolveCandidateGroupingSource,
    pub kind: ResolveCandidateGroupingKind,
    pub candidates: Vec<String>,
    pub rationale: Option<String>,
}

#[derive(Default)]
pub struct ResolveCandidateGroupingSet {
    groups: HashMap<String, Vec<ResolveCandidateGrouping>>,
}

impl ResolveCandidateGroupingSet {
    fn add_grouping(&mut self, provider: impl Into<String>, grouping: ResolveCandidateGrouping) {
        let groups = self.groups.entry(provider.into()).or_default();
        groups.push(grouping);
    }

    pub fn groups_for(&self, provider: &str) -> &[ResolveCandidateGrouping] {
        self.groups.get(provider).map_or(&[], Vec::as_slice)
    }
}

#[derive(Default)]
pub struct ProviderCandidateSet {
    entries: HashMap<String, Vec<ResolveCandidateEntry>>,
}

#[derive(Debug, Clone)]
pub struct ResolveCandidateEntry {
    pub candidate: String,
    pub provenance: ResolveCandidateProvenance,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PackageCandidateProvenance {
    pub package_name: String,
    pub package_kind: String,
    pub runtime_provider: String,
    pub current_source: String,
    pub source_kind: String,
}

#[derive(Debug, Clone)]
pub enum ResolveCandidateProvenance {
    External,
    PackageIndex(PackageCandidateProvenance),
}

impl ProviderCandidateSet {
    fn add_candidate_entry(&mut self, provider: impl Into<String>, entry: ResolveCandidateEntry) {
        let entries = self.entries.entry(provider.into()).or_default();
        entries.push(entry);
    }

    pub fn add_candidates<S>(
        &mut self,
        provider: impl Into<String>,
        candidates: impl IntoIterator<Item = S>,
    ) where
        S: Into<String>,
    {
        let entries = self.entries.entry(provider.into()).or_default();
        for candidate in candidates {
            entries.push(ResolveCandidateEntry {
                candidate: candidate.into(),
                provenance: ResolveCandidateProvenance::External,
            });
        }
    }

    fn candidates_for(&self, provider: &str) -> &[ResolveCandidateEntry] {
        self.entries
            .get(provider)
            .map_or(&[], std::vec::Vec::as_slice)
    }
}

impl ResolveCandidateContext {
    pub fn candidates_for<'a>(&'a self, provider: &'a str) -> &'a [ResolveCandidateEntry] {
        self.provider_candidates
            .entries
            .get(provider)
            .map_or(&[], Vec::as_slice)
    }

    pub fn add_candidates<S>(
        &mut self,
        provider: impl Into<String>,
        candidates: impl IntoIterator<Item = S>,
    ) where
        S: Into<String>,
    {
        self.provider_candidates
            .add_candidates(provider, candidates);
    }

    pub fn add_closure_metadata(
        &mut self,
        provider: impl Into<String>,
        id: impl Into<String>,
        source: ResolveCandidateGroupingSource,
        candidates: impl IntoIterator<Item = impl Into<String>>,
    ) {
        self.candidate_grouping.add_grouping(
            provider,
            ResolveCandidateGrouping {
                id: id.into(),
                source,
                kind: ResolveCandidateGroupingKind::Closure,
                candidates: candidates.into_iter().map(Into::into).collect(),
                rationale: None,
            },
        );
    }

    pub fn add_recommendation_metadata(
        &mut self,
        provider: impl Into<String>,
        id: impl Into<String>,
        source: ResolveCandidateGroupingSource,
        candidates: impl IntoIterator<Item = impl Into<String>>,
        rationale: Option<String>,
    ) {
        self.candidate_grouping.add_grouping(
            provider,
            ResolveCandidateGrouping {
                id: id.into(),
                source,
                kind: ResolveCandidateGroupingKind::RecommendationSet,
                candidates: candidates.into_iter().map(Into::into).collect(),
                rationale,
            },
        );
    }

    pub fn grouping_for<'a>(&'a self, provider: &'a str) -> &'a [ResolveCandidateGrouping] {
        self.candidate_grouping.groups_for(provider)
    }

    pub fn with_service_origin_candidates(
        mut self,
        adapter: &dyn ServiceOriginCandidateAdapter,
        payload: &[ServiceOriginCandidatePayload],
    ) -> Self {
        adapter.synthesize(&mut self, payload);
        self
    }

    pub fn from_package_index_with_service_candidates(package_index: &PackageIndex) -> Self {
        Self::from_package_index_with_service_origin_adapter_and_payload(
            package_index,
            &SynthesizedServiceOriginCandidateAdapter,
            &ServiceOriginCandidatePayload::from_package_index(package_index),
        )
    }

    pub fn from_package_index_with_service_origin_adapter_and_payload(
        package_index: &PackageIndex,
        adapter: &dyn ServiceOriginCandidateAdapter,
        payload: &[ServiceOriginCandidatePayload],
    ) -> Self {
        Self::from_package_index(package_index).with_service_origin_candidates(adapter, payload)
    }

    pub fn from_package_index_with_service_origin_adapter(
        package_index: &PackageIndex,
        adapter: &dyn ServiceOriginCandidateAdapter,
    ) -> Self {
        Self::from_package_index(package_index).with_service_origin_candidates(
            adapter,
            &ServiceOriginCandidatePayload::from_package_index(package_index),
        )
    }

    pub fn from_package_index(package_index: &PackageIndex) -> Self {
        let mut context = Self::default();
        for package in &package_index.package_sources {
            let runtime_provider = package.runtime_provider_name();
            if runtime_provider != package.name {
                context.provider_candidates.add_candidate_entry(
                    package.name.clone(),
                    ResolveCandidateEntry {
                        candidate: runtime_provider.clone(),
                        provenance: ResolveCandidateProvenance::PackageIndex(
                            PackageCandidateProvenance {
                                package_name: package.name.clone(),
                                package_kind: package.package_kind.clone(),
                                runtime_provider: package.runtime_provider.clone(),
                                current_source: package.current_source.clone(),
                                source_kind: package.kind.clone(),
                            },
                        ),
                    },
                );

                context.candidate_grouping.add_grouping(
                    package.name.clone(),
                    ResolveCandidateGrouping {
                        id: format!("index:{}:closure", package.name),
                        source: ResolveCandidateGroupingSource::Index,
                        kind: ResolveCandidateGroupingKind::Closure,
                        candidates: vec![runtime_provider.clone()],
                        rationale: Some("package-index-runtime-provider".into()),
                    },
                );
            }
        }
        context
    }
}

fn expand_manifest_capabilities(manifest: &AppManifest) -> Vec<String> {
    let mut ordered = Vec::new();
    let mut seen = HashSet::new();

    for capability in &manifest.requires.capabilities {
        if seen.insert(capability.clone()) {
            ordered.push(capability.clone());
        }
    }

    for feature in &manifest.features.default_enabled {
        if let Some(binding) = manifest.feature_bindings.get(feature) {
            for capability in &binding.requires {
                if seen.insert(capability.clone()) {
                    ordered.push(capability.clone());
                }
            }
        }
    }

    ordered
}

fn provider_candidates_from_package_index<'a>(
    provider: &'a str,
    package_index: Option<&'a PackageIndex>,
) -> Vec<String> {
    let mut candidates = vec![provider.to_string()];
    if let Some(runtime_provider) = package_index
        .and_then(|index| index.get(provider))
        .map(|pkg| pkg.runtime_provider_name())
        .filter(|runtime_provider| runtime_provider != provider)
    {
        candidates.push(runtime_provider);
    }
    candidates
}

fn provider_candidates<'a>(
    provider: &'a str,
    package_index: Option<&'a PackageIndex>,
    resolve_candidate_context: &ResolveCandidateContext,
) -> Vec<String> {
    let mut candidates = provider_candidates_from_package_index(provider, package_index);

    for candidate in resolve_candidate_context
        .provider_candidates
        .candidates_for(provider)
    {
        if !candidates.contains(&candidate.candidate) {
            candidates.push(candidate.candidate.clone());
        }
    }

    candidates
}

pub fn resolve_app_manifest(
    manifest: &AppManifest,
    packages: &[DiscoveredPackage],
) -> Result<ResolvedApp> {
    resolve_app_manifest_with_policy(manifest, packages, None, None, None, None)
}

pub fn resolve_product_package_declaration(
    declaration: &AppManifest,
    packages: &[DiscoveredPackage],
) -> Result<ResolvedApp> {
    resolve_app_manifest(declaration, packages)
}

pub fn resolve_app_manifest_with_policy(
    manifest: &AppManifest,
    packages: &[DiscoveredPackage],
    profile: Option<AppProfile>,
    policy: Option<&CorePolicy>,
    core_capabilities: Option<&CapabilityRegistry>,
    package_index: Option<&PackageIndex>,
) -> Result<ResolvedApp> {
    resolve_app_manifest_with_policy_and_candidate_context(
        manifest,
        packages,
        profile,
        policy,
        core_capabilities,
        package_index,
        &ResolveCandidateContext::default(),
    )
}

pub fn resolve_app_manifest_with_policy_and_candidate_context(
    manifest: &AppManifest,
    packages: &[DiscoveredPackage],
    profile: Option<AppProfile>,
    policy: Option<&CorePolicy>,
    core_capabilities: Option<&CapabilityRegistry>,
    package_index: Option<&PackageIndex>,
    resolve_candidate_context: &ResolveCandidateContext,
) -> Result<ResolvedApp> {
    let mut bindings = Vec::new();
    let mut errors = Vec::new();
    let flattened_bindings = manifest.flattened_bindings();
    let expanded_capabilities = expand_manifest_capabilities(manifest);

    for capability in &expanded_capabilities {
        if let (Some(prof), Some(pol)) = (profile, policy) {
            let decision = pol.check(capability, prof);
            if !decision.allowed {
                errors.push(format!(
                    "Policy blocks capability '{}': {}",
                    capability, decision.reason
                ));
                continue;
            }
        }

        let binding = flattened_bindings
            .get(capability)
            .ok_or_else(|| anyhow!("Missing binding for capability '{}'", capability))?;

        let provider_candidates =
            provider_candidates(&binding.provider, package_index, resolve_candidate_context);

        let matched = packages.iter().find(|package| {
            provider_candidates.contains(&package.manifest.package_info.name)
                && package
                    .manifest
                    .resolved_provides()
                    .iter()
                    .any(|provided| provided == capability)
        });

        let matched_core = binding.provider == "core"
            && core_capabilities
                .and_then(|registry| registry.get(capability))
                .map(|entry| {
                    entry
                        .providers
                        .iter()
                        .any(|provider| provider.provider == "core" && provider.runtime == "core")
                })
                .unwrap_or(false);

        let matched_index_package = package_index
            .and_then(|index| index.get(&binding.provider))
            .map(|package| {
                !requires_real_package_source(&package.name)
                    && package.kind == "metadata"
                    && (package
                        .provides
                        .iter()
                        .any(|provided| provided == capability)
                        || package.name == binding.provider
                        || package.runtime_provider_name() == binding.provider)
            })
            .unwrap_or(false);

        if matched.is_none() && !matched_core && !matched_index_package {
            return Err(anyhow!(
                "Provider '{}' for capability '{}' is not available",
                binding.provider,
                capability
            ));
        }

        bindings.push(AppBindingResolution {
            capability: capability.clone(),
            provider: binding.provider.clone(),
            mutable: binding.mutable,
            source: "declaration-default".into(),
        });
    }

    let status = if errors.is_empty() {
        ResolvedAppStatus::Resolved
    } else {
        ResolvedAppStatus::Unresolved
    };

    Ok(ResolvedApp {
        name: manifest.app.name.clone(),
        version: manifest.app.version.clone(),
        display_name: manifest.app.display_name.clone(),
        description: manifest.app.description.clone(),
        capabilities: expanded_capabilities,
        enabled_features: manifest.features.default_enabled.clone(),
        bindings,
        validation_checks: manifest.validation.checks.clone(),
        config_path: None,
        status,
        errors,
        sources: ResolvedAppSources::default(),
    })
}

pub fn resolve_product_package_declaration_with_policy(
    declaration: &AppManifest,
    packages: &[DiscoveredPackage],
    profile: Option<AppProfile>,
    policy: Option<&CorePolicy>,
    core_capabilities: Option<&CapabilityRegistry>,
    package_index: Option<&PackageIndex>,
) -> Result<ResolvedApp> {
    resolve_app_manifest_with_policy(
        declaration,
        packages,
        profile,
        policy,
        core_capabilities,
        package_index,
    )
}

pub fn resolve_product_package_declaration_with_policy_and_candidate_context(
    declaration: &AppManifest,
    packages: &[DiscoveredPackage],
    profile: Option<AppProfile>,
    policy: Option<&CorePolicy>,
    core_capabilities: Option<&CapabilityRegistry>,
    package_index: Option<&PackageIndex>,
    resolve_candidate_context: &ResolveCandidateContext,
) -> Result<ResolvedApp> {
    resolve_app_manifest_with_policy_and_candidate_context(
        declaration,
        packages,
        profile,
        policy,
        core_capabilities,
        package_index,
        resolve_candidate_context,
    )
}

#[cfg(test)]
mod tests {
    use super::{
        resolve_app_manifest_with_policy, resolve_app_manifest_with_policy_and_candidate_context,
        PackageCandidateProvenance, ResolveCandidateContext, ResolveCandidateGroupingKind,
        ResolveCandidateGroupingSource, ResolveCandidateProvenance, ResolveInputCoordinator,
        ServiceContractCandidateClosure, ServiceContractCandidatePayload,
        ServiceContractCandidateProvenance, ServiceOriginCandidatePayload,
        ServiceOriginCandidatePayloadFixture, SynthesizedServiceOriginCandidateAdapter,
    };
    use crate::app::{CapabilityRegistry, PackageIndex, PackageSource, ResolvedAppStatus};
    use crate::package::{DiscoveredPackage, PackageRuntime};

    fn package_source(
        name: &str,
        kind: &str,
        current_source: &str,
        runtime_provider: &str,
    ) -> PackageSource {
        PackageSource {
            name: name.into(),
            kind: kind.into(),
            package_kind: String::new(),
            runtime_provider: runtime_provider.into(),
            current_source: current_source.into(),
            trusted: true,
            signature: "builtin:official".into(),
            source_authority: String::new(),
            source_public_keys: vec![],
            provides: vec![],
            requires: vec![],
        }
    }

    fn manifest_with_agent_runtime_dependency() -> crate::app::config::AppManifest {
        toml::from_str(
            r#"
        [app]
        name = "contract-matrix-app"
        version = "0.1.0"
        display_name = "Contract Matrix App"
        description = "test"

        [requires]
        capabilities = ["agent.runtime"]

        [bindings.agent.runtime]
        provider = "agent-runtime"
        mutable = false
        "#,
        )
        .expect("manifest parses")
    }

    fn proxy_packages() -> Vec<DiscoveredPackage> {
        vec![DiscoveredPackage {
            manifest: toml::from_str(
                r#"
            [package_info]
            name = "agent-runtime-proxy"
            version = "0.1.0"
            description = "agent runtime proxy"
            entry = "package.wasm"

            [capability]
            provides = ["agent.runtime"]
            "#,
            )
            .expect("agent runtime proxy manifest parses"),
            dir: std::path::PathBuf::from("agent-runtime-proxy"),
            entry_path: None,
            runtime: PackageRuntime::Wasm,
        }]
    }

    fn manifest_package_index_with_service_proxy() -> PackageIndex {
        PackageIndex {
            version: 1,
            revision: "test-rev".into(),
            source_url: "local://packages".into(),
            package_sources: vec![PackageSource {
                name: "agent-runtime-proxy".into(),
                kind: "service".into(),
                package_kind: "provider".into(),
                runtime_provider: "agent-runtime".into(),
                current_source: "services/agent-runtime-proxy".into(),
                trusted: true,
                signature: "service-proxy".into(),
                source_authority: String::new(),
                source_public_keys: vec![],
                provides: vec!["agent.runtime".into()],
                requires: vec![],
            }],
        }
    }

    #[test]
    fn ui_metadata_prefers_existing_package_ui_declaration() {
        let repo_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
        let product_dir = repo_root.join("packages").join("weft-claw");
        let manifest = crate::app::load_product_package_declaration(&product_dir)
            .expect("product package declaration loads");
        let package_index = PackageIndex {
            version: 1,
            revision: "test-rev".into(),
            source_url: "local://packages".into(),
            package_sources: vec![
                PackageSource {
                    ..package_source("agent-runtime", "wasm", "packages/official/agent-core", "agent-runtime")
                },
                PackageSource {
                    ..package_source("skills-runtime", "wasm", "packages/official/skills", "skills-runtime")
                },
                PackageSource {
                    ..package_source("memory-store", "wasm", "packages/official/memory", "memory-store")
                },
                PackageSource {
                    ..package_source("channel-core", "wasm", "packages/official/channels", "channel-core")
                },
                PackageSource {
                    ..package_source(
                        "workflow-orchestrator",
                        "wasm",
                        "packages/official/workflow-orchestrator",
                        "workflow-orchestrator",
                    )
                },
                PackageSource {
                    ..package_source(
                        "team-runtime",
                        "wasm",
                        "packages/official/team-runtime",
                        "team-runtime",
                    )
                },
                PackageSource {
                    ..package_source(
                        "team-task-board",
                        "wasm",
                        "packages/official/team-task-board",
                        "team-task-board",
                    )
                },
                PackageSource {
                    ..package_source(
                        "workflow-template-devteam",
                        "wasm",
                        "packages/official/workflow-template-devteam",
                        "workflow-template-devteam",
                    )
                },
                PackageSource {
                    name: "weft-claw-ui".into(),
                    kind: "embedded".into(),
                    package_kind: "provider".into(),
                    runtime_provider: "weft-claw-ui".into(),
                    current_source: "packages/installed/weft-claw".into(),
                    trusted: true,
                    signature: "ed25519:/RckOFqgx1tk+3jNYC+h2ZH96/drE8WO1wLqyDXp9hg=:W5R67MWOB0WbCerqoefO7toDvZ5POEQ7NtLCLZGhi4Vgv/h5eF6SjfCShELaxa3OEki6KI7iTnRvMNPDKmPZBw==".into(),
                    source_authority: "local-installed".into(),
                    source_public_keys: vec!["/RckOFqgx1tk+3jNYC+h2ZH96/drE8WO1wLqyDXp9hg=".into()],
                    provides: vec!["ui.surface".into()],
                    requires: vec![],
                },
              ],
          };
        let packages = vec![
            DiscoveredPackage {
                manifest: toml::from_str(
                    r#"
[package_info]
name = "agent-runtime"
version = "0.1.0"
description = "agent"
entry = "package.wasm"

[capability]
provides = ["agent.runtime", "team.delegate"]
"#,
                )
                .expect("agent-core manifest parses"),
                dir: repo_root
                    .join("packages")
                    .join("official")
                    .join("agent-core"),
                entry_path: None,
                runtime: PackageRuntime::Wasm,
            },
            DiscoveredPackage {
                manifest: toml::from_str(
                    r#"
[package_info]
name = "skills-runtime"
version = "0.1.0"
description = "skills"
entry = "package.wasm"

[capability]
provides = ["ext.skills"]
"#,
                )
                .expect("skills manifest parses"),
                dir: repo_root.join("packages").join("official").join("skills"),
                entry_path: None,
                runtime: PackageRuntime::Wasm,
            },
            DiscoveredPackage {
                manifest: toml::from_str(
                    r#"
[package_info]
name = "memory-store"
version = "0.1.0"
description = "memory"
entry = "package.wasm"

[capability]
provides = ["memory.store"]
"#,
                )
                .expect("memory manifest parses"),
                dir: repo_root.join("packages").join("official").join("memory"),
                entry_path: None,
                runtime: PackageRuntime::Wasm,
            },
            DiscoveredPackage {
                manifest: toml::from_str(
                    r#"
[package_info]
name = "channel-core"
version = "0.1.0"
description = "channels"
entry = "package.wasm"

[capability]
provides = ["channel.bridge"]
"#,
                )
                .expect("channels manifest parses"),
                dir: repo_root.join("packages").join("official").join("channels"),
                entry_path: None,
                runtime: PackageRuntime::Wasm,
            },
            DiscoveredPackage {
                manifest: toml::from_str(
                    r#"
[package_info]
name = "workflow-orchestrator"
version = "0.1.0"
description = "workflow"
entry = "package.wasm"

[capability]
provides = ["workflow.orchestration"]
"#,
                )
                .expect("workflow-orchestrator manifest parses"),
                dir: repo_root
                    .join("packages")
                    .join("official")
                    .join("workflow-orchestrator"),
                entry_path: None,
                runtime: PackageRuntime::Wasm,
            },
            DiscoveredPackage {
                manifest: toml::from_str(
                    r#"
[package_info]
name = "team-runtime"
version = "0.1.0"
description = "team runtime"
entry = "package.wasm"

[capability]
provides = ["team.runtime", "team.role.catalog", "team.context.shared"]
"#,
                )
                .expect("team-runtime manifest parses"),
                dir: repo_root
                    .join("packages")
                    .join("official")
                    .join("team-runtime"),
                entry_path: None,
                runtime: PackageRuntime::Wasm,
            },
            DiscoveredPackage {
                manifest: toml::from_str(
                    r#"
[package_info]
name = "team-task-board"
version = "0.1.0"
description = "team task board"
entry = "package.wasm"

[capability]
provides = ["team.taskboard", "team.handoff"]
"#,
                )
                .expect("team-task-board manifest parses"),
                dir: repo_root
                    .join("packages")
                    .join("official")
                    .join("team-task-board"),
                entry_path: None,
                runtime: PackageRuntime::Wasm,
            },
            DiscoveredPackage {
                manifest: toml::from_str(
                    r#"
[package_info]
name = "workflow-template-devteam"
version = "0.1.0"
description = "devteam workflow template"
entry = "package.wasm"

[capability]
provides = ["workflow.template.devteam"]
"#,
                )
                .expect("workflow-template-devteam manifest parses"),
                dir: repo_root
                    .join("packages")
                    .join("official")
                    .join("workflow-template-devteam"),
                entry_path: None,
                runtime: PackageRuntime::Wasm,
            },
            DiscoveredPackage {
                manifest: toml::from_str(
                    r#"
[package_info]
name = "mcp-client"
version = "0.1.0"
description = "mcp"
entry = "package.wasm"

[capability]
provides = ["ext.mcp"]
"#,
                )
                .expect("mcp-client manifest parses"),
                dir: repo_root
                    .join("packages")
                    .join("official")
                    .join("mcp-client"),
                entry_path: None,
                runtime: PackageRuntime::Wasm,
            },
            DiscoveredPackage {
                manifest: toml::from_str(
                    r#"
[package_info]
name = "session-events"
version = "0.1.0"
description = "session events"
entry = "package.wasm"

[capability]
provides = ["session.events"]
"#,
                )
                .expect("session-events manifest parses"),
                dir: repo_root
                    .join("packages")
                    .join("official")
                    .join("session-events"),
                entry_path: None,
                runtime: PackageRuntime::Wasm,
            },
            DiscoveredPackage {
                manifest: toml::from_str(
                    r#"
[package_info]
name = "prompt-system"
version = "0.1.0"
description = "prompt system"
entry = "package.wasm"

[capability]
provides = ["prompt.system"]
"#,
                )
                .expect("prompt-system manifest parses"),
                dir: repo_root
                    .join("packages")
                    .join("official")
                    .join("prompt-system"),
                entry_path: None,
                runtime: PackageRuntime::Wasm,
            },
            DiscoveredPackage {
                manifest: toml::from_str(
                    r#"
[package_info]
name = "weft-claw"
version = "0.1.0"
description = "weft claw runtime"
entry = "package.wasm"

[capability]
provides = ["weft_claw.turn", "ui.surface"]
"#,
                )
                .expect("weft-claw manifest parses"),
                dir: repo_root
                    .join("packages")
                    .join("official")
                    .join("weft-claw"),
                entry_path: None,
                runtime: PackageRuntime::Wasm,
            },
            DiscoveredPackage {
                manifest: toml::from_str(
                    r#"
[package_info]
name = "weft-claw-ui"
version = "0.1.0"
description = "Weft Claw UI shell"

[ui]
title = "Weft Claw"
icon = "robot"
mode = "native:ai-director"

[capability]
provides = ["ui.surface"]
"#,
                )
                .expect("weft-claw ui manifest parses"),
                dir: repo_root
                    .join("packages")
                    .join("installed")
                    .join("weft-claw"),
                entry_path: None,
                runtime: PackageRuntime::Wasm,
            },
        ];

        let mut registry = CapabilityRegistry::new();
        registry.insert(
            "core.execution".into(),
            crate::app::CapabilityRegistryEntry {
                capability: "core.execution".into(),
                providers: vec![crate::app::CapabilityProviderRecord {
                    provider: "core".into(),
                    runtime: "core".into(),
                    priority: 0,
                }],
                bindings: vec![],
            },
        );

        let resolved = resolve_app_manifest_with_policy(
            &manifest,
            &packages,
            None,
            None,
            Some(&registry),
            Some(&package_index),
        )
        .expect("app resolves");

        assert_eq!(resolved.status, ResolvedAppStatus::Resolved);

        let matched_package = packages
            .iter()
            .find(|package| package.manifest.package_info.name == "weft-claw")
            .expect("weft-claw runtime package discovered");
        assert!(matches!(matched_package.runtime, PackageRuntime::Wasm));
    }

    #[test]
    fn resolve_with_candidate_context_can_resolve_aliased_provider() {
        let manifest: crate::app::config::AppManifest = toml::from_str(
            r#"
        [app]
        name = "alias-app"
        version = "0.1.0"
        display_name = "Alias App"
        description = "test"

        [requires]
        capabilities = ["agent.runtime"]

        [bindings.agent.runtime]
        provider = "agent-runtime-alias"
        mutable = false
        "#,
        )
        .expect("manifest parses");

        let packages = vec![DiscoveredPackage {
            manifest: toml::from_str(
                r#"
            [package_info]
            name = "agent-runtime"
            version = "0.1.0"
            description = "agent"
            entry = "package.wasm"

            [capability]
            provides = ["agent.runtime"]
            "#,
            )
            .expect("agent manifest parses"),
            dir: std::path::PathBuf::from("agent-runtime"),
            entry_path: None,
            runtime: PackageRuntime::Wasm,
        }];

        let err = resolve_app_manifest_with_policy(&manifest, &packages, None, None, None, None)
            .expect_err("missing alias should fail without external candidates");

        assert!(err.to_string().contains(
            "Provider 'agent-runtime-alias' for capability 'agent.runtime' is not available"
        ));

        let mut resolve_candidate_context = ResolveCandidateContext::default();
        resolve_candidate_context.add_candidates("agent-runtime-alias", vec!["agent-runtime"]);

        let resolved = resolve_app_manifest_with_policy_and_candidate_context(
            &manifest,
            &packages,
            None,
            None,
            None,
            None,
            &resolve_candidate_context,
        )
        .expect("external candidate source should resolve alias provider");

        assert_eq!(resolved.status, ResolvedAppStatus::Resolved);
        assert_eq!(resolved.bindings[0].capability, "agent.runtime");
        assert_eq!(resolved.bindings[0].provider, "agent-runtime-alias");
        assert_eq!(resolved.bindings.len(), 1);
    }

    #[test]
    fn candidate_context_can_store_grouping_metadata() {
        let mut context = ResolveCandidateContext::default();

        context.add_closure_metadata(
            "agent-runtime-alias",
            "closure-1",
            ResolveCandidateGroupingSource::Local,
            vec!["agent-runtime", "agent-runtime-v2"],
        );
        context.add_recommendation_metadata(
            "agent-runtime-alias",
            "recommendation-1",
            ResolveCandidateGroupingSource::Service,
            vec!["agent-runtime", "agent-runtime-pro"],
            Some("preferred runtime set".to_string()),
        );

        let groups = context.grouping_for("agent-runtime-alias");
        assert_eq!(groups.len(), 2);

        let closure = groups
            .iter()
            .find(|group| matches!(group.kind, ResolveCandidateGroupingKind::Closure))
            .expect("closure grouping should exist");
        assert_eq!(closure.id, "closure-1");
        assert_eq!(closure.candidates.len(), 2);

        let recommendation = groups
            .iter()
            .find(|group| matches!(group.kind, ResolveCandidateGroupingKind::RecommendationSet))
            .expect("recommendation grouping should exist");
        assert_eq!(
            recommendation.rationale,
            Some("preferred runtime set".to_string())
        );
    }

    #[test]
    fn service_origin_can_supply_synthesized_candidates() {
        let manifest: crate::app::config::AppManifest = toml::from_str(
            r#"
        [app]
        name = "service-origin-app"
        version = "0.1.0"
        display_name = "Service-Origin App"
        description = "test"

        [requires]
        capabilities = ["agent.runtime"]

        [bindings.agent.runtime]
        provider = "agent-runtime"
        mutable = false
        "#,
        )
        .expect("manifest parses");

        let packages = vec![DiscoveredPackage {
            manifest: toml::from_str(
                r#"
            [package_info]
            name = "agent-runtime-proxy"
            version = "0.1.0"
            description = "agent proxy"
            entry = "package.wasm"

            [capability]
            provides = ["agent.runtime"]
            "#,
            )
            .expect("agent runtime proxy manifest parses"),
            dir: std::path::PathBuf::from("agent-runtime-proxy"),
            entry_path: None,
            runtime: PackageRuntime::Wasm,
        }];

        let package_index = PackageIndex {
            version: 1,
            revision: "test-rev".into(),
            source_url: "local://packages".into(),
            package_sources: vec![PackageSource {
                name: "agent-runtime-proxy".into(),
                kind: "service".into(),
                package_kind: "provider".into(),
                runtime_provider: "agent-runtime".into(),
                current_source: "services/agent-runtime-proxy".into(),
                trusted: true,
                signature: "service-proxy".into(),
                source_authority: String::new(),
                source_public_keys: vec![],
                provides: vec!["agent.runtime".into()],
                requires: vec![],
            }],
        };

        let err = resolve_app_manifest_with_policy_and_candidate_context(
            &manifest,
            &packages,
            None,
            None,
            None,
            None,
            &ResolveCandidateContext::from_package_index(&package_index),
        )
        .expect_err("missing synthesized service candidates should fail without helper");

        assert!(err
            .to_string()
            .contains("Provider 'agent-runtime' for capability 'agent.runtime' is not available"));

        let resolve_candidate_context =
            ResolveCandidateContext::from_package_index_with_service_candidates(&package_index);

        let resolved = resolve_app_manifest_with_policy_and_candidate_context(
            &manifest,
            &packages,
            None,
            None,
            None,
            None,
            &resolve_candidate_context,
        )
        .expect("service-origin candidates should resolve provider alias");

        let groups = resolve_candidate_context.grouping_for("agent-runtime");
        assert_eq!(groups.len(), 1);
        assert!(matches!(
            groups[0].source,
            ResolveCandidateGroupingSource::Service
        ));

        assert_eq!(resolved.status, ResolvedAppStatus::Resolved);
        assert_eq!(resolved.bindings[0].provider, "agent-runtime");
    }

    #[test]
    fn resolve_input_coordinator_prepares_service_candidates() {
        use crate::app::ResolveCandidateContext;

        let package_index = PackageIndex {
            version: 1,
            revision: "test-rev".into(),
            source_url: "local://packages".into(),
            package_sources: vec![PackageSource {
                name: "agent-runtime-proxy".into(),
                kind: "service".into(),
                package_kind: "provider".into(),
                runtime_provider: "agent-runtime".into(),
                current_source: "services/agent-runtime-proxy".into(),
                trusted: true,
                signature: "service-proxy".into(),
                source_authority: String::new(),
                source_public_keys: vec![],
                provides: vec!["agent.runtime".into()],
                requires: vec![],
            }],
        };

        let coordinator = ResolveInputCoordinator::from_package_index(&package_index)
            .with_synthesized_service_origin_candidates()
            .build();

        let groups = coordinator
            .resolve_candidate_context()
            .grouping_for("agent-runtime");

        let mut has_service_closure = false;
        for group in groups {
            if matches!(group.source, ResolveCandidateGroupingSource::Service)
                && matches!(group.kind, ResolveCandidateGroupingKind::Closure)
                && group
                    .candidates
                    .iter()
                    .any(|candidate| candidate == "agent-runtime")
            {
                has_service_closure = true;
                break;
            }
        }
        assert!(has_service_closure);

        let base_context = ResolveCandidateContext::from_package_index(&package_index);
        let base_groupings = base_context.grouping_for("agent-runtime");
        assert_eq!(base_groupings.len(), 0);
    }

    #[test]
    fn service_origin_fixture_payload_maps_multiple_dto_shapes() {
        let fixture_payload = vec![
            ServiceOriginCandidatePayloadFixture {
                provider: "  agent-runtime  ".to_string(),
                candidates: vec![
                    "agent-runtime-proxy".to_string(),
                    "  agent-runtime  ".to_string(),
                    "   ".to_string(),
                ],
                closure_candidates: vec![
                    "  ".to_string(),
                    "agent-runtime".to_string(),
                    "agent-runtime-proxy".to_string(),
                ],
                closure_id: Some("   ".to_string()),
                closure_rationale: Some("   ".to_string()),
                provenance: Some(PackageCandidateProvenance {
                    package_name: "agent-runtime-proxy".into(),
                    package_kind: "provider".into(),
                    runtime_provider: "agent-runtime".into(),
                    current_source: "services/agent-runtime-proxy".into(),
                    source_kind: "service".into(),
                }),
            },
            ServiceOriginCandidatePayloadFixture {
                provider: "  ".to_string(),
                candidates: vec!["should-not-appear".to_string()],
                closure_candidates: vec!["agent-runtime".to_string()],
                closure_id: None,
                closure_rationale: None,
                provenance: None,
            },
            ServiceOriginCandidatePayloadFixture {
                provider: "memory-provider".to_string(),
                candidates: vec!["memory-provider".to_string()],
                closure_candidates: vec!["memory-provider".to_string()],
                closure_id: Some("memory-provider:closure".to_string()),
                closure_rationale: Some("provider overlap test".to_string()),
                provenance: None,
            },
        ];

        let payload = ServiceOriginCandidatePayload::from_fixtures(&fixture_payload);

        assert_eq!(payload.len(), 2);

        let runtime_payload = payload
            .iter()
            .find(|entry| entry.provider == "agent-runtime");
        let runtime_payload = runtime_payload.expect("runtime payload maps");

        assert_eq!(
            runtime_payload.candidates[0].candidate,
            "agent-runtime-proxy"
        );
        assert_eq!(runtime_payload.candidates[1].candidate, "agent-runtime");
        assert_eq!(
            runtime_payload.closure_metadata.id,
            "service-fixture:agent-runtime:closure"
        );
        assert_eq!(
            runtime_payload.closure_metadata.rationale,
            Some("service-candidate-fixture".to_string())
        );
        assert_eq!(
            runtime_payload.closure_metadata.candidates,
            vec![
                "agent-runtime".to_string(),
                "agent-runtime-proxy".to_string()
            ]
        );
        assert!(matches!(
            runtime_payload.candidates[0]
                .provenance
                .as_ref()
                .expect("provenance maps")
                .package_name
                .as_str(),
            "agent-runtime-proxy"
        ));

        let mut context = ResolveCandidateContext::default();
        context = context
            .with_service_origin_candidates(&SynthesizedServiceOriginCandidateAdapter, &payload);

        let resolved = resolve_app_manifest_with_policy_and_candidate_context(
            &manifest_with_agent_runtime_dependency(),
            &proxy_packages(),
            None,
            None,
            None,
            None,
            &context,
        )
        .expect("fixture mapper payload should drive resolution");

        assert_eq!(resolved.status, ResolvedAppStatus::Resolved);
        assert_eq!(resolved.bindings[0].provider, "agent-runtime");
        assert_eq!(
            context.candidates_for("agent-runtime").len(),
            2,
            "fixture entry should include overlap and non-empty candidate values"
        );
        assert!(matches!(
            context.candidates_for("agent-runtime")[1].provenance,
            ResolveCandidateProvenance::PackageIndex(_)
        ));
    }

    #[test]
    fn service_origin_fixture_payload_trims_and_defaults_optional_fields() {
        let fixture_payload = vec![
            ServiceOriginCandidatePayloadFixture {
                provider: "  memory-provider  ".to_string(),
                candidates: vec![
                    "  ".to_string(),
                    "memory-provider".to_string(),
                    "memory-provider-proxy".to_string(),
                ],
                closure_candidates: vec!["  ".to_string()],
                closure_id: None,
                closure_rationale: Some("  ".to_string()),
                provenance: Some(PackageCandidateProvenance {
                    package_name: "memory-provider-proxy".into(),
                    package_kind: "provider".into(),
                    runtime_provider: "memory-provider".into(),
                    current_source: "services/memory-provider-proxy".into(),
                    source_kind: "service".into(),
                }),
            },
            ServiceOriginCandidatePayloadFixture {
                provider: "   ".to_string(),
                candidates: vec!["ignore-me".to_string()],
                closure_candidates: vec!["memory-provider".to_string()],
                closure_id: Some("memory-provider:closure".to_string()),
                closure_rationale: Some("unused".to_string()),
                provenance: None,
            },
        ];

        let payload = ServiceOriginCandidatePayload::from_fixtures(&fixture_payload);

        assert_eq!(payload.len(), 1);

        let memory_payload = &payload[0];
        assert_eq!(memory_payload.provider, "memory-provider");
        assert_eq!(
            memory_payload.closure_metadata.id,
            "service-fixture:memory-provider:closure"
        );
        assert_eq!(
            memory_payload.closure_metadata.candidates,
            vec!["memory-provider".to_string()]
        );
        assert_eq!(
            memory_payload.closure_metadata.rationale,
            Some("service-candidate-fixture".to_string())
        );
        assert_eq!(memory_payload.candidates.len(), 2);
        assert_eq!(memory_payload.candidates[0].candidate, "memory-provider");
        assert_eq!(
            memory_payload.candidates[1].candidate,
            "memory-provider-proxy"
        );
    }

    #[test]
    fn service_contract_payload_handles_optional_closure_fields_and_provenance() {
        let contract_payload = vec![
            ServiceContractCandidatePayload {
                provider: "  memory-provider  ".to_string(),
                candidates: vec![
                    "memory-provider-proxy".to_string(),
                    "memory-provider".to_string(),
                ],
                closure: ServiceContractCandidateClosure {
                    id: None,
                    candidates: vec!["   ".to_string(), "memory-provider".to_string()],
                    rationale: Some("  ".to_string()),
                },
                provenance: Some(ServiceContractCandidateProvenance {
                    package_name: "memory-provider-proxy".into(),
                    package_kind: "provider".into(),
                    runtime_provider: "memory-provider".into(),
                    current_source: "services/memory-provider-proxy".into(),
                    source_kind: "service".into(),
                }),
            },
            ServiceContractCandidatePayload {
                provider: "  \tagent-runtime\t  ".to_string(),
                candidates: vec!["   ".to_string(), "agent-runtime-proxy".to_string()],
                closure: ServiceContractCandidateClosure {
                    id: Some("\t".to_string()),
                    candidates: vec!["agent-runtime".to_string()],
                    rationale: None,
                },
                provenance: None,
            },
        ];

        let payload =
            ServiceOriginCandidatePayload::from_service_contract_payload(&contract_payload);

        assert_eq!(payload.len(), 2);

        let memory_payload = payload
            .iter()
            .find(|entry| entry.provider == "memory-provider")
            .expect("memory provider should map");
        assert_eq!(
            memory_payload.closure_metadata.id,
            "service-contract:memory-provider:closure"
        );
        assert_eq!(memory_payload.candidates.len(), 2);
        assert_eq!(
            memory_payload.candidates[0].candidate,
            "memory-provider-proxy"
        );
        assert_eq!(memory_payload.candidates[1].candidate, "memory-provider");
        for candidate in &memory_payload.candidates {
            let provenance = candidate
                .provenance
                .as_ref()
                .expect("provenance should be propagated");
            assert_eq!(provenance.package_name, "memory-provider-proxy");
            assert_eq!(provenance.package_kind, "provider");
            assert_eq!(provenance.runtime_provider, "memory-provider");
            assert_eq!(provenance.current_source, "services/memory-provider-proxy");
            assert_eq!(provenance.source_kind, "service");
        }
        assert_eq!(
            memory_payload.closure_metadata.candidates,
            vec!["memory-provider".to_string()]
        );
        assert_eq!(
            memory_payload.closure_metadata.rationale,
            Some("service-candidate-contract".to_string())
        );

        let runtime_payload = payload
            .iter()
            .find(|entry| entry.provider == "agent-runtime")
            .expect("agent runtime should map");
        assert_eq!(
            runtime_payload.closure_metadata.id,
            "service-contract:agent-runtime:closure"
        );
        assert_eq!(runtime_payload.candidates.len(), 1);
        assert_eq!(
            runtime_payload.candidates[0].candidate,
            "agent-runtime-proxy"
        );
        assert!(runtime_payload.candidates[0].provenance.is_none());

        let context = ResolveCandidateContext::default()
            .with_service_origin_candidates(&SynthesizedServiceOriginCandidateAdapter, &payload);

        assert!(matches!(
            context.candidates_for("memory-provider")[0].provenance,
            ResolveCandidateProvenance::PackageIndex(_)
        ));
        assert!(matches!(
            context.candidates_for("agent-runtime")[0].provenance,
            ResolveCandidateProvenance::External
        ));

        let package_index = manifest_package_index_with_service_proxy();
        let coordinator = ResolveInputCoordinator::from_package_index(&package_index)
            .with_service_origin_candidates(&SynthesizedServiceOriginCandidateAdapter, &payload)
            .build();

        let groups = coordinator
            .resolve_candidate_context()
            .grouping_for("memory-provider");
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].id, "service-contract:memory-provider:closure");
        assert_eq!(groups[0].candidates, vec!["memory-provider".to_string()]);
    }

    #[test]
    fn service_origin_fixture_payload_handles_optional_closure_fields_and_overlap() {
        let fixture_payload = vec![ServiceOriginCandidatePayloadFixture {
            provider: "  memory-provider  ".to_string(),
            candidates: vec![
                "memory-provider".to_string(),
                "memory-provider-proxy".to_string(),
                "   ".to_string(),
            ],
            closure_candidates: vec!["  ".to_string()],
            closure_id: None,
            closure_rationale: None,
            provenance: Some(PackageCandidateProvenance {
                package_name: "memory-provider-proxy".into(),
                package_kind: "provider".into(),
                runtime_provider: "memory-provider".into(),
                current_source: "services/memory-provider-proxy".into(),
                source_kind: "service".into(),
            }),
        }];

        let payload = ServiceOriginCandidatePayload::from_fixtures(&fixture_payload);

        assert_eq!(payload.len(), 1);

        let memory_payload = &payload[0];
        assert_eq!(memory_payload.provider, "memory-provider");
        assert_eq!(
            memory_payload.closure_metadata.id,
            "service-fixture:memory-provider:closure"
        );
        assert_eq!(
            memory_payload.closure_metadata.rationale,
            Some("service-candidate-fixture".to_string())
        );
        assert_eq!(
            memory_payload.closure_metadata.candidates,
            vec!["memory-provider".to_string()]
        );
        assert_eq!(memory_payload.candidates[0].candidate, "memory-provider");
        assert_eq!(
            memory_payload.candidates[1].candidate,
            "memory-provider-proxy"
        );
        assert_eq!(memory_payload.candidates.len(), 2);

        let context = ResolveCandidateContext::default()
            .with_service_origin_candidates(&SynthesizedServiceOriginCandidateAdapter, &payload);

        let candidates = context.candidates_for("memory-provider");
        assert_eq!(candidates.len(), 2);
        assert_eq!(candidates[0].candidate, "memory-provider");
        assert_eq!(candidates[1].candidate, "memory-provider-proxy");
        assert!(matches!(
            candidates[0].provenance,
            ResolveCandidateProvenance::PackageIndex(_)
        ));
    }

    #[test]
    fn service_contract_payload_maps_defaults_and_drives_resolution() {
        let contract_payload = vec![
            ServiceContractCandidatePayload {
                provider: " agent-runtime ".to_string(),
                candidates: vec![
                    " agent-runtime-proxy ".to_string(),
                    "   ".to_string(),
                    "agent-runtime".to_string(),
                ],
                closure: ServiceContractCandidateClosure {
                    id: Some("  ".to_string()),
                    candidates: vec!["  ".to_string(), " agent-runtime ".to_string()],
                    rationale: Some("   ".to_string()),
                },
                provenance: None,
            },
            ServiceContractCandidatePayload {
                provider: "  ".to_string(),
                candidates: vec!["agent-runtime-proxy".to_string()],
                closure: ServiceContractCandidateClosure {
                    id: None,
                    candidates: vec!["agent-runtime".to_string()],
                    rationale: None,
                },
                provenance: Some(ServiceContractCandidateProvenance {
                    package_name: "agent-runtime-proxy".into(),
                    package_kind: "provider".into(),
                    runtime_provider: "agent-runtime".into(),
                    current_source: "services/agent-runtime-proxy".into(),
                    source_kind: "service".into(),
                }),
            },
        ];

        let payload =
            ServiceOriginCandidatePayload::from_service_contract_payload(&contract_payload);

        assert_eq!(payload.len(), 1);
        let mapped = &payload[0];

        assert_eq!(mapped.provider, "agent-runtime");
        assert_eq!(
            mapped.closure_metadata.id,
            "service-contract:agent-runtime:closure"
        );
        assert_eq!(
            mapped.closure_metadata.rationale,
            Some("service-candidate-contract".to_string())
        );
        assert_eq!(mapped.candidates[0].candidate, "agent-runtime-proxy");
        assert_eq!(mapped.candidates[1].candidate, "agent-runtime");

        let mut context = ResolveCandidateContext::default();
        context = context
            .with_service_origin_candidates(&SynthesizedServiceOriginCandidateAdapter, &payload);

        assert!(matches!(
            context.candidates_for("agent-runtime")[0].provenance,
            ResolveCandidateProvenance::External
        ));

        let resolved = resolve_app_manifest_with_policy_and_candidate_context(
            &manifest_with_agent_runtime_dependency(),
            &proxy_packages(),
            None,
            None,
            None,
            Some(&manifest_package_index_with_service_proxy()),
            &context,
        )
        .expect("contract compatibility payload should resolve");

        assert_eq!(resolved.status, ResolvedAppStatus::Resolved);
        assert_eq!(resolved.bindings[0].provider, "agent-runtime");

        let proxy_index = manifest_package_index_with_service_proxy();
        let coordinator = ResolveInputCoordinator::from_package_index(&proxy_index)
            .with_service_origin_candidates(&SynthesizedServiceOriginCandidateAdapter, &payload)
            .build();

        let groups = coordinator
            .resolve_candidate_context()
            .grouping_for("agent-runtime");
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].id, "service-contract:agent-runtime:closure");
        assert_eq!(
            groups[0].rationale,
            Some("service-candidate-contract".to_string())
        );
    }

    #[test]
    fn from_service_contract_payload_trims_provider_and_skips_empty_values() {
        let contract_payload = vec![
            ServiceContractCandidatePayload {
                provider: "  agent-runtime  ".to_string(),
                candidates: vec![
                    "   ".to_string(),
                    "agent-runtime-proxy".to_string(),
                    "\t".to_string(),
                    "agent-runtime".to_string(),
                ],
                closure: ServiceContractCandidateClosure {
                    id: None,
                    candidates: vec!["  ".to_string(), "agent-runtime".to_string()],
                    rationale: None,
                },
                provenance: None,
            },
            ServiceContractCandidatePayload {
                provider: "   ".to_string(),
                candidates: vec!["agent-runtime-proxy".to_string()],
                closure: ServiceContractCandidateClosure {
                    id: None,
                    candidates: vec!["agent-runtime".to_string()],
                    rationale: None,
                },
                provenance: None,
            },
        ];

        let payload =
            ServiceOriginCandidatePayload::from_service_contract_payload(&contract_payload);

        assert_eq!(payload.len(), 1);
        assert_eq!(payload[0].provider, "agent-runtime");
        assert_eq!(payload[0].candidates.len(), 2);
        assert_eq!(payload[0].candidates[0].candidate, "agent-runtime-proxy");
        assert_eq!(payload[0].candidates[1].candidate, "agent-runtime");
        assert_eq!(
            payload[0].closure_metadata.candidates,
            vec!["agent-runtime".to_string()]
        );
        assert_eq!(
            payload[0].closure_metadata.id,
            "service-contract:agent-runtime:closure"
        );
        assert_eq!(
            payload[0].closure_metadata.rationale,
            Some("service-candidate-contract".to_string())
        );
    }

    #[test]
    fn from_service_contract_payload_injects_provider_into_closure_without_duplicates() {
        let contract_payload = vec![ServiceContractCandidatePayload {
            provider: "agent-runtime".to_string(),
            candidates: vec![
                "agent-runtime".to_string(),
                "agent-runtime-proxy".to_string(),
            ],
            closure: ServiceContractCandidateClosure {
                id: Some("custom-closure-id".to_string()),
                candidates: vec!["agent-runtime".to_string()],
                rationale: Some("service contract overlap".to_string()),
            },
            provenance: None,
        }];

        let payload =
            ServiceOriginCandidatePayload::from_service_contract_payload(&contract_payload);

        assert_eq!(payload.len(), 1);
        assert_eq!(payload[0].provider, "agent-runtime");
        assert_eq!(payload[0].candidates[0].candidate, "agent-runtime");
        assert_eq!(payload[0].candidates[1].candidate, "agent-runtime-proxy");
        assert_eq!(
            payload[0].closure_metadata.candidates,
            vec!["agent-runtime".to_string()]
        );
        assert_eq!(payload[0].closure_metadata.id, "custom-closure-id");
        assert_eq!(
            payload[0].closure_metadata.rationale,
            Some("service contract overlap".to_string())
        );
    }

    #[test]
    fn from_service_contract_payload_falls_back_when_closure_fields_are_blank() {
        let contract_payload = vec![ServiceContractCandidatePayload {
            provider: "agent-runtime".to_string(),
            candidates: vec!["agent-runtime-proxy".to_string()],
            closure: ServiceContractCandidateClosure {
                id: Some("   ".to_string()),
                candidates: vec!["agent-runtime".to_string()],
                rationale: Some("   ".to_string()),
            },
            provenance: None,
        }];

        let payload =
            ServiceOriginCandidatePayload::from_service_contract_payload(&contract_payload);

        assert_eq!(payload.len(), 1);
        assert_eq!(
            payload[0].closure_metadata.id,
            "service-contract:agent-runtime:closure"
        );
        assert_eq!(
            payload[0].closure_metadata.rationale,
            Some("service-candidate-contract".to_string())
        );
    }

    #[test]
    fn service_contract_payload_provenance_option_drives_coordinator_resolution_path() {
        let manifest: crate::app::AppManifest = toml::from_str(
            r#"
        [app]
        name = "contract-provenance-app"
        version = "0.1.0"
        display_name = "Contract Provenance App"
        description = "test"

        [requires]
        capabilities = ["agent.runtime"]

        [bindings.agent.runtime]
        provider = "agent-runtime"
        mutable = false
        "#,
        )
        .expect("manifest parses");

        let contract_payload = vec![
            ServiceContractCandidatePayload {
                provider: "agent-runtime".to_string(),
                candidates: vec!["agent-runtime-proxy".to_string()],
                closure: ServiceContractCandidateClosure {
                    id: None,
                    candidates: vec!["agent-runtime".to_string()],
                    rationale: Some("contract provenance path".to_string()),
                },
                provenance: Some(ServiceContractCandidateProvenance {
                    package_name: "agent-runtime-proxy".into(),
                    package_kind: "provider".into(),
                    runtime_provider: "agent-runtime".into(),
                    current_source: "services/agent-runtime-proxy".into(),
                    source_kind: "service".into(),
                }),
            },
            ServiceContractCandidatePayload {
                provider: "agent-runtime-alt".to_string(),
                candidates: vec!["agent-runtime-alt".to_string()],
                closure: ServiceContractCandidateClosure {
                    id: None,
                    candidates: vec!["agent-runtime-alt".to_string()],
                    rationale: None,
                },
                provenance: None,
            },
        ];

        let payload =
            ServiceOriginCandidatePayload::from_service_contract_payload(&contract_payload);

        let mapped_with_provenance = payload
            .iter()
            .find(|entry| entry.provider == "agent-runtime")
            .expect("mapped contract payload contains agent-runtime")
            .candidates[0]
            .provenance
            .as_ref()
            .expect("provenance is preserved");

        assert_eq!(mapped_with_provenance.package_name, "agent-runtime-proxy");
        assert_eq!(mapped_with_provenance.package_kind, "provider");
        assert_eq!(mapped_with_provenance.runtime_provider, "agent-runtime");

        let mapped_without_provenance = payload
            .iter()
            .find(|entry| entry.provider == "agent-runtime-alt")
            .expect("mapped contract payload contains agent-runtime-alt")
            .candidates[0]
            .provenance
            .as_ref();

        assert!(mapped_without_provenance.is_none());

        let package_index = manifest_package_index_with_service_proxy();
        let coordinator = ResolveInputCoordinator::from_package_index(&package_index)
            .with_service_origin_candidates(&SynthesizedServiceOriginCandidateAdapter, &payload)
            .build();

        let contract_candidates = coordinator
            .resolve_candidate_context()
            .candidates_for("agent-runtime");
        assert_eq!(contract_candidates.len(), 1);
        assert_eq!(contract_candidates[0].candidate, "agent-runtime-proxy");
        let contract_candidate_provenance = match &contract_candidates[0].provenance {
            ResolveCandidateProvenance::PackageIndex(provenance) => provenance,
            ResolveCandidateProvenance::External => {
                panic!("contract provenance should be converted to package index provenance")
            }
        };
        assert_eq!(
            contract_candidate_provenance.package_name,
            "agent-runtime-proxy"
        );

        let alt_candidates = coordinator
            .resolve_candidate_context()
            .candidates_for("agent-runtime-alt");
        assert_eq!(alt_candidates.len(), 1);
        assert_eq!(alt_candidates[0].candidate, "agent-runtime-alt");
        assert!(matches!(
            alt_candidates[0].provenance,
            ResolveCandidateProvenance::External
        ));

        let groups = coordinator
            .resolve_candidate_context()
            .grouping_for("agent-runtime");
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].id, "service-contract:agent-runtime:closure");
        assert_eq!(
            groups[0].rationale,
            Some("contract provenance path".to_string())
        );

        let resolved = resolve_app_manifest_with_policy_and_candidate_context(
            &manifest,
            &proxy_packages(),
            None,
            None,
            None,
            Some(&package_index),
            coordinator.resolve_candidate_context(),
        )
        .expect("contract provenance payload should resolve via coordinator path");

        assert_eq!(resolved.status, ResolvedAppStatus::Resolved);
        assert_eq!(resolved.bindings[0].provider, "agent-runtime");
    }

    #[test]
    fn fixture_service_origin_payload_drives_coordinator_path() {
        let manifest: crate::app::AppManifest = toml::from_str(
            r#"
        [app]
        name = "fixture-service-origin-app"
        version = "0.1.0"
        display_name = "Fixture Service Origin App"
        description = "test"

        [requires]
        capabilities = ["agent.runtime"]

        [bindings.agent.runtime]
        provider = "agent-runtime"
        mutable = false
        "#,
        )
        .expect("manifest parses");

        let packages = vec![DiscoveredPackage {
            manifest: toml::from_str(
                r#"
            [package_info]
            name = "agent-runtime-proxy"
            version = "0.1.0"
            description = "agent runtime proxy"
            entry = "package.wasm"

            [capability]
            provides = ["agent.runtime"]
            "#,
            )
            .expect("agent runtime proxy manifest parses"),
            dir: std::path::PathBuf::from("agent-runtime-proxy"),
            entry_path: None,
            runtime: PackageRuntime::Wasm,
        }];

        let err = resolve_app_manifest_with_policy_and_candidate_context(
            &manifest,
            &packages,
            None,
            None,
            None,
            None,
            &ResolveCandidateContext::default(),
        )
        .expect_err("missing fixture candidates should fail");

        assert!(err
            .to_string()
            .contains("Provider 'agent-runtime' for capability 'agent.runtime' is not available"));

        let fixture_payload = vec![ServiceOriginCandidatePayloadFixture {
            provider: "agent-runtime".to_string(),
            candidates: vec!["agent-runtime-proxy".to_string()],
            closure_candidates: vec!["agent-runtime".to_string()],
            closure_id: Some("fixture:agent-runtime:closure".to_string()),
            closure_rationale: Some("fixture-based service origin".to_string()),
            provenance: None,
        }];

        let payload = ServiceOriginCandidatePayload::from_fixtures(&fixture_payload);
        let mut resolve_candidate_context = ResolveCandidateContext::default();
        resolve_candidate_context = resolve_candidate_context
            .with_service_origin_candidates(&SynthesizedServiceOriginCandidateAdapter, &payload);

        let fixture_package_index = PackageIndex {
            version: 1,
            revision: "test-rev".into(),
            source_url: "local://packages".into(),
            package_sources: vec![],
        };

        let coordinator = ResolveInputCoordinator::from_package_index(&fixture_package_index)
            .with_service_origin_candidates(&SynthesizedServiceOriginCandidateAdapter, &payload)
            .build();

        let resolved_with_coordinator = resolve_app_manifest_with_policy_and_candidate_context(
            &manifest,
            &packages,
            None,
            None,
            None,
            None,
            coordinator.resolve_candidate_context(),
        )
        .expect("fixture payload should resolve via coordinator path");

        assert_eq!(
            resolved_with_coordinator.status,
            ResolvedAppStatus::Resolved
        );
        assert_eq!(
            resolved_with_coordinator.bindings[0].provider,
            "agent-runtime"
        );

        let candidates = resolve_candidate_context.candidates_for("agent-runtime");
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].candidate, "agent-runtime-proxy");

        let resolved = resolve_app_manifest_with_policy_and_candidate_context(
            &manifest,
            &packages,
            None,
            None,
            None,
            None,
            &resolve_candidate_context,
        )
        .expect("fixture payload should resolve via direct context path");

        assert_eq!(resolved.status, ResolvedAppStatus::Resolved);
        assert_eq!(resolved.bindings[0].provider, "agent-runtime");

        let groups = coordinator
            .resolve_candidate_context()
            .grouping_for("agent-runtime");
        assert_eq!(groups.len(), 1);
        assert!(matches!(
            groups[0].source,
            ResolveCandidateGroupingSource::Service
        ));
        assert_eq!(groups[0].id, "fixture:agent-runtime:closure");
        assert_eq!(groups[0].candidates, vec!["agent-runtime".to_string()]);
        assert_eq!(
            groups[0].rationale,
            Some("fixture-based service origin".to_string())
        );
    }

    #[test]
    fn service_contract_payload_drives_resolve_input_path() {
        let manifest: crate::app::AppManifest = toml::from_str(
            r#"
        [app]
        name = "contract-service-origin-app"
        version = "0.1.0"
        display_name = "Contract Service Origin App"
        description = "test"

        [requires]
        capabilities = ["agent.runtime"]

        [bindings.agent.runtime]
        provider = "agent-runtime"
        mutable = false
        "#,
        )
        .expect("manifest parses");

        let packages = vec![DiscoveredPackage {
            manifest: toml::from_str(
                r#"
            [package_info]
            name = "agent-runtime-proxy"
            version = "0.1.0"
            description = "agent runtime proxy"
            entry = "package.wasm"

            [capability]
            provides = ["agent.runtime"]
            "#,
            )
            .expect("agent runtime proxy manifest parses"),
            dir: std::path::PathBuf::from("agent-runtime-proxy"),
            entry_path: None,
            runtime: PackageRuntime::Wasm,
        }];

        let package_index = PackageIndex {
            version: 1,
            revision: "test-rev".into(),
            source_url: "local://packages".into(),
            package_sources: vec![PackageSource {
                name: "agent-runtime-proxy".into(),
                kind: "service".into(),
                package_kind: "provider".into(),
                runtime_provider: "agent-runtime".into(),
                current_source: "services/agent-runtime-proxy".into(),
                trusted: true,
                signature: "service-proxy".into(),
                source_authority: String::new(),
                source_public_keys: vec![],
                provides: vec!["agent.runtime".into()],
                requires: vec![],
            }],
        };

        let contract_payload = vec![ServiceContractCandidatePayload {
            provider: "agent-runtime".into(),
            candidates: vec!["agent-runtime-proxy".into()],
            closure: ServiceContractCandidateClosure {
                id: Some("service-contract:agent-runtime:closure".into()),
                candidates: vec!["agent-runtime".into()],
                rationale: Some("contract-driven service origin".into()),
            },
            provenance: Some(ServiceContractCandidateProvenance {
                package_name: "agent-runtime-proxy".into(),
                package_kind: "provider".into(),
                runtime_provider: "agent-runtime".into(),
                current_source: "services/agent-runtime-proxy".into(),
                source_kind: "service".into(),
            }),
        }];

        let payload =
            ServiceOriginCandidatePayload::from_service_contract_payload(&contract_payload);
        let resolve_candidate_context = ResolveCandidateContext::default()
            .with_service_origin_candidates(&SynthesizedServiceOriginCandidateAdapter, &payload);

        let coordinator = ResolveInputCoordinator::from_package_index(&package_index)
            .with_service_origin_candidates(&SynthesizedServiceOriginCandidateAdapter, &payload)
            .build();

        let resolved = resolve_app_manifest_with_policy_and_candidate_context(
            &manifest,
            &packages,
            None,
            None,
            None,
            Some(&package_index),
            coordinator.resolve_candidate_context(),
        )
        .expect("contract payload should resolve alias provider via coordinator path");

        assert_eq!(resolved.status, ResolvedAppStatus::Resolved);
        assert_eq!(resolved.bindings[0].provider, "agent-runtime");

        let candidates = resolve_candidate_context.candidates_for("agent-runtime");
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].candidate, "agent-runtime-proxy");

        let groups = coordinator
            .resolve_candidate_context()
            .grouping_for("agent-runtime");
        assert_eq!(groups.len(), 1);
        assert!(matches!(
            groups[0].source,
            ResolveCandidateGroupingSource::Service
        ));
        assert_eq!(groups[0].id, "service-contract:agent-runtime:closure");
        assert_eq!(groups[0].candidates, vec!["agent-runtime".to_string()]);
        assert_eq!(
            groups[0].rationale,
            Some("contract-driven service origin".to_string())
        );

        let resolved_direct = resolve_app_manifest_with_policy_and_candidate_context(
            &manifest,
            &packages,
            None,
            None,
            None,
            Some(&package_index),
            &resolve_candidate_context,
        )
        .expect("contract payload should resolve alias provider via direct context path");

        assert_eq!(resolved_direct.status, ResolvedAppStatus::Resolved);
        assert_eq!(resolved_direct.bindings[0].provider, "agent-runtime");
    }

    #[test]
    fn metadata_provider_is_available_from_package_index_without_package_runtime() {
        let manifest: crate::app::config::AppManifest = toml::from_str(
            r#"
[app]
name = "metadata-app"
version = "0.1.0"
display_name = "Metadata App"
description = "test"

[requires]
capabilities = ["ui.surface"]

[bindings.ui.surface]
provider = "metadata-ui"
mutable = false
"#,
        )
        .expect("manifest parses");

        let package_index = PackageIndex {
            version: 1,
            revision: "test-rev".into(),
            source_url: "local://packages".into(),
            package_sources: vec![PackageSource {
                name: "metadata-ui".into(),
                kind: "metadata".into(),
                package_kind: "provider".into(),
                runtime_provider: "metadata-app".into(),
                current_source: "apps/metadata-app".into(),
                trusted: true,
                signature: "builtin:app".into(),
                source_authority: "official-apps".into(),
                source_public_keys: vec![],
                provides: vec!["ui.surface".into()],
                requires: vec![],
            }],
        };

        let resolved = resolve_app_manifest_with_policy(
            &manifest,
            &[],
            None,
            None,
            None,
            Some(&package_index),
        )
        .expect("metadata-backed app resolves");

        assert_eq!(resolved.status, ResolvedAppStatus::Resolved);
        assert_eq!(resolved.bindings.len(), 1);
        assert_eq!(resolved.bindings[0].capability, "ui.surface");
        assert_eq!(resolved.bindings[0].provider, "metadata-ui");
    }

    #[test]
    fn weft_claw_required_provider_rejects_metadata_only_package_index_entry() {
        let manifest: crate::app::config::AppManifest = toml::from_str(
            r#"
[app]
name = "weft-claw"
version = "0.1.0"
display_name = "Weft Claw"
description = "test"

[requires]
capabilities = ["prompt.system"]

[bindings.prompt.system]
provider = "prompt-system"
mutable = false
"#,
        )
        .expect("manifest parses");

        let package_index = PackageIndex {
            version: 1,
            revision: "test-rev".into(),
            source_url: "local://packages".into(),
            package_sources: vec![PackageSource {
                name: "prompt-system".into(),
                kind: "metadata".into(),
                package_kind: "foundation".into(),
                runtime_provider: "agent-core".into(),
                current_source: "packages/weft-claw".into(),
                trusted: true,
                signature: "builtin:product-package".into(),
                source_authority: "product-package-instance".into(),
                source_public_keys: vec![],
                provides: vec!["prompt.system".into()],
                requires: vec![],
            }],
        };

        let error = resolve_app_manifest_with_policy(
            &manifest,
            &[],
            None,
            None,
            None,
            Some(&package_index),
        )
        .expect_err("weft-claw required packages must not resolve from metadata-only entries");

        assert!(error
            .to_string()
            .contains("Provider 'prompt-system' for capability 'prompt.system' is not available"));
    }

    #[test]
    fn weft_claw_required_provider_resolves_from_real_package_source() {
        let manifest: crate::app::config::AppManifest = toml::from_str(
            r#"
[app]
name = "weft-claw"
version = "0.1.0"
display_name = "Weft Claw"
description = "test"

[requires]
capabilities = ["prompt.system"]

[bindings.prompt.system]
provider = "prompt-system"
mutable = false
"#,
        )
        .expect("manifest parses");

        let package_index = PackageIndex {
            version: 1,
            revision: "test-rev".into(),
            source_url: "local://packages".into(),
            package_sources: vec![PackageSource {
                name: "prompt-system".into(),
                kind: "wasm".into(),
                package_kind: "foundation".into(),
                runtime_provider: "prompt-system".into(),
                current_source: "packages/official/prompt-system".into(),
                trusted: true,
                signature: "builtin:official".into(),
                source_authority: "official".into(),
                source_public_keys: vec![],
                provides: vec!["prompt.system".into()],
                requires: vec![],
            }],
        };

        let packages = vec![DiscoveredPackage {
            manifest: toml::from_str(
                r#"
[package_info]
name = "prompt-system"
version = "0.1.0"
description = "prompt system"
entry = "package.wasm"

[identity]
name = "prompt-system"
version = "0.1.0"
description = "prompt system"

[package]
entry = "package.wasm"
runtime = "wasm"
api_version = "v1"
provides = ["prompt.system"]

[capability]
provides = ["prompt.system"]
"#,
            )
            .expect("prompt-system manifest parses"),
            dir: std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("..")
                .join("packages")
                .join("official")
                .join("prompt-system"),
            entry_path: None,
            runtime: PackageRuntime::Wasm,
        }];

        let resolved = resolve_app_manifest_with_policy(
            &manifest,
            &packages,
            None,
            None,
            None,
            Some(&package_index),
        )
        .expect("real package-backed app resolves");

        assert_eq!(resolved.status, ResolvedAppStatus::Resolved);
        assert_eq!(resolved.bindings.len(), 1);
        assert_eq!(resolved.bindings[0].provider, "prompt-system");
    }
}
