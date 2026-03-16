use crate::iterm2::client::{iterm2, iterm2::list_sessions_response, Iterm2Transport};
use std::collections::HashSet;

/// Pairs of (jobName substring, canonical agent identifier).
/// Order matters: first match wins.
const KNOWN_AGENTS: &[(&str, &str)] = &[
    ("claude", "claude-code"),
    ("codex", "codex"),
    ("adal", "adal"),
    ("aider", "aider"),
    ("gemini", "gemini-cli"),
    ("opencode", "opencode"),
];

/// Return the canonical agent name if `job_name` matches a known agent.
pub fn detect_agent(job_name: &str) -> Option<&'static str> {
    KNOWN_AGENTS
        .iter()
        .find(|(pat, _)| job_name.contains(*pat))
        .map(|(_, name)| *name)
}

#[derive(Debug)]
pub struct AdoptionCandidate {
    pub iterm2_session_id: String,
    pub cwd: String,
    /// Canonical name of the detected coding agent (e.g. "claude-code", "aider").
    pub agent_name: String,
}

pub struct Scanner {
    adopted: HashSet<String>,
}

impl Scanner {
    pub fn new(adopted: HashSet<String>) -> Self {
        Self { adopted }
    }

    pub fn mark_adopted(&mut self, session_id: String) {
        self.adopted.insert(session_id);
    }

    pub fn mark_terminated(&mut self, session_id: &str) {
        self.adopted.remove(session_id);
    }

    /// One scan pass: list sessions, query jobName for unadopted ones,
    /// return candidates where jobName matches a known coding agent.
    pub async fn scan(
        &mut self,
        transport: &mut dyn Iterm2Transport,
    ) -> anyhow::Result<Vec<AdoptionCandidate>> {
        // 1. List all sessions
        let resp = transport.send_recv(iterm2::ClientOriginatedMessage {
            id: None,
            submessage: Some(iterm2::client_originated_message::Submessage::ListSessionsRequest(
                iterm2::ListSessionsRequest {},
            )),
        }).await?;

        let list_resp = match resp.submessage {
            Some(iterm2::server_originated_message::Submessage::ListSessionsResponse(r)) => r,
            _ => anyhow::bail!("expected ListSessionsResponse"),
        };

        // 2. Walk windows → tabs → SplitTreeNode tree
        let session_ids: Vec<String> = collect_session_ids(&list_resp.windows);

        // 3. For each unadopted session, query jobName
        let mut candidates = Vec::new();
        for session_id in session_ids {
            if self.adopted.contains(&session_id) {
                continue;
            }

            // Query jobName
            let job_resp = transport.send_recv(iterm2::ClientOriginatedMessage {
                id: None,
                submessage: Some(iterm2::client_originated_message::Submessage::VariableRequest(
                    iterm2::VariableRequest {
                        get: vec!["jobName".to_string()],
                        scope: Some(iterm2::variable_request::Scope::SessionId(session_id.clone())),
                        set: vec![],
                    }
                )),
            }).await?;

            let job_name = match job_resp.submessage {
                Some(iterm2::server_originated_message::Submessage::VariableResponse(vr)) => {
                    vr.values.into_iter().next().unwrap_or_default()
                }
                _ => String::new(),
            };

            // values are JSON-encoded: check if it matches a known coding agent
            let Some(agent_name) = detect_agent(&job_name) else {
                continue;
            };
            let agent_name = agent_name.to_string();

            // Query CWD (path variable, requires shell integration)
            let path_resp = transport.send_recv(iterm2::ClientOriginatedMessage {
                id: None,
                submessage: Some(iterm2::client_originated_message::Submessage::VariableRequest(
                    iterm2::VariableRequest {
                        get: vec!["path".to_string()],
                        scope: Some(iterm2::variable_request::Scope::SessionId(session_id.clone())),
                        set: vec![],
                    }
                )),
            }).await?;

            let cwd = match path_resp.submessage {
                Some(iterm2::server_originated_message::Submessage::VariableResponse(vr)) => {
                    // JSON-encoded string: strip surrounding quotes if present
                    let raw = vr.values.into_iter().next().unwrap_or_default();
                    serde_json::from_str::<String>(&raw).unwrap_or(raw)
                }
                _ => String::new(),
            };

            // Subscribe to screen updates for this session
            transport.send_only(iterm2::ClientOriginatedMessage {
                id: None,
                submessage: Some(iterm2::client_originated_message::Submessage::NotificationRequest(
                    iterm2::NotificationRequest {
                        session: Some(session_id.clone()),
                        subscribe: Some(true),
                        notification_type: Some(iterm2::NotificationType::NotifyOnScreenUpdate as i32),
                        arguments: None,
                    }
                )),
            }).await?;

            candidates.push(AdoptionCandidate { iterm2_session_id: session_id, cwd, agent_name });
        }
        Ok(candidates)
    }

