//! Keystroke counting from per-frame judge readings.
//!
//! The recognizer reports what it sees on each frame; this state machine
//! turns that into "the current play" and "today's total":
//! - a play starts when the play screen appears, or when the counts drop
//!   (retry / next song without leaving the screen);
//! - a play ends when the play screen is gone, and its last trusted counts
//!   are added to the day;
//! - a reading that goes backwards or jumps too far is dropped as a
//!   misread instead of corrupting the total.

use serde::{Deserialize, Serialize};

use crate::play::{Judge, PlayLayout};

/// More new notes than this between two accepted frames is treated as a
/// misread. At 30 fps even the densest charts add far fewer per frame;
/// the margin also covers a few dropped frames.
pub const MAX_NOTES_PER_STEP: u32 = 200;

/// Readings that went backwards and keep agreeing with each other for this
/// many frames mean a restart we did not see happen (the zero frames were
/// missed), not a misread. A misread is a one-off; ~1/3 s at 30 fps is not.
pub const RESTART_CONFIRM_FRAMES: u32 = 10;

/// What the recognizer saw on one frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Observation {
    /// Not a play screen (select, result, loading…).
    NotPlay,
    /// Play screen, counts read.
    Play { layout: PlayLayout, judge: Judge },
    /// Play screen, but the counts could not be read this frame.
    PlayUnreadable { layout: PlayLayout },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CurrentPlay {
    pub layout: PlayLayout,
    pub judge: Judge,
}

/// Something worth telling the rest of the app.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CounterEvent {
    PlayStarted {
        layout: PlayLayout,
    },
    JudgeUpdated {
        judge: Judge,
    },
    /// The play ended; `judge` is what was added to today's total.
    PlayFinished {
        layout: PlayLayout,
        judge: Judge,
    },
    /// A reading was rejected as implausible.
    Rejected {
        reading: Judge,
        previous: Judge,
    },
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Counter {
    /// Sum of finished plays today.
    pub today: Judge,
    /// Number of finished plays today.
    pub plays_today: u32,
    pub current: Option<CurrentPlay>,
    /// Consecutive backwards readings: the latest one and how many.
    #[serde(skip)]
    backwards: Option<(Judge, u32)>,
}

impl Counter {
    /// Today's total including the play in progress.
    pub fn today_with_current(&self) -> Judge {
        match self.current {
            Some(c) => add(self.today, c.judge),
            None => self.today,
        }
    }

    pub fn observe(&mut self, obs: Observation) -> Vec<CounterEvent> {
        let mut events = Vec::new();
        match obs {
            Observation::NotPlay => self.finish(&mut events),
            Observation::PlayUnreadable { layout } => {
                if self.current.is_none() {
                    self.start(layout, Judge::default(), &mut events);
                }
            }
            Observation::Play { layout, judge } => match self.current {
                None => {
                    self.start(layout, judge, &mut events);
                    if judge != Judge::default() {
                        events.push(CounterEvent::JudgeUpdated { judge });
                    }
                }
                // Only a reset of the counts means a new play. A layout
                // change alone (graph/BGA swapped mid-song) keeps the play:
                // the counts carry on, and the checks below still apply.
                Some(cur) if restarted(cur.judge, judge) => {
                    self.finish(&mut events);
                    self.start(layout, judge, &mut events);
                }
                Some(cur) if judge == cur.judge => {
                    self.backwards = None;
                    self.current = Some(CurrentPlay { layout, judge });
                }
                Some(cur) => {
                    let step = judge.notes_hit().saturating_sub(cur.judge.notes_hit());
                    if judge.is_monotonic_after(cur.judge) && step <= MAX_NOTES_PER_STEP {
                        self.backwards = None;
                        self.current = Some(CurrentPlay { layout, judge });
                        events.push(CounterEvent::JudgeUpdated { judge });
                    } else if self.confirms_unseen_restart(cur.judge, judge) {
                        self.finish(&mut events);
                        self.start(layout, judge, &mut events);
                        events.push(CounterEvent::JudgeUpdated { judge });
                    } else {
                        events.push(CounterEvent::Rejected {
                            reading: judge,
                            previous: cur.judge,
                        });
                    }
                }
            },
        }
        events
    }

