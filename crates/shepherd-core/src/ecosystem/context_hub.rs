use super::superpowers::InstallScope;
use super::{EcosystemPlugin, PluginDetectionResult};
use std::path::{Path, PathBuf};

/// Return the shared plugin definition for context-hub (chub).
pub fn plugin() -> EcosystemPlugin {
    EcosystemPlugin {
        name: "context-hub",
        description: "Andrew Ng's Context Hub — curated, versioned API docs for coding agents",
        compatible_agents: &["claude-code", "codex"],
        feature_key: "get-api-docs",
        plugin_cache_dirs: &[("claude-code", ".claude/skills/get-api-docs")],
        user_settings_paths: &[("claude-code", ".claude/skills/get-api-docs/SKILL.md")],
        project_settings_paths: &[],
        install_targets: &[
            (
                "claude-code",
                true,
                "~/.claude/skills/get-api-docs/SKILL.md",
            ),
            ("codex", true, "~/.codex/skills/get-api-docs/SKILL.md"),
        ],
        config_content: SKILL_CONTENT,
    }
}

/// The npm package to install globally.
pub const NPM_PACKAGE: &str = "@aisuite/chub";

/// The SKILL.md content installed under ~/.claude/skills/get-api-docs/.
const SKILL_CONTENT: &str = r#"---
name: get-api-docs
description: >
  Use this skill when you need documentation for a third-party library, SDK, or API
  before writing code that uses it — for example, "use the OpenAI API", "call the
  Stripe API", "use the Anthropic SDK", "query Pinecone", or any time the user asks
  you to write code against an external service and you need current API reference.
  Fetch the docs with chub before answering, rather than relying on training knowledge.
---

# Get API Docs via chub

When you need documentation for a library or API, fetch it with the `chub` CLI
rather than guessing from training data. This gives you the current, correct API.

## Step 1 — Find the right doc ID

```bash
chub search "<library name>" --json
```

Pick the best-matching `id` from the results (e.g. `openai/chat`, `anthropic/sdk`,
`stripe/api`). If nothing matches, try a broader term.

## Step 2 — Fetch the docs

```bash
chub get <id> --lang py    # or --lang js, --lang ts
```

Omit `--lang` if the doc has only one language variant — it will be auto-selected.

## Step 3 — Use the docs

Read the fetched content and use it to write accurate code or answer the question.
Do not rely on memorized API shapes — use what the docs say.

## Step 4 — Annotate what you learned

After completing the task, if you discovered something not in the doc — a gotcha,
workaround, version quirk, or project-specific detail — save it so future sessions
start smarter:

```bash
chub annotate <id> "Webhook verification requires raw body — do not parse before verifying"
```

Annotations are local, persist across sessions, and appear automatically on future
`chub get` calls. Keep notes concise and actionable. Don't repeat what's already in
the doc.

## Step 5 — Give feedback

Rate the doc so authors can improve it. Ask the user before sending.

```bash
chub feedback <id> up                        # doc worked well
chub feedback <id> down --label outdated     # doc needs updating
```

Available labels: `outdated`, `inaccurate`, `incomplete`, `wrong-examples`,
`wrong-version`, `poorly-structured`, `accurate`, `well-structured`, `helpful`,
`good-examples`.

## Quick reference

| Goal | Command |
|------|---------|
| List everything | `chub search` |
| Find a doc | `chub search "stripe"` |
| Exact id detail | `chub search stripe/api` |
| Fetch Python docs | `chub get stripe/api --lang py` |
| Fetch JS docs | `chub get openai/chat --lang js` |
| Save to file | `chub get anthropic/sdk --lang py -o docs.md` |
| Fetch multiple | `chub get openai/chat stripe/api --lang py` |
| Save a note | `chub annotate stripe/api "needs raw body"` |
| List notes | `chub annotate --list` |
| Rate a doc | `chub feedback stripe/api up` |

## Notes

- `chub search` with no query lists everything available
- IDs are `<author>/<name>` — confirm the ID from search before fetching
- If multiple languages exist and you don't pass `--lang`, chub will tell you which are available
"#;

