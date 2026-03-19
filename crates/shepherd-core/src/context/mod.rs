//! Context Orchestrator — Layer 3 of Shepherd's intelligence stack.
//!
//! Automatically assembles relevant context for agent sessions by:
//! 1. Extracting intent from Kanban task descriptions (file paths, symbols, keywords)
//! 2. Querying structural providers (file references, imports, symbol definitions)
//! 3. Querying semantic providers (keyword matching, content similarity)
//! 4. Deduplicating, ranking, and assembling into a context package
//! 5. Suggesting MCP queries for Serena (LSP) and Sourcegraph (search)
//! 6. Tracking which context helped agents succeed (feedback loop)

pub mod extractor;
pub mod feedback;
pub mod index;
pub mod injection;
pub mod mcp_executor;
pub mod package;
pub mod provider;
pub mod tfidf;
pub mod tfidf_provider;

pub use extractor::{extract_intent, TaskIntent};
pub use index::{scan_and_index, IndexedFile};
pub use injection::{prepare_injection, InjectionPayload, InjectionStrategy};
pub use package::*;
pub use provider::{ContextProvider, SemanticProvider, StructuralProvider};
pub use tfidf::TfIdfCorpus;
pub use tfidf_provider::TfIdfProvider;

/// The main context orchestrator that combines multiple providers.
pub struct ContextOrchestrator {
    providers: Vec<Box<dyn ContextProvider + Send + Sync>>,
}

impl ContextOrchestrator {
    /// Create a new orchestrator with the default provider stack
    /// (structural + semantic).
    pub fn new() -> Self {
        Self {
            providers: vec![Box::new(StructuralProvider), Box::new(SemanticProvider)],
        }
    }

    /// Create an orchestrator with custom providers.
    // tarpaulin-start-ignore
    pub fn with_providers(providers: Vec<Box<dyn ContextProvider + Send + Sync>>) -> Self {
        Self { providers }
    }
    // tarpaulin-stop-ignore

