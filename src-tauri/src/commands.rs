use tauri::AppHandle;

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
