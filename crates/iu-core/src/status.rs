//! What the engine reports: the snapshot shown by the app, the OBS pages
//! and the phone page.

use serde::{Deserialize, Serialize};

use crate::play::{Judge, PlayLayout};
use crate::result::ResultRecord;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Connection {
    #[default]
    Starting,
    /// No Spout sender with the configured name (OBS closed, filter missing).
    Waiting,
    Connected,
    UnsupportedResolution,
    Error,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub connection: Connection,
    /// Technical detail for errors (English, for logs and bug reports).
    pub detail: Option<String>,
    pub sender: String,
    pub resolution: Option<[u32; 2]>,
    /// Layout name while on the play screen.
    pub layout: Option<String>,
    pub current: Option<Judge>,
    /// The last finished play of the game day: shown on the result screen
    /// and until the next play starts.
    pub last_play: Option<LastPlay>,
    /// The last result screen recorded this game day.
    pub last_result: Option<ResultRecord>,
    /// The song being (or last) played, read from the play screen's info
    /// box; kept until the next play starts.
    pub current_song: Option<SongRef>,
    /// Whether Windows' Japanese OCR is available (`None`: not known yet).
    /// Without it, titles are only found from learned masks or by the user.
    pub ocr_available: Option<bool>,
    pub today: Judge,
    pub today_notes: u32,
    pub plays_today: u32,
    pub dev_capture: bool,
}

/// A song of the song list, as shown.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SongRef {
    pub id: String,
    pub title: String,
    pub artist: String,
}

impl From<&crate::songs::Song> for SongRef {
    fn from(s: &crate::songs::Song) -> Self {
        Self {
            id: s.id.clone(),
            title: s.title.clone(),
            artist: s.artist.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LastPlay {
    pub layout: String,
    pub judge: Judge,
}

impl LastPlay {
    pub fn new(layout: PlayLayout, judge: Judge) -> Self {
        Self {
            layout: layout.name().to_owned(),
            judge,
        }
    }
}

/// The latest snapshot, for whoever needs it on demand (a new WebSocket
/// client, `get_status`). Updated by the engine alongside each event.
pub type SharedStatus = std::sync::Arc<std::sync::Mutex<Status>>;
