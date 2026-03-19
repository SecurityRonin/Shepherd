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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_full_toml_roundtrip() {
        let toml_str = r#"
[agent]
name = "claude-code"
command = "claude"
args = ["--json"]
args_interactive = ["--interactive"]
version_check = "claude --version"
icon = "brain"

[hooks]
type = "mcp"
install = "manual"
state_dir = "/tmp/hooks"

[status]
working_patterns = ["Thinking", "Writing"]
idle_patterns = ["Waiting"]
input_patterns = ["?"]
error_patterns = ["Error"]

[permissions]
approve = "y\n"
approve_all = "Y\n"
deny = "n\n"

[capabilities]
supports_hooks = true
supports_prompt_arg = true
supports_resume = false
supports_mcp = true
supports_worktree = false
"#;
        let config: AdapterConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.agent.name, "claude-code");
        assert_eq!(config.agent.command, "claude");
        assert_eq!(config.agent.args, vec!["--json"]);
        assert_eq!(config.agent.args_interactive, vec!["--interactive"]);
        assert_eq!(
            config.agent.version_check.as_deref(),
            Some("claude --version")
        );
        assert_eq!(config.agent.icon.as_deref(), Some("brain"));

        // Roundtrip: serialize back to TOML and re-parse (before partial move)
        let serialized = toml::to_string(&config).unwrap();
        let config2: AdapterConfig = toml::from_str(&serialized).unwrap();
        assert_eq!(config2.agent.name, "claude-code");
        assert_eq!(config2.agent.command, "claude");

        let hooks = config.hooks.unwrap();
        assert_eq!(hooks.hook_type, "mcp");
        assert_eq!(hooks.install, "manual");
        assert_eq!(hooks.state_dir.as_deref(), Some("/tmp/hooks"));

        assert_eq!(config.status.working_patterns, vec!["Thinking", "Writing"]);
        assert_eq!(config.status.idle_patterns, vec!["Waiting"]);

        assert_eq!(config.permissions.approve, "y\n");
        assert_eq!(config.permissions.approve_all, "Y\n");
        assert_eq!(config.permissions.deny, "n\n");

        assert!(config.capabilities.supports_hooks);
        assert!(config.capabilities.supports_prompt_arg);
        assert!(!config.capabilities.supports_resume);
        assert!(config.capabilities.supports_mcp);
        assert!(!config.capabilities.supports_worktree);
    }

    #[test]
    fn test_hooks_section_install_defaults_to_auto() {
        let toml_str = r#"
[agent]
name = "test"
command = "test"

[hooks]
type = "mcp"

[status]

[permissions]
"#;
        let config: AdapterConfig = toml::from_str(toml_str).unwrap();
        let hooks = config.hooks.unwrap();
        assert_eq!(hooks.install, "auto");
        assert!(hooks.state_dir.is_none());
    }

    #[test]
    fn test_permissions_section_defaults() {
        let toml_str = r#"
[agent]
name = "test"
command = "test"

[status]

[permissions]
"#;
        let config: AdapterConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.permissions.approve, "y\n");
        assert_eq!(config.permissions.approve_all, "Y\n");
        assert_eq!(config.permissions.deny, "n\n");
    }

    #[test]
    fn test_partial_deserialization_missing_optional_fields() {
        let toml_str = r#"
[agent]
name = "minimal"
command = "agent"

[status]

[permissions]
"#;
        let config: AdapterConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.agent.name, "minimal");
        assert!(config.agent.args.is_empty());
        assert!(config.agent.args_interactive.is_empty());
        assert!(config.agent.version_check.is_none());
        assert!(config.agent.icon.is_none());
        assert!(config.hooks.is_none());
        assert!(config.status.working_patterns.is_empty());
        assert!(config.status.idle_patterns.is_empty());
        assert!(config.status.input_patterns.is_empty());
        assert!(config.status.error_patterns.is_empty());
    }

    #[test]
    fn test_capabilities_section_defaults_all_false() {
        let caps = CapabilitiesSection::default();
        assert!(!caps.supports_hooks);
        assert!(!caps.supports_prompt_arg);
        assert!(!caps.supports_resume);
        assert!(!caps.supports_mcp);
        assert!(!caps.supports_worktree);
    }

    #[test]
    fn test_agent_section_with_optional_fields() {
        let toml_str = r#"
[agent]
name = "codex"
command = "codex"
version_check = "codex --version"
icon = "robot"

[status]

[permissions]
"#;
        let config: AdapterConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(
            config.agent.version_check.as_deref(),
            Some("codex --version")
        );
        assert_eq!(config.agent.icon.as_deref(), Some("robot"));
    }
}
