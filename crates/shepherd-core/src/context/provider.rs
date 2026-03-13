use std::path::Path;
use super::extractor::TaskIntent;
use super::package::{ContextItem, ContextSource, McpQuery};

/// A source of context for agent sessions.
pub trait ContextProvider {
    fn name(&self) -> &str;
    fn source_type(&self) -> ContextSource;
    fn find_context(&self, intent: &TaskIntent, repo_path: &Path) -> Vec<ContextItem>;
    fn suggest_mcp_queries(&self, intent: &TaskIntent) -> Vec<McpQuery>;
}

// ── Structural Provider ──────────────────────────────────────────────

/// Finds context via file system structure: direct file references,
/// import/dependency analysis, and module hierarchy.
pub struct StructuralProvider;

impl StructuralProvider {
    /// Walk directory and collect source files up to a depth limit.
    pub(crate) fn collect_source_files(repo_path: &Path, max_depth: usize) -> Vec<std::path::PathBuf> {
        let mut files = Vec::new();
        Self::walk_dir(repo_path, repo_path, max_depth, 0, &mut files);
        files
    }

    fn walk_dir(
        base: &Path,
        dir: &Path,
        max_depth: usize,
        depth: usize,
        files: &mut Vec<std::path::PathBuf>,
    ) {
        if depth > max_depth {
            return;
        }
        let entries = match std::fs::read_dir(dir) {
            Ok(e) => e,
            Err(_) => return,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            // Skip hidden dirs and common non-source dirs
            if name.starts_with('.')
                || name == "node_modules"
                || name == "target"
                || name == "__pycache__"
                || name == "dist"
                || name == "build"
                || name == "vendor"
            {
                continue;
            }
            if path.is_dir() {
                Self::walk_dir(base, &path, max_depth, depth + 1, files);
            } else if Self::is_source_file(&name) {
                if let Ok(rel) = path.strip_prefix(base) {
                    files.push(rel.to_path_buf());
                }
            }
        }
    }

    fn is_source_file(name: &str) -> bool {
        let ext = name.rsplit('.').next().unwrap_or("");
        matches!(
            ext,
            "rs" | "ts" | "tsx" | "js" | "jsx" | "py" | "go" | "java"
                | "toml" | "yaml" | "yml" | "json" | "sql" | "html"
                | "css" | "vue" | "svelte" | "rb" | "c" | "cpp"
                | "h" | "swift" | "kt" | "sh"
        )
    }

    /// Parse simple import/use statements from a file to find dependencies.
    fn extract_imports(file_content: &str) -> Vec<String> {
        let mut imports = Vec::new();
        for line in file_content.lines() {
            let trimmed = line.trim();
            // Rust: use crate::foo::bar;  or  mod foo;
            if let Some(rest) = trimmed.strip_prefix("use ") {
                if let Some(path) = rest.strip_prefix("crate::") {
                    let module_path = path.trim_end_matches(';')
                        .split("::")
                        .next()
                        .unwrap_or("");
                    if !module_path.is_empty() {
                        imports.push(module_path.to_string());
                    }
                }
            } else if let Some(rest) = trimmed.strip_prefix("mod ") {
                let module_name = rest.trim_end_matches(';').trim();
                if !module_name.is_empty() && !module_name.starts_with('{') {
                    imports.push(module_name.to_string());
                }
            }
            // TypeScript/JavaScript: import ... from '...'
            else if trimmed.starts_with("import ") {
                if let Some(from_idx) = trimmed.find("from ") {
                    let after_from = &trimmed[from_idx + 5..];
                    let path = after_from
                        .trim_matches(|c: char| c == '\'' || c == '"' || c == ';' || c == ' ');
                    if !path.is_empty() {
                        imports.push(path.to_string());
                    }
                }
            }
            // Python: from foo import bar  or  import foo
            else if let Some(rest) = trimmed.strip_prefix("from ") {
                let module = rest.split_whitespace().next().unwrap_or("");
                if !module.is_empty() {
                    imports.push(module.to_string());
                }
            }
        }
        imports
    }
}

