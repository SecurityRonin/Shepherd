use tauri::{Emitter, Manager};
use tauri::tray::TrayIconBuilder;
use tauri_plugin_deep_link::DeepLinkExt;

mod tray;

/// Stores the embedded server port for the frontend to discover.
struct ServerPort(u16);

/// Shared CloudClient so deep-link handler and commands can reuse it.
struct SharedCloudClient(shepherd_core::cloud::CloudClient);

#[tauri::command]
fn get_server_port(state: tauri::State<'_, ServerPort>) -> u16 {
    state.0
}

/// Fallback command for manual auth callback handling (when deep links don't work).
/// The frontend can invoke this with the full callback URL.
#[tauri::command]
async fn handle_auth_callback_cmd(
    url: String,
    client_state: tauri::State<'_, SharedCloudClient>,
) -> Result<String, String> {
    let profile = shepherd_core::cloud::auth::handle_auth_callback(&url, &client_state.0)
        .await
        .map_err(|e| format!("{e}"))?;
    serde_json::to_string(&profile).map_err(|e| format!("Failed to serialize profile: {e}"))
}

/// Process a deep-link auth callback URL: parse tokens, store JWT, fetch profile.
/// Emits `auth-callback-success` or `auth-callback-error` events to the frontend.
fn process_auth_callback(app_handle: &tauri::AppHandle, url: &str) {
    let handle = app_handle.clone();
    let url = url.to_string();
    tauri::async_runtime::spawn(async move {
        let client = handle.state::<SharedCloudClient>();
        match shepherd_core::cloud::auth::handle_auth_callback(&url, &client.0).await {
            Ok(profile) => {
                let _ = handle.emit("auth-callback-success", &profile);
            }
            Err(e) => {
                let msg = format!("{e}");
                let _ = handle.emit("auth-callback-error", &msg);
            }
        }
    });
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_notification::init())
        .invoke_handler(tauri::generate_handler![
            get_server_port,
            handle_auth_callback_cmd,
            tray::set_dock_badge,
            tray::update_tray_status
        ])
        .setup(|app| {
            // Load config from ~/.shepherd/config.toml (or defaults)
            let config = shepherd_core::config::load_config(None)
                .unwrap_or_default();
            let port = config.port;

            // Manage the port so frontend can discover it via get_server_port command
            app.manage(ServerPort(port));

            // Create a shared CloudClient for auth operations
            app.manage(SharedCloudClient(
                shepherd_core::cloud::CloudClient::new(),
            ));

            // Register deep links at runtime for dev mode on Linux/Windows
            #[cfg(any(target_os = "linux", all(debug_assertions, windows)))]
            app.deep_link().register_all()?;

            // Listen for deep link open events (shepherd://auth/callback?...)
            let handle = app.handle().clone();
            app.deep_link().on_open_url(move |event| {
                for url in event.urls() {
                    let url_str = url.as_str();
                    if url_str.starts_with("shepherd://auth/callback") {
                        process_auth_callback(&handle, url_str);
                    } else {
                        tracing::warn!("Unhandled deep link URL: {}", url_str);
                    }
                }
            });

            // System tray icon with tooltip
            TrayIconBuilder::with_id("main")
                .icon(app.default_window_icon().cloned().unwrap_or_else(|| {
                    tauri::image::Image::new(&[], 0, 0) // fallback empty icon
                }))
                .tooltip("Shepherd — idle")
                .build(app)?;

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
