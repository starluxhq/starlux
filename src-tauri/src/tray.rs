use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::AppHandle;

use crate::windows;

/// Closing the Workspace hides it and the bar has no chrome at all, so without
/// the tray there is nothing on screen to bring Starlux back — and on macOS
/// there is no Dock icon either.
pub fn setup(app: &AppHandle) -> tauri::Result<()> {
    let bar = MenuItem::with_id(app, "quickbar", "Quick Bar", true, None::<&str>)?;
    let workspace = MenuItem::with_id(app, "workspace", "Workspace", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit Starlux", true, None::<&str>)?;
    let menu = Menu::with_items(
        app,
        &[
            &bar,
            &workspace,
            &PredefinedMenuItem::separator(app)?,
            &quit,
        ],
    )?;

    let mut tray = TrayIconBuilder::with_id("starlux")
        .tooltip("Starlux")
        .menu(&menu)
        // Left click toggles the bar; the menu stays on right click. Linux tray
        // icons deliver no click events at all, so there the menu is both.
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "quickbar" => {
                let _ = windows::toggle_quickbar(app);
            }
            "workspace" => {
                let _ = windows::open_workspace(app);
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let _ = windows::toggle_quickbar(tray.app_handle());
            }
        });

    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }

    tray.build(app)?;
    Ok(())
}
