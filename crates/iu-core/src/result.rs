//! Results: what the result screen shows, as stored in `data.db`, and the
//! rule for when a reading is final.

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use crate::label::{DjLevel, Lamp, ResultJudge};
use crate::play::Side;
use crate::songs::Chart;

/// One result screen. Words the recognizer did not know yet are `None`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResultRecord {
    /// Row ID in `data.db`; `None` before it is stored.
    #[serde(default)]
    pub id: Option<i64>,
    pub game_day: NaiveDate,
    pub recorded_at: DateTime<Utc>,
    pub side: Side,
    pub chart: Option<Chart>,
    pub level: Option<u8>,
    /// Song list ID; `None` until the title is recognized (or picked by
    /// the user).
    pub song_id: Option<String>,
    /// The song's title, from the song list.
    #[serde(default)]
    pub title: Option<String>,
    pub notes: u32,
    pub lamp: Option<Lamp>,
    /// `None` for "—" or not recognized.
    pub best_lamp: Option<Lamp>,
    pub dj_level: Option<DjLevel>,
    pub best_dj_level: Option<DjLevel>,
    pub score: u32,
    pub best_score: Option<u32>,
    pub miss_count: Option<u32>,
    pub best_miss_count: Option<u32>,
    pub judge: ResultJudge,
    pub combo_break: u32,
    pub fast: u32,
    pub slow: u32,
    /// The play stopped early (DEAD POINT shown).
    pub dead: bool,
    pub use_option: Option<String>,
}

/// Same readings this many frames in a row make a result final: the screen
/// slides in and its numbers settle over a few frames.
pub const STABLE_FRAMES: u32 = 10;

/// Decides when to record a result: once per visit to the result screen,
/// after the reading has been stable.
#[derive(Debug)]
pub struct ResultTracker<T> {
    last: Option<T>,
    same: u32,
    recorded: bool,
}

impl<T> Default for ResultTracker<T> {
    fn default() -> Self {
        Self {
            last: None,
            same: 0,
            recorded: false,
        }
    }
}

impl<T: PartialEq + Clone> ResultTracker<T> {
    /// Feed one frame: `Some(reading)` on a result screen, `None` elsewhere.
    /// Returns the reading to record, at most once per visit.
    pub fn observe(&mut self, reading: Option<T>) -> Option<T> {
        let Some(r) = reading else {
            *self = Self::default();
            return None;
        };
        if self.last.as_ref() == Some(&r) {
            self.same += 1;
        } else {
            self.last = Some(r.clone());
            self.same = 1;
        }
        if !self.recorded && self.same >= STABLE_FRAMES {
            self.recorded = true;
            return Some(r);
        }
        None
    }

    /// This visit's result has been returned already.
    pub fn recorded(&self) -> bool {
        self.recorded
    }

    /// An unreadable frame on the result screen (covered, fading): it
    /// breaks the run but does not end the visit.
    pub fn unreadable(&mut self) {
        self.same = 0;
        self.last = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_once_after_stable_frames() {
        let mut t = ResultTracker::default();
        for _ in 0..STABLE_FRAMES - 1 {
            assert_eq!(t.observe(Some(1)), None);
        }
        assert_eq!(t.observe(Some(1)), Some(1));
        for _ in 0..30 {
            assert_eq!(t.observe(Some(1)), None);
        }
    }

    #[test]
    fn changing_readings_restart_the_count() {
        let mut t = ResultTracker::default();
        for i in 0..STABLE_FRAMES * 2 {
            assert_eq!(t.observe(Some(i)), None);
        }
        t.unreadable();
        for _ in 0..STABLE_FRAMES - 1 {
            assert_eq!(t.observe(Some(7)), None);
        }
        assert_eq!(t.observe(Some(7)), Some(7));
    }

    #[test]
    fn leaving_the_screen_allows_the_next_result() {
        let mut t = ResultTracker::default();
        for _ in 0..STABLE_FRAMES {
            t.observe(Some(1));
        }
        t.observe(None);
        let got: Vec<_> = (0..STABLE_FRAMES)
            .filter_map(|_| t.observe(Some(1)))
            .collect();
        assert_eq!(got, [1]);
    }
}
