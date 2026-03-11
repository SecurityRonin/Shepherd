use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdapterConfig {
    pub agent: AgentSection,
    #[serde(default)]
    pub hooks: Option<HooksSection>,
    pub status: StatusSection,
    pub permissions: PermissionsSection,
    #[serde(default)]
    pub capabilities: CapabilitiesSection,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSection {
    pub name: String,
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub args_interactive: Vec<String>,
    #[serde(default)]
    pub version_check: Option<String>,
    #[serde(default)]
    pub icon: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HooksSection {
    #[serde(rename = "type")]
    pub hook_type: String,
    #[serde(default = "default_install")]
    pub install: String,
    #[serde(default)]
    pub state_dir: Option<String>,
}

fn default_install() -> String {
    "auto".into()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusSection {
    #[serde(default)]
    pub working_patterns: Vec<String>,
    #[serde(default)]
    pub idle_patterns: Vec<String>,
    #[serde(default)]
    pub input_patterns: Vec<String>,
    #[serde(default)]
    pub error_patterns: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionsSection {
    #[serde(default = "default_approve")]
    pub approve: String,
    #[serde(default = "default_approve_all")]
    pub approve_all: String,
    #[serde(default = "default_deny")]
    pub deny: String,
}

fn default_approve() -> String {
    "y\n".into()
}
fn default_approve_all() -> String {
    "Y\n".into()
}
fn default_deny() -> String {
    "n\n".into()
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CapabilitiesSection {
    #[serde(default)]
    pub supports_hooks: bool,
    #[serde(default)]
    pub supports_prompt_arg: bool,
    #[serde(default)]
    pub supports_resume: bool,
    #[serde(default)]
    pub supports_mcp: bool,
    #[serde(default)]
    pub supports_worktree: bool,
}
