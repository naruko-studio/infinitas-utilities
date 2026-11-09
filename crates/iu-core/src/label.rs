//! Labels for screenshots: the expected recognition result, written by the
//! annotator next to each image (`dev-capture\<w>x<h>\<screen>\<name>.json`)
//! and read by `resgen` and the regression tests.
//!
//! The screen kind is the folder name; the JSON holds that screen's fields.
//! Fields the screen does not show (or that are "—") are `null`.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::play::{Judge, PlayLayout, PlayStyle, Side};
use crate::songs::Chart;

/// Screen kinds, also the fixture sub-folder names.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Screen {
    Play,
    Result,
    Select,
    Option,
    Other,
}

impl Screen {
    pub const ALL: [Screen; 5] = [
        Screen::Play,
        Screen::Result,
        Screen::Select,
        Screen::Option,
        Screen::Other,
    ];

    pub fn folder(self) -> &'static str {
        match self {
            Screen::Play => "play",
            Screen::Result => "result",
            Screen::Select => "select",
            Screen::Option => "option",
            Screen::Other => "other",
        }
    }
}

impl fmt::Display for Screen {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.folder())
    }
}

impl FromStr for Screen {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, String> {
        Screen::ALL
            .into_iter()
            .find(|k| k.folder() == s)
            .ok_or_else(|| format!("unknown screen `{s}`"))
    }
}

/// Clear lamps, in the game's order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Lamp {
    NoPlay,
    Failed,
    AssistClear,
    EasyClear,
    Clear,
    HardClear,
    ExHardClear,
    FullCombo,
}

/// DJ LEVEL.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum DjLevel {
    F,
    E,
    D,
    C,
    B,
    A,
    AA,
    AAA,
}

/// Groove gauge selected on the option screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Gauge {
    Off,
    AssistEasy,
    Easy,
    Hard,
    ExHard,
}

/// Where play stopped early (HARD/EX-HARD gauge empty, or quit): the
/// result screen's DEAD POINT box.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeadPoint {
    pub note: u32,
    pub measure: u32,
}

