//! TF-IDF-based context provider.
//!
//! Uses term frequency–inverse document frequency to score files by
//! keyword relevance. More accurate than simple substring matching
//! because it accounts for term rarity across the codebase.

use std::path::Path;

use super::extractor::TaskIntent;
use super::package::{ContextItem, ContextSource, McpQuery};
use super::provider::{ContextProvider, StructuralProvider};
use super::tfidf::{tokenize, TfIdfCorpus};

/// A context provider that scores files using TF-IDF.
pub struct TfIdfProvider;

impl TfIdfProvider {
    /// Build a TF-IDF corpus from source files in a repo.
    fn build_corpus(repo_path: &Path) -> (TfIdfCorpus, Vec<std::path::PathBuf>) {
        let source_files = StructuralProvider::collect_source_files(repo_path, 8);
        let mut corpus = TfIdfCorpus::new();

        for file_path in &source_files {
            let full_path = repo_path.join(file_path);
            let content = match std::fs::read_to_string(&full_path) {
                Ok(c) => c,
                Err(_) => continue, // tarpaulin-start-ignore
            }; // tarpaulin-stop-ignore
            // Limit content to first 300 lines for performance
            let preview: String = content.lines().take(300).collect::<Vec<_>>().join("\n");
            // Include file path in the document for path-based matching
            let doc_content = format!("{} {}", file_path.to_string_lossy(), preview);
            corpus.add_document(&file_path.to_string_lossy(), &doc_content);
        }

        (corpus, source_files)
    }

    /// Build query terms from a TaskIntent.
    fn build_query_terms(intent: &TaskIntent) -> Vec<String> {
        let mut terms = Vec::new();

        // Add keywords directly
        for kw in &intent.keywords {
            terms.push(kw.clone());
        }

        // Tokenize symbols (CamelCase → individual words)
        for symbol in &intent.symbols {
            let tokens = tokenize(symbol);
            terms.extend(tokens);
        }

        // Deduplicate
        terms.sort();
        terms.dedup();
        terms
    }
}

impl ContextProvider for TfIdfProvider {
    fn name(&self) -> &str {
        "tfidf"
    }

    fn source_type(&self) -> ContextSource {
        ContextSource::Semantic
    }

    fn find_context(&self, intent: &TaskIntent, repo_path: &Path) -> Vec<ContextItem> {
        if intent.keywords.is_empty() && intent.symbols.is_empty() {
            return Vec::new();
        }

        let (corpus, _source_files) = Self::build_corpus(repo_path);
        let query_terms = Self::build_query_terms(intent);

        // tarpaulin-start-ignore
        if query_terms.is_empty() || corpus.is_empty() {
            return Vec::new();
        }
        // tarpaulin-stop-ignore

        let query_refs: Vec<&str> = query_terms.iter().map(|s| s.as_str()).collect();
        let ranked = corpus.rank_documents(&query_refs);

        // Normalize scores to 0.0-1.0 range
        let max_score = ranked.first().map(|(_, s)| *s).unwrap_or(1.0);
        // tarpaulin-start-ignore
        if max_score == 0.0 {
            return Vec::new();
        }
        // tarpaulin-stop-ignore

        ranked
            .into_iter()
            .map(|(doc_id, score)| {
                let normalized = (score / max_score).min(1.0);
                ContextItem {
                    source: ContextSource::Semantic,
                    file_path: std::path::PathBuf::from(&doc_id),
                    relevance_score: normalized,
                    reason: format!(
                        "TF-IDF score {:.3} for terms: {}",
                        score,
                        query_terms.join(", ")
                    ),
                }
            })
            .collect()
    }

