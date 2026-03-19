use tauri::Manager;

// Plan 3 integration: tray.rs contains set_dock_badge and update_tray_status commands.
// Once tauri-plugin-notification is added to Cargo.toml, register them here:
//   .invoke_handler(tauri::generate_handler![get_server_port, tray::set_dock_badge, tray::update_tray_status])
#[allow(unused)]
mod tray;

/// Stores the embedded server port for the frontend to discover.
struct ServerPort(u16);

#[tauri::command]
fn get_server_port(state: tauri::State<'_, ServerPort>) -> u16 {
    state.0
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![get_server_port])
        .setup(|app| {
            // Load config from ~/.shepherd/config.toml (or defaults)
            let config = shepherd_core::config::load_config(None)
                .unwrap_or_default();
            let port = config.port;

            // Manage the port so frontend can discover it via get_server_port command
            app.manage(ServerPort(port));

            // Start embedded Axum server on a background Tokio task
            tauri::async_runtime::spawn(async move {
                match shepherd_server::startup::start_server(config).await {
                    Ok((addr, _state, handle)) => {
                        tracing::info!("Embedded server started on {}", addr);
                        // Await the server handle so the task stays alive
                        let _ = handle.await;
                    }
                    Err(e) => {
                        eprintln!("Failed to start embedded server: {}", e);
                    }
                }
            });

            Ok(())
        })
        .on_window_event(|_window, event| {
            if let tauri::WindowEvent::Destroyed = event {
                // Clean up server.json lockfile on exit
                shepherd_server::startup::ServerInfo::remove();
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running shepherd desktop");
}
