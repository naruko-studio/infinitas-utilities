//! Runs the web server (`iu-server`) and restarts it when its settings
//! change.

use std::time::Duration;

use iu_core::config::ServerConfig;
use iu_server::{Context, Server};
use serde::Serialize;
use tauri::async_runtime::Mutex;

pub struct Web {
    ctx: Context,
    state: Mutex<State>,
}

#[derive(Default)]
struct State {
    server: Option<Server>,
    config: ServerConfig,
    error: Option<String>,
}

/// What the settings screen shows.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerInfo {
    pub running: bool,
    pub port: u16,
    pub lan_access: bool,
    /// Why the server is not running (e.g. the port is taken).
    pub error: Option<String>,
    /// Base URL for OBS on this PC.
    pub local_url: String,
    /// Base URL for phones, when LAN access is on and an address is known.
    pub lan_url: Option<String>,
}

impl Web {
    pub fn new(ctx: Context) -> Self {
        Self {
            ctx,
            state: Mutex::new(State::default()),
        }
    }

    /// Starts (or restarts) the server with `config`. Failures are kept for
    /// [`Web::info`] rather than returned: the app works without the server.
    pub async fn apply(&self, config: ServerConfig) {
        let mut state = self.state.lock().await;
        if let Some(old) = state.server.take() {
            old.stop();
        }
        state.config = config.clone();
        state.error = None;
        // The old listener closes asynchronously; retry the bind briefly.
        let mut last_err = None;
        for _ in 0..20 {
            match Server::start(self.ctx.clone(), &config).await {
                Ok(server) => {
                    state.server = Some(server);
                    return;
                }
                Err(e) => {
                    last_err = Some(e);
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
            }
        }
        let e = last_err.expect("at least one attempt");
        tracing::error!(port = config.port, "cannot start the web server: {e}");
        state.error = Some(e.to_string());
    }

    pub async fn info(&self) -> ServerInfo {
        let state = self.state.lock().await;
        let port = state
            .server
            .as_ref()
            .map_or(state.config.port, |s| s.addr().port());
        ServerInfo {
            running: state.server.is_some(),
            port,
            lan_access: state.config.lan_access,
            error: state.error.clone(),
            local_url: format!("http://localhost:{port}"),
            lan_url: state
                .config
                .lan_access
                .then(iu_server::lan_ip)
                .flatten()
                .map(|ip| format!("http://{ip}:{port}")),
        }
    }
}
