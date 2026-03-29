use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShepherdConfig {
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_max_agents")]
    pub max_agents: usize,
    #[serde(default = "default_permission_mode")]
    pub default_permission_mode: String,
    #[serde(default = "default_isolation")]
    pub default_isolation: String,
    #[serde(default = "default_agent")]
    pub default_agent: String,
    #[serde(default)]
    pub sound_enabled: bool,
    #[serde(default)]
    pub sandbox: SandboxConfig,
    #[serde(default)]
    pub ecosystem: EcosystemConfig,
    #[serde(default)]
    pub cloud: CloudFeaturesConfig,
}

fn default_port() -> u16 {
    7532
}
fn default_max_agents() -> usize {
    10
}
fn default_permission_mode() -> String {
    "ask".into()
}
fn default_isolation() -> String {
    "worktree".into()
}
fn default_agent() -> String {
    "claude-code".into()
}

impl Default for ShepherdConfig {
    fn default() -> Self {
        Self {
            port: default_port(),
            max_agents: default_max_agents(),
            default_permission_mode: default_permission_mode(),
            default_isolation: default_isolation(),
            default_agent: default_agent(),
            sound_enabled: false,
            sandbox: SandboxConfig::default(),
            ecosystem: EcosystemConfig::default(),
            cloud: CloudFeaturesConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxConfig {
    #[serde(default = "default_sandbox_enabled")]
    pub enabled: bool,
    #[serde(default)]
    pub extra_blocked_paths: Vec<String>,
    #[serde(default)]
    pub block_network: bool,
}

fn default_sandbox_enabled() -> bool {
    true
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            enabled: default_sandbox_enabled(),
            extra_blocked_paths: vec![],
            block_network: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EcosystemConfig {
    #[serde(default = "default_true")]
    pub auto_detect_superpowers: bool,
    #[serde(default = "default_true")]
    pub auto_detect_context_mode: bool,
    #[serde(default = "default_true")]
    pub auto_detect_context7: bool,
    #[serde(default = "default_true")]
    pub auto_detect_ralph_loop: bool,
    #[serde(default = "default_true")]
    pub auto_detect_frontend_design: bool,
    #[serde(default = "default_true")]
    pub auto_detect_context_hub: bool,
    #[serde(default = "default_true")]
    pub auto_detect_rtk: bool,
    #[serde(default = "default_true")]
    pub offer_install_on_new_task: bool,
    /// Disable non-essential telemetry (Statsig, Sentry, RTK analytics)
    /// in spawned agents.  Defaults to `false` — telemetry stays enabled
    /// unless the user explicitly opts out.
    #[serde(default)]
    pub disable_agent_telemetry: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudFeaturesConfig {
    #[serde(default = "default_true")]
    pub cloud_generation_enabled: bool,
    #[serde(default = "default_true")]
    pub sync_enabled: bool,
    #[serde(default)]
    pub sync_machine_id: Option<String>,
    #[serde(default = "default_true")]
    pub observability_push_enabled: bool,
    #[serde(default = "default_true")]
    pub notifications_enabled: bool,
}

impl Default for CloudFeaturesConfig {
    fn default() -> Self {
        Self {
            cloud_generation_enabled: true,
            sync_enabled: true,
            sync_machine_id: None,
            observability_push_enabled: true,
            notifications_enabled: true,
        }
    }
}

impl Default for EcosystemConfig {
    fn default() -> Self {
        Self {
            auto_detect_superpowers: true,
            auto_detect_context_mode: true,
            auto_detect_context7: true,
            auto_detect_ralph_loop: true,
            auto_detect_frontend_design: true,
            auto_detect_context_hub: true,
            auto_detect_rtk: true,
            offer_install_on_new_task: true,
            disable_agent_telemetry: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cloud_features_config_defaults() {
        let config = CloudFeaturesConfig::default();
        assert!(config.cloud_generation_enabled);
        assert!(config.sync_enabled);
        assert!(config.sync_machine_id.is_none());
        assert!(config.observability_push_enabled);
        assert!(config.notifications_enabled);
    }

    #[test]
    fn cloud_features_config_serde_roundtrip() {
        let config = CloudFeaturesConfig {
            cloud_generation_enabled: true,
            sync_enabled: false,
            sync_machine_id: Some("mbp-2024".to_string()),
            observability_push_enabled: true,
            notifications_enabled: false,
        };
        let json = serde_json::to_string(&config).unwrap();
        let parsed: CloudFeaturesConfig = serde_json::from_str(&json).unwrap();
        assert!(!parsed.sync_enabled);
        assert_eq!(parsed.sync_machine_id, Some("mbp-2024".to_string()));
        assert!(parsed.observability_push_enabled);
        assert!(!parsed.notifications_enabled);
    }

    #[test]
    fn shepherd_config_defaults() {
        let config = ShepherdConfig::default();
        assert_eq!(config.port, 7532);
        assert_eq!(config.max_agents, 10);
        assert!(config.cloud.sync_enabled);
        assert!(config.cloud.notifications_enabled);
    }

    #[test]
    fn shepherd_config_deserialize_without_cloud_section() {
        // Existing configs without [cloud] should still parse correctly
        let toml_str = r#"
            port = 8080
            max_agents = 5
        "#;
        let config: ShepherdConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.port, 8080);
        assert_eq!(config.max_agents, 5);
        // cloud defaults kick in
        assert!(config.cloud.sync_enabled);
        assert!(config.cloud.observability_push_enabled);
        assert!(config.cloud.notifications_enabled);
        assert!(config.cloud.sync_machine_id.is_none());
    }

    #[test]
    fn default_true_returns_true() {
        assert!(default_true());
    }
}