    /// Clears today's totals (new day). The play in progress is kept.
    pub fn reset_day(&mut self) {
        self.today = Judge::default();
        self.plays_today = 0;
    }

    /// Tracks backwards readings; true once [`RESTART_CONFIRM_FRAMES`] of
    /// them in a row are consistent with each other.
    fn confirms_unseen_restart(&mut self, current: Judge, reading: Judge) -> bool {
        if total(reading) >= total(current) {
            // Too far ahead, not backwards: a plain misread.
            self.backwards = None;
            return false;
        }
        let count = match self.backwards {
            Some((last, n)) if reading.is_monotonic_after(last) => n + 1,
            _ => 1,
        };
        if count >= RESTART_CONFIRM_FRAMES {
            self.backwards = None;
            true
        } else {
            self.backwards = Some((reading, count));
            false
        }
    }

    fn start(&mut self, layout: PlayLayout, judge: Judge, events: &mut Vec<CounterEvent>) {
        self.backwards = None;
        self.current = Some(CurrentPlay { layout, judge });
        events.push(CounterEvent::PlayStarted { layout });
    }

    fn finish(&mut self, events: &mut Vec<CounterEvent>) {
        if let Some(cur) = self.current.take() {
            self.today = add(self.today, cur.judge);
            if cur.judge.notes_hit() > 0 {
                self.plays_today += 1;
            }
            events.push(CounterEvent::PlayFinished {
                layout: cur.layout,
                judge: cur.judge,
            });
        }
    }
}

/// The counts went back to zero: a new play on the same screen (retry
/// mid-song). Also covers a previous play with only POORs.
fn restarted(previous: Judge, now: Judge) -> bool {
    previous != Judge::default() && now == Judge::default()
}

fn total(j: Judge) -> u32 {
    j.to_array().iter().sum()
}