    fn suggest_mcp_queries(&self, intent: &TaskIntent) -> Vec<McpQuery> {
        // Delegate to sourcegraph search for semantic queries
        let mut queries = Vec::new();
        let terms = Self::build_query_terms(intent);
        if !terms.is_empty() {
            let query = terms.join(" ");
            queries.push(McpQuery {
                server: "sourcegraph".into(),
                tool: "search".into(),
                params: serde_json::json!({ "query": query, "type": "file" }),
                reason: format!("TF-IDF semantic search for: {query}"),
            });
        }
        queries
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_repo() -> tempfile::TempDir {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("src");
        std::fs::create_dir_all(src.join("auth")).unwrap();
        std::fs::create_dir_all(src.join("db")).unwrap();
        std::fs::create_dir_all(src.join("api")).unwrap();

        std::fs::write(
            src.join("auth/service.rs"),
            "pub struct AuthService;\nimpl AuthService {\n    pub fn authenticate(&self, token: &str) -> bool { true }\n    pub fn check_permissions(&self) -> bool { true }\n}\n",
        ).unwrap();
        std::fs::write(
            src.join("auth/middleware.rs"),
            "use super::AuthService;\npub fn auth_middleware(req: Request) -> Response {\n    let auth = AuthService;\n    auth.authenticate(\"token\");\n}\n",
        ).unwrap();
        std::fs::write(
            src.join("db/connection.rs"),
            "pub struct Database;\nimpl Database {\n    pub fn connect(url: &str) -> Self { Self }\n    pub fn query(&self, sql: &str) -> Vec<Row> { vec![] }\n}\nstruct Row;\n",
        ).unwrap();
        std::fs::write(
            src.join("api/handler.rs"),
            "pub fn handle_request() {}\npub fn parse_json() {}\npub fn validate_input() {}\n",
        ).unwrap();
        std::fs::write(
            src.join("api/search.rs"),
            "pub fn search_documents(query: &str) -> Vec<String> { vec![] }\npub fn index_document(doc: &str) {}\n",
        ).unwrap();

        tmp
    }

    // ── Query term building ───────────────────────────────────────

    #[test]
    fn build_query_terms_from_keywords() {
        let intent = TaskIntent {
            file_paths: vec![],
            symbols: vec![],
            keywords: vec!["authenticate".into(), "login".into()],
        };
        let terms = TfIdfProvider::build_query_terms(&intent);
        assert!(terms.contains(&"authenticate".to_string()));
        assert!(terms.contains(&"login".to_string()));
    }

    #[test]
    fn build_query_terms_expands_camel_case() {
        let intent = TaskIntent {
            file_paths: vec![],
            symbols: vec!["AuthService".into()],
            keywords: vec![],
        };
        let terms = TfIdfProvider::build_query_terms(&intent);
        assert!(terms.contains(&"auth".to_string()));
        assert!(terms.contains(&"service".to_string()));
    }

    #[test]
    fn build_query_terms_deduplicates() {
        let intent = TaskIntent {
            file_paths: vec![],
            symbols: vec!["AuthService".into()],
            keywords: vec!["auth".into()],
        };
        let terms = TfIdfProvider::build_query_terms(&intent);
        assert_eq!(terms.iter().filter(|t| *t == "auth").count(), 1);
    }

    // ── Corpus building ───────────────────────────────────────────

    #[test]
    fn build_corpus_from_repo() {
        let repo = create_test_repo();
        let (corpus, files) = TfIdfProvider::build_corpus(repo.path());
        assert_eq!(corpus.len(), files.len());
        assert!(corpus.len() >= 5); // 5 source files
    }

    // ── Context finding ───────────────────────────────────────────

    #[test]
    fn finds_auth_files_for_auth_query() {
        let repo = create_test_repo();
        let provider = TfIdfProvider;
        let intent = TaskIntent {
            file_paths: vec![],
            symbols: vec![],
            keywords: vec!["authenticate".into()],
        };

        let items = provider.find_context(&intent, repo.path());
        assert!(!items.is_empty());
        // Auth files should rank highest
        assert!(items[0]
            .file_path
            .to_string_lossy()
            .contains("auth"));
    }

    #[test]
    fn finds_database_files_for_db_query() {
        let repo = create_test_repo();
        let provider = TfIdfProvider;
        let intent = TaskIntent {
            file_paths: vec![],
            symbols: vec![],
            keywords: vec!["database".into(), "connect".into()],
        };

        let items = provider.find_context(&intent, repo.path());
        assert!(!items.is_empty());
        assert!(items[0]
            .file_path
            .to_string_lossy()
            .contains("db"));
    }

    #[test]
    fn scores_normalized_to_one() {
        let repo = create_test_repo();
        let provider = TfIdfProvider;
        let intent = TaskIntent {
            file_paths: vec![],
            symbols: vec![],
            keywords: vec!["authenticate".into()],
        };

        let items = provider.find_context(&intent, repo.path());
        for item in &items {
            assert!(item.relevance_score <= 1.0);
            assert!(item.relevance_score > 0.0);
        }
        // Top result should be 1.0 (normalized)
        if !items.is_empty() {
            assert!((items[0].relevance_score - 1.0).abs() < f64::EPSILON);
        }
    }

    #[test]
    fn sorted_by_relevance_descending() {
        let repo = create_test_repo();
        let provider = TfIdfProvider;
        let intent = TaskIntent {
            file_paths: vec![],
            symbols: vec![],
            keywords: vec!["search".into()],
        };

        let items = provider.find_context(&intent, repo.path());
        for pair in items.windows(2) {
            assert!(pair[0].relevance_score >= pair[1].relevance_score);
        }
    }

    #[test]
    fn empty_intent_returns_empty() {
        let repo = create_test_repo();
        let provider = TfIdfProvider;
        let intent = TaskIntent::default();
        let items = provider.find_context(&intent, repo.path());
        assert!(items.is_empty());
    }

    #[test]
    fn source_type_is_semantic() {
        let provider = TfIdfProvider;
        assert_eq!(provider.source_type(), ContextSource::Semantic);
        assert_eq!(provider.name(), "tfidf");
    }

    #[test]
    fn symbol_query_finds_files() {
        let repo = create_test_repo();
        let provider = TfIdfProvider;
        let intent = TaskIntent {
            file_paths: vec![],
            symbols: vec!["AuthService".into()],
            keywords: vec![],
        };

        let items = provider.find_context(&intent, repo.path());
        assert!(!items.is_empty());
        // Files containing AuthService should rank high
        assert!(items
            .iter()
            .any(|i| i.file_path.to_string_lossy().contains("auth")));
    }

    #[test]
    fn suggests_sourcegraph_mcp_query() {
        let provider = TfIdfProvider;
        let intent = TaskIntent {
            file_paths: vec![],
            symbols: vec![],
            keywords: vec!["search".into()],
        };

        let queries = provider.suggest_mcp_queries(&intent);
        assert!(!queries.is_empty());
        assert!(queries
            .iter()
            .any(|q| q.server == "sourcegraph"));
    }

    #[test]
    fn reason_includes_query_terms() {
        let repo = create_test_repo();
        let provider = TfIdfProvider;
        let intent = TaskIntent {
            file_paths: vec![],
            symbols: vec![],
            keywords: vec!["authenticate".into()],
        };

        let items = provider.find_context(&intent, repo.path());
        if !items.is_empty() {
            assert!(items[0].reason.contains("TF-IDF"));
            assert!(items[0].reason.contains("authenticate"));
        }
    }
}
