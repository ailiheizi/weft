use crate::app::config::AppLockPackage;
use crate::app::generation::ValidationResult;
use crate::app::policy::AppProfile;
use sha2::{Digest, Sha512};
use std::path::{Path, PathBuf};

const STORE_CHECK_PREFIX: &str = "store-policy:";

pub fn verify_local_store_packages(
    instance_dir: Option<&Path>,
    packages: &[AppLockPackage],
    profile: AppProfile,
) -> Vec<ValidationResult> {
    packages
        .iter()
        .map(|package| verify_local_store_package(instance_dir, package, profile))
        .collect()
}

pub fn is_store_check(result: &ValidationResult) -> bool {
    result.check.starts_with(STORE_CHECK_PREFIX)
}

fn verify_local_store_package(
    instance_dir: Option<&Path>,
    package: &AppLockPackage,
    profile: AppProfile,
) -> ValidationResult {
    let identity = package.identity();
    let check = format!("{STORE_CHECK_PREFIX}{identity}");
    let metadata_present = !package.store_path.trim().is_empty();

    if !metadata_present {
        return ValidationResult {
            check,
            passed: true,
            message: format!(
                "Warning: package '{}' has no explicit local store path metadata in the generation lock; offline store verification was skipped for backward compatibility under profile '{}'",
                identity,
                profile.as_str()
            ),
        };
    }

    let store_path = package.store_path.trim();

    let resolved_path = resolve_store_path(instance_dir, store_path);
    let resolved_display = resolved_path.display().to_string();
    if !resolved_path.exists() {
        return policy_result(
            profile,
            check,
            format!(
                "Package '{}' requires local store object '{}' at '{}' but the path is missing",
                identity, package.store_object_id, resolved_display
            ),
            format!(
                "Warning: package '{}' requires local store object '{}' at '{}' but the path is missing; developer profile surfaced the offline store verification warning without failing activation",
                identity, package.store_object_id, resolved_display
            ),
        );
    }

    let expected_digest = expected_store_sha512(package);
    match expected_digest {
        Some(expected) => match compute_store_sha512(&resolved_path) {
            Ok(actual) if actual == expected => ValidationResult {
                check,
                passed: true,
                message: format!(
                    "Package '{}' local store path '{}' exists and matches expected store digest '{}{}'",
                    identity,
                    resolved_display,
                    if package.store_object_id.starts_with("store:sha512:") {
                        "store:sha512:"
                    } else {
                        ""
                    },
                    expected
                ),
            },
            Ok(actual) => policy_result(
                profile,
                check,
                format!(
                    "Package '{}' local store path '{}' exists but its computed sha512 '{}' does not match expected '{}'",
                    identity, resolved_display, actual, expected
                ),
                format!(
                    "Warning: package '{}' local store path '{}' exists but its computed sha512 '{}' does not match expected '{}'; developer profile surfaced the mismatch without failing activation",
                    identity, resolved_display, actual, expected
                ),
            ),
            Err(error) => policy_result(
                profile,
                check,
                format!(
                    "Package '{}' local store path '{}' exists but sha512 verification failed: {}",
                    identity, resolved_display, error
                ),
                format!(
                    "Warning: package '{}' local store path '{}' exists but sha512 verification failed: {}; developer profile surfaced the verification issue without failing activation",
                    identity, resolved_display, error
                ),
            ),
        },
        None => ValidationResult {
            check,
            passed: true,
            message: format!(
                "Package '{}' local store path '{}' exists; no comparable sha512 store digest metadata was present, so offline verification recorded existence only",
                identity, resolved_display
            ),
        },
    }
}

fn policy_result(
    profile: AppProfile,
    check: String,
    strict_message: String,
    developer_message: String,
) -> ValidationResult {
    match profile {
        AppProfile::Developer => ValidationResult {
            check,
            passed: true,
            message: developer_message,
        },
        AppProfile::Safe | AppProfile::Trusted => ValidationResult {
            check,
            passed: false,
            message: strict_message,
        },
    }
}

fn expected_store_sha512(package: &AppLockPackage) -> Option<String> {
    if let Some(value) = package.store_object_id.trim().strip_prefix("store:sha512:") {
        if is_valid_sha512_hex(value) {
            return Some(value.to_string());
        }
    }

    let sha512 = package.sha512.trim();
    if is_valid_sha512_hex(sha512) {
        return Some(sha512.to_string());
    }

    None
}

fn is_valid_sha512_hex(value: &str) -> bool {
    value.len() == 128 && value.chars().all(|c| c.is_ascii_hexdigit())
}

