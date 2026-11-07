use std::path::Path;

use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::fmt;
use tracing_subscriber::prelude::*;

/// Logs to a daily-rotated file in `log_dir` and, in debug builds, to stderr.
///
/// `RUST_LOG` overrides `level` (the config's `log_level`). If the log folder
/// cannot be created, logging falls back to stderr only and returns `None`.
pub fn init(log_dir: &Path, level: &str) -> Option<WorkerGuard> {
    let filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new(level))
        .unwrap_or_else(|_| EnvFilter::new("info"));

    let stderr = cfg!(debug_assertions).then(|| fmt::layer().with_writer(std::io::stderr));

    // The appender prunes old files by listing the folder, so it must exist.
    if let Err(e) = std::fs::create_dir_all(log_dir) {
        eprintln!("cannot create log folder {}: {e}", log_dir.display());
    }
    let file = tracing_appender::rolling::Builder::new()
        .rotation(tracing_appender::rolling::Rotation::DAILY)
        .filename_prefix("infinitas-utilities")
        .filename_suffix("log")
        .max_log_files(14)
        .build(log_dir);

    let (file_layer, guard) = match file {
        Ok(appender) => {
            let (writer, guard) = tracing_appender::non_blocking(appender);
            let layer = fmt::layer().with_ansi(false).with_writer(writer);
            (Some(layer), Some(guard))
        }
        Err(e) => {
            eprintln!("cannot open log folder {}: {e}", log_dir.display());
            (None, None)
        }
    };

    tracing_subscriber::registry()
        .with(filter)
        .with(stderr)
        .with(file_layer)
        .init();
    guard
}
