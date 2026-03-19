use std::path::PathBuf;

/// Sandbox profile controlling what agents can access.
/// Uses nono.sh (Seatbelt on macOS, Landlock on Linux) for kernel-level enforcement.
#[derive(Debug, Clone)]
pub struct SandboxProfile {
    pub enabled: bool,
    pub blocked_paths: Vec<String>,
    pub block_network: bool,
    pub extra_flags: Vec<String>,
}

impl Default for SandboxProfile {
    fn default() -> Self {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"));
        let home_str = home.to_string_lossy();
        Self {
            enabled: true,
            blocked_paths: vec![
                format!("{home_str}/.ssh"),
                format!("{home_str}/.aws"),
                format!("{home_str}/.gnupg"),
                format!("{home_str}/.config/gcloud"),
                format!("{home_str}/.azure"),
                format!("{home_str}/.kube"),
                format!("{home_str}/.bashrc"),
                format!("{home_str}/.zshrc"),
                format!("{home_str}/.profile"),
                format!("{home_str}/.bash_profile"),
                format!("{home_str}/.netrc"),
                format!("{home_str}/.npmrc"),
            ],
            block_network: false,
            extra_flags: vec![],
        }
    }
}

impl SandboxProfile {
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            blocked_paths: vec![],
            block_network: false,
            extra_flags: vec![],
        }
    }

    /// Wrap a command and args with nono.sh sandbox enforcement.
    /// If sandbox is disabled, returns the command unchanged.
    pub fn wrap_command(&self, command: &str, args: &[String]) -> (String, Vec<String>) {
        if !self.enabled {
            return (command.to_string(), args.to_vec());
        }

        let mut nono_args = Vec::new();

        for path in &self.blocked_paths {
            nono_args.push("--block".to_string());
            nono_args.push(path.clone());
        }

        if self.block_network {
            nono_args.push("--no-network".to_string());
        }

        for flag in &self.extra_flags {
            nono_args.push(flag.clone());
        }

        nono_args.push("--".to_string());
        nono_args.push(command.to_string());
        nono_args.extend(args.iter().cloned());

        ("nono".to_string(), nono_args)
    }

    /// Check if nono.sh is installed on the system
    pub fn is_available() -> bool {
        std::process::Command::new("nono")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_sandbox_profile_blocks_ssh() {
        let profile = SandboxProfile::default();
        assert!(profile.blocked_paths.iter().any(|p| p.contains(".ssh")));
    }

    #[test]
    fn test_default_sandbox_profile_blocks_aws() {
        let profile = SandboxProfile::default();
        assert!(profile.blocked_paths.iter().any(|p| p.contains(".aws")));
    }

    #[test]
    fn test_sandbox_command_wraps_with_nono() {
        let profile = SandboxProfile::default();
        let (cmd, args) = profile.wrap_command("claude", &["--auto".into()]);
        assert_eq!(cmd, "nono");
        assert!(args.contains(&"claude".to_string()));
    }

    #[test]
    fn test_sandbox_disabled_passes_through() {
        let profile = SandboxProfile::disabled();
        let (cmd, args) = profile.wrap_command("claude", &["--auto".into()]);
        assert_eq!(cmd, "claude");
        assert_eq!(args, vec!["--auto"]);
    }

    #[test]
    fn test_custom_blocked_paths() {
        let mut profile = SandboxProfile::default();
        profile.blocked_paths.push("/custom/secret".into());
        assert!(profile.blocked_paths.iter().any(|p| p == "/custom/secret"));
    }

    #[test]
    fn test_block_network_flag() {
        let mut profile = SandboxProfile::default();
        profile.block_network = true;
        let (cmd, args) = profile.wrap_command("claude", &["--auto".into()]);
        assert_eq!(cmd, "nono");
        assert!(args.contains(&"--no-network".to_string()));
    }

    #[test]
    fn test_extra_flags_passed_through() {
        let mut profile = SandboxProfile::default();
        profile.extra_flags = vec!["--verbose".into(), "--log=/tmp/nono.log".into()];
        let (cmd, args) = profile.wrap_command("claude", &[]);
        assert_eq!(cmd, "nono");
        assert!(args.contains(&"--verbose".to_string()));
        assert!(args.contains(&"--log=/tmp/nono.log".to_string()));
    }

    #[test]
    fn test_disabled_profile_fields() {
        let profile = SandboxProfile::disabled();
        assert!(!profile.enabled);
        assert!(profile.blocked_paths.is_empty());
        assert!(!profile.block_network);
        assert!(profile.extra_flags.is_empty());
    }

    #[test]
    fn test_default_profile_is_enabled() {
        let profile = SandboxProfile::default();
        assert!(profile.enabled);
        assert!(!profile.blocked_paths.is_empty());
        assert!(!profile.block_network);
    }

    #[test]
    fn test_default_blocks_sensitive_paths() {
        let profile = SandboxProfile::default();
        let sensitive = [".ssh", ".aws", ".gnupg", ".kube", ".netrc", ".npmrc"];
        for path_fragment in &sensitive {
            assert!(
                profile
                    .blocked_paths
                    .iter()
                    .any(|p| p.contains(path_fragment)),
                "Should block {path_fragment}"
            );
        }
    }

    #[test]
    fn test_wrap_command_separator() {
        let profile = SandboxProfile {
            enabled: true,
            blocked_paths: vec![],
            block_network: false,
            extra_flags: vec![],
        };
        let (cmd, args) = profile.wrap_command("echo", &["hello".into()]);
        assert_eq!(cmd, "nono");
        // Should have -- separator before the actual command
        let separator_pos = args.iter().position(|a| a == "--").unwrap();
        assert_eq!(args[separator_pos + 1], "echo");
        assert_eq!(args[separator_pos + 2], "hello");
    }

    #[test]
    fn test_wrap_preserves_arg_order() {
        let profile = SandboxProfile::default();
        let (_, args) =
            profile.wrap_command("git", &["push".into(), "origin".into(), "main".into()]);
        let cmd_pos = args.iter().position(|a| a == "git").unwrap();
        assert_eq!(args[cmd_pos + 1], "push");
        assert_eq!(args[cmd_pos + 2], "origin");
        assert_eq!(args[cmd_pos + 3], "main");
    }
}
