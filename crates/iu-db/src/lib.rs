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
use iu_core::label::ResultJudge;
use iu_core::songs::{Song, SongList};
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
    // 2: results (one row per result screen)
    "CREATE TABLE results (
        id              INTEGER PRIMARY KEY,
        game_day        TEXT    NOT NULL,
        recorded_at     TEXT    NOT NULL,  -- RFC 3339, UTC
        side            TEXT    NOT NULL,  -- 1p / 2p
        chart           TEXT,              -- SPN…; NULL = not recognized
        level           INTEGER,
        song_id         TEXT,              -- songs.json id, once titles are read
        notes           INTEGER NOT NULL,
        lamp            TEXT,              -- failed, clear, full-combo…
        best_lamp       TEXT,
        dj_level        TEXT,
        best_dj_level   TEXT,
        score           INTEGER NOT NULL,
        best_score      INTEGER,
        miss_count      INTEGER,           -- NULL when shown as —
        best_miss_count INTEGER,
        pg INTEGER NOT NULL,
        gr INTEGER NOT NULL,
        gd INTEGER NOT NULL,
        bd INTEGER NOT NULL,
        pr INTEGER NOT NULL,
        combo_break     INTEGER NOT NULL,
        fast            INTEGER NOT NULL,
        slow            INTEGER NOT NULL,
        dead            INTEGER NOT NULL,  -- 0/1: stopped early
        use_option      TEXT
    );
    CREATE INDEX results_game_day ON results (game_day);",
    // 3: song titles. The title line's mask is kept with each result so an
    // unrecognized one can be named later; named masks are learned.
    "ALTER TABLE results ADD COLUMN title TEXT;
    ALTER TABLE results ADD COLUMN title_mask BLOB;  -- packed bits, see TitleMaskData
    ALTER TABLE results ADD COLUMN title_mask_width INTEGER;
    ALTER TABLE results ADD COLUMN title_mask_height INTEGER;
    CREATE TABLE learned_titles (
        id         INTEGER PRIMARY KEY,
        song_id    TEXT    NOT NULL,
        width      INTEGER NOT NULL,
        height     INTEGER NOT NULL,
        mask       BLOB    NOT NULL,
        learned_at TEXT    NOT NULL   -- RFC 3339, UTC
    );",
    // 4: title lines of the play screen's info box, learned once a play's
    // song is known, to know the song at the next play's start. Per layout:
    // the box is in another place in each.
    "CREATE TABLE learned_play_titles (
        id         INTEGER PRIMARY KEY,
        song_id    TEXT    NOT NULL,
        layout     TEXT    NOT NULL,  -- PlayLayout name
        width      INTEGER NOT NULL,
        height     INTEGER NOT NULL,
        mask       BLOB    NOT NULL,
        learned_at TEXT    NOT NULL
    );",
];

/// A title-line mask as stored: size and packed bits (the recognizer's
/// `TitleMask::to_bytes`). Kept opaque here so this crate does not depend
/// on the recognizer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TitleMaskData {
    pub width: u32,
    pub height: u32,
    pub bytes: Vec<u8>,
}

/// A result whose song is not known yet, with its title mask.
#[derive(Debug, Clone)]
pub struct UnnamedResult {
    pub record: ResultRecord,
    pub mask: TitleMaskData,
}

/// A song the user can pick for an unnamed result.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Candidate {
    pub id: String,
    pub title: String,
    pub artist: String,
}

impl From<&Song> for Candidate {
    fn from(s: &Song) -> Self {
        Self {
            id: s.id.clone(),
            title: s.title.clone(),
            artist: s.artist.clone(),
        }
    }
}

/// An unnamed result as shown to the user: the result and the songs its
/// chart, level and NOTES count allow.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UnnamedView {
    pub result: ResultRecord,
    pub candidates: Vec<Candidate>,
}

/// Songs whose title contains `query` (case-insensitive), at most `limit`:
/// for naming a result the song list has no candidate for.
pub fn search_songs(songs: &SongList, query: &str, limit: usize) -> Vec<Candidate> {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return Vec::new();
    }
    songs
        .songs
        .iter()
        .filter(|s| s.title.to_lowercase().contains(&q))
        .take(limit)
        .map(Candidate::from)
        .collect()
}

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error(transparent)]
    Sqlite(#[from] rusqlite::Error),
    #[error(
        "data.db has schema version {found}, newer than this app ({supported}); update the app"
    )]
    TooNew { found: u32, supported: u32 },
}

