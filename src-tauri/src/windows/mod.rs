#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
use macos as imp;

#[cfg(not(target_os = "macos"))]
mod generic;
#[cfg(not(target_os = "macos"))]
use generic as imp;

use std::sync::OnceLock;

use tauri::{AppHandle, Emitter, Manager, WebviewWindow};

use crate::state::AppState;

pub const QUICKBAR: &str = "quickbar";
pub const WORKSPACE: &str = "workspace";

/// Tells the Workspace which conversation to show when it comes up.
const FOCUS_EVENT: &str = "starlux://focus";

const BAR_WIDTH: f64 = 680.0;
/// The bar asks for its own height, so a row of attachments or an open model
/// list grows the window instead of being clipped by it. The extra room the
/// list needs is transparent, which is what lets it read as floating outside
/// the bar. Bounded here rather than trusted, and only ever a few sizes on
/// discrete events — a window that resized per token is what crashes WebKitGTK.
const BAR_MIN_HEIGHT: f64 = 44.0;
const BAR_MAX_HEIGHT: f64 = 720.0;

fn window(app: &AppHandle, label: &str) -> tauri::Result<WebviewWindow> {
    app.get_webview_window(label)
        .ok_or_else(|| tauri::Error::WindowNotFound)
}

pub fn setup(app: &AppHandle) -> tauri::Result<()> {
    let quickbar = window(app, QUICKBAR)?;
    // The bar drives its own height, so it has to be allowed to be short. A
    // non-resizable window is pinned to its start size by the toolkit, which is
    // why the width is bounded here instead.
    quickbar.set_min_size(Some(tauri::LogicalSize::new(BAR_WIDTH, BAR_MIN_HEIGHT)))?;
    quickbar.set_max_size(Some(tauri::LogicalSize::new(BAR_WIDTH, BAR_MAX_HEIGHT)))?;
    imp::configure_quickbar(&quickbar)?;
    imp::configure_workspace(&window(app, WORKSPACE)?)
}

pub fn show_quickbar(app: &AppHandle) -> tauri::Result<()> {
    imp::show_quickbar(app)
}

pub fn hide_quickbar(app: &AppHandle) -> tauri::Result<()> {
    imp::hide_quickbar(app)
}

pub fn set_quickbar_height(app: &AppHandle, height: f64) -> tauri::Result<()> {
    let height = height.clamp(BAR_MIN_HEIGHT, BAR_MAX_HEIGHT);
    window(app, QUICKBAR)?.set_size(tauri::LogicalSize::new(BAR_WIDTH, height))
}

pub fn toggle_quickbar(app: &AppHandle) -> tauri::Result<()> {
    if imp::quickbar_visible(app)? {
        hide_quickbar(app)
    } else {
        show_quickbar(app)
    }
}

/// Set `STARLUX_NO_BLUR_HIDE=1` to keep the bar up when it loses focus, which
/// is what makes it possible to screenshot or inspect.
fn blur_hide_disabled() -> bool {
    static DISABLED: OnceLock<bool> = OnceLock::new();
    *DISABLED.get_or_init(|| std::env::var_os("STARLUX_NO_BLUR_HIDE").is_some())
}

pub fn hide_quickbar_on_blur(app: &AppHandle) {
    if blur_hide_disabled() {
        return;
    }
    if !app.state::<AppState>().blur_hide_suppressed() {
        let _ = hide_quickbar(app);
    }
}

pub fn open_workspace(app: &AppHandle) -> tauri::Result<()> {
    let workspace = window(app, WORKSPACE)?;
    let _ = hide_quickbar(app);
    imp::before_workspace_shown(app);
    workspace.show()?;
    workspace.set_focus()?;
    app.emit_to(
        WORKSPACE,
        FOCUS_EVENT,
        app.state::<AppState>().active_conversation(),
    )
}

pub fn hide_workspace(app: &AppHandle) -> tauri::Result<()> {
    window(app, WORKSPACE)?.hide()?;
    imp::after_workspace_hidden(app);
    Ok(())
}
