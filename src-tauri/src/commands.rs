use tauri::ipc::Channel;
use tauri::AppHandle;

use crate::engine::cli::{self, Runs};
use crate::engine::providers::{self, Provider};
use crate::engine::{RunRequest, StreamEvent};
use crate::state::AppState;
use crate::windows;

#[tauri::command]
pub fn hide_quickbar(app: AppHandle) -> Result<(), String> {
    windows::hide_quickbar(&app).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn toggle_quickbar(app: AppHandle) -> Result<(), String> {
    windows::toggle_quickbar(&app).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn open_workspace(app: AppHandle) -> Result<(), String> {
    windows::open_workspace(&app).map_err(|e| e.to_string())
}

/// Keeps the Quick Bar from hiding while a native dialog or drag has focus.
#[tauri::command]
pub fn set_blur_hide_suppressed(state: tauri::State<'_, AppState>, suppressed: bool) {
    state.set_blur_hide_suppressed(suppressed);
}

#[tauri::command]
pub fn list_providers() -> Vec<Provider> {
    providers::detect()
}

#[tauri::command]
pub async fn run_prompt(
    app: AppHandle,
    request: RunRequest,
    on_event: Channel<StreamEvent>,
) -> Result<(), String> {
    cli::run(app, request, on_event).await
}

#[tauri::command]
pub fn cancel_run(state: tauri::State<'_, Runs>, run_id: String) -> bool {
    state.cancel(&run_id)
}
