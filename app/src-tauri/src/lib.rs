mod annotator;
mod commands;
mod engine;
mod logging;
mod ocr;
mod songs;
mod web;

use std::path::PathBuf;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};

use iu_core::Config;
use iu_core::config::CONFIG_FILE_NAME;
use iu_core::event::{Bus, Event};
use iu_core::status::{SharedStatus, Status};
use iu_db::DATABASE_FILE_NAME;
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::broadcast::error::RecvError;

/// Tauri events the UI listens to (payloads as in [`iu_core::event`]).
pub const STATUS_EVENT: &str = "engine://status";
pub const PLAY_EVENT: &str = "engine://play";
pub const RESULT_EVENT: &str = "engine://result";
pub const RESULT_UPDATED_EVENT: &str = "engine://result-updated";

/// App-wide state shared with the commands.
pub struct AppState {
    pub config: Mutex<Config>,
    pub config_path: PathBuf,
    pub status: SharedStatus,
    pub engine: Mutex<Sender<engine::Command>>,
    pub web: web::Web,
    pub annotator: Option<annotator::Annotator>,
    /// `data.db`, for read-only queries from commands.
    pub db_path: PathBuf,
    /// Keeps the background log writer alive; dropping it flushes the log.
    _log_guard: Option<tracing_appender::non_blocking::WorkerGuard>,
}

pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let paths = app.path();
            let config_path = paths.app_config_dir()?.join(CONFIG_FILE_NAME);
            let data_dir = paths.app_data_dir()?;
            let log_dir = paths.app_log_dir()?;

            // Logging needs the configured level, so read the config first;
            // what happened while loading is logged right after init.
            let config = Config::load(&config_path);
            let level = config.as_ref().map_or("info", |c| c.log_level.as_str());
            let log_guard = logging::init(&log_dir, level);
            let config = match config {
                Ok(config) => config,
                Err(_) => Config::load_or_recover(&config_path),
            };

            tracing::info!(
                version = env!("CARGO_PKG_VERSION"),
                config = %config_path.display(),
                data = %data_dir.display(),
                logs = %log_dir.display(),
                "infinitas-utilities started"
            );

            let bus = iu_core::event::bus();
            let status: SharedStatus = Arc::new(Mutex::new(Status::default()));
            forward_to_ui(app.handle().clone(), &bus);

            let engine = engine::spawn(
                bus.clone(),
                config.capture.clone(),
                config.day_start_jst,
                data_dir.clone(),
                status.clone(),
            );
            let web = web::Web::new(iu_server::Context {
                bus,
                status: status.clone(),
                db_path: data_dir.join(DATABASE_FILE_NAME),
                day_start_jst: config.day_start_jst,
                songs: songs::song_list(),
                // Naming from the phone is a development aid: debug builds
                // (`pnpm tauri dev`) only.
                name_result: cfg!(debug_assertions).then(|| engine::name_result_fn(engine.clone())),
            });
            let server_config = config.server.clone();
            let config_capture_game_dir = config.capture.game_screenshot_dir.clone();

            app.manage(AppState {
                config: Mutex::new(config),
                config_path,
                status,
                engine: Mutex::new(engine),
                web,
                annotator: annotator::Annotator::new(config_capture_game_dir),
                db_path: data_dir.join(DATABASE_FILE_NAME),
                _log_guard: log_guard,
            });
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                handle.state::<AppState>().web.apply(server_config).await;
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_config,
            commands::set_language,
            commands::get_status,
            commands::set_dev_capture,
            commands::capture_frame,
            commands::get_server_info,
            commands::set_lan_access,
            commands::annotator_list,
            commands::annotator_image,
            commands::annotator_load,
            commands::annotator_check,
            commands::annotator_save,
            commands::annotator_discard,
            commands::song_list,
            commands::unnamed_results,
            commands::name_result,
        ])
        .run(tauri::generate_context!())
        .expect("error while running infinitas-utilities");
}

/// The app window is one more bus listener.
fn forward_to_ui(app: AppHandle, bus: &Bus) {
    let mut events = bus.subscribe();
    std::thread::Builder::new()
        .name("ui-events".into())
        .spawn(move || {
            loop {
                let result = match events.blocking_recv() {
                    Ok(Event::Status(s)) => app.emit(STATUS_EVENT, s),
                    Ok(Event::PlayFinished(p)) => app.emit(PLAY_EVENT, p),
                    Ok(Event::ResultRecorded(r)) => app.emit(RESULT_EVENT, r),
                    Ok(Event::ResultUpdated(r)) => app.emit(RESULT_UPDATED_EVENT, r),
                    Err(RecvError::Lagged(_)) => continue,
                    Err(RecvError::Closed) => return,
                };
                if let Err(e) = result {
                    tracing::warn!("cannot emit to the UI: {e}");
                }
            }
        })
        .expect("spawn ui-events thread");
}
