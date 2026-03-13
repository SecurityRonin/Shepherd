//! Context injection — converts a ContextPackage into injectable
//! payloads for different agent types.
//!
//! Each agent accepts context differently:
//! - Claude Code: reads CLAUDE.md in the project root
//! - Agents with --prompt args: prepend context as initial prompt
//! - Generic agents: pipe context as first stdin message
//!
//! This module generates the injection payload without performing I/O,
//! so it's testable and decoupled from the PTY layer.

use super::package::ContextPackage;

/// How context should be delivered to the agent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InjectionStrategy {
    /// Append to CLAUDE.md (or create it) before spawning.
    ClaudeMd,
    /// Pass as --prompt or -p argument.
    PromptArg,
    /// Write to agent stdin as the first message.
    StdinMessage,
    /// Create a context file the agent can reference.
    ContextFile,
}

/// Result of preparing context injection.
#[derive(Debug, Clone)]
pub struct InjectionPayload {
    pub strategy: InjectionStrategy,
    /// The formatted text content to inject.
    pub content: String,
    /// Additional CLI args to append (for PromptArg strategy).
    pub extra_args: Vec<String>,
    /// File paths the agent should be aware of (for --file flags).
    pub file_hints: Vec<String>,
}

/// Determine which injection strategy to use based on agent capabilities.
pub fn select_strategy(agent_id: &str, supports_prompt_arg: bool) -> InjectionStrategy {
    // Claude Code and its variants use CLAUDE.md
    if agent_id.starts_with("claude") {
        return InjectionStrategy::ClaudeMd;
    }
    // Agents with prompt arg support get CLI injection
    if supports_prompt_arg {
        return InjectionStrategy::PromptArg;
    }
    // Fallback: stdin message
    InjectionStrategy::StdinMessage
}

/// Format a context package into injectable text.
pub fn format_context(package: &ContextPackage) -> String {
    let mut sections = Vec::new();

    // Header
    sections.push("# Shepherd Context".to_string());
    sections.push(String::new());
    sections.push(format!("> {}", package.summary));
    sections.push(String::new());

    // Relevant files
    if !package.items.is_empty() {
        sections.push("## Relevant Files".to_string());
        sections.push(String::new());
        for item in &package.items {
            let path = item.file_path.to_string_lossy();
            sections.push(format!(
                "- `{}` (score: {:.2}) — {}",
                path, item.relevance_score, item.reason
            ));
        }
        sections.push(String::new());
    }

    // MCP queries as suggested reads
    if !package.mcp_queries.is_empty() {
        sections.push("## Suggested Investigations".to_string());
        sections.push(String::new());
        for query in &package.mcp_queries {
            sections.push(format!(
                "- {} `{}`: {}",
                query.server, query.tool, query.reason
            ));
        }
        sections.push(String::new());
    }

    sections.join("\n")
}

/// Format context as a compact initial prompt (for --prompt or stdin).
pub fn format_prompt(package: &ContextPackage) -> String {
    let mut parts = Vec::new();

    parts.push(format!("Context: {}", package.summary));

    if !package.items.is_empty() {
        let files: Vec<String> = package
            .items
            .iter()
            .map(|i| i.file_path.to_string_lossy().to_string())
            .collect();
        parts.push(format!("Key files: {}", files.join(", ")));
    }

    parts.join("\n")
}

/// Build a complete injection payload for a given agent.
pub fn prepare_injection(
    package: &ContextPackage,
    agent_id: &str,
    supports_prompt_arg: bool,
) -> InjectionPayload {
    let strategy = select_strategy(agent_id, supports_prompt_arg);

    let content = match strategy {
        InjectionStrategy::ClaudeMd | InjectionStrategy::ContextFile => format_context(package),
        InjectionStrategy::PromptArg | InjectionStrategy::StdinMessage => format_prompt(package),
    };

    let extra_args = if strategy == InjectionStrategy::PromptArg {
        vec!["-p".to_string(), format_prompt(package)]
    } else {
        vec![]
    };

    let file_hints: Vec<String> = package
        .items
        .iter()
        .map(|i| i.file_path.to_string_lossy().to_string())
        .collect();

    InjectionPayload {
        strategy,
        content,
        extra_args,
        file_hints,
    }
}

