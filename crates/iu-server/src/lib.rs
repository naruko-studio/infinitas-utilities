//! The web side: OBS browser-source pages, the phone page, a WebSocket feed
//! of bus events and a small read-only JSON API, all on one port.
//!
//! - `GET /` phone page; `GET /obs/*.html` OBS pages; `GET /assets/*`
//! - `GET /ws` WebSocket: the current status right away, then every bus
//!   event as JSON (see [`iu_core::event`])
//! - `GET /api/status`, `GET /api/today`, `GET /api/days?limit=N`
//!
//! Nothing here writes: the API and pages only read. By default the server
//! listens on 127.0.0.1 only; [`ServerConfig::lan_access`] opens it to the
//! LAN for phones.

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::PathBuf;

use axum::Router;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Query, State};
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use chrono::{NaiveTime, Utc};
use iu_core::config::ServerConfig;
use iu_core::day::game_day;
use iu_core::event::{Bus, Event};
use iu_core::play::{DaySummary, DayTotal, PlayRecord};
use iu_core::status::{SharedStatus, Status};
use iu_db::Store;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast::error::RecvError;
use tokio::sync::oneshot;

/// What the handlers need.
#[derive(Clone)]
pub struct Context {
    pub bus: Bus,
    pub status: SharedStatus,
    /// `data.db`; opened per API request (cheap, and keeps the handlers
    /// free of a shared connection).
    pub db_path: PathBuf,
    pub day_start_jst: NaiveTime,
}

/// A running server. Dropping it (or calling [`Server::stop`]) shuts it
/// down.
pub struct Server {
    addr: SocketAddr,
    shutdown: Option<oneshot::Sender<()>>,
}

impl Server {
    /// Binds and starts serving on the current Tokio runtime.
    pub async fn start(ctx: Context, config: &ServerConfig) -> std::io::Result<Self> {
        let ip = if config.lan_access {
            IpAddr::V4(Ipv4Addr::UNSPECIFIED)
        } else {
            IpAddr::V4(Ipv4Addr::LOCALHOST)
        };
        let listener = tokio::net::TcpListener::bind((ip, config.port)).await?;
        let addr = listener.local_addr()?;
        let (tx, rx) = oneshot::channel::<()>();
        tokio::spawn(async move {
            let shutdown = async {
                let _ = rx.await;
            };
            if let Err(e) = axum::serve(listener, router(ctx))
                .with_graceful_shutdown(shutdown)
                .await
            {
                tracing::error!("web server stopped: {e}");
            }
        });
        tracing::info!(%addr, "web server listening");
        Ok(Self {
            addr,
            shutdown: Some(tx),
        })
    }

    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    pub fn stop(mut self) {
        self.shutdown_now();
    }

    fn shutdown_now(&mut self) {
        if let Some(tx) = self.shutdown.take() {
            let _ = tx.send(());
        }
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.shutdown_now();
    }
}

/// This PC's address on the LAN (the interface the default route uses), for
/// showing the phone URL. No packet is sent: connecting a UDP socket only
/// picks a route.
pub fn lan_ip() -> Option<IpAddr> {
    let socket = std::net::UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0)).ok()?;
    socket.connect((Ipv4Addr::new(192, 0, 2, 1), 9)).ok()?;
    let ip = socket.local_addr().ok()?.ip();
    (!ip.is_unspecified() && !ip.is_loopback()).then_some(ip)
}

pub fn router(ctx: Context) -> Router {
    Router::new()
        .route("/", get(|| asset("index.html")))
        .route("/obs/{page}", get(obs_page))
        .route("/assets/{file}", get(asset_file))
        .route("/ws", get(ws))
        .route("/api/status", get(api_status))
        .route("/api/today", get(api_today))
        .route("/api/days", get(api_days))
        .with_state(ctx)
}

// ---- static pages --------------------------------------------------------

/// Pages are embedded in the binary: nothing to install next to the exe.
const ASSETS: &[(&str, &str, &str)] = &[
    ("index.html", "text/html", include_str!("../web/index.html")),
    (
        "counter.html",
        "text/html",
        include_str!("../web/obs/counter.html"),
    ),
    (
        "judge.html",
        "text/html",
        include_str!("../web/obs/judge.html"),
    ),
    ("iu.js", "text/javascript", include_str!("../web/iu.js")),
];

