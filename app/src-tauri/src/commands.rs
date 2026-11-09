//! Tauri commands. Errors are returned as strings for the UI to show.

use std::time::Duration;

use iu_core::{Config, Language};
use tauri::State;

use crate::AppState;
use crate::engine::{Command, Status};

#[tauri::command]
pub fn get_config(state: State<'_, AppState>) -> Config {
    state.config.lock().expect("config lock poisoned").clone()
}

#[tauri::command]
pub fn set_language(state: State<'_, AppState>, language: Language) -> Result<(), String> {
    update_config(&state, |c| c.language = language)
}

#[tauri::command]
pub fn get_status(state: State<'_, AppState>) -> Status {
    state.status.lock().expect("status lock poisoned").clone()
}

#[tauri::command]
pub fn set_dev_capture(state: State<'_, AppState>, enabled: bool) -> Result<(), String> {
    update_config(&state, |c| c.capture.dev_capture = enabled)
}

/// Saves the latest frame to the dev-capture folder; returns the file path.
#[tauri::command]
pub async fn capture_frame(state: State<'_, AppState>) -> Result<String, String> {
    let (tx, rx) = std::sync::mpsc::channel();
    send(&state, Command::CaptureFrame(tx))?;
    let path = rx
        .recv_timeout(Duration::from_secs(5))
        .map_err(|_| "the capture engine did not answer".to_owned())??;
    Ok(path.display().to_string())
}

/// Applies `change`, saves the config and passes capture settings to the engine.
fn update_config(state: &AppState, change: impl FnOnce(&mut Config)) -> Result<(), String> {
    let mut config = state.config.lock().expect("config lock poisoned");
    change(&mut config);
    config.save(&state.config_path).map_err(|e| {
        tracing::error!("{e}");
        e.to_string()
    })?;
    send(state, Command::SetConfig(config.capture.clone()))
}

fn send(state: &AppState, cmd: Command) -> Result<(), String> {
    state
        .engine
        .lock()
        .expect("engine lock poisoned")
        .send(cmd)
        .map_err(|_| "the capture engine has stopped".to_owned())
}