    /// Build a context package for a task.
    ///
    /// This is the main entry point. Given a task's title and description,
    /// it extracts intent, queries all providers, deduplicates and ranks
    /// results, generates MCP query suggestions, and returns a ready-to-use
    /// context package.
    pub fn build_context(&self, request: &ContextRequest) -> ContextPackage {
        let intent = extract_intent(&request.task_title, &request.task_description);

        let mut all_items = Vec::new();
        let mut all_queries = Vec::new();

        // Query each provider
        for provider in &self.providers {
            let items = provider.find_context(&intent, &request.repo_path);
            all_items.extend(items);
            let queries = provider.suggest_mcp_queries(&intent);
            all_queries.extend(queries);
        }

        // Deduplicate by file path, keeping highest-scoring entry
        let items = Self::deduplicate_and_rank(all_items, request.max_files);

        // Generate summary
        let summary = Self::generate_summary(&intent, &items);

        // Generate ID from timestamp
        let id = format!("ctx-{}", chrono::Utc::now().format("%Y%m%d%H%M%S%3f"));

        ContextPackage {
            id,
            task_id: request.task_id,
            items,
            mcp_queries: all_queries,
            summary,
            created_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    /// Deduplicate items by file path, keeping the highest score per file,
    /// then sort by score and truncate to max_files.
    fn deduplicate_and_rank(items: Vec<ContextItem>, max_files: usize) -> Vec<ContextItem> {
        let mut best_per_file: std::collections::HashMap<String, ContextItem> =
            std::collections::HashMap::new();

        for item in items {
            let key = item.file_path.to_string_lossy().to_string();
            let entry = best_per_file.entry(key);
            entry
                .and_modify(|existing| {
                    if item.relevance_score > existing.relevance_score {
                        *existing = item.clone();
                    }
                })
                .or_insert(item);
        }

        let mut ranked: Vec<ContextItem> = best_per_file.into_values().collect();
        ranked.sort_by(|a, b| {
            b.relevance_score
                .partial_cmp(&a.relevance_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        ranked.truncate(max_files);
        ranked
    }

    /// Generate a natural-language summary of the context package.
    fn generate_summary(intent: &TaskIntent, items: &[ContextItem]) -> String {
        let mut parts = Vec::new();

        if !items.is_empty() {
            let file_refs = items
                .iter()
                .filter(|i| i.source == ContextSource::FileReference)
                .count();
            let structural = items
                .iter()
                .filter(|i| i.source == ContextSource::Structural)
                .count();
            let semantic = items
                .iter()
                .filter(|i| i.source == ContextSource::Semantic)
                .count();

            parts.push(format!("Found {} relevant files", items.len()));
            let mut breakdown = Vec::new();
            if file_refs > 0 {
                breakdown.push(format!("{file_refs} directly referenced"));
            }
            if structural > 0 {
                breakdown.push(format!("{structural} via structural analysis"));
            }
            if semantic > 0 {
                breakdown.push(format!("{semantic} via semantic matching"));
            }
            if !breakdown.is_empty() {
                parts.push(format!("({})", breakdown.join(", ")));
            }
        }

        if !intent.symbols.is_empty() {
            parts.push(format!(
                "Key symbols: {}",
                intent
                    .symbols
                    .iter()
                    .take(5)
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }

        if parts.is_empty() {
            "No relevant context found for this task.".to_string()
        } else {
            parts.join(". ") + "."
        }
    }
}

// tarpaulin-start-ignore
impl Default for ContextOrchestrator {
    fn default() -> Self {
        Self::new()
    }
}
// tarpaulin-stop-ignore

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn create_test_repo() -> tempfile::TempDir {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("src");
        std::fs::create_dir_all(src.join("auth")).unwrap();
        std::fs::create_dir_all(src.join("db")).unwrap();
        std::fs::create_dir_all(src.join("api")).unwrap();

        std::fs::write(
            src.join("main.rs"),
            "use crate::auth;\nuse crate::db;\n\nfn main() {\n    AuthService::new();\n}\n",
        )
        .unwrap();
        std::fs::write(
            src.join("auth/mod.rs"),
            "use crate::db;\npub struct AuthService;\nimpl AuthService {\n    pub fn new() -> Self { Self }\n}\n",
        ).unwrap();
        std::fs::write(
            src.join("db/mod.rs"),
            "pub struct Database;\nimpl Database {\n    pub fn connect() -> Self { Self }\n}\n",
        )
        .unwrap();
        std::fs::write(
            src.join("api/handler.rs"),
            "use crate::auth::AuthService;\npub fn handle_login() {}\n",
        )
        .unwrap();

        tmp
    }

    // ── Orchestrator integration tests ───────────────────────────

    #[test]
    fn build_context_finds_referenced_files() {
        let repo = create_test_repo();
        let orchestrator = ContextOrchestrator::new();
        let request = ContextRequest {
            task_id: Some(1),
            task_title: "Fix auth bug".into(),
            task_description: "The bug is in src/auth/mod.rs".into(),
            repo_path: repo.path().to_path_buf(),
            agent: "claude-code".into(),
            max_files: 20,
        };

        let pkg = orchestrator.build_context(&request);
        assert!(pkg
            .items
            .iter()
            .any(|i| i.file_path == PathBuf::from("src/auth/mod.rs")));
    }

    #[test]
    fn build_context_finds_dependencies() {
        let repo = create_test_repo();
        let orchestrator = ContextOrchestrator::new();
        let request = ContextRequest {
            task_id: Some(1),
            task_title: "Fix auth".into(),
            task_description: "Check src/auth/mod.rs".into(),
            repo_path: repo.path().to_path_buf(),
            agent: "claude-code".into(),
            max_files: 20,
        };

        let pkg = orchestrator.build_context(&request);
        // auth/mod.rs imports db, so db/mod.rs should appear
        assert!(pkg
            .items
            .iter()
            .any(|i| i.file_path == PathBuf::from("src/db/mod.rs")));
    }

    #[test]
    fn build_context_respects_max_files() {
        let repo = create_test_repo();
        let orchestrator = ContextOrchestrator::new();
        let request = ContextRequest {
            task_id: Some(1),
            task_title: "Fix everything".into(),
            task_description: "Check all files in the auth and db modules".into(),
            repo_path: repo.path().to_path_buf(),
            agent: "claude-code".into(),
            max_files: 2,
        };

        let pkg = orchestrator.build_context(&request);
        assert!(pkg.items.len() <= 2);
    }

    #[test]
    fn build_context_generates_mcp_queries() {
        let repo = create_test_repo();
        let orchestrator = ContextOrchestrator::new();
        let request = ContextRequest {
            task_id: Some(1),
            task_title: "Fix AuthService".into(),
            task_description: "The AuthService check_access method is broken".into(),
            repo_path: repo.path().to_path_buf(),
            agent: "claude-code".into(),
            max_files: 20,
        };

        let pkg = orchestrator.build_context(&request);
        // Should have Serena queries for AuthService
        assert!(pkg
            .mcp_queries
            .iter()
            .any(|q| q.server == "serena" && q.tool == "find_symbol"));
        // Should have Sourcegraph search queries
        assert!(pkg.mcp_queries.iter().any(|q| q.server == "sourcegraph"));
    }

    #[test]
    fn build_context_deduplicates_files() {
        let repo = create_test_repo();
        let orchestrator = ContextOrchestrator::new();
        let request = ContextRequest {
            task_id: Some(1),
            task_title: "Fix auth".into(),
            task_description: "The auth module at src/auth/mod.rs with AuthService".into(),
            repo_path: repo.path().to_path_buf(),
            agent: "claude-code".into(),
            max_files: 20,
        };

        let pkg = orchestrator.build_context(&request);
        // auth/mod.rs should appear only once despite being found by
        // multiple providers (file reference + symbol match + semantic)
        let auth_count = pkg
            .items
            .iter()
            .filter(|i| i.file_path == PathBuf::from("src/auth/mod.rs"))
            .count();
        assert_eq!(auth_count, 1);
    }

    #[test]
    fn build_context_sorted_by_relevance() {
        let repo = create_test_repo();
        let orchestrator = ContextOrchestrator::new();
        let request = ContextRequest {
            task_id: Some(1),
            task_title: "Fix auth".into(),
            task_description: "Check src/auth/mod.rs for the bug".into(),
            repo_path: repo.path().to_path_buf(),
            agent: "claude-code".into(),
            max_files: 20,
        };

        let pkg = orchestrator.build_context(&request);
        if pkg.items.len() >= 2 {
            assert!(pkg.items[0].relevance_score >= pkg.items[1].relevance_score);
        }
    }

    #[test]
    fn build_context_generates_summary() {
        let repo = create_test_repo();
        let orchestrator = ContextOrchestrator::new();
        let request = ContextRequest {
            task_id: Some(1),
            task_title: "Fix AuthService".into(),
            task_description: "src/auth/mod.rs is broken".into(),
            repo_path: repo.path().to_path_buf(),
            agent: "claude-code".into(),
            max_files: 20,
        };

        let pkg = orchestrator.build_context(&request);
        assert!(!pkg.summary.is_empty());
        assert!(pkg.summary.contains("relevant files") || pkg.summary.contains("Key symbols"));
    }

    #[test]
    fn build_context_has_valid_id_and_timestamp() {
        let repo = create_test_repo();
        let orchestrator = ContextOrchestrator::new();
        let request = ContextRequest {
            task_id: Some(42),
            task_title: "Test".into(),
            task_description: "Testing".into(),
            repo_path: repo.path().to_path_buf(),
            agent: "claude-code".into(),
            max_files: 10,
        };

        let pkg = orchestrator.build_context(&request);
        assert!(pkg.id.starts_with("ctx-"));
        assert!(!pkg.created_at.is_empty());
        assert_eq!(pkg.task_id, Some(42));
    }

    #[test]
    fn build_context_empty_task_produces_empty_package() {
        let repo = create_test_repo();
        let orchestrator = ContextOrchestrator::new();
        let request = ContextRequest {
            task_id: None,
            task_title: "Do something".into(),
            task_description: "".into(),
            repo_path: repo.path().to_path_buf(),
            agent: "claude-code".into(),
            max_files: 10,
        };

        let pkg = orchestrator.build_context(&request);
        // Should still produce a valid package even with no matches
        assert!(pkg.id.starts_with("ctx-"));
    }

    #[test]
    fn with_providers_creates_custom_orchestrator() {
        let orchestrator = ContextOrchestrator::with_providers(vec![Box::new(StructuralProvider)]);
        let repo = create_test_repo();
        let request = ContextRequest {
            task_id: Some(1),
            task_title: "Fix auth".into(),
            task_description: "Check src/auth/mod.rs".into(),
            repo_path: repo.path().to_path_buf(),
            agent: "claude-code".into(),
            max_files: 20,
        };
        let pkg = orchestrator.build_context(&request);
        // With only StructuralProvider, there should be no Semantic items
        assert!(pkg
            .items
            .iter()
            .all(|i| i.source != ContextSource::Semantic));
    }

    #[test]
    fn default_orchestrator_same_as_new() {
        let repo = create_test_repo();
        let orchestrator = ContextOrchestrator::default();
        let request = ContextRequest {
            task_id: Some(1),
            task_title: "Fix auth".into(),
            task_description: "Check src/auth/mod.rs".into(),
            repo_path: repo.path().to_path_buf(),
            agent: "claude-code".into(),
            max_files: 20,
        };
        let pkg = orchestrator.build_context(&request);
        assert!(pkg.id.starts_with("ctx-"));
        // Default has both Structural and Semantic providers
        assert!(!pkg.items.is_empty());
    }

    // ── Deduplication tests ──────────────────────────────────────

    #[test]
    fn deduplicate_keeps_highest_score() {
        let items = vec![
            ContextItem {
                source: ContextSource::Semantic,
                file_path: PathBuf::from("src/auth.rs"),
                relevance_score: 0.3,
                reason: "keyword match".into(),
            },
            ContextItem {
                source: ContextSource::FileReference,
                file_path: PathBuf::from("src/auth.rs"),
                relevance_score: 1.0,
                reason: "directly referenced".into(),
            },
        ];

        let deduped = ContextOrchestrator::deduplicate_and_rank(items, 10);
        assert_eq!(deduped.len(), 1);
        assert_eq!(deduped[0].relevance_score, 1.0);
        assert_eq!(deduped[0].source, ContextSource::FileReference);
    }

    #[test]
    fn deduplicate_respects_max_files() {
        let items: Vec<ContextItem> = (0..10)
            .map(|i| ContextItem {
                source: ContextSource::Semantic,
                file_path: PathBuf::from(format!("src/file_{i}.rs")),
                relevance_score: 0.5 - (i as f64 * 0.01),
                reason: "test".into(),
            })
            .collect();

        let deduped = ContextOrchestrator::deduplicate_and_rank(items, 3);
        assert_eq!(deduped.len(), 3);
    }

    // ── Summary generation tests ─────────────────────────────────

    #[test]
    fn summary_mentions_file_count() {
        let items = vec![
            ContextItem {
                source: ContextSource::FileReference,
                file_path: PathBuf::from("a.rs"),
                relevance_score: 1.0,
                reason: "test".into(),
            },
            ContextItem {
                source: ContextSource::Structural,
                file_path: PathBuf::from("b.rs"),
                relevance_score: 0.7,
                reason: "test".into(),
            },
        ];
        let intent = TaskIntent::default();
        let summary = ContextOrchestrator::generate_summary(&intent, &items);
        assert!(summary.contains("2 relevant files"));
    }

    #[test]
    fn summary_mentions_symbols() {
        let items = vec![];
        let intent = TaskIntent {
            symbols: vec!["AuthService".into()],
            ..Default::default()
        };
        let summary = ContextOrchestrator::generate_summary(&intent, &items);
        assert!(summary.contains("AuthService"));
    }

    #[test]
    fn summary_no_context_message() {
        let intent = TaskIntent::default();
        let summary = ContextOrchestrator::generate_summary(&intent, &[]);
        assert!(summary.contains("No relevant context"));
    }
}
