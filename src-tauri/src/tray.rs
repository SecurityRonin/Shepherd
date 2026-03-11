// Tray and dock badge integration for Shepherd Desktop.
// These commands will be registered in lib.rs once the full Tauri
// build environment is available (Plan 3).

use tauri::command;

/// Sets the macOS dock badge text (e.g., number of tasks needing input).
/// On non-macOS platforms this is a no-op.
#[command]
pub fn set_dock_badge(text: String) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        // TODO (Plan 3): Use cocoa/objc bindings to set NSApp.dockTile.badgeLabel
        let _ = text;
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = text;
    }
    Ok(())
}

/// Updates the system tray icon/tooltip to reflect overall task status.
#[command]
pub fn update_tray_status(running: u32, input: u32, error: u32) -> Result<(), String> {
    // TODO (Plan 3): Update the tray icon and tooltip based on task counts.
    // - If `input > 0`: use attention icon, tooltip "N tasks need input"
    // - If `error > 0`: use error icon, tooltip "N tasks errored"
    // - If `running > 0`: use active icon, tooltip "N tasks running"
    // - Otherwise: use idle icon, tooltip "Shepherd — idle"
    let _ = (running, input, error);
    Ok(())
}
