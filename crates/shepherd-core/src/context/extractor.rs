/// Extracted intent from a Kanban task's title and description.
#[derive(Debug, Clone, Default)]
pub struct TaskIntent {
    /// File paths mentioned in the task (e.g. "src/auth.rs", "lib/utils.ts").
    pub file_paths: Vec<String>,
    /// Symbol names found (CamelCase types, snake_case functions).
    pub symbols: Vec<String>,
    /// Significant keywords after stop-word removal.
    pub keywords: Vec<String>,
}

/// Stop words to filter out when extracting keywords.
const STOP_WORDS: &[&str] = &[
    "a", "an", "the", "is", "are", "was", "were", "be", "been", "being",
    "have", "has", "had", "do", "does", "did", "will", "would", "could",
    "should", "may", "might", "shall", "can", "need", "must",
    "in", "on", "at", "to", "for", "of", "with", "by", "from", "as",
    "into", "through", "during", "before", "after", "above", "below",
    "and", "but", "or", "nor", "not", "so", "yet", "both", "either",
    "it", "its", "this", "that", "these", "those", "he", "she", "they",
    "we", "you", "i", "me", "my", "your", "our", "their", "them",
    "what", "which", "who", "when", "where", "how", "why",
    "if", "then", "else", "also", "just", "only", "very", "too",
    "all", "each", "every", "any", "some", "no", "more", "most",
    "other", "than", "such", "like",
    "add", "fix", "update", "implement", "create", "make", "use",
    "get", "set", "new", "change", "remove", "delete",
];

/// Extract structured intent from a task's title and description.
pub fn extract_intent(title: &str, description: &str) -> TaskIntent {
    let combined = format!("{title} {description}");

    TaskIntent {
        file_paths: extract_file_paths(&combined),
        symbols: extract_symbols(&combined),
        keywords: extract_keywords(&combined),
    }
}

/// Find file path patterns in text.
///
/// Matches patterns like: src/foo.rs, lib/bar/baz.ts, ./config.toml
fn extract_file_paths(text: &str) -> Vec<String> {
    let mut paths = Vec::new();
    for word in text.split_whitespace() {
        let cleaned = word.trim_matches(|c: char| c == ',' || c == '.' || c == ':' || c == ';' || c == '`' || c == '\'' || c == '"' || c == '(' || c == ')');
        if is_file_path(cleaned) {
            paths.push(cleaned.to_string());
        }
    }
    paths.sort();
    paths.dedup();
    paths
}

/// Check if a string looks like a file path.
fn is_file_path(s: &str) -> bool {
    if s.len() < 3 {
        return false;
    }
    // Must contain a known file extension
    let has_extension = s.contains('.') && {
        let ext = s.rsplit('.').next().unwrap_or("");
        matches!(ext,
            "rs" | "ts" | "tsx" | "js" | "jsx" | "py" | "go" | "java" |
            "toml" | "yaml" | "yml" | "json" | "md" | "sql" | "html" |
            "css" | "scss" | "vue" | "svelte" | "rb" | "c" | "cpp" |
            "h" | "hpp" | "swift" | "kt" | "sh" | "bash" | "zsh"
        )
    };
    // Must have a recognized extension and look path-like (no spaces, valid chars)
    has_extension
        && !s.contains(' ')
        && s.chars().all(|c| c.is_alphanumeric() || c == '/' || c == '.' || c == '_' || c == '-')
}

/// Extract CamelCase type names and multi-word snake_case identifiers.
fn extract_symbols(text: &str) -> Vec<String> {
    let mut symbols = Vec::new();
    for word in text.split(|c: char| c.is_whitespace() || c == ',' || c == ';' || c == ':' || c == '(' || c == ')' || c == '`' || c == '.') {
        let cleaned = word.trim_matches(|c: char| !c.is_alphanumeric() && c != '_');
        if cleaned.is_empty() || cleaned.len() < 2 {
            continue;
        }
        // CamelCase: starts with uppercase, has at least one lowercase after
        let is_camel = cleaned.starts_with(|c: char| c.is_uppercase())
            && cleaned.chars().any(|c| c.is_lowercase())
            && cleaned.len() >= 3
            && cleaned.chars().filter(|c| c.is_uppercase()).count() >= 2;
        // snake_case with underscores: at least one underscore, all alphanumeric + underscore
        let is_snake = cleaned.contains('_')
            && cleaned.chars().all(|c| c.is_alphanumeric() || c == '_')
            && cleaned.len() >= 3;
        if is_camel || is_snake {
            symbols.push(cleaned.to_string());
        }
    }
    symbols.sort();
    symbols.dedup();
    symbols
}