pub use iu_core::play::{DaySummary, DayTotal, PlayRecord};
pub use iu_core::result::ResultRecord;

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

    /// Stores a result (its `id` is ignored) with its title mask; returns
    /// the new row ID.
    pub fn insert_result(
        &self,
        r: &ResultRecord,
        mask: Option<&TitleMaskData>,
    ) -> Result<i64, StoreError> {
        let word = |v: serde_json::Value| v.as_str().map(str::to_owned);
        let opt = |v: Option<serde_json::Value>| v.and_then(word);
        let j = r.judge;
        self.conn.execute(
            "INSERT INTO results (game_day, recorded_at, side, chart, level, song_id, notes,
                lamp, best_lamp, dj_level, best_dj_level, score, best_score, miss_count,
                best_miss_count, pg, gr, gd, bd, pr, combo_break, fast, slow, dead, use_option,
                title, title_mask, title_mask_width, title_mask_height)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16,
                ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27, ?28, ?29)",
            params![
                r.game_day,
                r.recorded_at,
                word(json(&r.side)),
                r.chart.map(|c| c.key()),
                r.level,
                r.song_id,
                r.notes,
                opt(r.lamp.map(|v| json(&v))),
                opt(r.best_lamp.map(|v| json(&v))),
                opt(r.dj_level.map(|v| json(&v))),
                opt(r.best_dj_level.map(|v| json(&v))),
                r.score,
                r.best_score,
                r.miss_count,
                r.best_miss_count,
                j.pg,
                j.gr,
                j.gd,
                j.bd,
                j.pr,
                r.combo_break,
                r.fast,
                r.slow,
                r.dead,
                r.use_option,
                r.title,
                mask.map(|m| &m.bytes),
                mask.map(|m| m.width),
                mask.map(|m| m.height),
            ],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    /// The results of one game day, oldest first.
    pub fn results_on(&self, day: NaiveDate) -> Result<Vec<ResultRecord>, StoreError> {
        let mut stmt = self.conn.prepare(&format!(
            "SELECT {RESULT_COLUMNS} FROM results WHERE game_day = ?1 ORDER BY recorded_at, id"
        ))?;
        let rows = stmt.query_map(params![day], result_row)?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    /// Results with a title mask but no song yet, oldest first.
    pub fn unnamed_results(&self) -> Result<Vec<UnnamedResult>, StoreError> {
        let mut stmt = self.conn.prepare(&format!(
            "SELECT {RESULT_COLUMNS}, title_mask, title_mask_width, title_mask_height
             FROM results WHERE song_id IS NULL AND title_mask IS NOT NULL
             ORDER BY recorded_at, id"
        ))?;
        let rows = stmt.query_map([], |r| {
            Ok(UnnamedResult {
                record: result_row(r)?,
                mask: TitleMaskData {
                    bytes: r.get(RESULT_COLUMN_COUNT)?,
                    width: r.get(RESULT_COLUMN_COUNT + 1)?,
                    height: r.get(RESULT_COLUMN_COUNT + 2)?,
                },
            })
        })?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    /// [`Store::unnamed_results`] with each one's candidate songs.
    pub fn unnamed_views(&self, songs: &SongList) -> Result<Vec<UnnamedView>, StoreError> {
        Ok(self
            .unnamed_results()?
            .into_iter()
            .map(|u| {
                let r = u.record;
                UnnamedView {
                    candidates: songs
                        .candidates(r.chart, r.level, r.notes)
                        .into_iter()
                        .map(Candidate::from)
                        .collect(),
                    result: r,
                }
            })
            .collect())
    }

    /// Names the song of a stored result.
    pub fn set_result_song(&self, id: i64, song_id: &str, title: &str) -> Result<(), StoreError> {
        self.conn.execute(
            "UPDATE results SET song_id = ?2, title = ?3 WHERE id = ?1",
            params![id, song_id, title],
        )?;
        Ok(())
    }

    /// One result by row ID.
    pub fn result(&self, id: i64) -> Result<Option<ResultRecord>, StoreError> {
        Ok(self
            .conn
            .query_row(
                &format!("SELECT {RESULT_COLUMNS} FROM results WHERE id = ?1"),
                params![id],
                result_row,
            )
            .optional()?)
    }

    /// Remembers that this mask is the title line of this song.
    pub fn learn_title(
        &self,
        song_id: &str,
        mask: &TitleMaskData,
        at: chrono::DateTime<chrono::Utc>,
    ) -> Result<(), StoreError> {
        self.conn.execute(
            "INSERT INTO learned_titles (song_id, width, height, mask, learned_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![song_id, mask.width, mask.height, mask.bytes, at],
        )?;
        Ok(())
    }

    /// Every learned title mask, oldest first.
    pub fn learned_titles(&self) -> Result<Vec<(String, TitleMaskData)>, StoreError> {
        let mut stmt = self
            .conn
            .prepare("SELECT song_id, width, height, mask FROM learned_titles ORDER BY id")?;
        let rows = stmt.query_map([], |r| {
            Ok((
                r.get(0)?,
                TitleMaskData {
                    width: r.get(1)?,
                    height: r.get(2)?,
                    bytes: r.get(3)?,
                },
            ))
        })?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    /// Remembers that this mask is the play screen's title line of this
    /// song in this layout.
    pub fn learn_play_title(
        &self,
        song_id: &str,
        layout: &str,
        mask: &TitleMaskData,
        at: chrono::DateTime<chrono::Utc>,
    ) -> Result<(), StoreError> {
        self.conn.execute(
            "INSERT INTO learned_play_titles (song_id, layout, width, height, mask, learned_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![song_id, layout, mask.width, mask.height, mask.bytes, at],
        )?;
        Ok(())
    }

    /// Every learned play-screen title mask: (song, layout, mask), oldest
    /// first.
    pub fn learned_play_titles(&self) -> Result<Vec<(String, String, TitleMaskData)>, StoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT song_id, layout, width, height, mask FROM learned_play_titles ORDER BY id",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok((
                r.get(0)?,
                r.get(1)?,
                TitleMaskData {
                    width: r.get(2)?,
                    height: r.get(3)?,
                    bytes: r.get(4)?,
                },
            ))
        })?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    /// Totals of the `limit` most recent game days that have plays, newest
    /// first. Days without plays are not listed.
    pub fn recent_days(&self, limit: u32) -> Result<Vec<DayTotal>, StoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT game_day, SUM(pg), SUM(gr), SUM(gd), SUM(bd), SUM(pr), SUM(cb),
                    COUNT(*) FILTER (WHERE pg + gr + gd + bd > 0)
             FROM plays GROUP BY game_day ORDER BY game_day DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![limit], |r| {
            Ok(DayTotal {
                game_day: r.get(0)?,
                summary: DaySummary {
                    judge: Judge {
                        pg: r.get(1)?,
                        gr: r.get(2)?,
                        gd: r.get(3)?,
                        bd: r.get(4)?,
                        pr: r.get(5)?,
                        cb: r.get(6)?,
                    },
                    plays: r.get(7)?,
                },
            })
        })?;
        Ok(rows.collect::<Result<_, _>>()?)
    }
}

