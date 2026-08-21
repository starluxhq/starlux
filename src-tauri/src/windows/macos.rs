use tauri::{ActivationPolicy, AppHandle, Manager, WebviewWindow};
use tauri_nspanel::{
    tauri_panel, CollectionBehavior, ManagerExt, PanelLevel, StyleMask, WebviewWindowExt,
};
use window_vibrancy::{apply_vibrancy, NSVisualEffectMaterial};

use super::QUICKBAR;

// `panel_event!` requires an explicit return type, including `-> ()`.
#[allow(clippy::unused_unit)]
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
    let _ = apply_vibrancy(window, NSVisualEffectMaterial::HudWindow, None, Some(12.0));

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
