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
