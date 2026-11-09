//! Play-screen types.

use std::fmt;
use std::str::FromStr;

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

/// The six judge counts shown on the play screen's judge panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Judge {
    pub pg: u32,
    pub gr: u32,
    pub gd: u32,
    pub bd: u32,
    pub pr: u32,
    pub cb: u32,
}

impl Judge {
    pub fn from_array([pg, gr, gd, bd, pr, cb]: [u32; 6]) -> Self {
        Self {
            pg,
            gr,
            gd,
            bd,
            pr,
            cb,
        }
    }

    pub fn to_array(self) -> [u32; 6] {
        [self.pg, self.gr, self.gd, self.bd, self.pr, self.cb]
    }

    /// EX SCORE = PG × 2 + GR.
    pub fn ex_score(self) -> u32 {
        self.pg * 2 + self.gr
    }

    /// Keystrokes that hit a note (PG + GR + GD + BD). Empty POORs are not
    /// counted, matching the reference counter's "notes" total.
    pub fn notes_hit(self) -> u32 {
        self.pg + self.gr + self.gd + self.bd
    }

    /// True if no count went down compared to `earlier` (same play).
    pub fn is_monotonic_after(self, earlier: Judge) -> bool {
        self.to_array()
            .iter()
            .zip(earlier.to_array())
            .all(|(&now, then)| now >= then)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PlayStyle {
    Sp,
    Dp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Side {
    #[serde(rename = "1p")]
    P1,
    #[serde(rename = "2p")]
    P2,
}

/// Arrangement of the play screen. Written as kebab-case strings in
/// resources and labels, e.g. `sp-1p-graph-inner`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PlayLayout {
    /// SP. `graph_inner`: GRAPH INFORMATION sits between the lane and the
    /// BGA (default); otherwise it is swapped with the BGA.
    Sp { side: Side, graph_inner: bool },
    /// DP. `info_left`: the info column (judge panel, BPM) is on the left.
    Dp { info_left: bool },
}

impl PlayLayout {
    pub const ALL: [PlayLayout; 6] = [
        PlayLayout::Sp {
            side: Side::P1,
            graph_inner: true,
        },
        PlayLayout::Sp {
            side: Side::P1,
            graph_inner: false,
        },
        PlayLayout::Sp {
            side: Side::P2,
            graph_inner: true,
        },
        PlayLayout::Sp {
            side: Side::P2,
            graph_inner: false,
        },
        PlayLayout::Dp { info_left: true },
        PlayLayout::Dp { info_left: false },
    ];

    pub fn style(self) -> PlayStyle {
        match self {
            PlayLayout::Sp { .. } => PlayStyle::Sp,
            PlayLayout::Dp { .. } => PlayStyle::Dp,
        }
    }

    pub fn side(self) -> Option<Side> {
        match self {
            PlayLayout::Sp { side, .. } => Some(side),
            PlayLayout::Dp { .. } => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            PlayLayout::Sp {
                side: Side::P1,
                graph_inner: true,
            } => "sp-1p-graph-inner",
            PlayLayout::Sp {
                side: Side::P1,
                graph_inner: false,
            } => "sp-1p-graph-outer",
            PlayLayout::Sp {
                side: Side::P2,
                graph_inner: true,
            } => "sp-2p-graph-inner",
            PlayLayout::Sp {
                side: Side::P2,
                graph_inner: false,
            } => "sp-2p-graph-outer",
            PlayLayout::Dp { info_left: true } => "dp-info-left",
            PlayLayout::Dp { info_left: false } => "dp-info-right",
        }
    }
}

impl fmt::Display for PlayLayout {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

impl FromStr for PlayLayout {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|l| l.name() == s)
            .ok_or_else(|| format!("unknown play layout `{s}`"))
    }
}

impl Serialize for PlayLayout {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(self.name())
    }
}

impl<'de> Deserialize<'de> for PlayLayout {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

/// A finished play, as stored in `data.db` and sent to the web pages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayRecord {
    /// Game day (see [`crate::day`]).
    pub game_day: NaiveDate,
    pub started_at: DateTime<Utc>,
    pub finished_at: DateTime<Utc>,
    pub layout: PlayLayout,
    pub judge: Judge,
}

/// Totals for one game day.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DaySummary {
    pub judge: Judge,
    /// Plays with at least one note hit (same rule as the counter).
    pub plays: u32,
}

/// One game day's totals, for history lists.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DayTotal {
    pub game_day: NaiveDate,
    #[serde(flatten)]
    pub summary: DaySummary,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_names_round_trip() {
        for layout in PlayLayout::ALL {
            assert_eq!(layout.name().parse::<PlayLayout>(), Ok(layout));
            let json = serde_json::to_string(&layout).unwrap();
            assert_eq!(serde_json::from_str::<PlayLayout>(&json).unwrap(), layout);
        }
        assert!("sp-3p".parse::<PlayLayout>().is_err());
    }

    #[test]
    fn judge_totals() {
        let j = Judge::from_array([215, 132, 134, 11, 18, 19]);
        assert_eq!(j.ex_score(), 562);
        assert_eq!(j.notes_hit(), 492);
        assert!(j.is_monotonic_after(Judge::from_array([200, 132, 100, 11, 18, 19])));
        assert!(!j.is_monotonic_after(Judge::from_array([216, 0, 0, 0, 0, 0])));
    }
}
