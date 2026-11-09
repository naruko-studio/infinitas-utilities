//! Tauri commands. Errors are returned as strings for the UI to show.

use std::time::Duration;

use iu_core::label::Label;
use iu_core::songs::SongList;
use iu_core::status::Status;
use iu_core::{Config, Language};
use tauri::State;

use crate::AppState;
use crate::annotator::{Annotator, Loaded, Shot};
use crate::engine::Command;
use crate::web::ServerInfo;

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

#[tauri::command]
pub async fn get_server_info(state: State<'_, AppState>) -> Result<ServerInfo, String> {
    Ok(state.web.info().await)
}

/// Opens (or closes) the web server to the LAN for phones, then restarts it.
#[tauri::command]
pub async fn set_lan_access(
    state: State<'_, AppState>,
    enabled: bool,
) -> Result<ServerInfo, String> {
    let server = {
        let mut config = state.config.lock().expect("config lock poisoned");
        config.server.lan_access = enabled;
        config.save(&state.config_path).map_err(|e| {
            tracing::error!("{e}");
            e.to_string()
        })?;
        config.server.clone()
    };
    state.web.apply(server).await;
    Ok(state.web.info().await)
}

// ---- annotator (developer screen) ------------------------------------------

fn annotator<'a>(state: &'a State<'_, AppState>) -> Result<&'a Annotator, String> {
    state
        .annotator
        .as_ref()
        .ok_or_else(|| "the Pictures folder was not found".to_owned())
}

#[tauri::command]
pub async fn annotator_list(state: State<'_, AppState>) -> Result<Vec<Shot>, String> {
    Ok(annotator(&state)?.list())
}

/// PNG bytes, sent as raw binary rather than JSON.
#[tauri::command]
pub async fn annotator_image(
    state: State<'_, AppState>,
    path: String,
) -> Result<tauri::ipc::Response, String> {
    Ok(tauri::ipc::Response::new(annotator(&state)?.image(&path)?))
}

#[tauri::command]
pub async fn annotator_load(state: State<'_, AppState>, path: String) -> Result<Loaded, String> {
    annotator(&state)?.load(&path)
}

#[tauri::command]
pub fn annotator_check(label: Label) -> Vec<String> {
    crate::annotator::check(&label)
}

#[tauri::command]
pub async fn annotator_save(
    state: State<'_, AppState>,
    path: String,
    label: Label,
) -> Result<String, String> {
    annotator(&state)?.save(&path, &label)
}

#[tauri::command]
pub async fn annotator_discard(state: State<'_, AppState>, path: String) -> Result<(), String> {
    annotator(&state)?.discard(&path)
}

/// The bundled song list, for picking the song of a result/select label.
#[tauri::command]
pub fn song_list() -> Result<SongList, String> {
    SongList::from_json(include_str!("../../../resources/songs.json")).map_err(|e| e.to_string())
}
