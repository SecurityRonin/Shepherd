pub mod auth;
pub mod client;
pub mod scanner;
pub mod session;

use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex};

use crate::db::models::CreateTask;
use crate::events::{ServerEvent, TaskEvent};

/// Encode a CWD path the same way Claude Code does when naming project directories.
/// Claude replaces '/' with '-', producing a leading '-' for absolute paths.
pub fn encode_cwd_for_projects(cwd: &str) -> String {
    cwd.replace('/', "-")
}

/// List Claude Code session IDs available for resume in a given projects directory.
/// Returns JSONL filename stems sorted newest-first by mtime.
pub fn list_claude_sessions(project_dir: &str) -> Vec<String> {
    let dir = Path::new(project_dir);
    if !dir.exists() {
        return vec![];
    }
    let mut entries: Vec<(std::time::SystemTime, String)> = std::fs::read_dir(dir)
        .map(|rd| {
            rd.filter_map(|e| e.ok())
                .filter(|e| e.path().extension().map_or(false, |x| x == "jsonl"))
                .filter_map(|e| {
                    let mtime = e.metadata().ok()?.modified().ok()?;
                    let stem = e.path().file_stem()?.to_str()?.to_string();
                    Some((mtime, stem))
                })
                .collect()
        })
        .unwrap_or_default();
    entries.sort_by(|a, b| b.0.cmp(&a.0));
    entries.into_iter().map(|(_, stem)| stem).collect()
}

/// Resolve the Claude projects directory for a given CWD.
pub fn claude_project_dir(cwd: &str) -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_default();
    let encoded = encode_cwd_for_projects(cwd);
    PathBuf::from(home)
        .join(".claude")
        .join("projects")
        .join(encoded)
}

/// Handle for the running iTerm2 integration.
pub struct Iterm2Manager {
    adopted: Arc<Mutex<std::collections::HashMap<String, session::AdoptedSession>>>,
    auth_path: PathBuf,
}

impl Iterm2Manager {
    pub fn new(auth_path: PathBuf) -> Self {
        Self {
            adopted: Arc::new(Mutex::new(std::collections::HashMap::new())),
            auth_path,
        }
    }

    /// Check whether the auth credentials file exists.
    pub fn is_auth_configured(&self) -> bool {
        self.auth_path.exists()
    }

    pub async fn get_adopted_cwd(&self, iterm2_session_id: &str) -> Option<String> {
        self.adopted
            .lock()
            .await
            .get(iterm2_session_id)
            .map(|s| s.cwd.clone())
    }

    pub async fn get_task_id_for_iterm2(&self, iterm2_session_id: &str) -> Option<i64> {
        self.adopted
            .lock()
            .await
            .get(iterm2_session_id)
            .map(|s| s.task_id)
    }

    /// Spawn the background adoption loop. Call once at startup.
    // tarpaulin-start-ignore
    pub fn spawn(
        self: Arc<Self>,
        db: Arc<Mutex<rusqlite::Connection>>,
        event_tx: broadcast::Sender<ServerEvent>,
    ) {
        tokio::spawn(async move {
            self.run_loop(db, event_tx).await;
        });
    }

    // tarpaulin-stop-ignore
    // tarpaulin-start-ignore
    async fn run_loop(
        &self,
        db: Arc<Mutex<rusqlite::Connection>>,
        event_tx: broadcast::Sender<ServerEvent>,
    ) {
        loop {
            match auth::load_auth(&self.auth_path) {
                Err(e) => {
                    tracing::debug!("iTerm2 auth not available ({e}), skipping scan");
                }
                Ok(auth) => {
                    if let Err(e) = self.run_connected(&auth, &db, &event_tx).await {
                        tracing::warn!("iTerm2 session loop error: {e:#}");
                    }
                }
            }
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        }
    }

    // tarpaulin-stop-ignore
    // tarpaulin-start-ignore
    async fn run_connected(
        &self,
        auth: &auth::Iterm2Auth,
        db: &Arc<Mutex<rusqlite::Connection>>,
        event_tx: &broadcast::Sender<ServerEvent>,
    ) -> anyhow::Result<()> {
        let socket = client::find_socket()?;
        let mut ws = client::WsClient::connect(&socket, auth).await?;

        let adopted_ids: std::collections::HashSet<String> =
            { self.adopted.lock().await.keys().cloned().collect() };
        let mut scanner = scanner::Scanner::new(adopted_ids);
        let candidates = scanner.scan(&mut ws).await?;

        if !candidates.is_empty() {
            scanner.subscribe_terminate(&mut ws).await?;
        }

        for candidate in candidates {
            let task = {
                let conn = db.lock().await;
                crate::db::queries::create_task(
                    &conn,
                    &CreateTask {
                        title: format!("{}: {}", candidate.agent_name, candidate.cwd),
                        prompt: None,
                        agent_id: format!("iterm2-{}", candidate.agent_name),
                        repo_path: Some(candidate.cwd.clone()),
                        isolation_mode: Some("none".to_string()),
                        iterm2_session_id: Some(candidate.iterm2_session_id.clone()),
                    },
                )?
            };
            tracing::info!(
                "Adopted {} session {} as task {}",
                candidate.agent_name,
                candidate.iterm2_session_id,
                task.id
            );
            let _ = event_tx.send(ServerEvent::TaskCreated(TaskEvent {
                id: task.id,
                title: task.title.clone(),
                agent_id: task.agent_id.clone(),
                status: task.status.as_str().to_string(),
                branch: task.branch.clone(),
                repo_path: task.repo_path.clone(),
                iterm2_session_id: Some(candidate.iterm2_session_id.clone()),
            }));
            self.adopted.lock().await.insert(
                candidate.iterm2_session_id.clone(),
                session::AdoptedSession::new(task.id, candidate.iterm2_session_id, candidate.cwd),
            );
        }
        Ok(())
    }
    // tarpaulin-stop-ignore
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_claude_sessions_empty_dir() {
        let sessions = list_claude_sessions("/nonexistent/path");
        assert!(sessions.is_empty());
    }

