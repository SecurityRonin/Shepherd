//! Tray and dock badge integration for Shepherd Desktop.

use tauri::command;

/// Sets the macOS dock badge text (e.g., number of tasks needing input).
/// On non-macOS platforms this is a no-op.
#[command]
pub fn set_dock_badge(text: String) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        use cocoa::appkit::NSApp;
        use cocoa::base::nil;
        use cocoa::foundation::NSString;
        use objc::{msg_send, sel, sel_impl};

        unsafe {
            let app = NSApp();
            let dock_tile: cocoa::base::id = msg_send![app, dockTile];
            let badge_label = if text.is_empty() {
                nil
            } else {
                NSString::alloc(nil).init_str(&text)
            };
            let _: () = msg_send![dock_tile, setBadgeLabel: badge_label];
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = text;
    }
    Ok(())
}

/// Updates the system tray icon/tooltip to reflect overall task status.
/// Logs the current task counts. Full tray icon support requires AppHandle
/// which will be added when tray menu is implemented.
#[command]
pub fn update_tray_status(running: u32, input: u32, error: u32) -> Result<(), String> {
    tracing::debug!(
        running = running,
        input = input,
        error = error,
        "Tray status update"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_dock_badge_accepts_empty_string() {
        // Should not panic on any platform
        assert!(set_dock_badge("".to_string()).is_ok());
    }

    #[test]
    fn set_dock_badge_accepts_number() {
        assert!(set_dock_badge("3".to_string()).is_ok());
    }

    #[test]
    fn update_tray_status_accepts_zeros() {
        assert!(update_tray_status(0, 0, 0).is_ok());
    }

    #[test]
    fn update_tray_status_accepts_counts() {
        assert!(update_tray_status(5, 2, 1).is_ok());
    }
}
