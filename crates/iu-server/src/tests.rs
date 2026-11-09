use std::sync::{Arc, Mutex};

use axum::body::Body;
use axum::http::Request;
use futures_util::StreamExt;
use http_body_util::BodyExt;
use iu_core::Judge;
use iu_core::play::{PlayLayout, Side};
use iu_core::status::Connection;
use tower::ServiceExt;

use super::*;

fn ctx(dir: &tempfile::TempDir) -> Context {
    Context {
        bus: iu_core::event::bus(),
        status: Arc::new(Mutex::new(Status {
            connection: Connection::Connected,
            today_notes: 123,
            ..Status::default()
        })),
        db_path: dir.path().join("data.db"),
        day_start_jst: iu_core::day::default_day_start(),
    }
}

fn play_today(ctx: &Context, pg: u32) -> PlayRecord {
    let now = Utc::now();
    PlayRecord {
        game_day: game_day(now, ctx.day_start_jst),
        started_at: now,
        finished_at: now,
        layout: PlayLayout::Sp {
            side: Side::P1,
            graph_inner: true,
        },
        judge: Judge {
            pg,
            ..Judge::default()
        },
    }
}

async fn get(app: Router, uri: &str) -> (StatusCode, String, String) {
    let res = app
        .oneshot(Request::get(uri).body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = res.status();
    let ctype = res
        .headers()
        .get(header::CONTENT_TYPE)
        .map(|v| v.to_str().unwrap().to_owned())
        .unwrap_or_default();
    let body = res.into_body().collect().await.unwrap().to_bytes();
    (status, ctype, String::from_utf8(body.to_vec()).unwrap())
}

#[tokio::test]
async fn pages_are_served_and_unknown_ones_are_not() {
    let dir = tempfile::tempdir().unwrap();
    let app = router(ctx(&dir));
    for uri in ["/", "/obs/counter.html", "/obs/judge.html", "/assets/iu.js"] {
        let (status, ctype, body) = get(app.clone(), uri).await;
        assert_eq!(status, StatusCode::OK, "{uri}");
        assert!(ctype.contains("charset=utf-8"), "{uri}: {ctype}");
        assert!(!body.is_empty());
    }
    for uri in [
        "/obs/iu.js",
        "/assets/index.html",
        "/obs/../index.html",
        "/nope",
    ] {
        let (status, ..) = get(app.clone(), uri).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{uri}");
    }
}

#[tokio::test]
async fn api_reports_status_today_and_days() {
    let dir = tempfile::tempdir().unwrap();
    let ctx = ctx(&dir);
    {
        let store = Store::open(&ctx.db_path).unwrap();
        store.insert_play(&play_today(&ctx, 10)).unwrap();
        store.insert_play(&play_today(&ctx, 20)).unwrap();
    }
    let app = router(ctx);

    let (_, _, body) = get(app.clone(), "/api/status").await;
    let status: Status = serde_json::from_str(&body).unwrap();
    assert_eq!(status.today_notes, 123);

    let (_, _, body) = get(app.clone(), "/api/today").await;
    let today: Today = serde_json::from_str(&body).unwrap();
    assert_eq!(today.summary.judge.pg, 30);
    assert_eq!(today.summary.plays, 2);
    assert_eq!(today.plays.len(), 2);

    let (_, _, body) = get(app.clone(), "/api/days?limit=3").await;
    let days: Vec<DayTotal> = serde_json::from_str(&body).unwrap();
    assert_eq!(days.len(), 1);
    assert_eq!(days[0].summary.judge.pg, 30);
}

#[tokio::test]
async fn websocket_sends_a_snapshot_then_bus_events() {
    let dir = tempfile::tempdir().unwrap();
    let ctx = ctx(&dir);
    let bus = ctx.bus.clone();
    let record = play_today(&ctx, 7);
    let server = Server::start(
        ctx,
        &ServerConfig {
            port: 0,
            lan_access: false,
        },
    )
    .await
    .unwrap();
    assert!(server.addr().ip().is_loopback());

    let url = format!("ws://{}/ws", server.addr());
    let (mut socket, _) = tokio_tungstenite::connect_async(url).await.unwrap();
    let next = |m: Option<Result<tokio_tungstenite::tungstenite::Message, _>>| {
        let text = m.unwrap().unwrap().into_text().unwrap();
        serde_json::from_str::<Event>(&text).unwrap()
    };

    match next(socket.next().await) {
        Event::Status(s) => assert_eq!(s.today_notes, 123),
        other => panic!("expected a status snapshot, got {other:?}"),
    }
    bus.send(Event::PlayFinished(record)).unwrap();
    assert_eq!(next(socket.next().await), Event::PlayFinished(record));

    server.stop();
}
