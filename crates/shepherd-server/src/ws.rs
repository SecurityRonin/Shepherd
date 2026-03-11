use axum::extract::{
    ws::{Message, WebSocket, WebSocketUpgrade},
    State,
};
use axum::response::IntoResponse;
use futures::{SinkExt, StreamExt};
use shepherd_core::events::{ClientEvent, ServerEvent};
use std::sync::Arc;

use crate::state::AppState;

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: Arc<AppState>) {
    let (mut sender, mut receiver) = socket.split();
    let mut rx = state.event_tx.subscribe();

    let send_task = tokio::spawn(async move {
        while let Ok(event) = rx.recv().await {
            if let Ok(json) = serde_json::to_string(&event) {
                if sender.send(Message::Text(json)).await.is_err() {
                    break;
                }
            }
        }
    });

    let state_clone = state.clone();
    let recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            if let Message::Text(text) = msg {
                if let Ok(event) = serde_json::from_str::<ClientEvent>(&text) {
                    handle_client_event(event, &state_clone).await;
                }
            }
        }
    });

    tokio::select! {
        _ = send_task => {},
        _ = recv_task => {},
    }
}

async fn handle_client_event(event: ClientEvent, state: &AppState) {
    match event {
        ClientEvent::TaskCreate {
            title,
            agent_id,
            repo_path,
            isolation_mode,
            prompt,
        } => {
            let db = state.db.lock().await;
            let input = shepherd_core::db::models::CreateTask {
                title,
                prompt,
                agent_id,
                repo_path,
                isolation_mode,
            };
            if let Ok(task) = shepherd_core::db::queries::create_task(&db, &input) {
                let _ = state
                    .event_tx
                    .send(ServerEvent::TaskCreated(
                        shepherd_core::events::TaskEvent {
                            id: task.id,
                            title: task.title,
                            agent_id: task.agent_id,
                            status: task.status.as_str().to_string(),
                            branch: task.branch,
                            repo_path: task.repo_path,
                        },
                    ));
            }
        }
        ClientEvent::TaskApprove { task_id } => {
            tracing::info!("Approving task {task_id}");
            let db = state.db.lock().await;
            if let Ok(tasks) = shepherd_core::db::queries::list_tasks(&db) {
                if let Some(task) = tasks.iter().find(|t| t.id == task_id) {
                    let approve_str = state
                        .adapters
                        .get(&task.agent_id)
                        .map(|a| a.permissions.approve.clone())
                        .unwrap_or_else(|| "y\n".into());
                    drop(db);
                    let _ = state.pty.write_to(task_id, &approve_str).await;
                }
            }
        }
        ClientEvent::TaskApproveAll => {
            tracing::info!("Approving all pending");
            let db = state.db.lock().await;
            if let Ok(tasks) = shepherd_core::db::queries::list_tasks(&db) {
                let pending: Vec<_> = tasks
                    .iter()
                    .filter(|t| t.status.as_str() == "input")
                    .cloned()
                    .collect();
                drop(db);
                for task in pending {
                    let approve_str = state
                        .adapters
                        .get(&task.agent_id)
                        .map(|a| a.permissions.approve.clone())
                        .unwrap_or_else(|| "y\n".into());
                    let _ = state.pty.write_to(task.id, &approve_str).await;
                }
            }
        }
        ClientEvent::TaskCancel { task_id } => {
            tracing::info!("Cancelling task {task_id}");
            let _ = state.pty.kill(task_id).await;
        }
        ClientEvent::TerminalInput { task_id, data } => {
            let _ = state.pty.write_to(task_id, &data).await;
        }
        ClientEvent::TerminalResize {
            task_id,
            cols,
            rows,
        } => {
            let _ = state.pty.resize(task_id, cols, rows).await;
        }
        ClientEvent::Subscribe => {
            tracing::info!("Client subscribed");
        }
    }
}
