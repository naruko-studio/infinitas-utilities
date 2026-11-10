//! The web side: OBS browser-source pages, the phone page, a WebSocket feed
//! of bus events and a small JSON API, all on one port.
//!
//! - `GET /` phone page; `GET /obs/*.html` OBS pages; `GET /assets/*`
//! - `GET /ws` WebSocket: the current status right away, then every bus
//!   event as JSON (see [`iu_core::event`])
//! - `GET /api/status`, `GET /api/today`, `GET /api/days?limit=N`,
//!   `GET /api/unnamed` (results whose song is not known), `GET /api/songs?q=`
//! - `POST /api/results/{id}/song` `{"songId"}`: name a result's song
//!
//! Naming a result is the only write. By default the server
//! listens on 127.0.0.1 only; [`ServerConfig::lan_access`] opens it to the
//! LAN for phones.

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::PathBuf;
use std::sync::Arc;

use axum::Router;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Query, State};
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use chrono::{NaiveTime, Utc};
use iu_core::config::ServerConfig;
use iu_core::day::game_day;
use iu_core::event::{Bus, Event};
use iu_core::play::{DaySummary, DayTotal, PlayRecord};
use iu_core::songs::SongList;
use iu_core::status::{SharedStatus, Status};
use iu_db::{Candidate, Store, search_songs};
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
    /// The bundled song list (candidates and search for naming results).
    pub songs: &'static SongList,
    /// Names a stored result's song (the engine records it and learns the
    /// title); blocking. `None`: naming from the web is not available.
    pub name_result: Option<NameResult>,
}

/// `(result id, song id)` → done or an error message.
pub type NameResult = Arc<dyn Fn(i64, String) -> Result<(), String> + Send + Sync>;

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
        .route("/api/unnamed", get(api_unnamed))
        .route("/api/songs", get(api_songs))
        .route("/api/results/{id}/song", post(api_name_result))
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
    (
        "song.html",
        "text/html",
        include_str!("../web/obs/song.html"),
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
        "counter.html" | "judge.html" | "song.html" => asset(&page).await,
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
    let snapshot = Event::Status(Box::new(current_status(&ctx)));
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
    /// Result screens recorded today, oldest first.
    pub results: Vec<iu_core::result::ResultRecord>,
}

async fn api_today(State(ctx): State<Context>) -> Result<axum::Json<Today>, ApiError> {
    let day = game_day(Utc::now(), ctx.day_start_jst);
    let today = with_store(ctx, move |s| {
        Ok(Today {
            game_day: day,
            summary: s.day_summary(day)?,
            plays: s.plays_on(day)?,
            results: s.results_on(day)?,
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

/// Results whose song was not recognized, with their candidate songs.
/// 404 when naming is not available, so the phone page hides the section.
async fn api_unnamed(State(ctx): State<Context>) -> Response {
    if ctx.name_result.is_none() {
        return StatusCode::NOT_FOUND.into_response();
    }
    let songs = ctx.songs;
    match with_store(ctx, move |s| s.unnamed_views(songs)).await {
        Ok(views) => axum::Json(views).into_response(),
        Err(e) => e.into_response(),
    }
}

#[derive(Deserialize)]
struct SongsQuery {
    q: String,
}

/// Title search, for naming a result without candidates.
async fn api_songs(
    State(ctx): State<Context>,
    Query(q): Query<SongsQuery>,
) -> axum::Json<Vec<Candidate>> {
    axum::Json(search_songs(ctx.songs, &q.q, 12))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct NameBody {
    song_id: String,
}

/// The one write: names a result's song. The body must be JSON, so a page
/// on another site cannot send it from the user's browser (a JSON POST
/// needs a CORS preflight, which this server never allows).
async fn api_name_result(
    State(ctx): State<Context>,
    axum::extract::Path(id): axum::extract::Path<i64>,
    axum::Json(body): axum::Json<NameBody>,
) -> Response {
    let Some(name) = ctx.name_result.clone() else {
        return (StatusCode::SERVICE_UNAVAILABLE, "naming is not available").into_response();
    };
    match tokio::task::spawn_blocking(move || name(id, body.song_id)).await {
        Ok(Ok(())) => StatusCode::NO_CONTENT.into_response(),
        Ok(Err(e)) => (StatusCode::UNPROCESSABLE_ENTITY, e).into_response(),
        Err(e) => ApiError(e.to_string()).into_response(),
    }
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