/// Extract significant keywords after stop-word removal.
fn extract_keywords(text: &str) -> Vec<String> {
    let mut keywords = Vec::new();
    for word in text.split(|c: char| !c.is_alphanumeric() && c != '_' && c != '-') {
        let lower = word.to_lowercase();
        if lower.len() < 3 {
            continue;
        }
        if STOP_WORDS.contains(&lower.as_str()) {
            continue;
        }
        // Skip pure numbers
        if lower.chars().all(|c| c.is_ascii_digit()) {
            continue;
        }
        keywords.push(lower);
    }
    keywords.sort();
    keywords.dedup();
    keywords
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── File path extraction ─────────────────────────────────────

    #[test]
    fn extracts_rust_file_paths() {
        let intent = extract_intent("Fix bug in src/auth.rs", "The login flow in src/cloud/mod.rs is broken");
        assert!(intent.file_paths.contains(&"src/auth.rs".to_string()));
        assert!(intent.file_paths.contains(&"src/cloud/mod.rs".to_string()));
    }

    #[test]
    fn extracts_typescript_file_paths() {
        let intent = extract_intent("Update route", "Fix src/app/api/generate/search/route.ts");
        assert!(intent.file_paths.contains(&"src/app/api/generate/search/route.ts".to_string()));
    }

    #[test]
    fn extracts_bare_filenames() {
        let intent = extract_intent("Update config.toml", "Check Cargo.toml");
        assert!(intent.file_paths.contains(&"config.toml".to_string()));
        assert!(intent.file_paths.contains(&"Cargo.toml".to_string()));
    }

    #[test]
    fn ignores_non_file_words() {
        let intent = extract_intent("Fix the authentication bug", "Users cannot login");
        assert!(intent.file_paths.is_empty());
    }

    #[test]
    fn handles_backtick_wrapped_paths() {
        let intent = extract_intent("Fix `src/lib.rs`", "Check `mod.rs`");
        assert!(intent.file_paths.contains(&"src/lib.rs".to_string()));
        assert!(intent.file_paths.contains(&"mod.rs".to_string()));
    }

    #[test]
    fn deduplicates_file_paths() {
        let intent = extract_intent("Fix src/lib.rs", "Also check src/lib.rs again");
        assert_eq!(intent.file_paths.iter().filter(|p| *p == "src/lib.rs").count(), 1);
    }

    // ── Symbol extraction ────────────────────────────────────────

    #[test]
    fn extracts_camel_case_types() {
        let intent = extract_intent("Fix UserService", "The CloudClient is broken");
        assert!(intent.symbols.contains(&"UserService".to_string()));
        assert!(intent.symbols.contains(&"CloudClient".to_string()));
    }

    #[test]
    fn extracts_snake_case_functions() {
        let intent = extract_intent("Fix detect_for_agent", "update check_access");
        assert!(intent.symbols.contains(&"detect_for_agent".to_string()));
        assert!(intent.symbols.contains(&"check_access".to_string()));
    }

    #[test]
    fn ignores_plain_words() {
        let intent = extract_intent("Fix the login bug", "Users cannot authenticate");
        // "login", "bug", "Users" etc. should not be symbols
        assert!(!intent.symbols.iter().any(|s| s == "login"));
        assert!(!intent.symbols.iter().any(|s| s == "bug"));
    }

    #[test]
    fn extracts_symbols_from_backticks() {
        let intent = extract_intent("Fix `TaskIntent` extraction", "Check `extract_symbols`");
        assert!(intent.symbols.contains(&"TaskIntent".to_string()));
        assert!(intent.symbols.contains(&"extract_symbols".to_string()));
    }

    // ── Keyword extraction ───────────────────────────────────────

    #[test]
    fn extracts_significant_keywords() {
        let intent = extract_intent("Fix authentication bug", "The login flow is broken for OAuth users");
        assert!(intent.keywords.contains(&"authentication".to_string()));
        assert!(intent.keywords.contains(&"login".to_string()));
        assert!(intent.keywords.contains(&"broken".to_string()));
        assert!(intent.keywords.contains(&"oauth".to_string()));
    }

    #[test]
    fn removes_stop_words() {
        let intent = extract_intent("Fix the bug", "It is broken and should be fixed");
        assert!(!intent.keywords.contains(&"the".to_string()));
        assert!(!intent.keywords.contains(&"is".to_string()));
        assert!(!intent.keywords.contains(&"and".to_string()));
        assert!(!intent.keywords.contains(&"should".to_string()));
    }

    #[test]
    fn ignores_short_words() {
        let intent = extract_intent("A is ok", "Do it to me");
        // All words are <=2 chars or stop words
        assert!(intent.keywords.is_empty());
    }

    #[test]
    fn deduplicates_keywords() {
        let intent = extract_intent("auth auth auth", "auth");
        assert_eq!(intent.keywords.iter().filter(|k| *k == "auth").count(), 1);
    }

    // ── Combined extraction ──────────────────────────────────────

    #[test]
    fn full_extraction_from_realistic_task() {
        let intent = extract_intent(
            "Add search endpoint",
            "Create a new API route at src/app/api/search/route.ts that calls the ExaClient.searchWeb method. Should validate the query parameter and return results.",
        );
        assert!(intent.file_paths.contains(&"src/app/api/search/route.ts".to_string()));
        assert!(intent.symbols.contains(&"ExaClient".to_string()));
        assert!(intent.keywords.contains(&"search".to_string()));
        assert!(intent.keywords.contains(&"endpoint".to_string()));
        assert!(intent.keywords.contains(&"validate".to_string()));
    }

    #[test]
    fn empty_input_produces_empty_intent() {
        let intent = extract_intent("", "");
        assert!(intent.file_paths.is_empty());
        assert!(intent.symbols.is_empty());
        assert!(intent.keywords.is_empty());
    }
}