/// The columns [`result_row`] reads, in order.
const RESULT_COLUMNS: &str = "id, game_day, recorded_at, side, chart, level, song_id, notes, lamp,
    best_lamp, dj_level, best_dj_level, score, best_score, miss_count, best_miss_count,
    pg, gr, gd, bd, pr, combo_break, fast, slow, dead, use_option, title";
const RESULT_COLUMN_COUNT: usize = 27;

fn result_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<ResultRecord> {
    let word = |i: usize| -> rusqlite::Result<Option<String>> { r.get(i) };
    let parsed = |i: usize| -> rusqlite::Result<Option<serde_json::Value>> {
        Ok(word(i)?.map(serde_json::Value::String))
    };
    let side: String = r.get(3)?;
    Ok(ResultRecord {
        id: r.get(0)?,
        game_day: r.get(1)?,
        recorded_at: r.get(2)?,
        side: from_word(serde_json::Value::String(side), 3)?,
        chart: word(4)?.and_then(|c| c.parse().ok()),
        level: r.get(5)?,
        song_id: r.get(6)?,
        notes: r.get(7)?,
        lamp: parsed(8)?.and_then(|v| serde_json::from_value(v).ok()),
        best_lamp: parsed(9)?.and_then(|v| serde_json::from_value(v).ok()),
        dj_level: parsed(10)?.and_then(|v| serde_json::from_value(v).ok()),
        best_dj_level: parsed(11)?.and_then(|v| serde_json::from_value(v).ok()),
        score: r.get(12)?,
        best_score: r.get(13)?,
        miss_count: r.get(14)?,
        best_miss_count: r.get(15)?,
        judge: ResultJudge {
            pg: r.get(16)?,
            gr: r.get(17)?,
            gd: r.get(18)?,
            bd: r.get(19)?,
            pr: r.get(20)?,
        },
        combo_break: r.get(21)?,
        fast: r.get(22)?,
        slow: r.get(23)?,
        dead: r.get(24)?,
        use_option: r.get(25)?,
        title: r.get(26)?,
    })
}

fn json<T: serde::Serialize>(v: &T) -> serde_json::Value {
    serde_json::to_value(v).expect("serializable")
}

