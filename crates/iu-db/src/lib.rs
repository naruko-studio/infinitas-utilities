//! `data.db`: the app's SQLite database.
//!
//! One row per finished play; "today" is a query over the current game day
//! (see [`iu_core::day`]) rather than a stored total, so past days stay
//! available for history screens.
//!
//! The schema version lives in `PRAGMA user_version`; [`MIGRATIONS`] are
//! applied in order on open. Add new steps at the end, never edit old ones.

use std::path::Path;

use chrono::NaiveDate;
use iu_core::{Judge, PlayLayout};
use rusqlite::{Connection, OptionalExtension, params};

pub const DATABASE_FILE_NAME: &str = "data.db";

/// Schema steps; step `i` upgrades `user_version` from `i` to `i + 1`.
const MIGRATIONS: &[&str] = &[
    // 1: plays
    "CREATE TABLE plays (
        id          INTEGER PRIMARY KEY,
        game_day    TEXT    NOT NULL,  -- YYYY-MM-DD, see iu_core::day
        started_at  TEXT    NOT NULL,  -- RFC 3339, UTC
        finished_at TEXT    NOT NULL,
        layout      TEXT    NOT NULL,  -- PlayLayout name, e.g. sp-1p-graph-inner
        pg INTEGER NOT NULL,
        gr INTEGER NOT NULL,
        gd INTEGER NOT NULL,
        bd INTEGER NOT NULL,
        pr INTEGER NOT NULL,
        cb INTEGER NOT NULL
    );
    CREATE INDEX plays_game_day ON plays (game_day);",
];

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error(transparent)]
    Sqlite(#[from] rusqlite::Error),
    #[error(
        "data.db has schema version {found}, newer than this app ({supported}); update the app"
    )]
    TooNew { found: u32, supported: u32 },
}

pub use iu_core::play::{DaySummary, PlayRecord};

pub struct Store {
    conn: Connection,
}

impl Store {
    /// Opens (creating if needed) the database and brings the schema up to
    /// date.
    pub fn open(path: &Path) -> Result<Self, StoreError> {
        if let Some(dir) = path.parent() {
            // Let SQLite report the real error if this fails.
            let _ = std::fs::create_dir_all(dir);
        }
        let conn = Connection::open(path)?;
        // WAL: a crash loses at most the play in progress, and readers
        // (e.g. a DB viewer) don't block the app.
        conn.pragma_update(None, "journal_mode", "WAL")?;
        Self::init(conn)
    }

    pub fn open_in_memory() -> Result<Self, StoreError> {
        Self::init(Connection::open_in_memory()?)
    }

    fn init(mut conn: Connection) -> Result<Self, StoreError> {
        let version: u32 = conn.pragma_query_value(None, "user_version", |r| r.get(0))?;
        let supported = MIGRATIONS.len() as u32;
        if version > supported {
            return Err(StoreError::TooNew {
                found: version,
                supported,
            });
        }
        for (i, sql) in MIGRATIONS.iter().enumerate().skip(version as usize) {
            let tx = conn.transaction()?;
            tx.execute_batch(sql)?;
            tx.pragma_update(None, "user_version", i as u32 + 1)?;
            tx.commit()?;
            tracing::info!(version = i + 1, "data.db schema upgraded");
        }
        Ok(Self { conn })
    }