/// Judge counts on the result screen (no CB in the table; it is shown
/// separately as COMBO BREAK).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResultJudge {
    pub pg: u32,
    pub gr: u32,
    pub gd: u32,
    pub bd: u32,
    pub pr: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayLabel {
    pub layout: PlayLayout,
    pub judge: Judge,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResultLabel {
    pub side: Side,
    pub chart: Chart,
    pub level: u8,
    /// Song list ID (see `songs`), when the song is known.
    pub song_id: Option<String>,
    /// Title as shown, for reference and for songs missing from the list.
    pub title: String,
    pub notes: u32,
    pub lamp: Lamp,
    pub best_lamp: Option<Lamp>,
    pub dj_level: DjLevel,
    /// `None` when shown as "—".
    #[serde(default)]
    pub best_dj_level: Option<DjLevel>,
    pub score: u32,
    pub best_score: Option<u32>,
    /// `None` when shown as "—" (assist options, early stop).
    pub miss_count: Option<u32>,
    pub best_miss_count: Option<u32>,
    pub judge: ResultJudge,
    pub combo_break: u32,
    pub fast: u32,
    pub slow: u32,
    pub dead: Option<DeadPoint>,
    /// The USE OPTION banner under the graph (e.g. `A-SCR`, `LEGACY`).
    #[serde(default)]
    pub use_option: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SelectLabel {
    pub style: PlayStyle,
    pub chart: Chart,
    pub level: u8,
    pub song_id: Option<String>,
    pub title: String,
    pub lamp: Lamp,
    /// `None` with NO PLAY.
    pub dj_level: Option<DjLevel>,
    pub score: u32,
    /// `None` when shown as "----".
    pub miss_count: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OptionLabel {
    pub style: PlayStyle,
    pub gauge: Gauge,
}

/// A label of any screen kind, tagged with the kind for the annotator API.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "screen", rename_all = "lowercase")]
pub enum Label {
    Play(PlayLabel),
    Result(ResultLabel),
    Select(SelectLabel),
    Option(OptionLabel),
    Other,
}

impl Label {
    pub fn screen(&self) -> Screen {
        match self {
            Label::Play(_) => Screen::Play,
            Label::Result(_) => Screen::Result,
            Label::Select(_) => Screen::Select,
            Label::Option(_) => Screen::Option,
            Label::Other => Screen::Other,
        }
    }

    /// The JSON stored next to the image: the fields only (the folder says
    /// the kind), so existing play labels stay valid.
    pub fn to_file_json(&self) -> serde_json::Value {
        let mut v = serde_json::to_value(self).expect("labels serialize");
        if let Some(obj) = v.as_object_mut() {
            obj.remove("screen");
        }
        v
    }

    /// Reads a label file of the given screen kind.
    pub fn from_file_json(screen: Screen, json: &str) -> Result<Label, serde_json::Error> {
        let mut v: serde_json::Value = serde_json::from_str(json)?;
        if let Some(obj) = v.as_object_mut() {
            obj.insert("screen".into(), screen.folder().into());
        } else if screen == Screen::Other {
            v = serde_json::json!({ "screen": "other" });
        }
        serde_json::from_value(v)
    }
}

/// Consistency rules between the numbers on a screen (PLAN 4.2). Each
/// broken rule is one message; an empty list means the label is
/// self-consistent.
pub fn check(label: &Label) -> Vec<String> {
    let mut problems = Vec::new();
    match label {
        Label::Play(p) => {
            if p.judge.cb < p.judge.bd {
                problems.push(format!("CB {} < BD {}", p.judge.cb, p.judge.bd));
            }
        }
        Label::Result(r) => {
            let j = r.judge;
            let ex = j.pg * 2 + j.gr;
            if r.score != ex {
                problems.push(format!("SCORE {} ≠ PG×2+GR = {}", r.score, ex));
            }
            if let Some(miss) = r.miss_count
                && miss != j.bd + j.pr
            {
                problems.push(format!("MISS COUNT {miss} ≠ BD+PR = {}", j.bd + j.pr));
            }
            let reached = j.pg + j.gr + j.gd + r.combo_break;
            match r.dead {
                Some(dead) if reached != dead.note => problems.push(format!(
                    "PG+GR+GD+CB = {reached} ≠ DEAD POINT NOTE {}",
                    dead.note
                )),
                // Usually equal, but it can fall short: auto-scratch
                // options leave scratch notes unjudged, and releasing a
                // charge note early loses its end judgement without a combo
                // break. It can never exceed NOTES.
                None if reached > r.notes => {
                    problems.push(format!("PG+GR+GD+CB = {reached} > NOTES {}", r.notes))
                }
                _ => {}
            }
            if r.combo_break < j.bd {
                problems.push(format!("COMBO BREAK {} < BD {}", r.combo_break, j.bd));
            }
            if r.lamp == Lamp::FullCombo && r.combo_break != 0 {
                problems.push("FULL COMBO with combo breaks".to_owned());
            }
            if r.dead.is_some() && r.lamp != Lamp::Failed {
                problems.push("DEAD POINT but not FAILED".to_owned());
            }
            if r.lamp == Lamp::AssistClear && r.use_option.is_none() {
                problems.push("ASSIST CLEAR without a USE OPTION banner".to_owned());
            }
            // The best column is either all "—" (never played this chart
            // without assist options) or all filled.
            let best = [
                r.best_lamp.is_some(),
                r.best_dj_level.is_some(),
                r.best_score.is_some(),
            ];
            if best.iter().any(|&b| b) && !best.iter().all(|&b| b) {
                problems.push("best lamp, DJ LEVEL and SCORE: some \"—\", some not".to_owned());
            }
        }
        Label::Select(s) => {
            if (s.lamp == Lamp::NoPlay) != s.dj_level.is_none() {
                problems.push("DJ LEVEL must be empty exactly when NO PLAY".to_owned());
            }
        }
        Label::Option(_) | Label::Other => {}
    }
    problems
}

#[cfg(test)]
mod tests {
    use super::*;

    fn result() -> ResultLabel {
        // Shooting Star [SPN4] F-COMBO sample (PLAN 4.2).
        ResultLabel {
            side: Side::P1,
            chart: Chart::Spn,
            level: 4,
            song_id: None,
            title: "Shooting Star".into(),
            notes: 420,
            lamp: Lamp::FullCombo,
            best_lamp: None,
            dj_level: DjLevel::A,
            best_dj_level: None,
            score: 193 * 2 + 138,
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
            fast: 0,
            slow: 0,
            dead: None,
            use_option: None,
        }
    }

    #[test]
    fn consistent_result_passes_and_broken_one_is_reported() {
        assert!(check(&Label::Result(result())).is_empty());
        let mut r = result();
        r.score += 1;
        r.miss_count = Some(3);
        let problems = check(&Label::Result(r));
        assert_eq!(problems.len(), 2, "{problems:?}");
    }

    #[test]
    fn dead_point_replaces_the_notes_rule() {
        let mut r = result();
        r.lamp = Lamp::Failed;
        r.judge = ResultJudge {
            pg: 14,
            gr: 8,
            gd: 14,
            bd: 0,
            pr: 0,
        };
        r.combo_break = 6;
        r.score = 36;
        r.miss_count = None;
        r.dead = Some(DeadPoint {
            note: 42,
            measure: 12,
        });
        assert!(check(&Label::Result(r)).is_empty());
    }

    #[test]
    fn file_json_omits_the_kind_and_reads_old_play_labels() {
        let old =
            r#"{"layout":"sp-1p-graph-inner","judge":{"pg":1,"gr":2,"gd":3,"bd":0,"pr":0,"cb":0}}"#;
        let label = Label::from_file_json(Screen::Play, old).unwrap();
        assert!(matches!(label, Label::Play(_)));
        let v = label.to_file_json();
        assert!(v.get("screen").is_none());
        assert_eq!(
            Label::from_file_json(Screen::Play, &v.to_string()).unwrap(),
            label
        );
        assert_eq!(
            Label::from_file_json(Screen::Other, "{}").unwrap(),
            Label::Other
        );
    }
}
