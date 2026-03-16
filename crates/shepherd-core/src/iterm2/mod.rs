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
    PathBuf::from(home).join(".claude").join("projects").join(encoded)
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
        self.adopted.lock().await.get(iterm2_session_id).map(|s| s.cwd.clone())
    }

    pub async fn get_task_id_for_iterm2(&self, iterm2_session_id: &str) -> Option<i64> {
        self.adopted.lock().await.get(iterm2_session_id).map(|s| s.task_id)
    }

    /// Spawn the background adoption loop. Call once at startup.
    pub fn spawn(
        self: Arc<Self>,
        db: Arc<Mutex<rusqlite::Connection>>,
        event_tx: broadcast::Sender<ServerEvent>,
    ) {
        tokio::spawn(async move {
            self.run_loop(db, event_tx).await;
        });
    }

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

    async fn run_connected(
        &self,
        auth: &auth::Iterm2Auth,
        db: &Arc<Mutex<rusqlite::Connection>>,
        event_tx: &broadcast::Sender<ServerEvent>,
    ) -> anyhow::Result<()> {
        let socket = client::find_socket()?;
        let mut ws = client::WsClient::connect(&socket, auth).await?;

        let adopted_ids: std::collections::HashSet<String> = {
            self.adopted.lock().await.keys().cloned().collect()
        };
        let mut scanner = scanner::Scanner::new(adopted_ids);
        let candidates = scanner.scan(&mut ws).await?;

        if !candidates.is_empty() {
            scanner.subscribe_terminate(&mut ws).await?;
        }

        for candidate in candidates {
            let task = {
                let conn = db.lock().await;
                crate::db::queries::create_task(&conn, &CreateTask {
                    title: format!("iTerm2: {}", candidate.cwd),
                    prompt: None,
                    agent_id: "iterm2-adopted".to_string(),
                    repo_path: Some(candidate.cwd.clone()),
                    isolation_mode: Some("none".to_string()),
                    iterm2_session_id: Some(candidate.iterm2_session_id.clone()),
                })?
            };
            tracing::info!(
                "Adopted iTerm2 session {} as task {}",
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
    fn test_encode_cwd_for_path() {
        let encoded = encode_cwd_for_projects("/home/user/myproject");
        assert_eq!(encoded, "-home-user-myproject");
    }
}