impl ContextProvider for StructuralProvider {
    fn name(&self) -> &str {
        "structural"
    }

    fn source_type(&self) -> ContextSource {
        ContextSource::Structural
    }

    fn find_context(&self, intent: &TaskIntent, repo_path: &Path) -> Vec<ContextItem> {
        let mut items = Vec::new();
        let source_files = Self::collect_source_files(repo_path, 8);

        // 1. Find directly referenced files
        let mut referenced_files = Vec::new();
        for mentioned_path in &intent.file_paths {
            for source_file in &source_files {
                let source_str = source_file.to_string_lossy();
                if source_str.ends_with(mentioned_path.as_str())
                    || source_str == mentioned_path.as_str()
                {
                    referenced_files.push(source_file.clone());
                    items.push(ContextItem {
                        source: ContextSource::FileReference,
                        file_path: source_file.clone(),
                        relevance_score: 1.0,
                        reason: format!("Directly mentioned in task: {mentioned_path}"),
                    });
                }
            }
        }

        // 2. Analyze imports of referenced files to find dependencies
        for ref_file in &referenced_files {
            let full_path = repo_path.join(ref_file);
            let content = match std::fs::read_to_string(&full_path) {
                Ok(c) => c,
                Err(_) => continue,
            };
            let imports = Self::extract_imports(&content);
            for import in &imports {
                for source_file in &source_files {
                    let source_str = source_file.to_string_lossy();
                    // Check if the import matches any source file
                    if source_str.contains(import.as_str())
                        && !items.iter().any(|i| i.file_path == *source_file)
                    {
                        items.push(ContextItem {
                            source: ContextSource::Structural,
                            file_path: source_file.clone(),
                            relevance_score: 0.7,
                            reason: format!(
                                "Imported by {}",
                                ref_file.to_string_lossy()
                            ),
                        });
                    }
                }
            }
        }

        // 3. Find files containing referenced symbols
        for symbol in &intent.symbols {
            for source_file in &source_files {
                if items.iter().any(|i| i.file_path == *source_file) {
                    continue;
                }
                let full_path = repo_path.join(source_file);
                // Only read first 200 lines for performance
                let content = match std::fs::read_to_string(&full_path) {
                    Ok(c) => c,
                    Err(_) => continue,
                };
                let preview: String = content.lines().take(200).collect::<Vec<_>>().join("\n");
                if preview.contains(symbol.as_str()) {
                    items.push(ContextItem {
                        source: ContextSource::Structural,
                        file_path: source_file.clone(),
                        relevance_score: 0.6,
                        reason: format!("Contains symbol: {symbol}"),
                    });
                }
            }
        }

        items
    }

    fn suggest_mcp_queries(&self, intent: &TaskIntent) -> Vec<McpQuery> {
        let mut queries = Vec::new();
        for symbol in &intent.symbols {
            queries.push(McpQuery {
                server: "serena".into(),
                tool: "find_symbol".into(),
                params: serde_json::json!({
                    "name_path_pattern": symbol,
                    "include_body": false,
                }),
                reason: format!("Find definition and usages of {symbol}"),
            });
            queries.push(McpQuery {
                server: "serena".into(),
                tool: "find_referencing_symbols".into(),
                params: serde_json::json!({
                    "name_path_pattern": symbol,
                }),
                reason: format!("Find all code that references {symbol}"),
            });
        }
        queries
    }
}

// ── Semantic Provider ────────────────────────────────────────────────

/// Finds context via keyword and content matching — semantic similarity
/// without vector embeddings (local heuristic approach).
pub struct SemanticProvider;

