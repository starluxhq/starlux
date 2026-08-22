mod artifacts;
mod commands;
mod db;
mod engine;
mod platform;
mod shell_env;
mod shortcut;
mod state;
mod tray;
mod windows;

use tauri::{Emitter, Manager, WindowEvent};

use state::AppState;

fn handle_argv(app: &tauri::AppHandle, argv: &[String]) {
    if let Some(prompt) = argv
        .iter()
        .position(|arg| arg == "--ask")
        .and_then(|at| argv.get(at + 1))
    {
        let _ = windows::show_quickbar(app);
        let _ = app.emit_to(windows::QUICKBAR, "starlux://ask", prompt.clone());
        return;
    }

    let _ = if argv.iter().any(|arg| arg == "--workspace") {
        windows::open_workspace(app)
    } else if argv.iter().any(|arg| arg == "--toggle") {
        windows::toggle_quickbar(app)
    } else {
        windows::show_quickbar(app)
    };
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Before anything resolves a binary: a packaged build has to widen its
    // PATH before it can find one.
    shell_env::import();
    platform::prepare_graphics();

    #[allow(unused_mut)]
    let mut builder = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            handle_argv(app, &argv);
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build());

    // Registers WebviewPanelManager; without it `to_panel()` panics at startup.
    #[cfg(target_os = "macos")]
    {
        builder = builder.plugin(tauri_nspanel::init());
    }

    builder
        .manage(AppState::default())
        .manage(artifacts::Artifacts::default())
        .manage(engine::cli::Runs::default())
        .register_uri_scheme_protocol("artifact", |ctx, request| {
            artifacts::response(
                ctx.app_handle().state::<artifacts::Artifacts>().inner(),
                request.uri().path(),
            )
        })
        .invoke_handler(tauri::generate_handler![
            commands::hide_quickbar,
            commands::toggle_quickbar,
            commands::set_quickbar_height,
            commands::open_workspace,
            commands::set_blur_hide_suppressed,
            commands::list_providers,
            commands::selected_model,
            commands::set_selected_model,
            commands::store_artifact,
            commands::active_conversation,
            commands::list_conversations,
            commands::load_conversation,
            commands::rename_conversation,
            commands::delete_conversation,
            commands::set_agent_dir,
            commands::run_prompt,
            commands::cancel_run,
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

            let data_dir = app.path().app_data_dir()?;
            let db = db::Db::open(&data_dir.join("starlux.db"))?;
            if let Ok(Some(id)) = db.setting(db::ACTIVE_CONVERSATION) {
                app.state::<AppState>().set_active_conversation(id);
            }
            app.manage(db);

            windows::setup(app.handle())?;
            tray::setup(app.handle())?;
            shortcut::register(app.handle());

            // Shown here rather than via tauri.conf `visible`, so the first
            // show goes through the same path as every later one. On macOS
            // that is what makes the panel key and able to take keystrokes.
            let _ = windows::show_quickbar(app.handle());

            let argv: Vec<String> = std::env::args().collect();
            if argv.iter().any(|arg| arg == "--ask") {
                let handle = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    tokio::time::sleep(std::time::Duration::from_millis(400)).await;
                    handle_argv(&handle, &argv);
                });
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
