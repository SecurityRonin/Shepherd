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

/// Formats the tray tooltip string from task counts.
fn format_tray_tooltip(running: u32, input: u32, error: u32) -> String {
    if input > 0 {
        format!("Shepherd — {} task(s) need input", input)
    } else if error > 0 {
        format!("Shepherd — {} task(s) errored", error)
    } else if running > 0 {
        format!("Shepherd — {} task(s) running", running)
    } else {
        "Shepherd — idle".to_string()
    }
}

/// Updates the system tray tooltip to reflect overall task status.
#[command]
pub fn update_tray_status(app: tauri::AppHandle, running: u32, input: u32, error: u32) -> Result<(), String> {
    let tooltip = format_tray_tooltip(running, input, error);

    // Update tray tooltip if tray exists
    if let Some(tray) = app.tray_by_id("main") {
        let _ = tray.set_tooltip(Some(&tooltip));
    }

    tracing::debug!(
        running = running,
        input = input,
        error = error,
        tooltip = %tooltip,
        "Tray status update"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tooltip_idle_when_all_zeros() {
        assert_eq!(format_tray_tooltip(0, 0, 0), "Shepherd — idle");
    }

    #[test]
    fn tooltip_shows_input_count() {
        assert_eq!(format_tray_tooltip(3, 2, 0), "Shepherd — 2 task(s) need input");
    }

    #[test]
    fn tooltip_shows_error_when_no_input() {
        assert_eq!(format_tray_tooltip(1, 0, 3), "Shepherd — 3 task(s) errored");
    }

    #[test]
    fn tooltip_shows_running_when_no_input_or_error() {
        assert_eq!(format_tray_tooltip(5, 0, 0), "Shepherd — 5 task(s) running");
    }

    #[test]
    fn tooltip_input_takes_priority_over_error() {
        assert_eq!(format_tray_tooltip(1, 1, 1), "Shepherd — 1 task(s) need input");
    }

    #[test]
    fn set_dock_badge_accepts_empty_string() {
        assert!(set_dock_badge("".to_string()).is_ok());
    }

    #[test]
    fn set_dock_badge_accepts_number() {
        assert!(set_dock_badge("3".to_string()).is_ok());
    }
}