impl SemanticProvider {
    /// Score a file's relevance based on keyword matches in its path and content.
    fn score_file(
        file_path: &Path,
        repo_path: &Path,
        keywords: &[String],
    ) -> Option<(f64, String)> {
        let path_str = file_path.to_string_lossy().to_lowercase();
        let mut score = 0.0;
        let mut reasons = Vec::new();

        // Score based on file path matching keywords
        for keyword in keywords {
            if path_str.contains(keyword.as_str()) {
                score += 0.4;
                reasons.push(format!("path contains '{keyword}'"));
            }
        }

        // Score based on file content matching keywords
        let full_path = repo_path.join(file_path);
        if let Ok(content) = std::fs::read_to_string(&full_path) {
            let preview: String = content.lines().take(100).collect::<Vec<_>>().join("\n");
            let lower_preview = preview.to_lowercase();
            let mut content_matches = 0;
            for keyword in keywords {
                let count = lower_preview.matches(keyword.as_str()).count();
                if count > 0 {
                    content_matches += count;
                }
            }
            if content_matches > 0 {
                // Logarithmic scaling: diminishing returns for many matches
                let content_score = (content_matches as f64).ln_1p() * 0.15;
                score += content_score.min(0.5);
                reasons.push(format!("{content_matches} keyword matches in content"));
            }
        }

        if score > 0.0 {
            Some((score.min(1.0), reasons.join("; ")))
        } else {
            None
        }
    }
}

impl ContextProvider for SemanticProvider {
    fn name(&self) -> &str {
        "semantic"
    }

    fn source_type(&self) -> ContextSource {
        ContextSource::Semantic
    }

    fn find_context(&self, intent: &TaskIntent, repo_path: &Path) -> Vec<ContextItem> {
        if intent.keywords.is_empty() {
            return Vec::new();
        }

        let source_files = StructuralProvider::collect_source_files(repo_path, 8);
        let mut scored_items: Vec<ContextItem> = Vec::new();

        for source_file in &source_files {
            if let Some((score, reason)) =
                Self::score_file(source_file, repo_path, &intent.keywords)
            {
                scored_items.push(ContextItem {
                    source: ContextSource::Semantic,
                    file_path: source_file.clone(),
                    relevance_score: score,
                    reason,
                });
            }
        }

        // Sort by score descending
        scored_items.sort_by(|a, b| b.relevance_score.partial_cmp(&a.relevance_score).unwrap());
        scored_items
    }

