//! TF-IDF scoring for semantic file matching.
//!
//! Provides term frequency–inverse document frequency scoring so the
//! context orchestrator can rank files by keyword relevance more
//! accurately than simple substring matching.

use std::collections::HashMap;

/// A corpus of documents for TF-IDF scoring.
#[derive(Debug, Clone)]
pub struct TfIdfCorpus {
    /// Map of document ID → term frequencies
    doc_terms: HashMap<String, HashMap<String, u32>>,
    /// Total number of documents
    doc_count: usize,
}

impl TfIdfCorpus {
    /// Create an empty corpus.
    pub fn new() -> Self {
        Self {
            doc_terms: HashMap::new(),
            doc_count: 0,
        }
    }

    /// Add a document to the corpus.
    ///
    /// `doc_id` is typically a file path. `content` is tokenized into
    /// terms for frequency counting.
    pub fn add_document(&mut self, doc_id: &str, content: &str) {
        let terms = tokenize(content);
        let mut freq: HashMap<String, u32> = HashMap::new();
        for term in terms {
            *freq.entry(term).or_insert(0) += 1;
        }
        self.doc_terms.insert(doc_id.to_string(), freq);
        self.doc_count = self.doc_terms.len();
    }

    /// Remove a document from the corpus.
    pub fn remove_document(&mut self, doc_id: &str) -> bool {
        let removed = self.doc_terms.remove(doc_id).is_some();
        self.doc_count = self.doc_terms.len();
        removed
    }

    /// Get the number of documents in the corpus.
    pub fn len(&self) -> usize {
        self.doc_count
    }

    /// Check if the corpus is empty.
    pub fn is_empty(&self) -> bool {
        self.doc_count == 0
    }

    /// Calculate the TF-IDF score of a term in a specific document.
    pub fn tf_idf(&self, term: &str, doc_id: &str) -> f64 {
        let tf = self.term_frequency(term, doc_id);
        let idf = self.inverse_document_frequency(term);
        tf * idf
    }

    /// Score a query (multiple terms) against a document.
    /// Returns the sum of TF-IDF scores for each query term.
    pub fn score_query(&self, query: &[&str], doc_id: &str) -> f64 {
        query.iter().map(|term| self.tf_idf(term, doc_id)).sum()
    }

    /// Rank all documents by relevance to a query.
    /// Returns (doc_id, score) pairs sorted by score descending.
    pub fn rank_documents(&self, query: &[&str]) -> Vec<(String, f64)> {
        let mut scores: Vec<(String, f64)> = self
            .doc_terms
            .keys()
            .map(|doc_id| {
                let score = self.score_query(query, doc_id);
                (doc_id.clone(), score)
            })
            .filter(|(_, score)| *score > 0.0)
            .collect();

        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scores
    }

    /// Term frequency: count of term in document / total terms in document.
    /// Uses augmented frequency to prevent bias towards longer documents.
    fn term_frequency(&self, term: &str, doc_id: &str) -> f64 {
        let terms = match self.doc_terms.get(doc_id) {
            Some(t) => t,
            None => return 0.0,
        };

        let raw_count = *terms.get(term).unwrap_or(&0) as f64;
        if raw_count == 0.0 {
            return 0.0;
        }

        let max_count = *terms.values().max().unwrap_or(&1) as f64;
        // Augmented TF: 0.5 + 0.5 * (count / max_count)
        0.5 + 0.5 * (raw_count / max_count)
    }

    /// Inverse document frequency: log(N / df) where df is the number
    /// of documents containing the term.
    fn inverse_document_frequency(&self, term: &str) -> f64 {
        if self.doc_count == 0 {
            return 0.0;
        }

        let doc_freq = self
            .doc_terms
            .values()
            .filter(|terms| terms.contains_key(term))
            .count();

        if doc_freq == 0 {
            return 0.0;
        }

        // Smoothed IDF: log(1 + N/df)
        (1.0 + self.doc_count as f64 / doc_freq as f64).ln()
    }
}

impl Default for TfIdfCorpus {
    fn default() -> Self {
        Self::new()
    }
}