// ── Backward-compatible API ──────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct DetectionResult {
    pub installed: bool,
    pub scope: InstallScope,
    pub path: Option<PathBuf>,
    pub cli_available: bool,
}

impl From<PluginDetectionResult> for DetectionResult {
    fn from(r: PluginDetectionResult) -> Self {
        Self {
            installed: r.installed,
            scope: r.scope,
            path: r.path,
            cli_available: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct InstallConfig {
    pub agent: String,
    pub scope: InstallScope,
    pub target_path: PathBuf,
    pub skill_content: String,
    pub npm_package: String,
}

pub fn detect_for_agent(agent: &str, home: &Path, project_root: Option<&Path>) -> DetectionResult {
    let mut result: DetectionResult = plugin().detect(agent, home, project_root).into();
    result.cli_available = is_cli_available();
    result
}

pub fn is_context_hub_compatible(agent: &str) -> bool {
    plugin().is_compatible(agent)
}

/// Check whether the `chub` CLI is available on PATH.
pub fn is_cli_available() -> bool {
    std::process::Command::new("chub")
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

impl InstallConfig {
    pub fn for_agent(agent: &str, scope: InstallScope) -> Option<Self> {
        let cfg = plugin().install_config(agent, scope)?;
        Some(Self {
            agent: cfg.agent,
            scope: cfg.scope,
            target_path: cfg.target_path,
            skill_content: SKILL_CONTENT.to_string(),
            npm_package: NPM_PACKAGE.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_not_installed() {
        let tmp = tempfile::tempdir().unwrap();
        let result = detect_for_agent("claude-code", tmp.path(), None);
        assert!(!result.installed);
    }

    #[test]
    fn test_detect_claude_code_skill_installed() {
        let tmp = tempfile::tempdir().unwrap();
        let skill_dir = tmp.path().join(".claude/skills/get-api-docs");
        std::fs::create_dir_all(&skill_dir).unwrap();
        std::fs::write(skill_dir.join("SKILL.md"), "get-api-docs").unwrap();
        let result = detect_for_agent("claude-code", tmp.path(), None);
        assert!(result.installed);
        assert_eq!(result.scope, InstallScope::User);
    }

    #[test]
    fn test_install_config_claude_code() {
        let config = InstallConfig::for_agent("claude-code", InstallScope::User).unwrap();
        assert!(config
            .target_path
            .to_string_lossy()
            .contains("get-api-docs"));
        assert!(config.skill_content.contains("chub"));
        assert_eq!(config.npm_package, "@aisuite/chub");
    }

    #[test]
    fn test_install_config_codex() {
        let config = InstallConfig::for_agent("codex", InstallScope::User).unwrap();
        assert!(config
            .target_path
            .to_string_lossy()
            .contains("get-api-docs"));
    }

    #[test]
    fn test_supported_agents() {
        assert!(is_context_hub_compatible("claude-code"));
        assert!(is_context_hub_compatible("codex"));
        assert!(!is_context_hub_compatible("aider"));
        assert!(!is_context_hub_compatible("gemini-cli"));
    }

    #[test]
    fn test_unsupported_agent_returns_none() {
        let config = InstallConfig::for_agent("aider", InstallScope::User);
        assert!(config.is_none());
    }

    #[test]
    fn test_skill_content_has_required_sections() {
        assert!(SKILL_CONTENT.contains("chub search"));
        assert!(SKILL_CONTENT.contains("chub get"));
        assert!(SKILL_CONTENT.contains("chub annotate"));
        assert!(SKILL_CONTENT.contains("chub feedback"));
    }

    #[test]
    fn test_plugin_definition() {
        let p = plugin();
        assert_eq!(p.name, "context-hub");
        assert_eq!(p.feature_key, "get-api-docs");
        assert!(p.compatible_agents.contains(&"claude-code"));
        assert!(p.compatible_agents.contains(&"codex"));
    }
}
