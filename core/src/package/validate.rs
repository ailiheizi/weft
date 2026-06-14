//! Manifest validation (B1).
//!
//! `load_manifest` collapses the multiple `package.toml` dialects
//! (`[package_info]` vs `[identity]+[package]`) into a single
//! [`PackageManifest`], but it is *lenient*: a manifest with an empty name or a
//! `kind`/`runtime` mismatch still loads. This module turns that implicit
//! tolerance into explicit, severity-rated findings so the runtime can warn at
//! startup and a future `weft package lint` command can surface them.
//!
//! Validation is pure (no I/O); callers that want to check the on-disk entry
//! file pass `entry_exists` explicitly.

use crate::package::config::PackageManifest;
use std::fmt;

/// Severity of a validation finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    /// Manifest is malformed in a way that breaks discovery or capability wiring.
    Error,
    /// Manifest loads but is inconsistent or under-specified.
    Warning,
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Severity::Error => write!(f, "error"),
            Severity::Warning => write!(f, "warning"),
        }
    }
}

/// A single validation finding for one manifest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManifestIssue {
    pub severity: Severity,
    /// Stable machine-readable code, e.g. `"missing-name"`.
    pub code: String,
    /// Human-readable explanation.
    pub message: String,
}

impl ManifestIssue {
    fn error(code: &str, message: impl Into<String>) -> Self {
        Self { severity: Severity::Error, code: code.into(), message: message.into() }
    }
    fn warning(code: &str, message: impl Into<String>) -> Self {
        Self { severity: Severity::Warning, code: code.into(), message: message.into() }
    }
}

/// Validates a parsed manifest. `entry_exists` is the caller's check of whether
/// the resolved entry file is present on disk (pass `None` to skip that check,
/// e.g. when validating in-memory without a package dir).
pub fn validate_manifest(
    manifest: &PackageManifest,
    entry_exists: Option<bool>,
) -> Vec<ManifestIssue> {
    let mut issues = Vec::new();

    // --- identity ---
    if manifest.package_info.name.trim().is_empty() {
        issues.push(ManifestIssue::error(
            "missing-name",
            "package has no name; declare [package_info].name or [identity].name",
        ));
    }
    if manifest.package_info.version.trim().is_empty() {
        issues.push(ManifestIssue::warning(
            "missing-version",
            "package has no version; declare [package_info].version or [identity].version",
        ));
    }

    // --- entry / runtime consistency ---
    // Product packages compose capabilities via [requires]/[bindings] and have
    // no executable entry of their own, so entry/runtime checks do not apply.
    if !manifest.is_product() {
        let runtime = manifest.runtime_kind();
        match manifest.resolved_entry() {
            None => issues.push(ManifestIssue::error(
                "missing-entry",
                "no entry resolved; declare [package_info].entry or [package].entry",
            )),
            Some(entry) => {
                // kind <-> entry mismatch: wasm runtime must point at a .wasm file.
                let entry_is_wasm = entry.ends_with(".wasm");
                if runtime == "wasm" && !entry_is_wasm {
                    issues.push(ManifestIssue::warning(
                        "runtime-entry-mismatch",
                        format!(
                            "runtime=wasm but entry '{entry}' is not a .wasm file; \
                             set runtime=service or fix the entry"
                        ),
                    ));
                }
                if runtime == "service" && entry_is_wasm {
                    issues.push(ManifestIssue::warning(
                        "runtime-entry-mismatch",
                        format!(
                            "runtime=service but entry '{entry}' is a .wasm file; \
                             set runtime=wasm or fix the entry"
                        ),
                    ));
                }
                if let Some(false) = entry_exists {
                    issues.push(ManifestIssue::error(
                        "entry-not-found",
                        format!("entry file '{entry}' does not exist on disk"),
                    ));
                }
            }
        }
    }

    // --- capability ---
    // Product packages declare [requires], not [provides]; empty provides is expected.
    if !manifest.is_product() && manifest.resolved_provides().is_empty() {
        issues.push(ManifestIssue::warning(
            "no-provides",
            "package declares no capabilities (provides is empty); \
             it cannot satisfy any binding",
        ));
    }

    issues
}

/// Convenience: true if any finding is an error.
pub fn has_errors(issues: &[ManifestIssue]) -> bool {
    issues.iter().any(|i| i.severity == Severity::Error)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::package::config::load_manifest;
    use std::io::Write;

    fn manifest_from(test_name: &str, toml: &str) -> PackageManifest {
        // Unique per-test dir avoids collisions under parallel test execution.
        let dir = std::env::temp_dir().join(format!("weft-validate-test-{test_name}"));
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::create_dir_all(&dir);
        let mut f = std::fs::File::create(dir.join("package.toml")).unwrap();
        f.write_all(toml.as_bytes()).unwrap();
        load_manifest(&dir).unwrap()
    }

    #[test]
    fn clean_wasm_manifest_has_no_issues() {
        let m = manifest_from(
            "clean-wasm",
            r#"
[package_info]
name = "agent-runtime"
version = "0.1.0"
description = "x"
entry = "package.wasm"
provides = ["agent.runtime"]
"#,
        );
        let issues = validate_manifest(&m, Some(true));
        assert!(!has_errors(&issues), "unexpected issues: {issues:?}");
    }

    #[test]
    fn empty_name_is_error() {
        // Mimics the orphan installed/memory dialect that resolved to an empty name.
        let m = manifest_from(
            "empty-name",
            r#"
[package]
runtime = "wasm"
entry = "package.wasm"
"#,
        );
        let issues = validate_manifest(&m, Some(true));
        assert!(has_errors(&issues));
        assert!(issues.iter().any(|i| i.code == "missing-name"));
    }

    #[test]
    fn wasm_runtime_with_non_wasm_entry_warns() {
        // Mimics the memory-store kind=wasm but service entry bug B2 fixed.
        let m = manifest_from(
            "wasm-nonwasm-entry",
            r#"
[package_info]
name = "memory-store"
version = "0.1.0"
description = "x"
entry = "start-memory-runtime.ps1"
provides = ["memory.store"]

[package]
runtime = "wasm"
"#,
        );
        let issues = validate_manifest(&m, Some(true));
        assert!(issues.iter().any(|i| i.code == "runtime-entry-mismatch"));
    }

    #[test]
    fn missing_entry_file_is_error() {
        let m = manifest_from(
            "stub-missing-entry",
            r#"
[package_info]
name = "stub"
version = "0.1.0"
description = "x"
entry = "package.wasm"
provides = ["x.y"]
"#,
        );
        let issues = validate_manifest(&m, Some(false));
        assert!(issues.iter().any(|i| i.code == "entry-not-found"));
    }

    #[test]
    fn product_package_skips_entry_and_provides_checks() {
        // Mimics packages/weft-code: kind=product, no entry, only [requires].
        // Must NOT report entry-not-found / missing-entry / no-provides.
        let m = manifest_from(
            "product-pkg",
            r#"
schema_version = 2

[identity]
name = "weft-code"
version = "0.1.0"
description = "product"

[package]
kind = "product"

[requires]
capabilities = ["agent.runtime", "weft_code.runtime"]
"#,
        );
        assert!(m.is_product(), "kind=product should be detected");
        // entry_exists=false would normally trigger entry-not-found for a wasm pkg
        let issues = validate_manifest(&m, Some(false));
        assert!(
            !issues.iter().any(|i| matches!(
                i.code.as_str(),
                "entry-not-found" | "missing-entry" | "runtime-entry-mismatch" | "no-provides"
            )),
            "product package should skip entry/provides checks, got: {issues:?}"
        );
    }
}
