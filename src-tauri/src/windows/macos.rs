// `panel_event!` mandates an explicit `-> ()`, and attributes on macro
// invocations are ignored, so this allow has to be file-scoped.
#![allow(clippy::unused_unit)]

use tauri::{ActivationPolicy, AppHandle, Manager, WebviewWindow};
use tauri_nspanel::{
    tauri_panel, CollectionBehavior, ManagerExt, PanelLevel, StyleMask, WebviewWindowExt,
};
use window_vibrancy::{apply_vibrancy, NSVisualEffectMaterial, NSVisualEffectState};

use super::QUICKBAR;

tauri_panel! {
    panel!(QuickBarPanel {
        config: {
            can_become_key_window: true,
            is_floating_panel: true
        }
    })

    panel_event!(QuickBarPanelEvents {
        window_did_resign_key(notification: &NSNotification) -> ()
    })
}

pub fn configure_quickbar(window: &WebviewWindow) -> tauri::Result<()> {
    // `Active` rather than the default: NSVisualEffectView otherwise follows the
    // window's active state and turns opaque whenever the panel is not key.
    let _ = apply_vibrancy(
        window,
        NSVisualEffectMaterial::HudWindow,
        Some(NSVisualEffectState::Active),
        Some(12.0),
    );

    let panel = window.to_panel::<QuickBarPanel>()?;
    panel.set_level(PanelLevel::Floating.value());
    panel.set_style_mask(StyleMask::empty().nonactivating_panel().into());
    panel.set_collection_behavior(
        CollectionBehavior::new()
            .full_screen_auxiliary()
            .can_join_all_spaces()
            .stationary()
            .into(),
    );
    // NSPanel defaults to hiding whenever the app deactivates, which makes the
    // bar vanish as soon as you click another app.
    panel.set_hides_on_deactivate(false);

    let handler = QuickBarPanelEvents::new();
    let app = window.app_handle().clone();
    handler.window_did_resign_key(move |_| {
        super::hide_quickbar_on_blur(&app);
    });
    panel.set_event_handler(Some(handler.as_ref()));
    std::mem::forget(handler);

    Ok(())
}

/// Already done by `titleBarStyle: "Overlay"` in the config, which leaves the
/// traffic lights floating over the page and takes the rest of the bar away.
/// Dropping the decorations outright would take the lights with them.
pub fn configure_workspace(_window: &WebviewWindow) -> tauri::Result<()> {
    Ok(())
}

pub fn show_quickbar(app: &AppHandle) -> tauri::Result<()> {
    app.get_webview_panel(QUICKBAR)
        .map(|panel| panel.show_and_make_key())
        .map_err(|_| tauri::Error::WindowNotFound)
}

pub fn hide_quickbar(app: &AppHandle) -> tauri::Result<()> {
    app.get_webview_panel(QUICKBAR)
        .map(|panel| panel.hide())
        .map_err(|_| tauri::Error::WindowNotFound)
}

pub fn quickbar_visible(app: &AppHandle) -> tauri::Result<bool> {
    app.get_webview_panel(QUICKBAR)
        .map(|panel| panel.is_visible())
        .map_err(|_| tauri::Error::WindowNotFound)
}

pub fn before_workspace_shown(app: &AppHandle) {
    let _ = app.set_activation_policy(ActivationPolicy::Regular);
}

pub fn after_workspace_hidden(app: &AppHandle) {
    let _ = app.set_activation_policy(ActivationPolicy::Accessory);
}
