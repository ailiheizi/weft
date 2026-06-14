use crate::package::config::PackagePermissions;
use anyhow::{bail, Result};

/// Enumeration of all permission categories.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Permission {
    Process,
    Network,
    Auth,
    Storage,
    Pipeline,
    Routes,
    Events,
    Log,
    Config,
    Scheduler,
    Ui,
}

impl std::fmt::Display for Permission {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Process => write!(f, "process"),
            Self::Network => write!(f, "network"),
            Self::Auth => write!(f, "auth"),
            Self::Storage => write!(f, "storage"),
            Self::Pipeline => write!(f, "pipeline"),
            Self::Routes => write!(f, "routes"),
            Self::Events => write!(f, "events"),
            Self::Log => write!(f, "log"),
            Self::Config => write!(f, "config"),
            Self::Scheduler => write!(f, "scheduler"),
            Self::Ui => write!(f, "ui"),
        }
    }
}

/// Checks whether a package has a given permission.
pub struct PermissionChecker {
    package_name: String,
    permissions: PackagePermissions,
}

impl PermissionChecker {
    pub fn new(package_name: &str, permissions: PackagePermissions) -> Self {
        Self {
            package_name: package_name.to_string(),
            permissions,
        }
    }

    /// Returns Ok(()) if allowed, Err with descriptive message if denied.
    pub fn check(&self, perm: Permission) -> Result<()> {
        let allowed = match perm {
            Permission::Process => self.permissions.process,
            Permission::Network => self.permissions.network,
            Permission::Auth => self.permissions.auth,
            Permission::Storage => self.permissions.storage,
            Permission::Pipeline => self.permissions.pipeline,
            Permission::Routes => self.permissions.routes,
            Permission::Events => self.permissions.events,
            Permission::Log => self.permissions.log,
            Permission::Config => self.permissions.config,
            Permission::Scheduler => self.permissions.scheduler,
            Permission::Ui => self.permissions.ui,
        };

        if allowed {
            Ok(())
        } else {
            bail!(
                "Package '{}' does not have '{}' permission. Declare it in package.toml [permissions].",
                self.package_name,
                perm
            )
        }
    }
}

impl Permission {
    /// All permission variants, for exhaustive iteration (e.g. escalation diff).
    pub const ALL: [Permission; 11] = [
        Permission::Process,
        Permission::Network,
        Permission::Auth,
        Permission::Storage,
        Permission::Pipeline,
        Permission::Routes,
        Permission::Events,
        Permission::Log,
        Permission::Config,
        Permission::Scheduler,
        Permission::Ui,
    ];
}

/// Returns the permissions present in `new` but absent in `old` — i.e. the
/// escalation a package upgrade would introduce (B4). Empty means the upgrade
/// requests no new capabilities and is safe to apply without re-approval.
pub fn escalated_permissions(
    old: &PackagePermissions,
    new: &PackagePermissions,
) -> Vec<Permission> {
    let granted = |perms: &PackagePermissions, perm: Permission| -> bool {
        let checker = PermissionChecker::new("", perms.clone());
        checker.check(perm).is_ok()
    };
    Permission::ALL
        .into_iter()
        .filter(|&perm| !granted(old, perm) && granted(new, perm))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_allowed() {
        let perms = PackagePermissions {
            process: true,
            network: true,
            ..Default::default()
        };
        let checker = PermissionChecker::new("test-package", perms);
        assert!(checker.check(Permission::Process).is_ok());
        assert!(checker.check(Permission::Network).is_ok());
    }

    #[test]
    fn test_check_denied() {
        let perms = PackagePermissions::default();
        let checker = PermissionChecker::new("test-package", perms);
        let err = checker.check(Permission::Process).unwrap_err();
        assert!(err.to_string().contains("test-package"));
        assert!(err.to_string().contains("process"));
    }

    #[test]
    fn no_escalation_when_permissions_unchanged() {
        let p = PackagePermissions {
            storage: true,
            network: true,
            ..Default::default()
        };
        assert!(escalated_permissions(&p, &p).is_empty());
    }

    #[test]
    fn detects_added_permissions() {
        let old = PackagePermissions {
            storage: true,
            ..Default::default()
        };
        let new = PackagePermissions {
            storage: true,
            process: true, // newly requested
            network: true, // newly requested
            ..Default::default()
        };
        let esc = escalated_permissions(&old, &new);
        assert_eq!(esc.len(), 2);
        assert!(esc.contains(&Permission::Process));
        assert!(esc.contains(&Permission::Network));
    }

    #[test]
    fn dropping_permissions_is_not_escalation() {
        let old = PackagePermissions {
            storage: true,
            process: true,
            ..Default::default()
        };
        let new = PackagePermissions {
            storage: true,
            ..Default::default()
        };
        assert!(escalated_permissions(&old, &new).is_empty());
    }
}
