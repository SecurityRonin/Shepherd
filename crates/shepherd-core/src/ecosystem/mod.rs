pub mod superpowers;
pub mod context_mode;
pub mod context7;
pub mod ralph_loop;
pub mod frontend_design;
pub mod docling;
pub mod whisper;
pub mod ffmpeg;
pub mod playwright;
pub mod exa;
pub mod serena;
pub mod sourcegraph;

use std::path::{Path, PathBuf};
use superpowers::InstallScope;

/// Detection result for an ecosystem plugin.
#[derive(Debug, Clone)]
pub struct PluginDetectionResult {
    pub installed: bool,
    pub scope: InstallScope,
    pub path: Option<PathBuf>,
}

/// Install configuration for an ecosystem plugin.
#[derive(Debug, Clone)]
pub struct PluginInstallConfig {
    pub agent: String,
    pub scope: InstallScope,
    pub target_path: PathBuf,
    pub config_content: String,
}

/// A shared, data-driven definition for an ecosystem plugin.
///
/// Each ecosystem module defines a `plugin()` function that returns one of
/// these, moving detection and installation logic into shared methods and
/// eliminating duplicated boilerplate.
#[derive(Debug, Clone)]
pub struct EcosystemPlugin {
    /// Human-readable name (e.g. "context-mode").
    pub name: &'static str,
    /// Short description of what the plugin provides.
    pub description: &'static str,
    /// Agents this plugin is compatible with.
    pub compatible_agents: &'static [&'static str],
    /// Feature string to search for in settings files.
    pub feature_key: &'static str,
    /// Optional plugin cache directories to check (relative to home).
    /// Each entry is `(agent, relative_path)`.
    pub plugin_cache_dirs: &'static [(&'static str, &'static str)],
    /// Settings file paths to check per agent (relative to home for user scope).
    /// Each entry is `(agent, relative_path)`.
    pub user_settings_paths: &'static [(&'static str, &'static str)],
    /// Settings file paths to check per agent (relative to project root for project scope).
    /// Each entry is `(agent, relative_path)`.
    pub project_settings_paths: &'static [(&'static str, &'static str)],
    /// Install target path per agent+scope: `(agent, scope_is_user, relative_path)`.
    pub install_targets: &'static [(&'static str, bool, &'static str)],
    /// Config content to write on install.
    pub config_content: &'static str,
}

impl EcosystemPlugin {
    /// Check whether this plugin is compatible with the given agent.
    pub fn is_compatible(&self, agent: &str) -> bool {
        self.compatible_agents.contains(&agent)
    }

    /// Detect whether this plugin is installed for the given agent.
    pub fn detect(&self, agent: &str, home: &Path, project_root: Option<&Path>) -> PluginDetectionResult {
        if !self.is_compatible(agent) {
            return PluginDetectionResult {
                installed: false,
                scope: InstallScope::User,
                path: None,
            };
        }

        // Check project scope first
        if let Some(project) = project_root {
            if let Some(result) = self.detect_project_scope(agent, project) {
                return result;
            }
        }

        self.detect_user_scope(agent, home)
    }

    fn detect_user_scope(&self, agent: &str, home: &Path) -> PluginDetectionResult {
        // Check plugin cache directories first
        for &(a, rel_path) in self.plugin_cache_dirs {
            if a == agent {
                let path = home.join(rel_path);
                if path.exists() {
                    return PluginDetectionResult {
                        installed: true,
                        scope: InstallScope::User,
                        path: Some(path),
                    };
                }
            }
        }

        // Check settings files
        for &(a, rel_path) in self.user_settings_paths {
            if a == agent {
                let settings_path = home.join(rel_path);
                if settings_path.exists() {
                    let content = std::fs::read_to_string(&settings_path).unwrap_or_default();
                    if content.contains(self.feature_key) {
                        return PluginDetectionResult {
                            installed: true,
                            scope: InstallScope::User,
                            path: Some(settings_path),
                        };
                    }
                }
            }
        }

        PluginDetectionResult {
            installed: false,
            scope: InstallScope::User,
            path: None,
        }
    }

    fn detect_project_scope(&self, agent: &str, project: &Path) -> Option<PluginDetectionResult> {
        for &(a, rel_path) in self.project_settings_paths {
            if a == agent {
                let config_path = project.join(rel_path);
                if config_path.exists() {
                    let content = std::fs::read_to_string(&config_path).unwrap_or_default();
                    if content.contains(self.feature_key) {
                        return Some(PluginDetectionResult {
                            installed: true,
                            scope: InstallScope::Project,
                            path: Some(config_path),
                        });
                    }
                }
            }
        }
        None
    }

    /// Build an install configuration for the given agent and scope.
    pub fn install_config(&self, agent: &str, scope: InstallScope) -> Option<PluginInstallConfig> {
        if !self.is_compatible(agent) {
            return None;
        }

        let is_user = scope == InstallScope::User;
        let target_path = self
            .install_targets
            .iter()
            .find(|&&(a, user, _)| a == agent && user == is_user)
            .map(|&(_, _, path)| PathBuf::from(path))?;

        Some(PluginInstallConfig {
            agent: agent.to_string(),
            scope,
            target_path,
            config_content: self.config_content.to_string(),
        })
    }
}