    /// Subscribe globally to session termination (call once after first adoption).
    pub async fn subscribe_terminate(&self, transport: &mut dyn Iterm2Transport) -> anyhow::Result<()> {
        transport.send_only(iterm2::ClientOriginatedMessage {
            id: None,
            submessage: Some(iterm2::client_originated_message::Submessage::NotificationRequest(
                iterm2::NotificationRequest {
                    session: None,
                    subscribe: Some(true),
                    notification_type: Some(iterm2::NotificationType::NotifyOnTerminateSession as i32),
                    arguments: None,
                }
            )),
        }).await
    }
}

/// Walk the ListSessionsResponse window tree to collect all session unique_identifiers.
fn collect_session_ids(windows: &[list_sessions_response::Window]) -> Vec<String> {
    let mut ids = Vec::new();
    for window in windows {
        for tab in &window.tabs {
            if let Some(root) = &tab.root {
                walk_node(root, &mut ids);
            }
        }
    }
    ids
}

fn walk_node(node: &iterm2::SplitTreeNode, out: &mut Vec<String>) {
    for link in &node.links {
        match &link.child {
            Some(iterm2::split_tree_node::split_tree_link::Child::Session(summary)) => {
                if let Some(id) = &summary.unique_identifier {
                    out.push(id.clone());
                }
            }
            Some(iterm2::split_tree_node::split_tree_link::Child::Node(child_node)) => {
                walk_node(child_node, out);
            }
            None => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::iterm2::client::iterm2;
    use crate::iterm2::client::iterm2::{
        client_originated_message, server_originated_message,
        list_sessions_response, split_tree_node,
    };

    // Helper: build a ServerOriginatedMessage with one session in a window
    fn make_list_response(sessions: Vec<(&str, &str)>) -> iterm2::ServerOriginatedMessage {
        let links: Vec<iterm2::split_tree_node::SplitTreeLink> = sessions
            .into_iter()
            .map(|(id, title)| iterm2::split_tree_node::SplitTreeLink {
                child: Some(split_tree_node::split_tree_link::Child::Session(
                    iterm2::SessionSummary {
                        unique_identifier: Some(id.to_string()),
                        title: Some(title.to_string()),
                        ..Default::default()
                    },
                )),
            })
            .collect();
        let root = iterm2::SplitTreeNode { links, ..Default::default() };
        let tab = list_sessions_response::Tab { root: Some(root), ..Default::default() };
        let window = list_sessions_response::Window { tabs: vec![tab], ..Default::default() };
        let lsr = iterm2::ListSessionsResponse { windows: vec![window], ..Default::default() };
        iterm2::ServerOriginatedMessage {
            submessage: Some(server_originated_message::Submessage::ListSessionsResponse(lsr)),
            ..Default::default()
        }
    }

    fn make_variable_response(value: &str) -> iterm2::ServerOriginatedMessage {
        let vr = iterm2::VariableResponse {
            values: vec![format!("\"{}\"", value)],  // JSON-encoded
            ..Default::default()
        };
        iterm2::ServerOriginatedMessage {
            submessage: Some(server_originated_message::Submessage::VariableResponse(vr)),
            ..Default::default()
        }
    }

    #[test]
    fn test_mark_adopted_tracks_session() {
        let mut scanner = Scanner::new(std::collections::HashSet::new());
        assert!(!scanner.adopted.contains("sess-a"));
        scanner.mark_adopted("sess-a".to_string());
        assert!(scanner.adopted.contains("sess-a"));
    }

    #[test]
    fn test_mark_terminated_removes_session() {
        let mut adopted = std::collections::HashSet::new();
        adopted.insert("sess-b".to_string());
        let mut scanner = Scanner::new(adopted);
        assert!(scanner.adopted.contains("sess-b"));
        scanner.mark_terminated("sess-b");
        assert!(!scanner.adopted.contains("sess-b"));
    }

    #[test]
    fn test_mark_terminated_noop_when_not_adopted() {
        let mut scanner = Scanner::new(std::collections::HashSet::new());
        // Should not panic when removing a session that was never added
        scanner.mark_terminated("never-adopted");
    }

    #[tokio::test]
    async fn test_scan_finds_claude_session() {
        struct MockT { calls: usize }
        #[async_trait::async_trait]
        impl crate::iterm2::client::Iterm2Transport for MockT {
            async fn send_recv(&mut self, _: iterm2::ClientOriginatedMessage) -> anyhow::Result<iterm2::ServerOriginatedMessage> {
                let r = match self.calls {
                    0 => make_list_response(vec![("sess-1", "bash")]),
                    1 => make_variable_response("claude"),
                    2 => make_variable_response("/home/user/myproject"),
                    _ => panic!("unexpected call {}", self.calls),
                };
                self.calls += 1;
                Ok(r)
            }
            async fn send_only(&mut self, _: iterm2::ClientOriginatedMessage) -> anyhow::Result<()> { Ok(()) }
            async fn recv(&mut self) -> anyhow::Result<iterm2::ServerOriginatedMessage> {
                futures_util::future::pending().await
            }
        }
        let mut scanner = Scanner::new(std::collections::HashSet::new());
        let candidates = scanner.scan(&mut MockT { calls: 0 }).await.unwrap();
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].iterm2_session_id, "sess-1");
        assert_eq!(candidates[0].cwd, "/home/user/myproject");
        assert_eq!(candidates[0].agent_name, "claude-code");
    }

    #[tokio::test]
    async fn test_scan_skips_non_claude_session() {
        struct MockT { calls: usize }
        #[async_trait::async_trait]
        impl crate::iterm2::client::Iterm2Transport for MockT {
            async fn send_recv(&mut self, _: iterm2::ClientOriginatedMessage) -> anyhow::Result<iterm2::ServerOriginatedMessage> {
                let r = match self.calls {
                    0 => make_list_response(vec![("sess-2", "vim")]),
                    1 => make_variable_response("vim"),
                    _ => panic!("unexpected call {}", self.calls),
                };
                self.calls += 1;
                Ok(r)
            }
            async fn send_only(&mut self, _: iterm2::ClientOriginatedMessage) -> anyhow::Result<()> { Ok(()) }
            async fn recv(&mut self) -> anyhow::Result<iterm2::ServerOriginatedMessage> {
                futures_util::future::pending().await
            }
        }
        let mut scanner = Scanner::new(std::collections::HashSet::new());
        let candidates = scanner.scan(&mut MockT { calls: 0 }).await.unwrap();
        assert!(candidates.is_empty());
    }

    #[tokio::test]
    async fn test_scan_deduplicates_already_adopted() {
        struct MockT;
        #[async_trait::async_trait]
        impl crate::iterm2::client::Iterm2Transport for MockT {
            async fn send_recv(&mut self, _: iterm2::ClientOriginatedMessage) -> anyhow::Result<iterm2::ServerOriginatedMessage> {
                Ok(make_list_response(vec![("sess-3", "claude")]))
            }
            async fn send_only(&mut self, _: iterm2::ClientOriginatedMessage) -> anyhow::Result<()> { Ok(()) }
            async fn recv(&mut self) -> anyhow::Result<iterm2::ServerOriginatedMessage> {
                futures_util::future::pending().await
            }
        }
        let mut adopted = std::collections::HashSet::new();
        adopted.insert("sess-3".to_string());
        let mut scanner = Scanner::new(adopted);
        let candidates = scanner.scan(&mut MockT).await.unwrap();
        assert!(candidates.is_empty());
    }

    // ── detect_agent unit tests ──────────────────────────────────────────────

    #[test]
    fn test_detect_agent_claude() {
        assert_eq!(detect_agent("claude"), Some("claude-code"));
    }

    #[test]
    fn test_detect_agent_codex() {
        assert_eq!(detect_agent("codex"), Some("codex"));
    }

    #[test]
    fn test_detect_agent_adal() {
        assert_eq!(detect_agent("adal"), Some("adal"));
    }

    #[test]
    fn test_detect_agent_aider() {
        assert_eq!(detect_agent("aider"), Some("aider"));
    }

    #[test]
    fn test_detect_agent_gemini() {
        assert_eq!(detect_agent("gemini"), Some("gemini-cli"));
    }

    #[test]
    fn test_detect_agent_opencode() {
        assert_eq!(detect_agent("opencode"), Some("opencode"));
    }

    #[test]
    fn test_detect_agent_unknown_returns_none() {
        assert_eq!(detect_agent("vim"), None);
        assert_eq!(detect_agent("bash"), None);
        assert_eq!(detect_agent(""), None);
    }

    #[test]
    fn test_detect_agent_substring_match() {
        // jobName values are JSON-encoded; may include quotes or path prefixes
        assert_eq!(detect_agent("\"claude\""), Some("claude-code"));
        assert_eq!(detect_agent("/usr/local/bin/aider"), Some("aider"));
    }

    // ── scan tests for non-claude agents ────────────────────────────────────

    /// Helper: a mock that returns a single-session list then two variable responses.
    macro_rules! make_agent_mock {
        ($name:ident, $sess:expr, $job:expr, $cwd:expr) => {
            struct $name { calls: usize }
            #[async_trait::async_trait]
            impl crate::iterm2::client::Iterm2Transport for $name {
                async fn send_recv(
                    &mut self,
                    _: iterm2::ClientOriginatedMessage,
                ) -> anyhow::Result<iterm2::ServerOriginatedMessage> {
                    let r = match self.calls {
                        0 => make_list_response(vec![($sess, "title")]),
                        1 => make_variable_response($job),
                        2 => make_variable_response($cwd),
                        _ => panic!("unexpected call {}", self.calls),
                    };
                    self.calls += 1;
                    Ok(r)
                }
                async fn send_only(
                    &mut self,
                    _: iterm2::ClientOriginatedMessage,
                ) -> anyhow::Result<()> {
                    Ok(())
                }
                async fn recv(
                    &mut self,
                ) -> anyhow::Result<iterm2::ServerOriginatedMessage> {
                    futures_util::future::pending().await
                }
            }
        };
    }

    #[tokio::test]
    async fn test_scan_finds_codex_session() {
        make_agent_mock!(MockCodex, "sess-codex", "codex", "/home/user/proj");
        let mut scanner = Scanner::new(std::collections::HashSet::new());
        let candidates = scanner.scan(&mut MockCodex { calls: 0 }).await.unwrap();
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].agent_name, "codex");
        assert_eq!(candidates[0].cwd, "/home/user/proj");
    }

    #[tokio::test]
    async fn test_scan_finds_adal_session() {
        make_agent_mock!(MockAdal, "sess-adal", "adal", "/src/myrepo");
        let mut scanner = Scanner::new(std::collections::HashSet::new());
        let candidates = scanner.scan(&mut MockAdal { calls: 0 }).await.unwrap();
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].agent_name, "adal");
    }

    #[tokio::test]
    async fn test_scan_finds_aider_session() {
        make_agent_mock!(MockAider, "sess-aider", "aider", "/src/myrepo");
        let mut scanner = Scanner::new(std::collections::HashSet::new());
        let candidates = scanner.scan(&mut MockAider { calls: 0 }).await.unwrap();
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].agent_name, "aider");
    }

    #[tokio::test]
    async fn test_scan_finds_gemini_session() {
        make_agent_mock!(MockGemini, "sess-gemini", "gemini", "/src/project");
        let mut scanner = Scanner::new(std::collections::HashSet::new());
        let candidates = scanner.scan(&mut MockGemini { calls: 0 }).await.unwrap();
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].agent_name, "gemini-cli");
    }

    #[tokio::test]
    async fn test_scan_finds_opencode_session() {
        make_agent_mock!(MockOpencode, "sess-opencode", "opencode", "/src/app");
        let mut scanner = Scanner::new(std::collections::HashSet::new());
        let candidates = scanner.scan(&mut MockOpencode { calls: 0 }).await.unwrap();
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].agent_name, "opencode");
    }
}
