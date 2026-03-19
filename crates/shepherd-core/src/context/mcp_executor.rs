//! MCP query executor — actually runs McpQuery suggestions against
//! available MCP servers and merges results into context packages.
//!
//! Uses JSON-RPC 2.0 protocol (the MCP standard). Designed with a
//! trait-based transport layer so it can be tested without spawning
//! real MCP server processes.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

use super::package::{ContextItem, ContextPackage, ContextSource, McpQuery};

/// A JSON-RPC 2.0 request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: u64,
    pub method: String,
    pub params: serde_json::Value,
}

/// A JSON-RPC 2.0 response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: u64,
    #[serde(default)]
    pub result: Option<serde_json::Value>,
    #[serde(default)]
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i64,
    pub message: String,
}

/// Result of executing a single MCP query.
#[derive(Debug, Clone)]
pub struct McpResult {
    pub query: McpQuery,
    pub success: bool,
    /// Files discovered by the MCP server.
    pub discovered_files: Vec<ContextItem>,
    /// Raw response for debugging.
    pub raw_response: Option<serde_json::Value>,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Trait for MCP communication transport.
/// Allows mocking in tests.
pub trait McpTransport: Send + Sync {
    fn call(&self, server: &str, request: &JsonRpcRequest) -> Result<JsonRpcResponse>;
    fn is_available(&self, server: &str) -> bool;
}

/// Configuration for known MCP servers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    pub server_name: String,
    pub command: String,
    pub args: Vec<String>,
    pub env: HashMap<String, String>,
}

/// Default MCP server configurations.
pub fn default_server_configs() -> Vec<McpServerConfig> {
    vec![
        McpServerConfig {
            server_name: "serena".into(),
            command: "uvx".into(),
            args: vec![
                "--from".into(),
                "git+https://github.com/oraios/serena".into(),
                "serena".into(),
                "start-mcp-server".into(),
            ],
            env: HashMap::new(),
        },
        McpServerConfig {
            server_name: "sourcegraph".into(),
            command: "npx".into(),
            args: vec!["-y".into(), "sourcegraph-mcp-server".into()],
            env: HashMap::from([
                ("SRC_ENDPOINT".into(), "".into()),
                ("SRC_ACCESS_TOKEN".into(), "".into()),
            ]),
        },
    ]
}

/// Execute MCP queries and merge results into a context package.
pub fn execute_queries(transport: &dyn McpTransport, package: &ContextPackage) -> Vec<McpResult> {
    let mut results = Vec::new();
    let mut request_id = 1u64;

    for query in &package.mcp_queries {
        if !transport.is_available(&query.server) {
            results.push(McpResult {
                query: query.clone(),
                success: false,
                discovered_files: vec![],
                raw_response: None,
                error: Some(format!("MCP server '{}' not available", query.server)),
            });
            continue;
        }

        let request = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: request_id,
            method: "tools/call".to_string(),
            params: serde_json::json!({
                "name": query.tool,
                "arguments": query.params,
            }),
        };
        request_id += 1;

        match transport.call(&query.server, &request) {
            Ok(response) => {
                let discovered = extract_files_from_response(&response);
                results.push(McpResult {
                    query: query.clone(),
                    success: response.error.is_none(),
                    discovered_files: discovered,
                    raw_response: response.result,
                    error: response.error.map(|e| e.message),
                });
            }
            Err(e) => {
                results.push(McpResult {
                    query: query.clone(),
                    success: false,
                    discovered_files: vec![],
                    raw_response: None,
                    error: Some(e.to_string()),
                });
            }
        }
    }

    results
}

/// Merge MCP results into an existing context package.
pub fn merge_results(package: &mut ContextPackage, results: &[McpResult]) {
    for result in results {
        if !result.success {
            continue;
        }
        for item in &result.discovered_files {
            // Don't add duplicates
            if !package
                .items
                .iter()
                .any(|existing| existing.file_path == item.file_path)
            {
                package.items.push(item.clone());
            }
        }
    }
}

