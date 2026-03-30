pub mod sandbox;
pub mod status;

use anyhow::{Context, Result};
use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use sandbox::SandboxProfile;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{broadcast, Mutex};

#[derive(Debug, Clone)]
pub struct PtyOutput {
    pub task_id: i64,
    pub data: Vec<u8>,
}

struct PtyHandle {
    writer: Box<dyn Write + Send>,
    child: Box<dyn portable_pty::Child + Send>,
    master: Box<dyn portable_pty::MasterPty + Send>,
    task_id: i64,
    last_output: Instant,
}

pub struct PtyManager {
    handles: Arc<Mutex<HashMap<i64, PtyHandle>>>,
    output_tx: broadcast::Sender<PtyOutput>,
    max_agents: usize,
    sandbox: SandboxProfile,
    disable_agent_telemetry: bool,
}

impl PtyManager {
    pub fn new(max_agents: usize, sandbox: SandboxProfile) -> Self {
        let sandbox = if sandbox.enabled && !SandboxProfile::is_available() {
            tracing::warn!("nono.sh not found — sandbox disabled. Install from https://nono.sh");
            SandboxProfile::disabled()
        } else {
            sandbox
        };
        let (output_tx, _) = broadcast::channel(1024);
        Self {
            handles: Arc::new(Mutex::new(HashMap::new())),
            output_tx,
            max_agents,
            sandbox,
            disable_agent_telemetry: false,
        }
    }

    pub fn with_disable_telemetry(mut self, disable: bool) -> Self {
        self.disable_agent_telemetry = disable;
        self
    }

    pub fn subscribe_output(&self) -> broadcast::Receiver<PtyOutput> {
        self.output_tx.subscribe()
    }

    // tarpaulin-start-ignore
    #[tracing::instrument(skip(self, args))]
    pub async fn spawn(
        &self,
        task_id: i64,
        command: &str,
        args: &[String],
        cwd: &str,
    ) -> Result<()> {
        let current = self.handles.lock().await.len();
        if current >= self.max_agents {
            anyhow::bail!(
                "Agent limit reached ({}/{}). Task will remain queued.",
                current,
                self.max_agents
            );
        }

        let pty_system = native_pty_system();
        let pair = pty_system.openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })?;

        let (actual_cmd, actual_args) = self.sandbox.wrap_command(command, args);
        let mut cmd = CommandBuilder::new(&actual_cmd);
        cmd.args(&actual_args);
        cmd.cwd(cwd);

        if self.disable_agent_telemetry {
            for (key, value) in cost_saving_env_vars() {
                cmd.env(key, value);
            }
        }

        let child = pair
            .slave
            .spawn_command(cmd)
            .context("Failed to spawn agent process")?;

        let writer = pair
            .master
            .take_writer()
            .context("Failed to take PTY writer")?;

        let mut reader = pair
            .master
            .try_clone_reader()
            .context("Failed to clone PTY reader")?;
        let output_tx = self.output_tx.clone();
        let handles_ref = self.handles.clone();
        std::thread::spawn(move || {
            let mut buf = [0u8; 4096];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        let _ = output_tx.send(PtyOutput {
                            task_id,
                            data: buf[..n].to_vec(),
                        });
                        if let Ok(mut handles) = handles_ref.try_lock() {
                            if let Some(handle) = handles.get_mut(&task_id) {
                                handle.last_output = Instant::now();
                            }
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        let handle = PtyHandle {
            writer,
            child,
            master: pair.master,
            task_id,
            last_output: Instant::now(),
        };
        self.handles.lock().await.insert(task_id, handle);

        tracing::info!("Spawned PTY for task {task_id}: {command}");
        Ok(())
    }
    // tarpaulin-stop-ignore

    #[tracing::instrument(skip(self, data))]
    pub async fn write_to(&self, task_id: i64, data: &str) -> Result<()> {
        let mut handles = self.handles.lock().await;
        if let Some(handle) = handles.get_mut(&task_id) {
            handle.writer.write_all(data.as_bytes())?;
        }
        Ok(())
    }

    #[tracing::instrument(skip(self))]
    pub async fn kill(&self, task_id: i64) -> Result<()> {
        let mut handles = self.handles.lock().await;
        if let Some(mut handle) = handles.remove(&task_id) {
            handle.child.kill()?;
            tracing::info!("Killed PTY for task {task_id}");
        }
        Ok(())
    }

    #[tracing::instrument(skip(self))]
    pub async fn resize(&self, task_id: i64, cols: u16, rows: u16) -> Result<()> {
        let handles = self.handles.lock().await;
        if let Some(handle) = handles.get(&task_id) {
            handle.master.resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })?;
        }
        Ok(())
    }

    #[tracing::instrument(skip(self))]
    pub async fn is_alive(&self, task_id: i64) -> bool {
        let mut handles = self.handles.lock().await;
        if let Some(handle) = handles.get_mut(&task_id) {
            match handle.child.try_wait() {
                Ok(Some(_)) => {
                    handles.remove(&task_id);
                    false
                }
                Ok(None) => true,
                Err(_) => false,
            }
        } else {
            false
        }
    }

    pub async fn stale_agents(&self, threshold: std::time::Duration) -> Vec<i64> {
        let handles = self.handles.lock().await;
        let now = Instant::now();
        handles
            .values()
            .filter(|h| now.duration_since(h.last_output) > threshold)
            .map(|h| h.task_id)
            .collect()
    }

    pub async fn count(&self) -> usize {
        self.handles.lock().await.len()
    }

    #[tracing::instrument(skip(self))]
    pub async fn shutdown_all(&self, grace_period: std::time::Duration) {
        let task_ids: Vec<i64> = {
            let handles = self.handles.lock().await;
            handles.keys().cloned().collect()
        };
        for &id in &task_ids {
            let _ = self.kill(id).await;
        }
        tokio::time::sleep(grace_period).await;
        self.handles.lock().await.clear();
        tracing::info!("All PTY processes shut down");
    }
}

