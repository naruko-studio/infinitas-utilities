//! Tauri commands. Errors are returned as strings for the UI to show.

use iu_core::{Config, Language};
use tauri::State;

use crate::AppState;

#[tauri::command]
pub fn get_config(state: State<'_, AppState>) -> Config {
    state.config.lock().expect("config lock poisoned").clone()
}

#[tauri::command]
pub fn set_language(state: State<'_, AppState>, language: Language) -> Result<(), String> {
    let mut config = state.config.lock().expect("config lock poisoned");
    config.language = language;
    config.save(&state.config_path).map_err(|e| {
        tracing::error!("{e}");
        e.to_string()
    })
}