/// Extract file references from an MCP response.
///
/// MCP tool results may contain file paths in various formats.
/// This tries common patterns in the response payload.
fn extract_files_from_response(response: &JsonRpcResponse) -> Vec<ContextItem> {
    let mut items = Vec::new();

    let result = match &response.result {
        Some(r) => r,
        None => return items,
    };

    // Try to extract content array (standard MCP tool result format)
    if let Some(content) = result.get("content") {
        if let Some(arr) = content.as_array() {
            for entry in arr {
                if let Some(text) = entry.get("text").and_then(|t| t.as_str()) {
                    // Look for file paths in the text
                    for path in extract_paths_from_text(text) {
                        items.push(ContextItem {
                            source: ContextSource::Structural,
                            file_path: PathBuf::from(&path),
                            relevance_score: 0.8,
                            reason: "Discovered via MCP query".into(),
                        });
                    }
                }
            }
        }
    }

    // Try to extract from a "symbols" array (Serena-style)
    if let Some(symbols) = result.get("symbols") {
        if let Some(arr) = symbols.as_array() {
            for sym in arr {
                if let Some(path) = sym.get("relative_path").and_then(|p| p.as_str()) {
                    items.push(ContextItem {
                        source: ContextSource::Structural,
                        file_path: PathBuf::from(path),
                        relevance_score: 0.85,
                        reason: format!(
                            "Symbol found via MCP: {}",
                            sym.get("name")
                                .and_then(|n| n.as_str())
                                .unwrap_or("unknown")
                        ),
                    });
                }
            }
        }
    }

    // Try to extract from a "results" array (Sourcegraph-style)
    if let Some(search_results) = result.get("results") {
        if let Some(arr) = search_results.as_array() {
            for res in arr {
                if let Some(path) = res.get("path").and_then(|p| p.as_str()) {
                    items.push(ContextItem {
                        source: ContextSource::Semantic,
                        file_path: PathBuf::from(path),
                        relevance_score: 0.75,
                        reason: "Found via Sourcegraph search".into(),
                    });
                }
            }
        }
    }

    items
}

/// Extract file-like paths from text content.
fn extract_paths_from_text(text: &str) -> Vec<String> {
    let mut paths = Vec::new();
    for word in text.split_whitespace() {
        let cleaned = word.trim_matches(|c: char| c == '`' || c == '\'' || c == '"' || c == ',');
        if looks_like_file_path(cleaned) {
            paths.push(cleaned.to_string());
        }
    }
    paths
}

