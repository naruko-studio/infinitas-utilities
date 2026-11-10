//! The bundled song list, parsed once.

use std::sync::OnceLock;

use iu_core::songs::SongList;

/// `resources/songs.json`. It is checked by the tests, so a broken one
/// never ships; if it somehow is, songs are simply not recognized.
pub fn song_list() -> &'static SongList {
    static SONGS: OnceLock<SongList> = OnceLock::new();
    SONGS.get_or_init(|| {
        SongList::from_json(include_str!("../../../resources/songs.json")).unwrap_or_else(|e| {
            tracing::error!("bundled song list is broken: {e}");
            SongList {
                source: String::new(),
                generated: String::new(),
                songs: Vec::new(),
            }
        })
    })
}