async fn asset(name: &str) -> Response {
    match ASSETS.iter().find(|(n, ..)| *n == name) {
        Some((_, mime, body)) => (
            [
                (header::CONTENT_TYPE, format!("{mime}; charset=utf-8")),
                (header::CACHE_CONTROL, "no-cache".to_owned()),
            ],
            *body,
        )
            .into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

async fn obs_page(axum::extract::Path(page): axum::extract::Path<String>) -> Response {
    match page.as_str() {
        "counter.html" | "judge.html" => asset(&page).await,
        _ => StatusCode::NOT_FOUND.into_response(),
    }
}

async fn asset_file(axum::extract::Path(file): axum::extract::Path<String>) -> Response {
    match file.as_str() {
        "iu.js" => asset(&file).await,
        _ => StatusCode::NOT_FOUND.into_response(),
    }
}

// ---- WebSocket -----------------------------------------------------------

async fn ws(State(ctx): State<Context>, upgrade: WebSocketUpgrade) -> Response {
    upgrade.on_upgrade(move |socket| feed(socket, ctx))
}

/// Sends the current status, then forwards bus events until the client
/// goes away.
async fn feed(mut socket: WebSocket, ctx: Context) {
    // Subscribe before taking the snapshot so nothing falls in between.
    let mut events = ctx.bus.subscribe();
    let snapshot = Event::Status(current_status(&ctx));
    if send(&mut socket, &snapshot).await.is_err() {
        return;
    }
    loop {
        tokio::select! {
            ev = events.recv() => match ev {
                Ok(ev) => {
                    if send(&mut socket, &ev).await.is_err() {
                        return;
                    }
                }
                // Status events are full snapshots: skipping is harmless.
                Err(RecvError::Lagged(n)) => tracing::debug!(n, "WebSocket client lagged"),
                Err(RecvError::Closed) => return,
            },
            msg = socket.recv() => match msg {
                // Clients don't send anything we act on.
                Some(Ok(Message::Close(_))) | None | Some(Err(_)) => return,
                Some(Ok(_)) => {}
            },
        }
    }
}

async fn send(socket: &mut WebSocket, ev: &Event) -> Result<(), axum::Error> {
    let json = serde_json::to_string(ev).expect("events serialize");
    socket.send(Message::text(json)).await
}

fn current_status(ctx: &Context) -> Status {
    ctx.status.lock().expect("status lock").clone()
}

// ---- API -----------------------------------------------------------------

async fn api_status(State(ctx): State<Context>) -> axum::Json<Status> {
    axum::Json(current_status(&ctx))
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Today {
    pub game_day: chrono::NaiveDate,
    pub summary: DaySummary,
    pub plays: Vec<PlayRecord>,
}

async fn api_today(State(ctx): State<Context>) -> Result<axum::Json<Today>, ApiError> {
    let day = game_day(Utc::now(), ctx.day_start_jst);
    let today = with_store(ctx, move |s| {
        Ok(Today {
            game_day: day,
            summary: s.day_summary(day)?,
            plays: s.plays_on(day)?,
        })
    })
    .await?;
    Ok(axum::Json(today))
}

#[derive(Deserialize)]
struct DaysQuery {
    limit: Option<u32>,
}

async fn api_days(
    State(ctx): State<Context>,
    Query(q): Query<DaysQuery>,
) -> Result<axum::Json<Vec<DayTotal>>, ApiError> {
    let limit = q.limit.unwrap_or(14).clamp(1, 366);
    let days = with_store(ctx, move |s| s.recent_days(limit)).await?;
    Ok(axum::Json(days))
}

/// Runs a query on a fresh connection off the async threads.
async fn with_store<T: Send + 'static>(
    ctx: Context,
    f: impl FnOnce(&Store) -> Result<T, iu_db::StoreError> + Send + 'static,
) -> Result<T, ApiError> {
    tokio::task::spawn_blocking(move || {
        let store = Store::open(&ctx.db_path)?;
        f(&store)
    })
    .await
    .map_err(|e| ApiError(e.to_string()))?
    .map_err(|e| ApiError(e.to_string()))
}

struct ApiError(String);

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        tracing::error!("API: {}", self.0);
        (StatusCode::INTERNAL_SERVER_ERROR, self.0).into_response()
    }
}

#[cfg(test)]
mod tests;
