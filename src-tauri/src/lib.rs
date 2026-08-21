mod commands;
mod platform;
mod state;
mod windows;

use tauri::{Manager, WindowEvent};

use state::AppState;

fn handle_argv(app: &tauri::AppHandle, argv: &[String]) {
    let _ = if argv.iter().any(|a| a == "--workspace") {
        windows::open_workspace(app)
    } else if argv.iter().any(|a| a == "--toggle") {
        windows::toggle_quickbar(app)
    } else {
        windows::show_quickbar(app)
    };
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    platform::prepare_graphics();

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            handle_argv(app, &argv);
        }))
        .plugin(tauri_plugin_opener::init())
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            commands::hide_quickbar,
            commands::toggle_quickbar,
            commands::open_workspace,
            commands::set_blur_hide_suppressed,
        ])
        .on_window_event(|window, event| match (window.label(), event) {
            #[cfg(not(target_os = "macos"))]
            (windows::QUICKBAR, WindowEvent::Focused(false)) => {
                windows::hide_quickbar_on_blur(window.app_handle());
            }
            (windows::WORKSPACE, WindowEvent::CloseRequested { api, .. }) => {
                api.prevent_close();
                let _ = windows::hide_workspace(window.app_handle());
            }
            _ => {}
        })
        .setup(|app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            windows::setup(app.handle())?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