    fn suggest_mcp_queries(&self, intent: &TaskIntent) -> Vec<McpQuery> {
        let mut queries = Vec::new();
        if !intent.keywords.is_empty() {
            let query = intent.keywords.join(" ");
            queries.push(McpQuery {
                server: "sourcegraph".into(),
                tool: "search".into(),
                params: serde_json::json!({
                    "query": query,
                }),
                reason: format!("Semantic search for: {query}"),
            });
        }
        for keyword in intent.keywords.iter().take(3) {
            queries.push(McpQuery {
                server: "sourcegraph".into(),
                tool: "search".into(),
                params: serde_json::json!({
                    "query": keyword,
                    "type": "file",
                }),
                reason: format!("Find files related to '{keyword}'"),
            });
        }
        queries
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn create_test_repo() -> tempfile::TempDir {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::create_dir_all(src.join("auth")).unwrap();
        std::fs::create_dir_all(src.join("db")).unwrap();
        std::fs::create_dir_all(src.join("api")).unwrap();

        std::fs::write(
            src.join("main.rs"),
            "use crate::auth;\nuse crate::db;\n\nfn main() {\n    let service = AuthService::new();\n}\n",
        ).unwrap();

        std::fs::write(
            src.join("auth/mod.rs"),
            "use crate::db;\n\npub struct AuthService;\n\nimpl AuthService {\n    pub fn new() -> Self { Self }\n    pub fn check_access(&self) -> bool { true }\n}\n",
        ).unwrap();

        std::fs::write(
            src.join("db/mod.rs"),
            "pub struct Database;\n\nimpl Database {\n    pub fn connect() -> Self { Self }\n}\n",
        ).unwrap();

        std::fs::write(
            src.join("api/routes.ts"),
            "import { AuthService } from '../auth';\nimport { search } from '../search';\n\nexport function handleLogin() {}\nexport function handleSearch() {}\n",
        ).unwrap();

        std::fs::write(
            src.join("api/search.ts"),
            "export function searchWeb(query: string) {\n  // semantic search implementation\n  return fetch('/api/search', { body: query });\n}\n",
        ).unwrap();

        tmp
    }

    // ── StructuralProvider tests ──────────────────────────────────

    #[test]
    fn structural_finds_directly_referenced_files() {
        let repo = create_test_repo();
        let provider = StructuralProvider;
        let intent = TaskIntent {
            file_paths: vec!["src/auth/mod.rs".into()],
            symbols: vec![],
            keywords: vec![],
        };

        let items = provider.find_context(&intent, repo.path());
        assert!(items.iter().any(|i|
            i.file_path == PathBuf::from("src/auth/mod.rs")
            && i.source == ContextSource::FileReference
            && i.relevance_score == 1.0
        ));
    }

    #[test]
    fn structural_finds_imports_of_referenced_files() {
        let repo = create_test_repo();
        let provider = StructuralProvider;
        let intent = TaskIntent {
            file_paths: vec!["src/auth/mod.rs".into()],
            symbols: vec![],
            keywords: vec![],
        };

        let items = provider.find_context(&intent, repo.path());
        // auth/mod.rs imports crate::db, so db/mod.rs should appear
        assert!(items.iter().any(|i|
            i.file_path == PathBuf::from("src/db/mod.rs")
            && i.source == ContextSource::Structural
        ));
    }

    #[test]
    fn structural_finds_files_containing_symbols() {
        let repo = create_test_repo();
        let provider = StructuralProvider;
        let intent = TaskIntent {
            file_paths: vec![],
            symbols: vec!["AuthService".into()],
            keywords: vec![],
        };

        let items = provider.find_context(&intent, repo.path());
        assert!(items.iter().any(|i|
            i.file_path == PathBuf::from("src/auth/mod.rs")
            && i.reason.contains("AuthService")
        ));
    }

    #[test]
    fn structural_generates_serena_mcp_queries() {
        let provider = StructuralProvider;
        let intent = TaskIntent {
            file_paths: vec![],
            symbols: vec!["CloudClient".into()],
            keywords: vec![],
        };

        let queries = provider.suggest_mcp_queries(&intent);
        assert!(queries.iter().any(|q|
            q.server == "serena" && q.tool == "find_symbol"
        ));
        assert!(queries.iter().any(|q|
            q.server == "serena" && q.tool == "find_referencing_symbols"
        ));
    }

    #[test]
    fn structural_empty_intent_returns_empty() {
        let repo = create_test_repo();
        let provider = StructuralProvider;
        let intent = TaskIntent::default();
        let items = provider.find_context(&intent, repo.path());
        assert!(items.is_empty());
    }

    #[test]
    fn structural_skips_hidden_directories() {
        let repo = create_test_repo();
        let hidden = repo.path().join(".git");
        std::fs::create_dir_all(&hidden).unwrap();
        std::fs::write(hidden.join("config.rs"), "should be ignored").unwrap();

        let files = StructuralProvider::collect_source_files(repo.path(), 8);
        assert!(!files.iter().any(|f| f.to_string_lossy().contains(".git")));
    }

    #[test]
    fn structural_skips_node_modules() {
        let repo = create_test_repo();
        let nm = repo.path().join("node_modules");
        std::fs::create_dir_all(&nm).unwrap();
        std::fs::write(nm.join("package.json"), "{}").unwrap();

        let files = StructuralProvider::collect_source_files(repo.path(), 8);
        assert!(!files.iter().any(|f| f.to_string_lossy().contains("node_modules")));
    }

    // ── Import parsing tests ─────────────────────────────────────

    #[test]
    fn parses_rust_imports() {
        let content = "use crate::auth;\nuse crate::db::models;\nmod config;\n";
        let imports = StructuralProvider::extract_imports(content);
        assert!(imports.contains(&"auth".to_string()));
        assert!(imports.contains(&"db".to_string()));
        assert!(imports.contains(&"config".to_string()));
    }

    #[test]
    fn parses_typescript_imports() {
        let content = "import { foo } from './auth';\nimport bar from '../db/models';\n";
        let imports = StructuralProvider::extract_imports(content);
        assert!(imports.contains(&"./auth".to_string()));
        assert!(imports.contains(&"../db/models".to_string()));
    }

    #[test]
    fn parses_python_imports() {
        let content = "from auth import check\nimport db.models\n";
        let imports = StructuralProvider::extract_imports(content);
        assert!(imports.contains(&"auth".to_string()));
    }

    // ── SemanticProvider tests ────────────────────────────────────

    #[test]
    fn semantic_finds_files_by_keyword_in_path() {
        let repo = create_test_repo();
        let provider = SemanticProvider;
        let intent = TaskIntent {
            file_paths: vec![],
            symbols: vec![],
            keywords: vec!["auth".into()],
        };

        let items = provider.find_context(&intent, repo.path());
        assert!(items.iter().any(|i|
            i.file_path.to_string_lossy().contains("auth")
            && i.source == ContextSource::Semantic
        ));
    }

    #[test]
    fn semantic_finds_files_by_keyword_in_content() {
        let repo = create_test_repo();
        let provider = SemanticProvider;
        let intent = TaskIntent {
            file_paths: vec![],
            symbols: vec![],
            keywords: vec!["search".into()],
        };

        let items = provider.find_context(&intent, repo.path());
        // api/search.ts and api/routes.ts both mention "search"
        assert!(items.iter().any(|i|
            i.file_path.to_string_lossy().contains("search")
        ));
    }

    #[test]
    fn semantic_scores_sorted_by_relevance() {
        let repo = create_test_repo();
        let provider = SemanticProvider;
        let intent = TaskIntent {
            file_paths: vec![],
            symbols: vec![],
            keywords: vec!["search".into()],
        };

        let items = provider.find_context(&intent, repo.path());
        if items.len() >= 2 {
            assert!(items[0].relevance_score >= items[1].relevance_score);
        }
    }

    #[test]
    fn semantic_empty_keywords_returns_empty() {
        let repo = create_test_repo();
        let provider = SemanticProvider;
        let intent = TaskIntent {
            file_paths: vec![],
            symbols: vec![],
            keywords: vec![],
        };
        let items = provider.find_context(&intent, repo.path());
        assert!(items.is_empty());
    }

    #[test]
    fn semantic_generates_sourcegraph_queries() {
        let provider = SemanticProvider;
        let intent = TaskIntent {
            file_paths: vec![],
            symbols: vec![],
            keywords: vec!["authentication".into(), "login".into()],
        };

        let queries = provider.suggest_mcp_queries(&intent);
        assert!(queries.iter().any(|q|
            q.server == "sourcegraph" && q.tool == "search"
        ));
    }

    #[test]
    fn semantic_score_capped_at_one() {
        let repo = create_test_repo();
        let provider = SemanticProvider;
        // Use many keywords that all match the same file
        let intent = TaskIntent {
            file_paths: vec![],
            symbols: vec![],
            keywords: vec![
                "search".into(), "web".into(), "query".into(),
                "api".into(), "fetch".into(),
            ],
        };

        let items = provider.find_context(&intent, repo.path());
        for item in &items {
            assert!(item.relevance_score <= 1.0, "Score {} exceeds 1.0", item.relevance_score);
        }
    }
}
