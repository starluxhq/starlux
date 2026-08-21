#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
use macos as imp;

#[cfg(not(target_os = "macos"))]
mod generic;
#[cfg(not(target_os = "macos"))]
use generic as imp;

use tauri::{AppHandle, Manager, WebviewWindow};

use crate::state::AppState;

pub const QUICKBAR: &str = "quickbar";
pub const WORKSPACE: &str = "workspace";

fn window(app: &AppHandle, label: &str) -> tauri::Result<WebviewWindow> {
    app.get_webview_window(label)
        .ok_or_else(|| tauri::Error::WindowNotFound)
}

pub fn setup(app: &AppHandle) -> tauri::Result<()> {
    imp::configure_quickbar(&window(app, QUICKBAR)?)
}

pub fn show_quickbar(app: &AppHandle) -> tauri::Result<()> {
    imp::show_quickbar(app)
}

pub fn hide_quickbar(app: &AppHandle) -> tauri::Result<()> {
    imp::hide_quickbar(app)
}

pub fn toggle_quickbar(app: &AppHandle) -> tauri::Result<()> {
    if imp::quickbar_visible(app)? {
        hide_quickbar(app)
    } else {
        show_quickbar(app)
    }
}

pub fn hide_quickbar_on_blur(app: &AppHandle) {
    if !app.state::<AppState>().blur_hide_suppressed() {
        let _ = hide_quickbar(app);
    }
}

pub fn open_workspace(app: &AppHandle) -> tauri::Result<()> {
    let workspace = window(app, WORKSPACE)?;
    let _ = hide_quickbar(app);
    imp::before_workspace_shown(app);
    workspace.show()?;
    workspace.set_focus()
}

pub fn hide_workspace(app: &AppHandle) -> tauri::Result<()> {
    window(app, WORKSPACE)?.hide()?;
    imp::after_workspace_hidden(app);
    Ok(())
}