fn resolve_store_path(instance_dir: Option<&Path>, store_path: &str) -> PathBuf {
    let store_path = Path::new(store_path);
    if store_path.is_absolute() || instance_dir.is_none() {
        return store_path.to_path_buf();
    }

    let instance_dir = instance_dir.expect("checked above");
    let mut candidates = vec![instance_dir.join(store_path)];
    if let Some(weft_dir) = instance_dir.parent() {
        candidates.push(weft_dir.join(store_path));
        if let Some(repo_root) = weft_dir.parent() {
            candidates.push(repo_root.join(store_path));
        }
    }

    candidates
        .iter()
        .find(|candidate| candidate.exists())
        .cloned()
        .unwrap_or_else(|| {
            if store_path
                .components()
                .next()
                .is_some_and(|component| component.as_os_str() == ".weft")
            {
                if let Some(repo_root) = instance_dir.parent().and_then(Path::parent) {
                    return repo_root.join(store_path);
                }
            }

            instance_dir.join(store_path)
        })
}

fn compute_store_sha512(path: &Path) -> Result<String, std::io::Error> {
    let mut hasher = Sha512::new();

    fn update_dir(hasher: &mut Sha512, root: &Path, path: &Path) -> Result<(), std::io::Error> {
        let mut entries = std::fs::read_dir(path)?
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .collect::<Vec<_>>();
        entries.sort();

        for entry in entries {
            let relative = entry
                .strip_prefix(root)
                .unwrap_or(&entry)
                .to_string_lossy()
                .replace('\\', "/");
            if entry.is_dir() {
                hasher.update(relative.as_bytes());
                update_dir(hasher, root, &entry)?;
            } else if entry.is_file() {
                hasher.update(relative.as_bytes());
                hasher.update(std::fs::read(&entry)?);
            }
        }

        Ok(())
    }

    if path.is_file() {
        hasher.update(std::fs::read(path)?);
    } else if path.is_dir() {
        update_dir(&mut hasher, path, path)?;
    }

    Ok(format!("{:x}", hasher.finalize()))
}

#[cfg(test)]
mod tests {
    use super::verify_local_store_packages;
    use crate::app::{AppLockEvidence, AppLockPackage, AppProfile};
    use sha2::{Digest, Sha512};
    use tempfile::tempdir;

    fn store_package(
        name: &str,
        store_object_id: &str,
        store_path: &str,
        sha512: &str,
    ) -> AppLockPackage {
        AppLockPackage {
            name: name.into(),
            version: "0.1.0".into(),
            store_object_id: store_object_id.into(),
            store_path: store_path.into(),
            sha512: sha512.into(),
            evidence: AppLockEvidence::default(),
            ..AppLockPackage::default()
        }
    }

    #[test]
    fn existing_store_path_passes_when_sha512_matches() {
        let root = tempdir().expect("tempdir");
        let instance_dir = root.path().join(".weft").join("weft-claw");
        let store_path = root
            .path()
            .join(".weft")
            .join("store")
            .join("agent-runtime-0.1.0");
        std::fs::create_dir_all(store_path.parent().expect("parent")).expect("store dir");
        std::fs::write(&store_path, b"store-bytes").expect("store file");
        let digest = format!("{:x}", Sha512::digest(b"store-bytes"));

        let results = verify_local_store_packages(
            Some(&instance_dir),
            &[store_package(
                "agent-runtime",
                &format!("store:sha512:{digest}"),
                ".weft/store/agent-runtime-0.1.0",
                &digest,
            )],
            AppProfile::Safe,
        );

        assert_eq!(results.len(), 1);
        assert!(results[0].passed);
        assert!(results[0].message.contains("matches expected store digest"));
    }

    #[test]
    fn missing_store_path_fails_for_safe_and_trusted_profiles() {
        let root = tempdir().expect("tempdir");
        let instance_dir = root.path().join(".weft").join("weft-claw");
        let package = store_package(
            "agent-runtime",
            "store:sha512:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            ".weft/store/missing-agent-runtime-0.1.0",
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        );

        for profile in [AppProfile::Safe, AppProfile::Trusted] {
            let results = verify_local_store_packages(
                Some(&instance_dir),
                std::slice::from_ref(&package),
                profile,
            );
            assert!(!results[0].passed);
            assert!(results[0].message.contains("path is missing"));
        }
    }

    #[test]
    fn developer_missing_store_path_warns_without_failing() {
        let root = tempdir().expect("tempdir");
        let instance_dir = root.path().join(".weft").join("weft-claw");
        let results = verify_local_store_packages(
            Some(&instance_dir),
            &[store_package(
                "agent-runtime",
                "store:sha512:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
                ".weft/store/missing-agent-runtime-0.1.0",
                "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            )],
            AppProfile::Developer,
        );

        assert!(results[0].passed);
        assert!(results[0].message.contains("Warning:"));
        assert!(results[0].message.contains("missing"));
    }

    #[test]
    fn absent_store_metadata_stays_backward_compatible_warning() {
        let results = verify_local_store_packages(
            None,
            &[store_package("agent-runtime", "", "", "")],
            AppProfile::Trusted,
        );

        assert!(results[0].passed);
        assert!(results[0].message.contains("Warning:"));
        assert!(results[0].message.contains("backward compatibility"));
    }
}
