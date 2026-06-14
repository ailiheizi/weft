use crate::app::state::AppBindingResolution;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppGeneration {
    pub id: u64,
    pub app_name: String,
    pub version: String,
    pub bindings: Vec<AppBindingResolution>,
    pub capabilities: Vec<String>,
    #[serde(default)]
    pub enabled_features: Vec<String>,
    #[serde(default)]
    pub scene: String,
    pub profile: String,
    #[serde(default)]
    pub binding_set_id: String,
    #[serde(default)]
    pub closure_id: String,
    #[serde(default)]
    pub lock_digest: String,
    #[serde(default)]
    pub lock_path: String,
    #[serde(default)]
    pub parent_generation: Option<u64>,
    #[serde(default)]
    pub created_by: String,
    pub status: GenerationStatus,
    #[serde(default)]
    pub validation_results: Vec<ValidationResult>,
    pub created_at: u64,
}

impl Default for AppGeneration {
    fn default() -> Self {
        Self {
            id: 0,
            app_name: String::new(),
            version: String::new(),
            bindings: vec![],
            capabilities: vec![],
            enabled_features: vec![],
            scene: String::new(),
            profile: String::new(),
            binding_set_id: String::new(),
            closure_id: String::new(),
            lock_digest: String::new(),
            lock_path: String::new(),
            parent_generation: None,
            created_by: String::new(),
            status: GenerationStatus::Candidate,
            validation_results: vec![],
            created_at: 0,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct AppGenerationSummaryMetadata {
    pub scene: String,
    pub binding_set_id: String,
    pub closure_id: String,
    pub lock_digest: String,
    pub lock_path: String,
    pub parent_generation: Option<u64>,
    pub created_by: String,
}

#[derive(Debug, Clone, Default)]
pub struct AppGenerationProposal {
    pub app_name: String,
    pub version: String,
    pub bindings: Vec<AppBindingResolution>,
    pub capabilities: Vec<String>,
    pub enabled_features: Vec<String>,
    pub profile: String,
    pub metadata: AppGenerationSummaryMetadata,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GenerationStatus {
    Candidate,
    Verified,
    Active,
    Rollback,
    Failed,
    Archived,
}

pub const GENERATION_INDEX_SCHEMA_VERSION: u64 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppGenerationIndex {
    #[serde(default = "default_generation_index_schema_version")]
    pub schema_version: u64,
    #[serde(default)]
    pub active: Option<u64>,
    #[serde(default)]
    pub previous: Option<u64>,
    #[serde(default)]
    pub candidate: Option<u64>,
    pub next_id: u64,
    #[serde(default)]
    pub generations: Vec<AppGeneration>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GenerationIndexDiagnosticLevel {
    Warning,
    RepairNeeded,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GenerationIndexDiagnostic {
    pub level: GenerationIndexDiagnosticLevel,
    pub code: String,
    pub message: String,
    #[serde(default)]
    pub pointer: Option<String>,
    #[serde(default)]
    pub generation_id: Option<u64>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GenerationIndexConsistencyReport {
    pub is_consistent: bool,
    pub repair_recommended: bool,
    #[serde(default)]
    pub diagnostics: Vec<GenerationIndexDiagnostic>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StartupGenerationStoreDiagnostics {
    pub active_pointer: Option<u64>,
    pub previous_pointer: Option<u64>,
    pub generation_index_present: bool,
    #[serde(default)]
    pub diagnostics: Vec<GenerationIndexDiagnostic>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActivationPersistenceStepKind {
    CheckTargetStatus,
    CheckTargetLockMetadata,
    WriteGenerationLock,
    WritePreviousPointer,
    ReplaceActivePointer,
    ReplaceRootLockMirror,
    UpdateGenerationIndex,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActivationPersistenceStep {
    pub kind: ActivationPersistenceStepKind,
    pub description: String,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub generation_id: Option<u64>,
    #[serde(default)]
    pub best_effort: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActivationPersistencePlan {
    pub target_generation_id: u64,
    pub target_lock_path: String,
    #[serde(default)]
    pub previous_active_generation_id: Option<u64>,
    #[serde(default)]
    pub steps: Vec<ActivationPersistenceStep>,
}

impl Default for AppGenerationIndex {
    fn default() -> Self {
        Self {
            schema_version: GENERATION_INDEX_SCHEMA_VERSION,
            active: None,
            previous: None,
            candidate: None,
            next_id: 1,
            generations: Vec::new(),
        }
    }
}

impl AppGenerationIndex {
    pub fn generation(&self, id: u64) -> Option<&AppGeneration> {
        self.generations
            .iter()
            .find(|generation| generation.id == id)
    }

    pub fn from_store(store: &AppGenerationStore) -> Self {
        let mut generations: Vec<AppGeneration> = Vec::new();
        let mut push_unique = |generation: &AppGeneration| {
            if generations
                .iter()
                .any(|existing| existing.id == generation.id)
            {
                return;
            }
            generations.push(generation.clone());
        };

        if let Some(active) = &store.active {
            push_unique(active);
        }
        if let Some(previous) = &store.rollback {
            push_unique(previous);
        }
        if let Some(candidate) = &store.candidate {
            push_unique(candidate);
        }

        generations.sort_by_key(|generation| generation.id);

        Self {
            schema_version: GENERATION_INDEX_SCHEMA_VERSION,
            active: store.active.as_ref().map(|generation| generation.id),
            previous: store.rollback.as_ref().map(|generation| generation.id),
            candidate: store.candidate.as_ref().map(|generation| generation.id),
            next_id: store.next_generation_id(),
            generations,
        }
    }

    pub fn from_active_summary(active: &AppGeneration) -> Self {
        Self {
            schema_version: GENERATION_INDEX_SCHEMA_VERSION,
            active: Some(active.id),
            previous: None,
            candidate: None,
            next_id: active.id.saturating_add(1).max(1),
            generations: vec![active.clone()],
        }
    }

    pub fn repair_from_sources(
        store: Option<&AppGenerationStore>,
        active_summary: Option<&AppGeneration>,
    ) -> Option<Self> {
        let store_has_data = store.is_some_and(|store| {
            store.active.is_some()
                || store.rollback.is_some()
                || store.candidate.is_some()
                || store.next_id != 0
        });
        if !store_has_data && active_summary.is_none() {
            return None;
        }

        let mut index = store.map(Self::from_store).unwrap_or_default();

        if let Some(active_summary) = active_summary {
            index.upsert_generation(active_summary.clone());
            index.active = Some(active_summary.id);
            index.next_id = index
                .next_id
                .max(active_summary.id.saturating_add(1))
                .max(1);
        }

        if index.next_id == 0 {
            index.next_id = 1;
        }
        index.generations.sort_by_key(|generation| generation.id);
        Some(index)
    }

    pub fn consistency_report(
        &self,
        active_pointer: Option<u64>,
        previous_pointer: Option<u64>,
    ) -> GenerationIndexConsistencyReport {
        let mut report = GenerationIndexConsistencyReport {
            is_consistent: true,
            repair_recommended: false,
            diagnostics: Vec::new(),
        };

        report.compare_pointer("active", active_pointer, self.active);
        report.compare_pointer("previous", previous_pointer, self.previous);

        for (pointer, generation_id) in [
            ("active", self.active),
            ("previous", self.previous),
            ("candidate", self.candidate),
        ] {
            if let Some(generation_id) = generation_id {
                match self.generation(generation_id) {
                    Some(generation) => {
                        if generation.lock_path.trim().is_empty() {
                            report.warn(
                                "missing_lock_path",
                                format!(
                                    "Generation {} referenced by {} is missing lock_path metadata",
                                    generation_id, pointer
                                ),
                                Some(pointer),
                                Some(generation_id),
                            );
                        }

                        let mut missing_fields = Vec::new();
                        if generation.scene.trim().is_empty() {
                            missing_fields.push("scene");
                        }
                        if generation.binding_set_id.trim().is_empty() {
                            missing_fields.push("binding_set_id");
                        }
                        if generation.closure_id.trim().is_empty() {
                            missing_fields.push("closure_id");
                        }
                        if generation.lock_digest.trim().is_empty() {
                            missing_fields.push("lock_digest");
                        }
                        if generation.created_by.trim().is_empty() {
                            missing_fields.push("created_by");
                        }

                        if !missing_fields.is_empty() {
                            report.warn(
                                "incomplete_generation_summary",
                                format!(
                                    "Generation {} referenced by {} is missing summary fields: {}",
                                    generation_id,
                                    pointer,
                                    missing_fields.join(", ")
                                ),
                                Some(pointer),
                                Some(generation_id),
                            );
                        }
                    }
                    None => report.repair_needed(
                        "pointer_target_missing_from_index",
                        format!(
                            "Generation {} referenced by {} is missing from generation index",
                            generation_id, pointer
                        ),
                        Some(pointer),
                        Some(generation_id),
                    ),
                }
            }
        }

        report
    }

    pub fn into_store(self) -> AppGenerationStore {
        let active = self.active.and_then(|id| self.generation(id).cloned());
        let rollback = self.previous.and_then(|id| self.generation(id).cloned());
        let candidate = self.candidate.and_then(|id| self.generation(id).cloned());

        AppGenerationStore {
            active,
            candidate,
            rollback,
            next_id: if self.next_id == 0 { 1 } else { self.next_id },
        }
    }

    fn normalized_for_save(&self) -> Self {
        let mut normalized = self.clone();
        if normalized.schema_version == 0 {
            normalized.schema_version = GENERATION_INDEX_SCHEMA_VERSION;
        }
        if normalized.next_id == 0 {
            normalized.next_id = 1;
        }
        normalized
            .generations
            .sort_by_key(|generation| generation.id);
        normalized
    }

    fn upsert_generation(&mut self, generation: AppGeneration) {
        if let Some(existing) = self
            .generations
            .iter_mut()
            .find(|existing| existing.id == generation.id)
        {
            *existing = generation;
            return;
        }

        self.generations.push(generation);
    }

    fn validate(&self, path: &Path) -> Result<()> {
        let mut seen = HashSet::new();
        for generation in &self.generations {
            if !seen.insert(generation.id) {
                anyhow::bail!(
                    "Duplicate generation id {} in {}",
                    generation.id,
                    path.display()
                );
            }
        }

        for (label, id) in [
            ("active", self.active),
            ("previous", self.previous),
            ("candidate", self.candidate),
        ] {
            if let Some(id) = id {
                if self.generation(id).is_none() {
                    anyhow::bail!(
                        "Generation index {} pointer references missing generation {} in {}",
                        label,
                        id,
                        path.display()
                    );
                }
            }
        }

        Ok(())
    }
}

fn default_generation_index_schema_version() -> u64 {
    GENERATION_INDEX_SCHEMA_VERSION
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub check: String,
    pub passed: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppGenerationStore {
    pub active: Option<AppGeneration>,
    pub candidate: Option<AppGeneration>,
    pub rollback: Option<AppGeneration>,
    pub next_id: u64,
}

impl AppGenerationStore {
    pub fn generation(&self, generation_id: u64) -> Option<&AppGeneration> {
        self.active
            .as_ref()
            .filter(|generation| generation.id == generation_id)
            .or_else(|| {
                self.candidate
                    .as_ref()
                    .filter(|generation| generation.id == generation_id)
            })
            .or_else(|| {
                self.rollback
                    .as_ref()
                    .filter(|generation| generation.id == generation_id)
            })
    }

    fn allocate_generation_id(&mut self) -> u64 {
        if self.next_id == 0 {
            self.next_id = 1;
        }
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    pub fn next_generation_id(&self) -> u64 {
        if self.next_id == 0 {
            1
        } else {
            self.next_id
        }
    }

    pub fn propose(&mut self, proposal: AppGenerationProposal) -> &AppGeneration {
        let id = self.allocate_generation_id();
        let AppGenerationProposal {
            app_name,
            version,
            bindings,
            capabilities,
            enabled_features,
            profile,
            metadata,
        } = proposal;

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        self.candidate = Some(AppGeneration {
            id,
            app_name,
            version,
            bindings,
            capabilities,
            enabled_features,
            scene: metadata.scene,
            profile,
            binding_set_id: metadata.binding_set_id,
            closure_id: metadata.closure_id,
            lock_digest: metadata.lock_digest,
            lock_path: metadata.lock_path,
            parent_generation: metadata.parent_generation,
            created_by: metadata.created_by,
            status: GenerationStatus::Candidate,
            validation_results: vec![],
            created_at: now,
        });

        self.candidate.as_ref().unwrap()
    }

    pub fn verify_candidate(
        &mut self,
        registry: Option<&crate::app::CapabilityRegistry>,
    ) -> Result<&AppGeneration, String> {
        let candidate = self
            .candidate
            .as_mut()
            .ok_or_else(|| "No candidate generation to verify".to_string())?;

        let mut results = Vec::new();

        let has_bindings = !candidate.bindings.is_empty();
        results.push(ValidationResult {
            check: "boot".into(),
            passed: has_bindings,
            message: if has_bindings {
                "Bindings present".into()
            } else {
                "No bindings resolved".into()
            },
        });

        let has_capabilities = !candidate.capabilities.is_empty();
        results.push(ValidationResult {
            check: "capabilities".into(),
            passed: has_capabilities,
            message: if has_capabilities {
                format!("{} capabilities declared", candidate.capabilities.len())
            } else {
                "No capabilities declared".into()
            },
        });

        let bound_caps: std::collections::HashSet<&str> = candidate
            .bindings
            .iter()
            .map(|b| b.capability.as_str())
            .collect();
        let unbound: Vec<&str> = candidate
            .capabilities
            .iter()
            .filter(|c| !bound_caps.contains(c.as_str()))
            .map(|c| c.as_str())
            .collect();
        let all_bound = unbound.is_empty();
        results.push(ValidationResult {
            check: "binding-coverage".into(),
            passed: all_bound,
            message: if all_bound {
                "All capabilities have bindings".into()
            } else {
                format!("Unbound capabilities: {:?}", unbound)
            },
        });

        if let Some(reg) = registry {
            let missing: Vec<&str> = candidate
                .capabilities
                .iter()
                .filter(|c| !reg.contains_key(c.as_str()))
                .map(|c| c.as_str())
                .collect();
            let all_in_registry = missing.is_empty();
            results.push(ValidationResult {
                check: "registry-coverage".into(),
                passed: all_in_registry,
                message: if all_in_registry {
                    "All capabilities found in registry".into()
                } else {
                    format!("Missing from registry: {:?}", missing)
                },
            });
        }

        let all_passed = results.iter().all(|r| r.passed);
        candidate.validation_results = results;
        candidate.status = if all_passed {
            GenerationStatus::Verified
        } else {
            GenerationStatus::Failed
        };

        if all_passed {
            Ok(candidate)
        } else {
            Err("Verification failed".into())
        }
    }

    pub fn activate(&mut self) -> Result<&AppGeneration, String> {
        let candidate = self
            .candidate
            .take()
            .ok_or_else(|| "No candidate generation to activate".to_string())?;

        if candidate.status != GenerationStatus::Verified {
            self.candidate = Some(candidate);
            return Err("Candidate must be verified before activation".into());
        }

        let previous_active = self.active.take();

        let mut activated = candidate;
        activated.status = GenerationStatus::Active;
        self.active = Some(activated.clone());

        if let Some(mut prev_active) = previous_active {
            prev_active.status = GenerationStatus::Rollback;
            self.rollback = Some(prev_active);
        }

        Ok(self.active.as_ref().unwrap())
    }

    pub fn rollback(&mut self) -> Result<&AppGeneration, String> {
        let rollback_gen = self
            .rollback
            .take()
            .ok_or_else(|| "No rollback generation available".to_string())?;

        if let Some(mut current) = self.active.take() {
            current.status = GenerationStatus::Failed;
            self.candidate = Some(current);
        }

        let mut restored = rollback_gen;
        restored.status = GenerationStatus::Active;
        self.active = Some(restored);

        Ok(self.active.as_ref().unwrap())
    }

    pub fn switch_to_existing(&mut self, generation_id: u64) -> Result<&AppGeneration, String> {
        if self
            .active
            .as_ref()
            .is_some_and(|generation| generation.id == generation_id)
        {
            return Ok(self.active.as_ref().unwrap());
        }

        let target_status = self
            .generation(generation_id)
            .map(|generation| generation.status)
            .ok_or_else(|| format!("Generation {} not found", generation_id))?;

        match target_status {
            GenerationStatus::Verified | GenerationStatus::Rollback => {}
            GenerationStatus::Candidate => {
                return Err(format!(
                    "Generation {} must be verified before activation",
                    generation_id
                ));
            }
            GenerationStatus::Failed => {
                return Err(format!(
                    "Generation {} failed verification and cannot be activated",
                    generation_id
                ));
            }
            GenerationStatus::Archived => {
                return Err(format!(
                    "Generation {} is archived and cannot be activated",
                    generation_id
                ));
            }
            GenerationStatus::Active => {
                return Err(format!(
                    "Generation {} is already active but not stored in the active slot",
                    generation_id
                ));
            }
        }

        let previous_active = self.active.take();
        let mut target = if self
            .candidate
            .as_ref()
            .is_some_and(|generation| generation.id == generation_id)
        {
            self.candidate.take().unwrap()
        } else if self
            .rollback
            .as_ref()
            .is_some_and(|generation| generation.id == generation_id)
        {
            self.rollback.take().unwrap()
        } else {
            return Err(format!("Generation {} not found", generation_id));
        };

        target.status = GenerationStatus::Active;
        self.active = Some(target);

        if let Some(mut previous_active) = previous_active {
            previous_active.status = GenerationStatus::Rollback;
            self.rollback = Some(previous_active);
        } else {
            self.rollback = None;
        }

        Ok(self.active.as_ref().unwrap())
    }
}

impl GenerationIndexConsistencyReport {
    fn compare_pointer(&mut self, pointer: &str, actual: Option<u64>, indexed: Option<u64>) {
        if actual != indexed {
            self.repair_needed(
                "pointer_mismatch",
                format!(
                    "{} pointer mismatch: authoritative pointer is {:?}, generation index records {:?}",
                    pointer, actual, indexed
                ),
                Some(pointer),
                actual.or(indexed),
            );
        }
    }

    fn warn(
        &mut self,
        code: &str,
        message: String,
        pointer: Option<&str>,
        generation_id: Option<u64>,
    ) {
        self.diagnostics.push(GenerationIndexDiagnostic {
            level: GenerationIndexDiagnosticLevel::Warning,
            code: code.into(),
            message,
            pointer: pointer.map(str::to_owned),
            generation_id,
        });
    }

    fn repair_needed(
        &mut self,
        code: &str,
        message: String,
        pointer: Option<&str>,
        generation_id: Option<u64>,
    ) {
        self.is_consistent = false;
        self.repair_recommended = true;
        self.diagnostics.push(GenerationIndexDiagnostic {
            level: GenerationIndexDiagnosticLevel::RepairNeeded,
            code: code.into(),
            message,
            pointer: pointer.map(str::to_owned),
            generation_id,
        });
    }
}

impl StartupGenerationStoreDiagnostics {
    pub fn is_clean(&self) -> bool {
        self.diagnostics.is_empty()
    }
}

pub type GenerationStoreMap = HashMap<String, AppGenerationStore>;

pub fn generation_index_path(instance_dir: &Path) -> PathBuf {
    instance_dir.join("generation-store.toml")
}

pub fn active_generation_pointer_path(instance_dir: &Path) -> PathBuf {
    instance_dir.join("active")
}

pub fn previous_generation_pointer_path(instance_dir: &Path) -> PathBuf {
    instance_dir.join("previous")
}

pub fn inspect_startup_generation_store(
    instance_dir: &Path,
    store: &AppGenerationStore,
) -> StartupGenerationStoreDiagnostics {
    let expected_active = store.active.as_ref().map(|generation| generation.id);
    let expected_previous = store.rollback.as_ref().map(|generation| generation.id);
    let mut diagnostics = StartupGenerationStoreDiagnostics::default();

    let active_pointer = match read_active_generation_pointer(instance_dir) {
        Ok(pointer) => pointer,
        Err(error) => {
            diagnostics.diagnostics.push(GenerationIndexDiagnostic {
                level: GenerationIndexDiagnosticLevel::Warning,
                code: "startup_active_pointer_unreadable".into(),
                message: format!(
                    "Startup ignored unreadable active pointer at '{}': {error:#}",
                    active_generation_pointer_path(instance_dir).display()
                ),
                pointer: Some("active".into()),
                generation_id: None,
            });
            None
        }
    };
    diagnostics.active_pointer = active_pointer;

    let previous_pointer = match read_previous_generation_pointer(instance_dir) {
        Ok(pointer) => pointer,
        Err(error) => {
            diagnostics.diagnostics.push(GenerationIndexDiagnostic {
                level: GenerationIndexDiagnosticLevel::Warning,
                code: "startup_previous_pointer_unreadable".into(),
                message: format!(
                    "Startup ignored unreadable previous pointer at '{}': {error:#}",
                    previous_generation_pointer_path(instance_dir).display()
                ),
                pointer: Some("previous".into()),
                generation_id: None,
            });
            None
        }
    };
    diagnostics.previous_pointer = previous_pointer;

    if let Some(active_pointer) = active_pointer {
        if Some(active_pointer) != expected_active {
            diagnostics.diagnostics.push(GenerationIndexDiagnostic {
                level: GenerationIndexDiagnosticLevel::Warning,
                code: "startup_active_pointer_mismatch".into(),
                message: format!(
                    "Startup retained lock-derived active generation {:?} while active pointer recorded {:?}",
                    expected_active, active_pointer
                ),
                pointer: Some("active".into()),
                generation_id: Some(active_pointer),
            });
        }
    }

    if let Some(previous_pointer) = previous_pointer {
        if Some(previous_pointer) != expected_previous {
            diagnostics.diagnostics.push(GenerationIndexDiagnostic {
                level: GenerationIndexDiagnosticLevel::Warning,
                code: "startup_previous_pointer_mismatch".into(),
                message: format!(
                    "Startup retained lock-derived previous generation {:?} while previous pointer recorded {:?}",
                    expected_previous, previous_pointer
                ),
                pointer: Some("previous".into()),
                generation_id: Some(previous_pointer),
            });
        }
    }

    match load_generation_index(instance_dir) {
        Ok(Some(index)) => {
            diagnostics.generation_index_present = true;

            if index.active != expected_active {
                diagnostics.diagnostics.push(GenerationIndexDiagnostic {
                    level: GenerationIndexDiagnosticLevel::Warning,
                    code: "startup_generation_index_active_mismatch".into(),
                    message: format!(
                        "Startup retained lock-derived active generation {:?} while generation-store.toml recorded {:?}",
                        expected_active, index.active
                    ),
                    pointer: Some("active".into()),
                    generation_id: index.active.or(expected_active),
                });
            }

            if index.previous != expected_previous {
                diagnostics.diagnostics.push(GenerationIndexDiagnostic {
                    level: GenerationIndexDiagnosticLevel::Warning,
                    code: "startup_generation_index_previous_mismatch".into(),
                    message: format!(
                        "Startup retained lock-derived previous generation {:?} while generation-store.toml recorded {:?}",
                        expected_previous, index.previous
                    ),
                    pointer: Some("previous".into()),
                    generation_id: index.previous.or(expected_previous),
                });
            }

            if let (Some(expected_active), Some(indexed_active)) = (
                store.active.as_ref(),
                index.active.and_then(|id| index.generation(id)),
            ) {
                let mut mismatched_fields = Vec::new();
                if indexed_active.lock_path != expected_active.lock_path {
                    mismatched_fields.push("lock_path");
                }
                if indexed_active.scene != expected_active.scene {
                    mismatched_fields.push("scene");
                }
                if indexed_active.profile != expected_active.profile {
                    mismatched_fields.push("profile");
                }
                if indexed_active.binding_set_id != expected_active.binding_set_id {
                    mismatched_fields.push("binding_set_id");
                }
                if indexed_active.closure_id != expected_active.closure_id {
                    mismatched_fields.push("closure_id");
                }

                if !mismatched_fields.is_empty() {
                    diagnostics.diagnostics.push(GenerationIndexDiagnostic {
                        level: GenerationIndexDiagnosticLevel::Warning,
                        code: "startup_generation_index_summary_mismatch".into(),
                        message: format!(
                            "Startup retained lock-derived active generation {} while generation-store.toml differed in: {}",
                            expected_active.id,
                            mismatched_fields.join(", ")
                        ),
                        pointer: Some("active".into()),
                        generation_id: Some(expected_active.id),
                    });
                }
            }
        }
        Ok(None) => {}
        Err(error) => {
            diagnostics.diagnostics.push(GenerationIndexDiagnostic {
                level: GenerationIndexDiagnosticLevel::Warning,
                code: "startup_generation_index_unreadable".into(),
                message: format!(
                    "Startup ignored unreadable generation-store.toml at '{}': {error:#}",
                    generation_index_path(instance_dir).display()
                ),
                pointer: None,
                generation_id: None,
            });
        }
    }

    diagnostics
}

pub fn plan_activation_persistence(
    instance_dir: &Path,
    target: &AppGeneration,
    store: Option<&AppGenerationStore>,
    index: Option<&AppGenerationIndex>,
) -> Result<ActivationPersistencePlan, String> {
    if !matches!(
        target.status,
        GenerationStatus::Verified | GenerationStatus::Active
    ) {
        return Err(format!(
            "Generation {} must be verified or active before activation persistence can be planned",
            target.id
        ));
    }

    if target.lock_path.trim().is_empty() {
        return Err(format!(
            "Generation {} is missing lock_path metadata required for activation persistence planning",
            target.id
        ));
    }

    if let Some(index) = index {
        if index.generation(target.id).is_none() {
            return Err(format!(
                "Cannot plan activation persistence: target generation {} is missing from generation index",
                target.id
            ));
        }
    }

    let store_active_id =
        store.and_then(|store| store.active.as_ref().map(|generation| generation.id));
    let index_active_id = index.and_then(|index| index.active);
    let current_active_id =
        reconcile_planned_generation_id("active", store_active_id, index_active_id)?;

    let store_previous_id =
        store.and_then(|store| store.rollback.as_ref().map(|generation| generation.id));
    let index_previous_id = index.and_then(|index| index.previous);
    let current_previous_id =
        reconcile_planned_generation_id("previous", store_previous_id, index_previous_id)?;

    let previous_active_generation_id = match target.status {
        GenerationStatus::Active => {
            if let Some(current_active_id) = current_active_id {
                if current_active_id != target.id {
                    return Err(format!(
                        "Generation {} is marked active but store/index currently point to generation {}",
                        target.id, current_active_id
                    ));
                }
            }
            current_previous_id
        }
        GenerationStatus::Verified => current_active_id,
        _ => unreachable!(),
    };

    let target_lock_path = planned_generation_lock_path(instance_dir, &target.lock_path)
        .display()
        .to_string();
    let previous_pointer_path = previous_generation_pointer_path(instance_dir)
        .display()
        .to_string();
    let active_pointer_path = active_generation_pointer_path(instance_dir)
        .display()
        .to_string();
    let root_lock_mirror_path = instance_dir.join("lock.toml").display().to_string();
    let generation_index_path = generation_index_path(instance_dir).display().to_string();

    Ok(ActivationPersistencePlan {
        target_generation_id: target.id,
        target_lock_path: target_lock_path.clone(),
        previous_active_generation_id,
        steps: vec![
            ActivationPersistenceStep {
                kind: ActivationPersistenceStepKind::CheckTargetStatus,
                description: format!(
                    "Confirm generation {} remains verified or active before planning activation persistence",
                    target.id
                ),
                path: None,
                generation_id: Some(target.id),
                best_effort: false,
            },
            ActivationPersistenceStep {
                kind: ActivationPersistenceStepKind::CheckTargetLockMetadata,
                description: format!(
                    "Confirm generation {} lock metadata resolves to {}",
                    target.id, target_lock_path
                ),
                path: Some(target_lock_path.clone()),
                generation_id: Some(target.id),
                best_effort: false,
            },
            ActivationPersistenceStep {
                kind: ActivationPersistenceStepKind::WriteGenerationLock,
                description: format!(
                    "Write immutable generation lock for generation {} before pointer changes",
                    target.id
                ),
                path: Some(target_lock_path),
                generation_id: Some(target.id),
                best_effort: false,
            },
            ActivationPersistenceStep {
                kind: ActivationPersistenceStepKind::WritePreviousPointer,
                description: match previous_active_generation_id {
                    Some(previous_active_generation_id) => format!(
                        "Write previous pointer to generation {} before replacing active pointer",
                        previous_active_generation_id
                    ),
                    None => "Clear previous pointer before replacing active pointer".into(),
                },
                path: Some(previous_pointer_path),
                generation_id: previous_active_generation_id,
                best_effort: false,
            },
            ActivationPersistenceStep {
                kind: ActivationPersistenceStepKind::ReplaceActivePointer,
                description: format!(
                    "Atomically replace active pointer with generation {}",
                    target.id
                ),
                path: Some(active_pointer_path),
                generation_id: Some(target.id),
                best_effort: false,
            },
            ActivationPersistenceStep {
                kind: ActivationPersistenceStepKind::ReplaceRootLockMirror,
                description: format!(
                    "Atomically replace root lock mirror from generation {} after active pointer update",
                    target.id
                ),
                path: Some(root_lock_mirror_path),
                generation_id: Some(target.id),
                best_effort: false,
            },
            ActivationPersistenceStep {
                kind: ActivationPersistenceStepKind::UpdateGenerationIndex,
                description: "Update generation index as repairable best-effort metadata after pointer changes".into(),
                path: Some(generation_index_path),
                generation_id: Some(target.id),
                best_effort: true,
            },
        ],
    })
}

fn reconcile_planned_generation_id(
    label: &str,
    store_id: Option<u64>,
    index_id: Option<u64>,
) -> Result<Option<u64>, String> {
    match (store_id, index_id) {
        (Some(store_id), Some(index_id)) if store_id != index_id => Err(format!(
            "Cannot plan activation persistence: {} generation differs between store ({}) and index ({})",
            label, store_id, index_id
        )),
        (Some(store_id), _) => Ok(Some(store_id)),
        (_, Some(index_id)) => Ok(Some(index_id)),
        (None, None) => Ok(None),
    }
}

fn planned_generation_lock_path(instance_dir: &Path, lock_path: &str) -> PathBuf {
    let lock_path = Path::new(lock_path);
    if lock_path.is_absolute() {
        lock_path.to_path_buf()
    } else {
        instance_dir.join(lock_path)
    }
}

pub fn load_generation_index(instance_dir: &Path) -> Result<Option<AppGenerationIndex>> {
    load_generation_index_from_path(&generation_index_path(instance_dir))
}

pub fn load_generation_index_from_path(path: &Path) -> Result<Option<AppGenerationIndex>> {
    let content = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(error).with_context(|| format!("Failed to read {}", path.display()));
        }
    };

    let mut index: AppGenerationIndex =
        toml::from_str(&content).with_context(|| format!("Failed to parse {}", path.display()))?;
    if index.schema_version == 0 {
        index.schema_version = GENERATION_INDEX_SCHEMA_VERSION;
    } else if index.schema_version > GENERATION_INDEX_SCHEMA_VERSION {
        anyhow::bail!(
            "Unsupported generation index schema version {} in {}",
            index.schema_version,
            path.display()
        );
    }
    if index.next_id == 0 {
        index.next_id = 1;
    }
    index.validate(path)?;

    Ok(Some(index))
}

pub fn save_generation_index(instance_dir: &Path, index: &AppGenerationIndex) -> Result<()> {
    save_generation_index_to_path(&generation_index_path(instance_dir), index)
}

pub fn save_generation_index_to_path(path: &Path, index: &AppGenerationIndex) -> Result<()> {
    let serializable = index.normalized_for_save();
    serializable.validate(path)?;

    let content = toml::to_string_pretty(&serializable)
        .with_context(|| "Failed to serialize generation index")?;
    write_string_safely(path, &content)
}

pub fn read_generation_pointer(path: &Path) -> Result<Option<u64>> {
    let content = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(error).with_context(|| format!("Failed to read {}", path.display()));
        }
    };

    let trimmed = content.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }

    let generation_id = trimmed
        .parse::<u64>()
        .with_context(|| format!("Failed to parse generation pointer {}", path.display()))?;
    Ok(Some(generation_id))
}

pub fn read_active_generation_pointer(instance_dir: &Path) -> Result<Option<u64>> {
    read_generation_pointer(&active_generation_pointer_path(instance_dir))
}

pub fn read_previous_generation_pointer(instance_dir: &Path) -> Result<Option<u64>> {
    read_generation_pointer(&previous_generation_pointer_path(instance_dir))
}

pub fn write_generation_pointer(path: &Path, generation_id: Option<u64>) -> Result<()> {
    match generation_id {
        Some(generation_id) => write_string_safely(path, &format!("{}\n", generation_id)),
        None => {
            if let Err(error) = fs::remove_file(path) {
                if error.kind() != ErrorKind::NotFound {
                    return Err(error)
                        .with_context(|| format!("Failed to remove {}", path.display()));
                }
            }
            Ok(())
        }
    }
}

pub fn write_active_generation_pointer(
    instance_dir: &Path,
    generation_id: Option<u64>,
) -> Result<()> {
    write_generation_pointer(&active_generation_pointer_path(instance_dir), generation_id)
}

pub fn write_previous_generation_pointer(
    instance_dir: &Path,
    generation_id: Option<u64>,
) -> Result<()> {
    write_generation_pointer(
        &previous_generation_pointer_path(instance_dir),
        generation_id,
    )
}

fn write_string_safely(path: &Path, content: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create {}", parent.display()))?;
    }

    let temp_path = temp_write_path(path);
    fs::write(&temp_path, content)
        .with_context(|| format!("Failed to write {}", temp_path.display()))?;

    if path.exists() {
        let backup_path = backup_write_path(path);
        if backup_path.exists() {
            fs::remove_file(&backup_path)
                .with_context(|| format!("Failed to clear {}", backup_path.display()))?;
        }

        fs::rename(path, &backup_path)
            .with_context(|| format!("Failed to stage existing {}", path.display()))?;

        match fs::rename(&temp_path, path) {
            Ok(()) => {
                fs::remove_file(&backup_path).with_context(|| {
                    format!("Failed to remove backup {}", backup_path.display())
                })?;
                Ok(())
            }
            Err(error) => {
                let restore_result = fs::rename(&backup_path, path);
                if restore_result.is_err() {
                    let _ = fs::rename(&temp_path, path);
                }
                Err(error).with_context(|| format!("Failed to move {} into place", path.display()))
            }
        }
    } else {
        fs::rename(&temp_path, path)
            .with_context(|| format!("Failed to move {} into place", path.display()))
    }?;

    Ok(())
}

fn temp_write_path(path: &Path) -> PathBuf {
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("generation.tmp");
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    path.with_file_name(format!("{}.{}.tmp", file_name, unique))
}

fn backup_write_path(path: &Path) -> PathBuf {
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("generation.bak");
    path.with_file_name(format!("{}.bak", file_name))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn app_generation_defaults_summary_metadata_safely() {
        let generation = AppGeneration::default();

        assert_eq!(generation.scene, "");
        assert_eq!(generation.binding_set_id, "");
        assert_eq!(generation.closure_id, "");
        assert_eq!(generation.lock_digest, "");
        assert_eq!(generation.lock_path, "");
        assert_eq!(generation.parent_generation, None);
        assert_eq!(generation.created_by, "");
        assert_eq!(generation.status, GenerationStatus::Candidate);
    }

    #[test]
    fn app_generation_deserializes_old_payload_with_defaulted_metadata() {
        let payload = serde_json::json!({
            "id": 7,
            "app_name": "weft-claw",
            "version": "0.1.0",
            "bindings": [],
            "capabilities": ["core.execution"],
            "enabled_features": [],
            "profile": "developer",
            "status": "candidate",
            "validation_results": [],
            "created_at": 123
        });

        let generation: AppGeneration =
            serde_json::from_value(payload).expect("legacy generation payload should deserialize");

        assert_eq!(generation.scene, "");
        assert_eq!(generation.binding_set_id, "");
        assert_eq!(generation.closure_id, "");
        assert_eq!(generation.lock_digest, "");
        assert_eq!(generation.lock_path, "");
        assert_eq!(generation.parent_generation, None);
        assert_eq!(generation.created_by, "");
    }

    #[test]
    fn propose_initializes_summary_metadata_from_supplied_values() {
        let mut store = AppGenerationStore::default();
        let generation = store.propose(AppGenerationProposal {
            app_name: "weft-claw".into(),
            version: "0.1.0".into(),
            bindings: vec![],
            capabilities: vec!["core.execution".into()],
            enabled_features: vec![],
            profile: "developer".into(),
            metadata: AppGenerationSummaryMetadata {
                scene: "team".into(),
                binding_set_id: "binding-set:sha256:test".into(),
                closure_id: "closure:sha256:test".into(),
                lock_digest: "sha256:lock".into(),
                lock_path: "generations/1.lock.toml".into(),
                parent_generation: Some(3),
                created_by: "api".into(),
            },
        });

        assert_eq!(generation.scene, "team");
        assert_eq!(generation.binding_set_id, "binding-set:sha256:test");
        assert_eq!(generation.closure_id, "closure:sha256:test");
        assert_eq!(generation.lock_digest, "sha256:lock");
        assert_eq!(generation.lock_path, "generations/1.lock.toml");
        assert_eq!(generation.parent_generation, Some(3));
        assert_eq!(generation.created_by, "api");
    }

    fn sample_generation(id: u64, status: GenerationStatus) -> AppGeneration {
        AppGeneration {
            id,
            app_name: "weft-claw".into(),
            version: "0.1.0".into(),
            bindings: vec![],
            capabilities: vec!["core.execution".into()],
            enabled_features: vec![],
            scene: "team".into(),
            profile: "developer".into(),
            binding_set_id: format!("binding-set:sha256:{}", id),
            closure_id: format!("closure:sha256:{}", id),
            lock_digest: format!("sha256:lock-{}", id),
            lock_path: format!("generations/{}.lock.toml", id),
            parent_generation: id.checked_sub(1),
            created_by: "cli".into(),
            status,
            validation_results: vec![],
            created_at: 1710000000 + id,
        }
    }

    #[test]
    fn generation_index_round_trips_with_generation_summaries() {
        let root = tempdir().expect("temp dir");
        let instance_dir = root.path().join(".weft").join("weft-claw");

        let index = AppGenerationIndex {
            schema_version: GENERATION_INDEX_SCHEMA_VERSION,
            active: Some(20),
            previous: Some(19),
            candidate: Some(21),
            next_id: 22,
            generations: vec![
                sample_generation(19, GenerationStatus::Verified),
                sample_generation(20, GenerationStatus::Active),
                sample_generation(21, GenerationStatus::Candidate),
            ],
        };

        save_generation_index(&instance_dir, &index).expect("index saved");
        let loaded = load_generation_index(&instance_dir)
            .expect("index loaded")
            .expect("index present");

        assert_eq!(loaded.schema_version, GENERATION_INDEX_SCHEMA_VERSION);
        assert_eq!(loaded.active, Some(20));
        assert_eq!(loaded.previous, Some(19));
        assert_eq!(loaded.candidate, Some(21));
        assert_eq!(loaded.next_id, 22);
        assert_eq!(loaded.generations.len(), 3);

        let active = loaded.generation(20).expect("active generation stored");
        assert_eq!(active.scene, "team");
        assert_eq!(active.profile, "developer");
        assert_eq!(active.binding_set_id, "binding-set:sha256:20");
        assert_eq!(active.closure_id, "closure:sha256:20");
        assert_eq!(active.lock_digest, "sha256:lock-20");
        assert_eq!(active.lock_path, "generations/20.lock.toml");
        assert_eq!(active.parent_generation, Some(19));
        assert_eq!(active.created_by, "cli");
    }

    #[test]
    fn missing_generation_index_returns_none() {
        let root = tempdir().expect("temp dir");
        let instance_dir = root.path().join(".weft").join("weft-claw");

        let loaded = load_generation_index(&instance_dir).expect("missing index handled");

        assert!(loaded.is_none());
    }

    #[test]
    fn generation_pointer_read_write_round_trip() {
        let root = tempdir().expect("temp dir");
        let instance_dir = root.path().join(".weft").join("weft-claw");

        write_active_generation_pointer(&instance_dir, Some(20)).expect("active pointer written");
        write_previous_generation_pointer(&instance_dir, Some(19))
            .expect("previous pointer written");

        assert_eq!(
            read_active_generation_pointer(&instance_dir).expect("active pointer read"),
            Some(20)
        );
        assert_eq!(
            read_previous_generation_pointer(&instance_dir).expect("previous pointer read"),
            Some(19)
        );

        write_previous_generation_pointer(&instance_dir, None).expect("previous pointer cleared");

        assert_eq!(
            read_previous_generation_pointer(&instance_dir).expect("cleared pointer read"),
            None
        );
        assert!(!previous_generation_pointer_path(&instance_dir).exists());
    }

    #[test]
    fn generation_index_store_conversion_preserves_summary_fields() {
        let active = sample_generation(30, GenerationStatus::Active);
        let previous = sample_generation(29, GenerationStatus::Verified);
        let candidate = sample_generation(31, GenerationStatus::Candidate);
        let store = AppGenerationStore {
            active: Some(active.clone()),
            candidate: Some(candidate.clone()),
            rollback: Some(previous.clone()),
            next_id: 32,
        };

        let index = AppGenerationIndex::from_store(&store);

        assert_eq!(index.active, Some(30));
        assert_eq!(index.previous, Some(29));
        assert_eq!(index.candidate, Some(31));
        assert_eq!(index.next_id, 32);
        assert_eq!(
            index.generation(30).expect("active summary").lock_path,
            active.lock_path
        );
        assert_eq!(
            index.generation(29).expect("previous summary").scene,
            previous.scene
        );
        assert_eq!(
            index
                .generation(31)
                .expect("candidate summary")
                .binding_set_id,
            candidate.binding_set_id
        );

        let restored_store = index.into_store();

        assert_eq!(restored_store.next_id, 32);
        assert_eq!(
            restored_store.active.expect("active restored").closure_id,
            active.closure_id
        );
        assert_eq!(
            restored_store
                .rollback
                .expect("previous restored")
                .created_by,
            previous.created_by
        );
        assert_eq!(
            restored_store
                .candidate
                .expect("candidate restored")
                .lock_digest,
            candidate.lock_digest
        );
    }

    #[test]
    fn generation_index_repair_rebuilds_missing_index_from_store() {
        let active = sample_generation(30, GenerationStatus::Active);
        let previous = sample_generation(29, GenerationStatus::Verified);
        let candidate = sample_generation(31, GenerationStatus::Candidate);
        let store = AppGenerationStore {
            active: Some(active.clone()),
            candidate: Some(candidate.clone()),
            rollback: Some(previous.clone()),
            next_id: 32,
        };

        let repaired = AppGenerationIndex::repair_from_sources(Some(&store), None)
            .expect("repair should rebuild index from store");

        assert_eq!(repaired.active, Some(30));
        assert_eq!(repaired.previous, Some(29));
        assert_eq!(repaired.candidate, Some(31));
        assert_eq!(repaired.next_id, 32);
        assert_eq!(repaired.generations.len(), 3);
        assert_eq!(
            repaired.generation(30).expect("active exists").scene,
            active.scene
        );
        assert_eq!(
            repaired
                .generation(31)
                .expect("candidate exists")
                .binding_set_id,
            candidate.binding_set_id
        );
    }

    #[test]
    fn generation_index_repair_uses_active_summary_when_store_missing() {
        let active = sample_generation(40, GenerationStatus::Active);

        let repaired = AppGenerationIndex::repair_from_sources(None, Some(&active))
            .expect("repair should build index from active summary");

        assert_eq!(repaired.active, Some(40));
        assert_eq!(repaired.previous, None);
        assert_eq!(repaired.candidate, None);
        assert_eq!(repaired.next_id, 41);
        assert_eq!(repaired.generations.len(), 1);
        assert_eq!(
            repaired.generation(40).expect("active exists").lock_path,
            active.lock_path
        );
    }

    #[test]
    fn generation_index_consistency_report_flags_pointer_mismatch() {
        let index = AppGenerationIndex {
            schema_version: GENERATION_INDEX_SCHEMA_VERSION,
            active: Some(20),
            previous: Some(19),
            candidate: None,
            next_id: 21,
            generations: vec![
                sample_generation(19, GenerationStatus::Verified),
                sample_generation(20, GenerationStatus::Active),
            ],
        };

        let report = index.consistency_report(Some(21), Some(19));

        assert!(!report.is_consistent);
        assert!(report.repair_recommended);
        assert_eq!(report.diagnostics.len(), 1);
        assert_eq!(
            report.diagnostics[0].level,
            GenerationIndexDiagnosticLevel::RepairNeeded
        );
        assert_eq!(report.diagnostics[0].code, "pointer_mismatch");
        assert_eq!(report.diagnostics[0].pointer.as_deref(), Some("active"));
        assert_eq!(report.diagnostics[0].generation_id, Some(21));
    }

    #[test]
    fn generation_index_consistency_report_is_clean_for_valid_index() {
        let index = AppGenerationIndex {
            schema_version: GENERATION_INDEX_SCHEMA_VERSION,
            active: Some(20),
            previous: Some(19),
            candidate: Some(21),
            next_id: 22,
            generations: vec![
                sample_generation(19, GenerationStatus::Verified),
                sample_generation(20, GenerationStatus::Active),
                sample_generation(21, GenerationStatus::Candidate),
            ],
        };

        let report = index.consistency_report(Some(20), Some(19));

        assert!(report.is_consistent);
        assert!(!report.repair_recommended);
        assert!(report.diagnostics.is_empty());
    }

    #[test]
    fn generation_index_consistency_report_warns_for_missing_lock_path_and_summary_fields() {
        let mut active = sample_generation(20, GenerationStatus::Active);
        active.lock_path.clear();
        active.scene.clear();
        active.binding_set_id.clear();
        active.closure_id.clear();
        active.lock_digest.clear();
        active.created_by.clear();

        let index = AppGenerationIndex {
            schema_version: GENERATION_INDEX_SCHEMA_VERSION,
            active: Some(20),
            previous: None,
            candidate: None,
            next_id: 21,
            generations: vec![active],
        };

        let report = index.consistency_report(Some(20), None);

        assert!(report.is_consistent);
        assert!(!report.repair_recommended);
        assert_eq!(report.diagnostics.len(), 2);
        assert_eq!(
            report.diagnostics[0].level,
            GenerationIndexDiagnosticLevel::Warning
        );
        assert_eq!(report.diagnostics[0].code, "missing_lock_path");
        assert_eq!(
            report.diagnostics[1].level,
            GenerationIndexDiagnosticLevel::Warning
        );
        assert_eq!(report.diagnostics[1].code, "incomplete_generation_summary");
    }

    #[test]
    fn startup_generation_store_diagnostics_ignore_missing_pointer_and_index_files() {
        let root = tempdir().expect("temp dir");
        let instance_dir = root.path().join(".weft").join("weft-claw");
        fs::create_dir_all(&instance_dir).expect("instance dir created");

        let active = sample_generation(20, GenerationStatus::Active);
        let store = AppGenerationStore {
            active: Some(active),
            candidate: None,
            rollback: None,
            next_id: 21,
        };

        let diagnostics = inspect_startup_generation_store(&instance_dir, &store);

        assert!(diagnostics.is_clean());
        assert_eq!(diagnostics.active_pointer, None);
        assert_eq!(diagnostics.previous_pointer, None);
        assert!(!diagnostics.generation_index_present);
        assert!(diagnostics.diagnostics.is_empty());
    }

    #[test]
    fn startup_generation_store_diagnostics_warn_on_pointer_and_index_mismatch() {
        let root = tempdir().expect("temp dir");
        let instance_dir = root.path().join(".weft").join("weft-claw");
        fs::create_dir_all(&instance_dir).expect("instance dir created");

        write_active_generation_pointer(&instance_dir, Some(21)).expect("active pointer written");
        write_previous_generation_pointer(&instance_dir, Some(18))
            .expect("previous pointer written");
        save_generation_index(
            &instance_dir,
            &AppGenerationIndex {
                schema_version: GENERATION_INDEX_SCHEMA_VERSION,
                active: Some(21),
                previous: Some(18),
                candidate: None,
                next_id: 22,
                generations: vec![
                    sample_generation(18, GenerationStatus::Verified),
                    sample_generation(21, GenerationStatus::Active),
                ],
            },
        )
        .expect("index saved");

        let store = AppGenerationStore {
            active: Some(sample_generation(20, GenerationStatus::Active)),
            candidate: None,
            rollback: Some(sample_generation(19, GenerationStatus::Verified)),
            next_id: 21,
        };

        let diagnostics = inspect_startup_generation_store(&instance_dir, &store);

        assert!(diagnostics.generation_index_present);
        assert_eq!(diagnostics.active_pointer, Some(21));
        assert_eq!(diagnostics.previous_pointer, Some(18));
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
        assert!(diagnostics
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "startup_generation_index_previous_mismatch"));
    }

    #[test]
    fn startup_generation_store_diagnostics_are_clean_for_matching_pointer_and_index() {
        let root = tempdir().expect("temp dir");
        let instance_dir = root.path().join(".weft").join("weft-claw");
        fs::create_dir_all(&instance_dir).expect("instance dir created");

        let active = sample_generation(20, GenerationStatus::Active);
        let previous = sample_generation(19, GenerationStatus::Verified);
        let store = AppGenerationStore {
            active: Some(active.clone()),
            candidate: None,
            rollback: Some(previous.clone()),
            next_id: 21,
        };

        write_active_generation_pointer(&instance_dir, Some(20)).expect("active pointer written");
        write_previous_generation_pointer(&instance_dir, Some(19))
            .expect("previous pointer written");
        save_generation_index(&instance_dir, &AppGenerationIndex::from_store(&store))
            .expect("index saved");

        let diagnostics = inspect_startup_generation_store(&instance_dir, &store);

        assert!(diagnostics.is_clean());
        assert!(diagnostics.generation_index_present);
        assert_eq!(diagnostics.active_pointer, Some(20));
        assert_eq!(diagnostics.previous_pointer, Some(19));
        assert!(diagnostics.diagnostics.is_empty());
    }

    #[test]
    fn generation_index_rejects_unknown_future_schema_version() {
        let root = tempdir().expect("temp dir");
        let index_path = generation_index_path(&root.path().join(".weft").join("weft-claw"));

        if let Some(parent) = index_path.parent() {
            fs::create_dir_all(parent).expect("instance dir created");
        }
        fs::write(
            &index_path,
            "schema_version = 99\nnext_id = 2\n[[generations]]\nid = 1\napp_name = 'weft-claw'\nversion = '0.1.0'\nbindings = []\ncapabilities = []\nprofile = 'developer'\nstatus = 'verified'\nlock_path = 'generations/1.lock.toml'\ncreated_at = 1710000001\n",
        )
        .expect("index fixture written");

        let error =
            load_generation_index_from_path(&index_path).expect_err("future schema should fail");

        assert!(format!("{error:#}").contains("Unsupported generation index schema version 99"));
    }

    #[test]
    fn app_generation_index_default_is_persistable() {
        let index = AppGenerationIndex::default();

        assert_eq!(index.schema_version, GENERATION_INDEX_SCHEMA_VERSION);
        assert_eq!(index.next_id, 1);
        assert!(index.generations.is_empty());
    }

    #[test]
    fn generation_index_rejects_pointer_to_missing_generation() {
        let root = tempdir().expect("temp dir");
        let instance_dir = root.path().join(".weft").join("weft-claw");
        let index_path = generation_index_path(&instance_dir);

        if let Some(parent) = index_path.parent() {
            fs::create_dir_all(parent).expect("instance dir created");
        }
        fs::write(&index_path, "schema_version = 1\nactive = 3\nnext_id = 4\n")
            .expect("index fixture written");

        let error = load_generation_index_from_path(&index_path)
            .expect_err("missing active generation should fail");

        assert!(format!("{error:#}").contains("active pointer references missing generation 3"));
    }

    #[test]
    fn generation_index_rejects_duplicate_generation_ids() {
        let root = tempdir().expect("temp dir");
        let instance_dir = root.path().join(".weft").join("weft-claw");
        let index = AppGenerationIndex {
            schema_version: GENERATION_INDEX_SCHEMA_VERSION,
            active: Some(1),
            previous: None,
            candidate: None,
            next_id: 2,
            generations: vec![
                sample_generation(1, GenerationStatus::Active),
                sample_generation(1, GenerationStatus::Verified),
            ],
        };

        let error = save_generation_index(&instance_dir, &index)
            .expect_err("duplicate generation ids should fail");

        assert!(format!("{error:#}").contains("Duplicate generation id 1"));
    }

    #[test]
    fn activation_persistence_plan_orders_checks_and_writes() {
        let root = tempdir().expect("temp dir");
        let instance_dir = root.path().join(".weft").join("weft-claw");
        let target = sample_generation(21, GenerationStatus::Verified);
        let store = AppGenerationStore {
            active: Some(sample_generation(20, GenerationStatus::Active)),
            candidate: Some(target.clone()),
            rollback: Some(sample_generation(19, GenerationStatus::Rollback)),
            next_id: 22,
        };
        let index = AppGenerationIndex::from_store(&store);

        let plan = plan_activation_persistence(&instance_dir, &target, Some(&store), Some(&index))
            .expect("plan should succeed");

        assert_eq!(plan.target_generation_id, 21);
        assert_eq!(plan.previous_active_generation_id, Some(20));
        assert_eq!(
            plan.target_lock_path,
            instance_dir
                .join("generations/21.lock.toml")
                .display()
                .to_string()
        );
        assert_eq!(
            plan.steps.iter().map(|step| step.kind).collect::<Vec<_>>(),
            vec![
                ActivationPersistenceStepKind::CheckTargetStatus,
                ActivationPersistenceStepKind::CheckTargetLockMetadata,
                ActivationPersistenceStepKind::WriteGenerationLock,
                ActivationPersistenceStepKind::WritePreviousPointer,
                ActivationPersistenceStepKind::ReplaceActivePointer,
                ActivationPersistenceStepKind::ReplaceRootLockMirror,
                ActivationPersistenceStepKind::UpdateGenerationIndex,
            ]
        );
        assert_eq!(
            plan.steps[2].path.as_deref(),
            Some(plan.target_lock_path.as_str())
        );
        assert_eq!(plan.steps[3].generation_id, Some(20));
        assert_eq!(
            plan.steps[3].path.as_deref(),
            Some(
                previous_generation_pointer_path(&instance_dir)
                    .display()
                    .to_string()
                    .as_str()
            )
        );
        assert_eq!(
            plan.steps[4].path.as_deref(),
            Some(
                active_generation_pointer_path(&instance_dir)
                    .display()
                    .to_string()
                    .as_str()
            )
        );
        assert!(plan.steps[6].best_effort);
    }

    #[test]
    fn activation_persistence_plan_rejects_unverified_target() {
        let root = tempdir().expect("temp dir");
        let instance_dir = root.path().join(".weft").join("weft-claw");
        let target = sample_generation(21, GenerationStatus::Candidate);

        let error = plan_activation_persistence(&instance_dir, &target, None, None)
            .expect_err("candidate target should be rejected");

        assert!(error.contains("must be verified or active"));
    }

    #[test]
    fn activation_persistence_plan_uses_existing_previous_for_active_target() {
        let root = tempdir().expect("temp dir");
        let instance_dir = root.path().join(".weft").join("weft-claw");
        let target = sample_generation(20, GenerationStatus::Active);
        let store = AppGenerationStore {
            active: Some(target.clone()),
            candidate: None,
            rollback: Some(sample_generation(19, GenerationStatus::Rollback)),
            next_id: 21,
        };

        let plan = plan_activation_persistence(&instance_dir, &target, Some(&store), None)
            .expect("active target plan should succeed");

        assert_eq!(plan.previous_active_generation_id, Some(19));
        assert_eq!(plan.steps[3].generation_id, Some(19));
        assert!(plan.steps[3]
            .description
            .contains("Write previous pointer to generation 19"));
    }

    #[test]
    fn activation_persistence_plan_rejects_missing_lock_path_metadata() {
        let root = tempdir().expect("temp dir");
        let instance_dir = root.path().join(".weft").join("weft-claw");
        let mut target = sample_generation(21, GenerationStatus::Verified);
        target.lock_path.clear();

        let error = plan_activation_persistence(&instance_dir, &target, None, None)
            .expect_err("missing lock_path should be rejected");

        assert!(error.contains("missing lock_path metadata"));
    }

    #[test]
    fn activation_persistence_plan_derives_previous_active_from_index_when_store_missing() {
        let root = tempdir().expect("temp dir");
        let instance_dir = root.path().join(".weft").join("weft-claw");
        let target = sample_generation(21, GenerationStatus::Verified);
        let index = AppGenerationIndex {
            schema_version: GENERATION_INDEX_SCHEMA_VERSION,
            active: Some(20),
            previous: Some(19),
            candidate: Some(21),
            next_id: 22,
            generations: vec![
                sample_generation(19, GenerationStatus::Rollback),
                sample_generation(20, GenerationStatus::Active),
                target.clone(),
            ],
        };

        let plan = plan_activation_persistence(&instance_dir, &target, None, Some(&index))
            .expect("index should provide previous active context");

        assert_eq!(plan.previous_active_generation_id, Some(20));
        assert_eq!(plan.steps[3].generation_id, Some(20));
    }

    #[test]
    fn activation_persistence_plan_rejects_index_missing_target_generation() {
        let root = tempdir().expect("temp dir");
        let instance_dir = root.path().join(".weft").join("weft-claw");
        let target = sample_generation(21, GenerationStatus::Verified);
        let index = AppGenerationIndex {
            schema_version: GENERATION_INDEX_SCHEMA_VERSION,
            active: Some(20),
            previous: Some(19),
            candidate: Some(21),
            next_id: 22,
            generations: vec![
                sample_generation(19, GenerationStatus::Rollback),
                sample_generation(20, GenerationStatus::Active),
            ],
        };

        let error = plan_activation_persistence(&instance_dir, &target, None, Some(&index))
            .expect_err("index missing target generation should fail");

        assert!(error.contains("target generation 21 is missing from generation index"));
    }
}
