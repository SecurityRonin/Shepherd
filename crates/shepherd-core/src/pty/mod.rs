pub mod status;

use anyhow::{Context, Result};
use portable_pty::{native_pty_system, CommandBuilder, PtySize};
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
}

impl PtyManager {
    pub fn new(max_agents: usize) -> Self {
        let (output_tx, _) = broadcast::channel(1024);
        Self {
            handles: Arc::new(Mutex::new(HashMap::new())),
            output_tx,
            max_agents,
        }
    }

    pub fn subscribe_output(&self) -> broadcast::Receiver<PtyOutput> {
        self.output_tx.subscribe()
    }

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

        let mut cmd = CommandBuilder::new(command);
        cmd.args(args);
        cmd.cwd(cwd);

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

    pub async fn write_to(&self, task_id: i64, data: &str) -> Result<()> {
        let mut handles = self.handles.lock().await;
        if let Some(handle) = handles.get_mut(&task_id) {
            handle.writer.write_all(data.as_bytes())?;
        }
        Ok(())
    }

    pub async fn kill(&self, task_id: i64) -> Result<()> {
        let mut handles = self.handles.lock().await;
        if let Some(mut handle) = handles.remove(&task_id) {
            handle.child.kill()?;
            tracing::info!("Killed PTY for task {task_id}");
        }
        Ok(())
    }

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
