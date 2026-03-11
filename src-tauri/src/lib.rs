use std::sync::Mutex;
use tauri::Manager;

pub struct ServerProcess(pub Mutex<Option<tokio::process::Child>>);

#[tauri::command]
fn get_server_port() -> u16 {
    std::env::var("SHEPHERD_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(9876)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(ServerProcess(Mutex::new(None)))
        .invoke_handler(tauri::generate_handler![get_server_port])
        .setup(|app| {
            let app_handle = app.handle().clone();

            tauri::async_runtime::spawn(async move {
                let server_binary = if cfg!(debug_assertions) {
                    "shepherd-server".to_string()
                } else {
                    let resource_dir = app_handle
                        .path()
                        .resource_dir()
                        .expect("Failed to get resource dir");
                    resource_dir
                        .join("shepherd-server")
                        .to_string_lossy()
                        .to_string()
                };

                match tokio::process::Command::new(&server_binary)
                    .kill_on_drop(true)
                    .spawn()
                {
                    Ok(child) => {
                        let state = app_handle.state::<ServerProcess>();
                        *state.0.lock().unwrap() = Some(child);
                        println!("Shepherd server started");
                    }
                    Err(e) => {
                        eprintln!(
                            "Failed to start shepherd-server: {}. \
                             Make sure it is built and in PATH (dev) or bundled (prod).",
                            e
                        );
                    }
                }
            });

            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::Destroyed = event {
                let state = window.state::<ServerProcess>();
                let mut guard = state.0.lock().unwrap();
                if let Some(mut child) = guard.take() {
                    drop(guard);
                    tauri::async_runtime::spawn(async move {
                        let _ = child.kill().await;
                    });
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running shepherd desktop");
}
