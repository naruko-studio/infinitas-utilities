mod commands;
mod engine;
mod logging;

use std::path::PathBuf;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};

use iu_core::Config;
use iu_core::config::CONFIG_FILE_NAME;
use tauri::Manager;

/// App-wide state shared with the commands.
pub struct AppState {
    pub config: Mutex<Config>,
    pub config_path: PathBuf,
    pub status: engine::SharedStatus,
    pub engine: Mutex<Sender<engine::Command>>,
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

            let status = Arc::new(Mutex::new(engine::Status::default()));
            let engine = engine::spawn(
                app.handle().clone(),
                config.capture.clone(),
                config.day_start_jst,
                data_dir,
                status.clone(),
            );

            app.manage(AppState {
                config: Mutex::new(config),
                config_path,
                status,
                engine: Mutex::new(engine),
                _log_guard: log_guard,
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_config,
            commands::set_language,
            commands::get_status,
            commands::set_dev_capture,
            commands::capture_frame,
        ])
        .run(tauri::generate_context!())
        .expect("error while running infinitas-utilities");
}
