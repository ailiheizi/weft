use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum AppProfile {
    #[default]
    Safe,
    Developer,
    Trusted,
}

impl AppProfile {
    pub fn from_str_loose(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "developer" | "dev" => Self::Developer,
            "trusted" | "trust" | "full" => Self::Trusted,
            _ => Self::Safe,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Safe => "safe",
            Self::Developer => "developer",
            Self::Trusted => "trusted",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyDecision {
    pub allowed: bool,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityPolicyRule {
    pub min_profile: AppProfile,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CorePolicy {
    pub rules: HashMap<String, CapabilityPolicyRule>,
}

impl CorePolicy {
    pub fn default_policy() -> Self {
        let mut rules = HashMap::new();

        rules.insert(
            "core.files".into(),
            CapabilityPolicyRule {
                min_profile: AppProfile::Developer,
                description: "File system read/write operations".into(),
            },
        );

        rules.insert(
            "core.execution".into(),
            CapabilityPolicyRule {
                min_profile: AppProfile::Developer,
                description: "Command execution and process management".into(),
            },
        );

        rules.insert(
            "core.native_execution".into(),
            CapabilityPolicyRule {
                min_profile: AppProfile::Trusted,
                description: "Native/dll/so package execution".into(),
            },
        );

        Self { rules }
    }

    pub fn check(&self, capability: &str, profile: AppProfile) -> PolicyDecision {
        if let Some(rule) = self.rules.get(capability) {
            let allowed = profile_rank(profile) >= profile_rank(rule.min_profile);
            PolicyDecision {
                allowed,
                reason: if allowed {
                    format!(
                        "Profile '{}' meets minimum '{}' for '{}'",
                        profile.as_str(),
                        rule.min_profile.as_str(),
                        capability
                    )
                } else {
                    format!(
                        "Profile '{}' does not meet minimum '{}' required for '{}'",
                        profile.as_str(),
                        rule.min_profile.as_str(),
                        capability
                    )
                },
            }
        } else {
            PolicyDecision {
                allowed: true,
                reason: format!("No policy restriction on '{}'", capability),
            }
        }
    }
}

fn profile_rank(profile: AppProfile) -> u8 {
    match profile {
        AppProfile::Safe => 0,
        AppProfile::Developer => 1,
        AppProfile::Trusted => 2,
    }
}