    #[test]
    fn test_list_claude_sessions_lists_jsonl_files() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("abc-111.jsonl"), "{}").unwrap();
        std::fs::write(dir.path().join("abc-222.jsonl"), "{}").unwrap();
        let sessions = list_claude_sessions(dir.path().to_str().unwrap());
        assert_eq!(sessions.len(), 2);
        assert!(sessions.iter().any(|s| s == "abc-111" || s == "abc-222"));
    }

    #[test]
    fn test_list_claude_sessions_ignores_non_jsonl() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("session.jsonl"), "{}").unwrap();
        std::fs::write(dir.path().join("readme.txt"), "ignore me").unwrap();
        let sessions = list_claude_sessions(dir.path().to_str().unwrap());
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0], "session");
    }

    #[test]
    fn test_encode_cwd_for_path() {
        let encoded = encode_cwd_for_projects("/home/user/myproject");
        assert_eq!(encoded, "-home-user-myproject");
    }

    #[test]
    fn test_claude_project_dir_structure() {
        let dir = claude_project_dir("/home/user/myproject");
        let s = dir.to_str().unwrap();
        assert!(s.contains(".claude"));
        assert!(s.contains("projects"));
        assert!(s.ends_with("-home-user-myproject"));
    }

    #[test]
    fn test_manager_is_auth_not_configured() {
        let mgr = Iterm2Manager::new(PathBuf::from("/nonexistent/iterm2-auth.json"));
        assert!(!mgr.is_auth_configured());
    }

    #[test]
    fn test_manager_is_auth_configured_when_file_exists() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("iterm2-auth.json");
        std::fs::write(&path, r#"{"cookie":"c","key":"k"}"#).unwrap();
        let mgr = Iterm2Manager::new(path);
        assert!(mgr.is_auth_configured());
    }

    #[tokio::test]
    async fn test_manager_get_adopted_cwd_returns_none_when_empty() {
        let mgr = Iterm2Manager::new(PathBuf::from("/tmp/test.json"));
        assert!(mgr.get_adopted_cwd("nonexistent-session").await.is_none());
    }

    #[tokio::test]
    async fn test_manager_get_task_id_returns_none_when_empty() {
        let mgr = Iterm2Manager::new(PathBuf::from("/tmp/test.json"));
        assert!(mgr
            .get_task_id_for_iterm2("nonexistent-session")
            .await
            .is_none());
    }

    #[test]
    fn test_encode_cwd_relative_path() {
        let encoded = encode_cwd_for_projects("relative/path");
        assert_eq!(encoded, "relative-path");
    }

    #[test]
    fn test_encode_cwd_single_dir() {
        let encoded = encode_cwd_for_projects("project");
        assert_eq!(encoded, "project");
    }

    #[test]
    fn test_encode_cwd_root() {
        let encoded = encode_cwd_for_projects("/");
        assert_eq!(encoded, "-");
    }

    #[test]
    fn test_list_claude_sessions_sorted_by_mtime() {
        let dir = tempfile::tempdir().unwrap();
        // Create files with different mtimes
        std::fs::write(dir.path().join("older.jsonl"), "{}").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(50));
        std::fs::write(dir.path().join("newer.jsonl"), "{}").unwrap();
        let sessions = list_claude_sessions(dir.path().to_str().unwrap());
        assert_eq!(sessions.len(), 2);
        // Newest first
        assert_eq!(sessions[0], "newer");
        assert_eq!(sessions[1], "older");
    }

    #[test]
    fn test_claude_project_dir_relative_path() {
        let dir = claude_project_dir("myproject");
        let s = dir.to_str().unwrap();
        assert!(s.contains(".claude"));
        assert!(s.contains("projects"));
        assert!(s.ends_with("myproject"));
    }

    #[tokio::test]
    async fn test_manager_adopted_map_after_insert() {
        let mgr = Iterm2Manager::new(PathBuf::from("/tmp/test.json"));
        mgr.adopted.lock().await.insert(
            "sess-42".to_string(),
            session::AdoptedSession::new(42, "sess-42".to_string(), "/repo".to_string()),
        );
        assert_eq!(
            mgr.get_adopted_cwd("sess-42").await.as_deref(),
            Some("/repo")
        );
        assert_eq!(mgr.get_task_id_for_iterm2("sess-42").await, Some(42));
        assert!(mgr.get_adopted_cwd("other").await.is_none());
    }
}
