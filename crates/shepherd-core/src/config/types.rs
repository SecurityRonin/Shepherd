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
}

fn default_port() -> u16 { 7532 }
fn default_max_agents() -> usize { 10 }
fn default_permission_mode() -> String { "ask".into() }
fn default_isolation() -> String { "worktree".into() }
fn default_agent() -> String { "claude-code".into() }

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

fn default_sandbox_enabled() -> bool { true }

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            enabled: default_sandbox_enabled(),
            extra_blocked_paths: vec![],
            block_network: false,
        }
    }
}