/// Generate CLAUDE.md context section marker for safe append/removal.
pub fn claude_md_section(content: &str) -> String {
    format!(
        "\n<!-- shepherd-context-start -->\n{content}<!-- shepherd-context-end -->\n"
    )
}

/// Remove previously injected Shepherd context from CLAUDE.md content.
pub fn remove_claude_md_section(claude_md: &str) -> String {
    if let Some(start) = claude_md.find("<!-- shepherd-context-start -->") {
        if let Some(end) = claude_md.find("<!-- shepherd-context-end -->") {
            let end = end + "<!-- shepherd-context-end -->".len();
            // Remove the section plus surrounding newlines
            let before = claude_md[..start].trim_end_matches('\n');
            let after = claude_md[end..].trim_start_matches('\n');
            if before.is_empty() {
                return after.to_string();
            }
            return format!("{before}\n{after}");
        }
    }
    claude_md.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::package::{ContextItem, ContextSource, McpQuery};
    use std::path::PathBuf;

    fn sample_package() -> ContextPackage {
        ContextPackage {
            id: "ctx-test".into(),
            task_id: Some(1),
            items: vec![
                ContextItem {
                    source: ContextSource::FileReference,
                    file_path: PathBuf::from("src/auth/mod.rs"),
                    relevance_score: 1.0,
                    reason: "Directly mentioned".into(),
                },
                ContextItem {
                    source: ContextSource::Structural,
                    file_path: PathBuf::from("src/db/mod.rs"),
                    relevance_score: 0.7,
                    reason: "Imported by auth".into(),
                },
            ],
            mcp_queries: vec![McpQuery {
                server: "serena".into(),
                tool: "find_symbol".into(),
                params: serde_json::json!({"name": "AuthService"}),
                reason: "Find AuthService definition".into(),
            }],
            summary: "Found 2 relevant files (1 directly referenced, 1 via structural analysis)."
                .into(),
            created_at: "2026-03-13T00:00:00Z".into(),
        }
    }

    fn empty_package() -> ContextPackage {
        ContextPackage {
            id: "ctx-empty".into(),
            task_id: None,
            items: vec![],
            mcp_queries: vec![],
            summary: "No relevant context found.".into(),
            created_at: "2026-03-13T00:00:00Z".into(),
        }
    }

    // ── Strategy selection ─────────────────────────────────────────

    #[test]
    fn strategy_claude_agents_use_claude_md() {
        assert_eq!(
            select_strategy("claude-code", false),
            InjectionStrategy::ClaudeMd
        );
        assert_eq!(
            select_strategy("claude-code", true),
            InjectionStrategy::ClaudeMd
        );
    }

    #[test]
    fn strategy_prompt_arg_agents() {
        assert_eq!(
            select_strategy("codex", true),
            InjectionStrategy::PromptArg
        );
        assert_eq!(
            select_strategy("aider", true),
            InjectionStrategy::PromptArg
        );
    }

    #[test]
    fn strategy_generic_agents_use_stdin() {
        assert_eq!(
            select_strategy("opencode", false),
            InjectionStrategy::StdinMessage
        );
    }

    // ── Context formatting ────────────────────────────────────────

    #[test]
    fn format_context_includes_header() {
        let pkg = sample_package();
        let formatted = format_context(&pkg);
        assert!(formatted.contains("# Shepherd Context"));
        assert!(formatted.contains("Found 2 relevant files"));
    }

    #[test]
    fn format_context_includes_files() {
        let pkg = sample_package();
        let formatted = format_context(&pkg);
        assert!(formatted.contains("src/auth/mod.rs"));
        assert!(formatted.contains("src/db/mod.rs"));
        assert!(formatted.contains("score: 1.00"));
    }

    #[test]
    fn format_context_includes_mcp_queries() {
        let pkg = sample_package();
        let formatted = format_context(&pkg);
        assert!(formatted.contains("serena"));
        assert!(formatted.contains("find_symbol"));
        assert!(formatted.contains("AuthService"));
    }

    #[test]
    fn format_context_empty_package() {
        let pkg = empty_package();
        let formatted = format_context(&pkg);
        assert!(formatted.contains("# Shepherd Context"));
        assert!(formatted.contains("No relevant context"));
        assert!(!formatted.contains("## Relevant Files"));
    }

    #[test]
    fn format_prompt_is_compact() {
        let pkg = sample_package();
        let prompt = format_prompt(&pkg);
        assert!(prompt.contains("Context:"));
        assert!(prompt.contains("Key files:"));
        assert!(prompt.contains("src/auth/mod.rs"));
        // Prompt should NOT contain markdown headers
        assert!(!prompt.contains("# Shepherd"));
    }

    // ── Injection payload ─────────────────────────────────────────

    #[test]
    fn prepare_injection_claude_code() {
        let pkg = sample_package();
        let payload = prepare_injection(&pkg, "claude-code", false);
        assert_eq!(payload.strategy, InjectionStrategy::ClaudeMd);
        assert!(payload.content.contains("# Shepherd Context"));
        assert!(payload.extra_args.is_empty());
        assert_eq!(payload.file_hints.len(), 2);
    }

    #[test]
    fn prepare_injection_prompt_arg() {
        let pkg = sample_package();
        let payload = prepare_injection(&pkg, "codex", true);
        assert_eq!(payload.strategy, InjectionStrategy::PromptArg);
        assert!(!payload.extra_args.is_empty());
        assert_eq!(payload.extra_args[0], "-p");
    }

    #[test]
    fn prepare_injection_stdin() {
        let pkg = sample_package();
        let payload = prepare_injection(&pkg, "opencode", false);
        assert_eq!(payload.strategy, InjectionStrategy::StdinMessage);
        assert!(payload.extra_args.is_empty());
        assert!(payload.content.contains("Context:"));
    }

    // ── CLAUDE.md section management ──────────────────────────────

    #[test]
    fn claude_md_section_wraps_content() {
        let section = claude_md_section("hello");
        assert!(section.contains("<!-- shepherd-context-start -->"));
        assert!(section.contains("<!-- shepherd-context-end -->"));
        assert!(section.contains("hello"));
    }

    #[test]
    fn remove_claude_md_section_cleans_up() {
        let original = "# My Project\n\nSome docs.\n\n<!-- shepherd-context-start -->\ninjected stuff\n<!-- shepherd-context-end -->\n\nMore docs.";
        let cleaned = remove_claude_md_section(original);
        assert!(!cleaned.contains("shepherd-context"));
        assert!(!cleaned.contains("injected stuff"));
        assert!(cleaned.contains("# My Project"));
        assert!(cleaned.contains("More docs."));
    }

    #[test]
    fn remove_claude_md_section_no_section() {
        let original = "# My Project\n\nNo injected section here.";
        let cleaned = remove_claude_md_section(original);
        assert_eq!(cleaned, original);
    }

    #[test]
    fn remove_claude_md_section_only_section() {
        let original = "<!-- shepherd-context-start -->\nstuff\n<!-- shepherd-context-end -->";
        let cleaned = remove_claude_md_section(original);
        assert!(cleaned.is_empty());
    }

    #[test]
    fn file_hints_lists_all_items() {
        let pkg = sample_package();
        let payload = prepare_injection(&pkg, "claude-code", false);
        assert!(payload.file_hints.contains(&"src/auth/mod.rs".to_string()));
        assert!(payload.file_hints.contains(&"src/db/mod.rs".to_string()));
    }
}