    pub fn insert_play(&self, play: &PlayRecord) -> Result<i64, StoreError> {
        let j = play.judge;
        self.conn.execute(
            "INSERT INTO plays
                (game_day, started_at, finished_at, layout, pg, gr, gd, bd, pr, cb)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                play.game_day,
                play.started_at,
                play.finished_at,
                play.layout.name(),
                j.pg,
                j.gr,
                j.gd,
                j.bd,
                j.pr,
                j.cb
            ],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn day_summary(&self, day: NaiveDate) -> Result<DaySummary, StoreError> {
        let row = self
            .conn
            .query_row(
                "SELECT
                    COALESCE(SUM(pg), 0), COALESCE(SUM(gr), 0), COALESCE(SUM(gd), 0),
                    COALESCE(SUM(bd), 0), COALESCE(SUM(pr), 0), COALESCE(SUM(cb), 0),
                    COUNT(*) FILTER (WHERE pg + gr + gd + bd > 0)
                 FROM plays WHERE game_day = ?1",
                params![day],
                |r| {
                    Ok(DaySummary {
                        judge: Judge {
                            pg: r.get(0)?,
                            gr: r.get(1)?,
                            gd: r.get(2)?,
                            bd: r.get(3)?,
                            pr: r.get(4)?,
                            cb: r.get(5)?,
                        },
                        plays: r.get(6)?,
                    })
                },
            )
            .optional()?;
        Ok(row.unwrap_or_default())
    }

    /// The plays of one game day, oldest first.
    pub fn plays_on(&self, day: NaiveDate) -> Result<Vec<PlayRecord>, StoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT game_day, started_at, finished_at, layout, pg, gr, gd, bd, pr, cb
             FROM plays WHERE game_day = ?1 ORDER BY started_at, id",
        )?;
        let rows = stmt.query_map(params![day], |r| {
            let layout: String = r.get(3)?;
            let layout = layout.parse::<PlayLayout>().map_err(|_| {
                rusqlite::Error::FromSqlConversionFailure(
                    3,
                    rusqlite::types::Type::Text,
                    format!("unknown layout {layout:?}").into(),
                )
            })?;
            Ok(PlayRecord {
                game_day: r.get(0)?,
                started_at: r.get(1)?,
                finished_at: r.get(2)?,
                layout,
                judge: Judge {
                    pg: r.get(4)?,
                    gr: r.get(5)?,
                    gd: r.get(6)?,
                    bd: r.get(7)?,
                    pr: r.get(8)?,
                    cb: r.get(9)?,
                },
            })
        })?;
        Ok(rows.collect::<Result<_, _>>()?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{DateTime, Utc};
    use iu_core::Side;

    const SP: PlayLayout = PlayLayout::Sp {
        side: Side::P1,
        graph_inner: true,
    };

    fn record(day: &str, pg: u32, pr: u32) -> PlayRecord {
        let t: DateTime<Utc> = DateTime::parse_from_rfc3339("2026-11-09T08:00:00Z")
            .unwrap()
            .into();
        PlayRecord {
            game_day: day.parse().unwrap(),
            started_at: t,
            finished_at: t,
            layout: SP,
            judge: Judge {
                pg,
                pr,
                ..Judge::default()
            },
        }
    }

    #[test]
    fn day_summary_sums_that_day_only() {
        let s = Store::open_in_memory().unwrap();
        s.insert_play(&record("2026-11-09", 100, 3)).unwrap();
        s.insert_play(&record("2026-11-09", 50, 0)).unwrap();
        s.insert_play(&record("2026-11-08", 999, 0)).unwrap();
        let sum = s.day_summary("2026-11-09".parse().unwrap()).unwrap();
        assert_eq!(sum.judge.pg, 150);
        let plays = s.plays_on("2026-11-09".parse().unwrap()).unwrap();
        assert_eq!(
            plays,
            [record("2026-11-09", 100, 3), record("2026-11-09", 50, 0)]
        );
        assert_eq!(sum.judge.pr, 3);
        assert_eq!(sum.plays, 2);
    }

    #[test]
    fn plays_without_hits_add_poors_but_not_a_play() {
        let s = Store::open_in_memory().unwrap();
        s.insert_play(&record("2026-11-09", 0, 50)).unwrap();
        let sum = s.day_summary("2026-11-09".parse().unwrap()).unwrap();
        assert_eq!(sum.judge.pr, 50);
        assert_eq!(sum.plays, 0);
    }

    #[test]
    fn empty_day_is_zero() {
        let s = Store::open_in_memory().unwrap();
        let sum = s.day_summary("2026-11-09".parse().unwrap()).unwrap();
        assert_eq!(sum, DaySummary::default());
    }

    #[test]
    fn reopening_keeps_data_and_schema() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(DATABASE_FILE_NAME);
        Store::open(&path)
            .unwrap()
            .insert_play(&record("2026-11-09", 10, 0))
            .unwrap();
        let s = Store::open(&path).unwrap();
        assert_eq!(
            s.day_summary("2026-11-09".parse().unwrap())
                .unwrap()
                .judge
                .pg,
            10
        );
    }

    #[test]
    fn newer_schema_is_refused() {
        let conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "user_version", 99).unwrap();
        assert!(matches!(
            Store::init(conn),
            Err(StoreError::TooNew { found: 99, .. })
        ));
    }
}