fn cost_saving_env_vars() -> &'static [(&'static str, &'static str)] {
    &[
        ("CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC", "1"),
        ("RTK_TELEMETRY_DISABLED", "1"),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_pty_manager_creation() {
        let mgr = PtyManager::new(4, sandbox::SandboxProfile::disabled());
        assert_eq!(mgr.max_agents, 4);
        assert_eq!(mgr.count().await, 0);
    }

    #[tokio::test]
    async fn test_pty_manager_subscribe() {
        let mgr = PtyManager::new(2, sandbox::SandboxProfile::disabled());
        let _rx = mgr.subscribe_output();
        // Subscribing should not panic
        assert_eq!(mgr.count().await, 0);
    }

    #[tokio::test]
    async fn test_pty_manager_stale_agents_empty() {
        let mgr = PtyManager::new(4, sandbox::SandboxProfile::disabled());
        let stale = mgr.stale_agents(std::time::Duration::from_secs(60)).await;
        assert!(stale.is_empty());
    }

    #[tokio::test]
    async fn test_pty_manager_kill_nonexistent() {
        let mgr = PtyManager::new(4, sandbox::SandboxProfile::disabled());
        // Killing a nonexistent task should not error
        let result = mgr.kill(999).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_pty_manager_write_to_nonexistent() {
        let mgr = PtyManager::new(4, sandbox::SandboxProfile::disabled());
        // Writing to a nonexistent task should not error
        let result = mgr.write_to(999, "hello").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_pty_manager_resize_nonexistent() {
        let mgr = PtyManager::new(4, sandbox::SandboxProfile::disabled());
        let result = mgr.resize(999, 120, 40).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_pty_manager_is_alive_nonexistent() {
        let mgr = PtyManager::new(4, sandbox::SandboxProfile::disabled());
        assert!(!mgr.is_alive(999).await);
    }

    #[tokio::test]
    async fn test_pty_manager_shutdown_all_empty() {
        let mgr = PtyManager::new(4, sandbox::SandboxProfile::disabled());
        mgr.shutdown_all(std::time::Duration::from_millis(10)).await;
        assert_eq!(mgr.count().await, 0);
    }

    #[tokio::test]
    async fn test_pty_manager_spawn_limit() {
        let mgr = PtyManager::new(0, sandbox::SandboxProfile::disabled());
        let result = mgr.spawn(1, "echo", &["hello".into()], "/tmp").await;
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Agent limit reached"));
    }

    #[test]
    fn test_pty_output_fields() {
        let output = PtyOutput {
            task_id: 42,
            data: vec![72, 101, 108, 108, 111], // "Hello"
        };
        assert_eq!(output.task_id, 42);
        assert_eq!(output.data, b"Hello");
    }

    #[test]
    fn test_pty_output_clone() {
        let output = PtyOutput {
            task_id: 10,
            data: vec![1, 2, 3],
        };
        let cloned = output.clone();
        assert_eq!(cloned.task_id, 10);
        assert_eq!(cloned.data, vec![1, 2, 3]);
    }

    #[test]
    fn test_pty_output_debug() {
        let output = PtyOutput {
            task_id: 7,
            data: vec![65, 66, 67], // "ABC"
        };
        let debug = format!("{:?}", output);
        assert!(debug.contains("7"));
        assert!(debug.contains("65"));
    }

    #[tokio::test]
    async fn test_pty_manager_multiple_subscribe() {
        let mgr = PtyManager::new(4, sandbox::SandboxProfile::disabled());
        let _rx1 = mgr.subscribe_output();
        let _rx2 = mgr.subscribe_output();
        // Multiple subscriptions should work without error
        assert_eq!(mgr.count().await, 0);
    }

    #[tokio::test]
    async fn test_pty_manager_sandbox_disabled_on_missing_nono() {
        // When sandbox is enabled but nono.sh not installed,
        // the manager should warn and disable sandbox
        let profile = sandbox::SandboxProfile {
            enabled: true,
            blocked_paths: vec![],
            block_network: false,
            extra_flags: vec![],
        };
        let mgr = PtyManager::new(4, profile);
        // If nono is not available, sandbox should be disabled
        if !sandbox::SandboxProfile::is_available() {
            assert!(!mgr.sandbox.enabled);
        }
    }

    // ── Telemetry injection TDD ────────────────────────────────────

    #[test]
    fn telemetry_disabled_by_default() {
        let mgr = PtyManager::new(4, sandbox::SandboxProfile::disabled());
        assert!(!mgr.disable_agent_telemetry);
    }

    #[test]
    fn with_disable_telemetry_enables_flag() {
        let mgr =
            PtyManager::new(4, sandbox::SandboxProfile::disabled()).with_disable_telemetry(true);
        assert!(mgr.disable_agent_telemetry);
    }

    #[test]
    fn with_disable_telemetry_false_keeps_default() {
        let mgr =
            PtyManager::new(4, sandbox::SandboxProfile::disabled()).with_disable_telemetry(false);
        assert!(!mgr.disable_agent_telemetry);
    }

    #[test]
    fn cost_saving_env_vars_contains_expected_keys() {
        let vars = cost_saving_env_vars();
        let keys: Vec<&str> = vars.iter().map(|(k, _)| *k).collect();
        assert!(keys.contains(&"CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC"));
        assert!(keys.contains(&"RTK_TELEMETRY_DISABLED"));
    }

    #[test]
    fn cost_saving_env_vars_all_set_to_one() {
        for (_, v) in cost_saving_env_vars() {
            assert_eq!(*v, "1");
        }
    }
}
