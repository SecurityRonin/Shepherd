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
        // Placeholder: cocoa/objc bindings will set NSApp.dockTile.badgeLabel.
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
    // Icon/tooltip selection:
    // - input > 0  -> attention icon, "N tasks need input"
    // - error > 0  -> error icon,     "N tasks errored"
    // - running > 0 -> active icon,   "N tasks running"
    // - otherwise   -> idle icon,     "Shepherd — idle"
    let _ = (running, input, error);
    Ok(())
}