fn from_word<T: serde::de::DeserializeOwned>(
    v: serde_json::Value,
    column: usize,
) -> rusqlite::Result<T> {
    serde_json::from_value(v).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(column, rusqlite::types::Type::Text, e.into())
    })
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
    fn recent_days_are_newest_first_and_limited() {
        let s = Store::open_in_memory().unwrap();
        s.insert_play(&record("2026-11-07", 5, 0)).unwrap();
        s.insert_play(&record("2026-11-09", 10, 0)).unwrap();
        s.insert_play(&record("2026-11-09", 20, 1)).unwrap();
        s.insert_play(&record("2026-11-08", 7, 0)).unwrap();
        let days = s.recent_days(2).unwrap();
        assert_eq!(days.len(), 2);
        assert_eq!(days[0].game_day.to_string(), "2026-11-09");
        assert_eq!(days[0].summary.judge.pg, 30);
        assert_eq!(days[0].summary.plays, 2);
        assert_eq!(days[1].game_day.to_string(), "2026-11-08");
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

    fn result(day: &str) -> ResultRecord {
        use iu_core::label::{DjLevel, Lamp};
        use iu_core::songs::Chart;
        let t: DateTime<Utc> = DateTime::parse_from_rfc3339("2026-11-09T08:00:00Z")
            .unwrap()
            .into();
        ResultRecord {
            id: None,
            title: None,
            game_day: day.parse().unwrap(),
            recorded_at: t,
            side: Side::P2,
            chart: Some(Chart::Spn),
            level: Some(4),
            song_id: None,
            notes: 420,
            lamp: Some(Lamp::FullCombo),
            best_lamp: None,
            dj_level: Some(DjLevel::B),
            best_dj_level: None,
            score: 524,
            best_score: None,
            miss_count: Some(0),
            best_miss_count: None,
            judge: ResultJudge {
                pg: 193,
                gr: 138,
                gd: 89,
                bd: 0,
                pr: 0,
            },
            combo_break: 0,
            fast: 197,
            slow: 30,
            dead: false,
            use_option: Some("A-SCR".into()),
        }
    }

    #[test]
    fn results_round_trip() {
        let s = Store::open_in_memory().unwrap();
        let mut unknown = result("2026-11-09");
        unknown.chart = None;
        unknown.lamp = None;
        let id = s.insert_result(&result("2026-11-09"), None).unwrap();
        let id2 = s.insert_result(&unknown, None).unwrap();
        s.insert_result(&result("2026-11-08"), None).unwrap();
        let got = s.results_on("2026-11-09".parse().unwrap()).unwrap();
        let with_id = |mut r: ResultRecord, id| {
            r.id = Some(id);
            r
        };
        assert_eq!(
            got,
            [with_id(result("2026-11-09"), id), with_id(unknown, id2)]
        );
        assert_eq!(s.result(id).unwrap(), Some(got[0].clone()));
        assert_eq!(s.result(999).unwrap(), None);
    }

    #[test]
    fn unnamed_results_are_named_and_titles_learned() {
        let s = Store::open_in_memory().unwrap();
        let mask = TitleMaskData {
            width: 8,
            height: 2,
            bytes: vec![1, 2, 3, 4, 5, 6, 7, 8],
        };
        let id = s.insert_result(&result("2026-11-09"), Some(&mask)).unwrap();
        s.insert_result(&result("2026-11-09"), None).unwrap(); // no mask: not listed
        let unnamed = s.unnamed_results().unwrap();
        assert_eq!(unnamed.len(), 1);
        assert_eq!(unnamed[0].record.id, Some(id));
        assert_eq!(unnamed[0].mask, mask);

        s.set_result_song(id, "abc", "Shooting Star").unwrap();
        assert!(s.unnamed_results().unwrap().is_empty());
        let named = s.result(id).unwrap().unwrap();
        assert_eq!(named.song_id.as_deref(), Some("abc"));
        assert_eq!(named.title.as_deref(), Some("Shooting Star"));

        s.learn_title("abc", &mask, named.recorded_at).unwrap();
        assert_eq!(
            s.learned_titles().unwrap(),
            [("abc".to_owned(), mask.clone())]
        );

        s.learn_play_title("abc", "sp-1p-graph-inner", &mask, named.recorded_at)
            .unwrap();
        assert_eq!(
            s.learned_play_titles().unwrap(),
            [("abc".to_owned(), "sp-1p-graph-inner".to_owned(), mask)]
        );
        assert_eq!(s.learned_titles().unwrap().len(), 1, "kept apart");
    }

    #[test]
    fn upgrades_a_version_1_database() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(MIGRATIONS[0]).unwrap();
        conn.pragma_update(None, "user_version", 1).unwrap();
        let s = Store::init(conn).unwrap();
        s.insert_result(&result("2026-11-09"), None).unwrap();
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