fn add(a: Judge, b: Judge) -> Judge {
    let (a, b) = (a.to_array(), b.to_array());
    Judge::from_array(std::array::from_fn(|i| a[i] + b[i]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::play::Side;

    const SP: PlayLayout = PlayLayout::Sp {
        side: Side::P1,
        graph_inner: true,
    };

    fn j(pg: u32, gr: u32) -> Judge {
        Judge {
            pg,
            gr,
            ..Judge::default()
        }
    }

    fn play(judge: Judge) -> Observation {
        Observation::Play { layout: SP, judge }
    }

    #[test]
    fn counts_a_play_into_today_when_it_ends() {
        let mut c = Counter::default();
        assert_eq!(
            c.observe(play(j(0, 0))),
            [CounterEvent::PlayStarted { layout: SP }]
        );
        c.observe(play(j(10, 2)));
        c.observe(play(j(30, 5)));
        assert_eq!(c.today_with_current().notes_hit(), 35);
        assert_eq!(c.today.notes_hit(), 0);
        let ev = c.observe(Observation::NotPlay);
        assert_eq!(
            ev,
            [CounterEvent::PlayFinished {
                layout: SP,
                judge: j(30, 5)
            }]
        );
        assert_eq!(c.today.notes_hit(), 35);
        assert_eq!(c.plays_today, 1);
        assert!(c.observe(Observation::NotPlay).is_empty());
    }

    #[test]
    fn rejects_readings_that_go_backwards_or_jump() {
        let mut c = Counter::default();
        c.observe(play(j(100, 10)));
        let ev = c.observe(play(j(90, 12)));
        assert!(matches!(ev[..], [CounterEvent::Rejected { .. }]));
        let ev = c.observe(play(j(400, 10)));
        assert!(matches!(ev[..], [CounterEvent::Rejected { .. }]));
        assert_eq!(c.current.unwrap().judge, j(100, 10));
        c.observe(play(j(105, 11)));
        assert_eq!(c.current.unwrap().judge, j(105, 11));
    }

    #[test]
    fn counts_reset_on_screen_means_a_new_play() {
        let mut c = Counter::default();
        c.observe(play(j(50, 5)));
        let ev = c.observe(play(j(0, 0)));
        assert_eq!(
            ev,
            [
                CounterEvent::PlayFinished {
                    layout: SP,
                    judge: j(50, 5)
                },
                CounterEvent::PlayStarted { layout: SP },
            ]
        );
        assert_eq!(c.today.notes_hit(), 55);
    }

    #[test]
    fn restart_after_only_poors_is_a_new_play() {
        let mut c = Counter::default();
        let poors = Judge {
            pr: 50,
            cb: 50,
            ..Judge::default()
        };
        c.observe(play(poors));
        let ev = c.observe(play(j(0, 0)));
        assert!(matches!(
            ev[..],
            [
                CounterEvent::PlayFinished { .. },
                CounterEvent::PlayStarted { .. }
            ]
        ));
        assert_eq!(c.today.pr, 50);
        c.observe(play(j(1, 0)));
        assert_eq!(c.current.unwrap().judge, j(1, 0));
    }

    #[test]
    fn restart_without_seeing_zero_is_confirmed_after_consistent_frames() {
        let mut c = Counter::default();
        c.observe(play(j(300, 100)));
        // The retry's zero frames were missed; readings resume at 3.
        for i in 0..RESTART_CONFIRM_FRAMES - 1 {
            let ev = c.observe(play(j(3 + i / 3, 0)));
            assert!(matches!(ev[..], [CounterEvent::Rejected { .. }]));
        }
        let ev = c.observe(play(j(6, 0)));
        assert!(matches!(
            ev[..],
            [
                CounterEvent::PlayFinished { .. },
                CounterEvent::PlayStarted { .. },
                CounterEvent::JudgeUpdated { .. }
            ]
        ));
        assert_eq!(c.today.notes_hit(), 400);
        assert_eq!(c.current.unwrap().judge, j(6, 0));
    }

    #[test]
    fn a_one_off_backwards_misread_does_not_restart() {
        let mut c = Counter::default();
        c.observe(play(j(300, 100)));
        for _ in 0..RESTART_CONFIRM_FRAMES * 2 {
            // Misread alternating with good readings.
            c.observe(play(j(30, 100)));
            c.observe(play(j(300, 100)));
        }
        assert_eq!(c.current.unwrap().judge, j(300, 100));
        assert_eq!(c.plays_today, 0);
    }

    #[test]
    fn layout_change_mid_song_keeps_the_play() {
        const OUTER: PlayLayout = PlayLayout::Sp {
            side: Side::P1,
            graph_inner: false,
        };
        let mut c = Counter::default();
        c.observe(Observation::Play {
            layout: OUTER,
            judge: j(66, 78),
        });
        let ev = c.observe(play(j(70, 80)));
        assert_eq!(ev, [CounterEvent::JudgeUpdated { judge: j(70, 80) }]);
        assert_eq!(c.current.unwrap().layout, SP);
        c.observe(Observation::NotPlay);
        assert_eq!(c.plays_today, 1);
        assert_eq!(c.today.notes_hit(), 150);
    }

    #[test]
    fn unreadable_frames_keep_the_play_going() {
        let mut c = Counter::default();
        c.observe(play(j(20, 0)));
        assert!(
            c.observe(Observation::PlayUnreadable { layout: SP })
                .is_empty()
        );
        c.observe(play(j(25, 0)));
        assert_eq!(c.current.unwrap().judge, j(25, 0));
    }

    #[test]
    fn empty_plays_do_not_count_as_plays() {
        let mut c = Counter::default();
        c.observe(play(j(0, 0)));
        c.observe(Observation::NotPlay);
        assert_eq!(c.plays_today, 0);
    }
}