/// Tokenize text into terms for indexing.
///
/// Splits on non-alphanumeric characters (except underscore),
/// converts to lowercase, expands CamelCase, and filters short tokens.
pub fn tokenize(text: &str) -> Vec<String> {
    let mut tokens = Vec::new();

    for word in text.split(|c: char| !c.is_alphanumeric() && c != '_') {
        if word.is_empty() {
            continue;
        }

        let lower = word.to_lowercase();
        if lower.len() >= 2 {
            tokens.push(lower.clone());
        }

        // Also expand CamelCase: "AuthService" → ["auth", "service"]
        let camel_parts = split_camel_case(word);
        if camel_parts.len() > 1 {
            for part in camel_parts {
                let p = part.to_lowercase();
                if p.len() >= 2 {
                    tokens.push(p);
                }
            }
        }

        // Also expand snake_case: "check_access" → ["check", "access"]
        if word.contains('_') {
            for part in word.split('_') {
                let p = part.to_lowercase();
                if p.len() >= 2 {
                    tokens.push(p);
                }
            }
        }
    }

    tokens
}

/// Split a CamelCase identifier into parts.
fn split_camel_case(s: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut start = 0;
    let bytes = s.as_bytes();

    for i in 1..bytes.len() {
        if bytes[i].is_ascii_uppercase() && bytes[i - 1].is_ascii_lowercase() {
            parts.push(&s[start..i]);
            start = i;
        }
    }

    if start < s.len() {
        parts.push(&s[start..]);
    }

    parts
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Tokenization ──────────────────────────────────────────────

    #[test]
    fn tokenize_basic_words() {
        let tokens = tokenize("hello world");
        assert!(tokens.contains(&"hello".to_string()));
        assert!(tokens.contains(&"world".to_string()));
    }

    #[test]
    fn tokenize_camel_case() {
        let tokens = tokenize("AuthService");
        assert!(tokens.contains(&"authservice".to_string()));
        assert!(tokens.contains(&"auth".to_string()));
        assert!(tokens.contains(&"service".to_string()));
    }

    #[test]
    fn tokenize_snake_case() {
        let tokens = tokenize("check_access");
        assert!(tokens.contains(&"check_access".to_string()));
        assert!(tokens.contains(&"check".to_string()));
        assert!(tokens.contains(&"access".to_string()));
    }

    #[test]
    fn tokenize_filters_short() {
        let tokens = tokenize("a b cd ef");
        assert!(!tokens.contains(&"a".to_string()));
        assert!(!tokens.contains(&"b".to_string()));
        assert!(tokens.contains(&"cd".to_string()));
    }

    #[test]
    fn tokenize_code_content() {
        let tokens = tokenize("pub fn calculate_cost(input: u32) -> f64 {");
        assert!(tokens.contains(&"calculate_cost".to_string()));
        assert!(tokens.contains(&"calculate".to_string()));
        assert!(tokens.contains(&"cost".to_string()));
        assert!(tokens.contains(&"input".to_string()));
    }

    // ── CamelCase splitting ───────────────────────────────────────

    #[test]
    fn split_camel_two_parts() {
        assert_eq!(split_camel_case("AuthService"), vec!["Auth", "Service"]);
    }

    #[test]
    fn split_camel_three_parts() {
        assert_eq!(
            split_camel_case("MyHttpClient"),
            vec!["My", "Http", "Client"]
        );
    }

    #[test]
    fn split_camel_single_word() {
        assert_eq!(split_camel_case("hello"), vec!["hello"]);
    }

    // ── TF-IDF corpus ─────────────────────────────────────────────

    #[test]
    fn empty_corpus() {
        let corpus = TfIdfCorpus::new();
        assert!(corpus.is_empty());
        assert_eq!(corpus.len(), 0);
    }

    #[test]
    fn add_and_count_documents() {
        let mut corpus = TfIdfCorpus::new();
        corpus.add_document("a.rs", "fn main() {}");
        corpus.add_document("b.rs", "pub struct Foo;");
        assert_eq!(corpus.len(), 2);
    }

    #[test]
    fn remove_document() {
        let mut corpus = TfIdfCorpus::new();
        corpus.add_document("a.rs", "fn main() {}");
        assert!(corpus.remove_document("a.rs"));
        assert!(corpus.is_empty());
        assert!(!corpus.remove_document("nonexistent"));
    }

    #[test]
    fn tf_idf_unique_term_scores_high() {
        let mut corpus = TfIdfCorpus::new();
        corpus.add_document("auth.rs", "AuthService authenticate login");
        corpus.add_document("db.rs", "Database connect query");
        corpus.add_document("api.rs", "handle request response");

        let auth_score = corpus.tf_idf("authenticate", "auth.rs");
        let auth_in_db = corpus.tf_idf("authenticate", "db.rs");

        assert!(auth_score > 0.0);
        assert_eq!(auth_in_db, 0.0);
    }

    #[test]
    fn tf_idf_common_term_scores_lower() {
        let mut corpus = TfIdfCorpus::new();
        // "fn" appears in all documents
        corpus.add_document("a.rs", "fn main fn helper");
        corpus.add_document("b.rs", "fn connect");
        corpus.add_document("c.rs", "fn handle");
        // "main" appears only in a.rs
        corpus.add_document("d.rs", "fn parse");

        let fn_score = corpus.tf_idf("fn", "a.rs");
        let main_score = corpus.tf_idf("main", "a.rs");

        // "main" is rarer so its IDF should be higher
        assert!(main_score > fn_score);
    }

    #[test]
    fn score_query_combines_terms() {
        let mut corpus = TfIdfCorpus::new();
        corpus.add_document("auth.rs", "authenticate login user session");
        corpus.add_document("db.rs", "database connect query pool");

        let auth_score = corpus.score_query(&["authenticate", "login"], "auth.rs");
        let db_score = corpus.score_query(&["authenticate", "login"], "db.rs");

        assert!(auth_score > db_score);
    }

    #[test]
    fn rank_documents_returns_sorted() {
        let mut corpus = TfIdfCorpus::new();
        corpus.add_document("auth.rs", "authenticate login user session");
        corpus.add_document("db.rs", "database connect query pool");
        corpus.add_document("api.rs", "authenticate handle request login");

        let ranked = corpus.rank_documents(&["authenticate", "login"]);
        assert!(!ranked.is_empty());

        // Scores should be descending
        for pair in ranked.windows(2) {
            assert!(pair[0].1 >= pair[1].1);
        }
    }

    #[test]
    fn rank_documents_filters_zero_scores() {
        let mut corpus = TfIdfCorpus::new();
        corpus.add_document("auth.rs", "authenticate login");
        corpus.add_document("db.rs", "database connect");

        let ranked = corpus.rank_documents(&["authenticate"]);
        // db.rs should not appear (score = 0)
        assert!(!ranked.iter().any(|(id, _)| id == "db.rs"));
    }

    #[test]
    fn tf_idf_missing_document_returns_zero() {
        let corpus = TfIdfCorpus::new();
        assert_eq!(corpus.tf_idf("term", "missing.rs"), 0.0);
    }

    #[test]
    fn tf_idf_missing_term_returns_zero() {
        let mut corpus = TfIdfCorpus::new();
        corpus.add_document("a.rs", "hello world");
        assert_eq!(corpus.tf_idf("missing", "a.rs"), 0.0);
    }

    #[test]
    fn add_document_twice_updates_no_duplicate() {
        let mut corpus = TfIdfCorpus::new();
        corpus.add_document("a.rs", "fn main() {}");
        corpus.add_document("a.rs", "pub fn new() -> Self {}");
        // Should still be 1 document, not 2
        assert_eq!(corpus.len(), 1);
        // The content should reflect the latest add
        let score = corpus.tf_idf("main", "a.rs");
        // "main" is not in new content, so score should be 0
        assert_eq!(score, 0.0);
    }

    #[test]
    fn rank_documents_empty_query_returns_empty() {
        let mut corpus = TfIdfCorpus::new();
        corpus.add_document("a.rs", "fn main() {}");
        corpus.add_document("b.rs", "pub struct Foo;");
        let ranked = corpus.rank_documents(&[]);
        // Empty query → all scores = 0.0 → filtered out
        assert!(ranked.is_empty());
    }

    #[test]
    fn corpus_default_is_empty() {
        let corpus = TfIdfCorpus::default();
        assert!(corpus.is_empty());
        assert_eq!(corpus.len(), 0);
    }

    #[test]
    fn corpus_clone_is_independent() {
        let mut corpus = TfIdfCorpus::new();
        corpus.add_document("a.rs", "authenticate login");
        let mut clone = corpus.clone();
        clone.add_document("b.rs", "database connect");
        // Original should not be affected
        assert_eq!(corpus.len(), 1);
        assert_eq!(clone.len(), 2);
    }

    #[test]
    fn tokenize_empty_string() {
        let tokens = tokenize("");
        assert!(tokens.is_empty());
    }

    #[test]
    fn tokenize_very_long_content() {
        let content = "authenticate ".repeat(500);
        let tokens = tokenize(&content);
        assert!(tokens.contains(&"authenticate".to_string()));
        // Should handle long content without issues
        assert!(!tokens.is_empty());
    }
}
