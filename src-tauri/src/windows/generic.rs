use tauri::{AppHandle, WebviewWindow};

use super::{window, QUICKBAR};

pub fn configure_quickbar(_window: &WebviewWindow) -> tauri::Result<()> {
    #[cfg(target_os = "windows")]
    {
        use window_vibrancy::{apply_acrylic, apply_mica};
        if apply_mica(_window, Some(true)).is_err() {
            let _ = apply_acrylic(_window, Some((18, 18, 20, 160)));
        }
    }
    Ok(())
}

/// The Workspace draws its own title bar, so the toolkit's comes off here
/// rather than in the config: macOS keeps a real title bar and only makes it
/// transparent, and there is no such half-measure to configure on these two.
pub fn configure_workspace(window: &WebviewWindow) -> tauri::Result<()> {
    window.set_decorations(false)
}

pub fn show_quickbar(app: &AppHandle) -> tauri::Result<()> {
    let quickbar = window(app, QUICKBAR)?;
    quickbar.show()?;
    quickbar.set_focus()
}

pub fn hide_quickbar(app: &AppHandle) -> tauri::Result<()> {
    window(app, QUICKBAR)?.hide()
}

pub fn quickbar_visible(app: &AppHandle) -> tauri::Result<bool> {
    window(app, QUICKBAR)?.is_visible()
}

pub fn before_workspace_shown(_app: &AppHandle) {}

pub fn after_workspace_hidden(_app: &AppHandle) {}