fn looks_like_file_path(s: &str) -> bool {
    if s.len() < 4 {
        return false;
    }
    let has_extension = s.contains('.') && {
        let ext = s.rsplit('.').next().unwrap_or("");
        matches!(
            ext,
            "rs" | "ts"
                | "tsx"
                | "js"
                | "jsx"
                | "py"
                | "go"
                | "java"
                | "toml"
                | "yaml"
                | "yml"
                | "json"
                | "sql"
                | "html"
                | "css"
                | "rb"
                | "c"
                | "cpp"
                | "h"
                | "swift"
                | "kt"
                | "sh"
        )
    };
    has_extension && !s.contains(' ')
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Mock transport that returns canned responses.
    struct MockTransport {
        responses: HashMap<String, JsonRpcResponse>,
        available_servers: Vec<String>,
    }

    impl MockTransport {
        fn new() -> Self {
            Self {
                responses: HashMap::new(),
                available_servers: vec![],
            }
        }

        fn add_server(&mut self, name: &str) {
            self.available_servers.push(name.to_string());
        }

        fn add_response(&mut self, server: &str, response: JsonRpcResponse) {
            self.responses.insert(server.to_string(), response);
        }
    }

    impl McpTransport for MockTransport {
        fn call(&self, server: &str, _request: &JsonRpcRequest) -> Result<JsonRpcResponse> {
            self.responses
                .get(server)
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("No mock response for {server}"))
        }

        fn is_available(&self, server: &str) -> bool {
            self.available_servers.contains(&server.to_string())
        }
    }

    fn sample_package() -> ContextPackage {
        ContextPackage {
            id: "ctx-test".into(),
            task_id: Some(1),
            items: vec![],
            mcp_queries: vec![
                McpQuery {
                    server: "serena".into(),
                    tool: "find_symbol".into(),
                    params: serde_json::json!({"name_path_pattern": "AuthService"}),
                    reason: "Find AuthService".into(),
                },
                McpQuery {
                    server: "sourcegraph".into(),
                    tool: "search".into(),
                    params: serde_json::json!({"query": "authenticate"}),
                    reason: "Search for authenticate".into(),
                },
            ],
            summary: "Test package".into(),
            created_at: "2026-03-13T00:00:00Z".into(),
        }
    }

    // ── JSON-RPC types ────────────────────────────────────────────

    #[test]
    fn json_rpc_request_serialization() {
        let req = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: 1,
            method: "tools/call".into(),
            params: serde_json::json!({"name": "test"}),
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("2.0"));
        assert!(json.contains("tools/call"));
    }

    #[test]
    fn json_rpc_response_with_result() {
        let resp = JsonRpcResponse {
            jsonrpc: "2.0".into(),
            id: 1,
            result: Some(serde_json::json!({"content": []})),
            error: None,
        };
        let json = serde_json::to_string(&resp).unwrap();
        let parsed: JsonRpcResponse = serde_json::from_str(&json).unwrap();
        assert!(parsed.result.is_some());
        assert!(parsed.error.is_none());
    }

    #[test]
    fn json_rpc_response_with_error() {
        let resp = JsonRpcResponse {
            jsonrpc: "2.0".into(),
            id: 1,
            result: None,
            error: Some(JsonRpcError {
                code: -32600,
                message: "Invalid request".into(),
            }),
        };
        assert!(resp.error.is_some());
    }

    // ── Execute queries ───────────────────────────────────────────

    #[test]
    fn execute_with_unavailable_server() {
        let transport = MockTransport::new(); // No servers available
        let pkg = sample_package();
        let results = execute_queries(&transport, &pkg);
        assert_eq!(results.len(), 2);
        assert!(!results[0].success);
        assert!(results[0].error.as_ref().unwrap().contains("not available"));
    }

    #[test]
    fn execute_with_serena_response() {
        let mut transport = MockTransport::new();
        transport.add_server("serena");
        transport.add_response(
            "serena",
            JsonRpcResponse {
                jsonrpc: "2.0".into(),
                id: 1,
                result: Some(serde_json::json!({
                    "symbols": [
                        {
                            "name": "AuthService",
                            "relative_path": "src/auth/service.rs",
                            "kind": "struct"
                        }
                    ]
                })),
                error: None,
            },
        );

        let pkg = ContextPackage {
            mcp_queries: vec![McpQuery {
                server: "serena".into(),
                tool: "find_symbol".into(),
                params: serde_json::json!({}),
                reason: "test".into(),
            }],
            ..sample_package()
        };

        let results = execute_queries(&transport, &pkg);
        assert_eq!(results.len(), 1);
        assert!(results[0].success);
        assert!(!results[0].discovered_files.is_empty());
        assert_eq!(
            results[0].discovered_files[0].file_path,
            PathBuf::from("src/auth/service.rs")
        );
    }

    #[test]
    fn execute_with_sourcegraph_response() {
        let mut transport = MockTransport::new();
        transport.add_server("sourcegraph");
        transport.add_response(
            "sourcegraph",
            JsonRpcResponse {
                jsonrpc: "2.0".into(),
                id: 1,
                result: Some(serde_json::json!({
                    "results": [
                        {"path": "src/auth/login.rs", "line": 42},
                        {"path": "src/api/handler.rs", "line": 10}
                    ]
                })),
                error: None,
            },
        );

        let pkg = ContextPackage {
            mcp_queries: vec![McpQuery {
                server: "sourcegraph".into(),
                tool: "search".into(),
                params: serde_json::json!({}),
                reason: "test".into(),
            }],
            ..sample_package()
        };

        let results = execute_queries(&transport, &pkg);
        assert_eq!(results[0].discovered_files.len(), 2);
        assert_eq!(
            results[0].discovered_files[0].source,
            ContextSource::Semantic
        );
    }

    #[test]
    fn execute_with_error_response() {
        let mut transport = MockTransport::new();
        transport.add_server("serena");
        transport.add_response(
            "serena",
            JsonRpcResponse {
                jsonrpc: "2.0".into(),
                id: 1,
                result: None,
                error: Some(JsonRpcError {
                    code: -32600,
                    message: "Symbol not found".into(),
                }),
            },
        );

        let pkg = ContextPackage {
            mcp_queries: vec![McpQuery {
                server: "serena".into(),
                tool: "find_symbol".into(),
                params: serde_json::json!({}),
                reason: "test".into(),
            }],
            ..sample_package()
        };

        let results = execute_queries(&transport, &pkg);
        assert!(!results[0].success);
        assert_eq!(results[0].error.as_deref(), Some("Symbol not found"));
    }

    // ── Merge results ─────────────────────────────────────────────

    #[test]
    fn merge_adds_discovered_files() {
        let mut pkg = sample_package();
        let results = vec![McpResult {
            query: pkg.mcp_queries[0].clone(),
            success: true,
            discovered_files: vec![ContextItem {
                source: ContextSource::Structural,
                file_path: PathBuf::from("src/new_file.rs"),
                relevance_score: 0.8,
                reason: "MCP".into(),
            }],
            raw_response: None,
            error: None,
        }];

        merge_results(&mut pkg, &results);
        assert_eq!(pkg.items.len(), 1);
        assert_eq!(pkg.items[0].file_path, PathBuf::from("src/new_file.rs"));
    }

    #[test]
    fn merge_skips_failed_results() {
        let mut pkg = sample_package();
        let results = vec![McpResult {
            query: pkg.mcp_queries[0].clone(),
            success: false,
            discovered_files: vec![ContextItem {
                source: ContextSource::Structural,
                file_path: PathBuf::from("src/bad.rs"),
                relevance_score: 0.8,
                reason: "MCP".into(),
            }],
            raw_response: None,
            error: Some("failed".into()),
        }];

        merge_results(&mut pkg, &results);
        assert!(pkg.items.is_empty());
    }

    #[test]
    fn merge_deduplicates() {
        let mut pkg = sample_package();
        pkg.items.push(ContextItem {
            source: ContextSource::FileReference,
            file_path: PathBuf::from("src/auth.rs"),
            relevance_score: 1.0,
            reason: "existing".into(),
        });

        let results = vec![McpResult {
            query: pkg.mcp_queries[0].clone(),
            success: true,
            discovered_files: vec![ContextItem {
                source: ContextSource::Structural,
                file_path: PathBuf::from("src/auth.rs"), // duplicate
                relevance_score: 0.8,
                reason: "MCP".into(),
            }],
            raw_response: None,
            error: None,
        }];

        merge_results(&mut pkg, &results);
        assert_eq!(pkg.items.len(), 1); // No duplicate added
    }

    // ── Path extraction ───────────────────────────────────────────

    #[test]
    fn extract_paths_from_text_finds_files() {
        let paths = extract_paths_from_text("Found in `src/auth.rs` and src/db/mod.rs");
        assert!(paths.contains(&"src/auth.rs".to_string()));
        assert!(paths.contains(&"src/db/mod.rs".to_string()));
    }

    #[test]
    fn extract_paths_ignores_non_files() {
        let paths = extract_paths_from_text("The function returns true");
        assert!(paths.is_empty());
    }

    #[test]
    fn looks_like_file_path_valid() {
        assert!(looks_like_file_path("src/main.rs"));
        assert!(looks_like_file_path("app/page.tsx"));
    }

    #[test]
    fn looks_like_file_path_invalid() {
        assert!(!looks_like_file_path("ab"));
        assert!(!looks_like_file_path("hello world.rs"));
    }

    // ── Server config ─────────────────────────────────────────────

    #[test]
    fn default_configs_have_known_servers() {
        let configs = default_server_configs();
        assert!(configs.iter().any(|c| c.server_name == "serena"));
        assert!(configs.iter().any(|c| c.server_name == "sourcegraph"));
    }

    #[test]
    fn extract_files_from_content_array() {
        let response = JsonRpcResponse {
            jsonrpc: "2.0".into(),
            id: 1,
            result: Some(serde_json::json!({
                "content": [
                    {"text": "Found definition in `src/auth/service.rs` at line 42"},
                    {"text": "Also referenced in src/db/models.rs"}
                ]
            })),
            error: None,
        };
        let files = extract_files_from_response(&response);
        assert!(files
            .iter()
            .any(|f| f.file_path == PathBuf::from("src/auth/service.rs")));
        assert!(files
            .iter()
            .any(|f| f.file_path == PathBuf::from("src/db/models.rs")));
        assert!(files.iter().all(|f| f.source == ContextSource::Structural));
    }

    #[test]
    fn extract_files_from_empty_result() {
        let response = JsonRpcResponse {
            jsonrpc: "2.0".into(),
            id: 1,
            result: None,
            error: None,
        };
        let files = extract_files_from_response(&response);
        assert!(files.is_empty());
    }

    #[test]
    fn extract_files_from_content_no_paths() {
        let response = JsonRpcResponse {
            jsonrpc: "2.0".into(),
            id: 1,
            result: Some(serde_json::json!({
                "content": [
                    {"text": "No file paths here, just text"}
                ]
            })),
            error: None,
        };
        let files = extract_files_from_response(&response);
        assert!(files.is_empty());
    }

    #[test]
    fn extract_files_combines_all_sources() {
        let response = JsonRpcResponse {
            jsonrpc: "2.0".into(),
            id: 1,
            result: Some(serde_json::json!({
                "content": [{"text": "See `src/api.rs`"}],
                "symbols": [{"name": "Handler", "relative_path": "src/handler.rs"}],
                "results": [{"path": "src/search.rs", "line": 1}]
            })),
            error: None,
        };
        let files = extract_files_from_response(&response);
        assert!(files.len() >= 3);
        assert!(files
            .iter()
            .any(|f| f.file_path == PathBuf::from("src/api.rs")));
        assert!(files
            .iter()
            .any(|f| f.file_path == PathBuf::from("src/handler.rs")));
        assert!(files
            .iter()
            .any(|f| f.file_path == PathBuf::from("src/search.rs")));
    }

    #[test]
    fn extract_paths_from_text_with_quotes() {
        let paths = extract_paths_from_text("Open '\"src/config.toml\"' for settings");
        assert!(paths.contains(&"src/config.toml".to_string()));
    }

    #[test]
    fn looks_like_file_path_many_extensions() {
        assert!(looks_like_file_path("app/page.tsx"));
        assert!(looks_like_file_path("script.py"));
        assert!(looks_like_file_path("main.go"));
        assert!(looks_like_file_path("test.java"));
        assert!(looks_like_file_path("config.toml"));
        assert!(looks_like_file_path("schema.sql"));
        assert!(looks_like_file_path("index.html"));
        assert!(looks_like_file_path("style.css"));
        assert!(looks_like_file_path("test.yaml"));
        assert!(looks_like_file_path("test.yml"));
        assert!(looks_like_file_path("data.json"));
        assert!(looks_like_file_path("code.rb"));
        assert!(looks_like_file_path("code.c"));
        assert!(looks_like_file_path("code.cpp"));
        assert!(looks_like_file_path("code.h"));
        assert!(looks_like_file_path("code.swift"));
        assert!(looks_like_file_path("code.kt"));
        assert!(looks_like_file_path("code.sh"));
    }

    #[test]
    fn looks_like_file_path_too_short() {
        assert!(!looks_like_file_path("a.c"));
        assert!(!looks_like_file_path("x.h"));
    }

    #[test]
    fn execute_with_transport_error() {
        struct FailTransport;
        impl McpTransport for FailTransport {
            fn call(&self, _server: &str, _request: &JsonRpcRequest) -> Result<JsonRpcResponse> {
                anyhow::bail!("Connection refused")
            }
            fn is_available(&self, _server: &str) -> bool {
                true
            }
        }

        let pkg = ContextPackage {
            mcp_queries: vec![McpQuery {
                server: "test".into(),
                tool: "search".into(),
                params: serde_json::json!({}),
                reason: "test".into(),
            }],
            ..sample_package()
        };

        let results = execute_queries(&FailTransport, &pkg);
        assert_eq!(results.len(), 1);
        assert!(!results[0].success);
        assert!(results[0]
            .error
            .as_ref()
            .unwrap()
            .contains("Connection refused"));
    }

    #[test]
    fn execute_empty_package() {
        let transport = MockTransport::new();
        let pkg = ContextPackage {
            id: "empty".into(),
            task_id: None,
            items: vec![],
            mcp_queries: vec![],
            summary: "Empty".into(),
            created_at: "now".into(),
        };
        let results = execute_queries(&transport, &pkg);
        assert!(results.is_empty());
    }

    #[test]
    fn merge_multiple_results() {
        let mut pkg = sample_package();
        let results = vec![
            McpResult {
                query: pkg.mcp_queries[0].clone(),
                success: true,
                discovered_files: vec![
                    ContextItem {
                        source: ContextSource::Structural,
                        file_path: PathBuf::from("src/a.rs"),
                        relevance_score: 0.8,
                        reason: "MCP".into(),
                    },
                    ContextItem {
                        source: ContextSource::Structural,
                        file_path: PathBuf::from("src/b.rs"),
                        relevance_score: 0.7,
                        reason: "MCP".into(),
                    },
                ],
                raw_response: None,
                error: None,
            },
            McpResult {
                query: pkg.mcp_queries[0].clone(),
                success: true,
                discovered_files: vec![ContextItem {
                    source: ContextSource::Semantic,
                    file_path: PathBuf::from("src/c.rs"),
                    relevance_score: 0.9,
                    reason: "search".into(),
                }],
                raw_response: None,
                error: None,
            },
        ];
        merge_results(&mut pkg, &results);
        assert_eq!(pkg.items.len(), 3);
    }

    #[test]
    fn mcp_server_config_serde() {
        let config = McpServerConfig {
            server_name: "test".into(),
            command: "npx".into(),
            args: vec!["-y".into(), "test-server".into()],
            env: HashMap::from([("KEY".into(), "VALUE".into())]),
        };
        let json = serde_json::to_string(&config).unwrap();
        let parsed: McpServerConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.server_name, "test");
        assert_eq!(parsed.args.len(), 2);
        assert_eq!(parsed.env.get("KEY").unwrap(), "VALUE");
    }

    #[test]
    fn mcp_result_fields() {
        let result = McpResult {
            query: McpQuery {
                server: "serena".into(),
                tool: "find_symbol".into(),
                params: serde_json::json!({}),
                reason: "test".into(),
            },
            success: true,
            discovered_files: vec![],
            raw_response: Some(serde_json::json!({"ok": true})),
            error: None,
        };
        assert!(result.success);
        assert!(result.raw_response.is_some());
        assert!(result.error.is_none());
    }

    #[test]
    fn execute_multiple_queries_increments_ids() {
        let mut transport = MockTransport::new();
        transport.add_server("serena");
        transport.add_response(
            "serena",
            JsonRpcResponse {
                jsonrpc: "2.0".into(),
                id: 0,
                result: Some(serde_json::json!({})),
                error: None,
            },
        );

        let pkg = ContextPackage {
            mcp_queries: vec![
                McpQuery {
                    server: "serena".into(),
                    tool: "a".into(),
                    params: serde_json::json!({}),
                    reason: "r".into(),
                },
                McpQuery {
                    server: "serena".into(),
                    tool: "b".into(),
                    params: serde_json::json!({}),
                    reason: "r".into(),
                },
            ],
            ..sample_package()
        };

        let results = execute_queries(&transport, &pkg);
        assert_eq!(results.len(), 2);
    }
}
