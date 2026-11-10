//! Screenshot regression test: Windows OCR plus the song matching name the
//! song of every labelled result screen. Runs only where the screenshots
//! and Japanese OCR exist.

use iu_core::label::{Label, Screen};
use iu_core::songs::{SongList, match_text};
use iu_ocr::{Ocr, areas};

#[test]
fn result_titles_are_read() {
    let Ok(ocr) = Ocr::new() else {
        eprintln!("no Japanese OCR on this machine; skipped");
        return;
    };
    let songs = SongList::from_json(include_str!("../../../resources/songs.json")).unwrap();
    iu_fixtures::run("result", |fixture, img| {
        let label = fixture.label.as_ref().ok_or("no label")?;
        let Label::Result(want) =
            Label::from_file_json(Screen::Result, &label.to_string()).map_err(|e| e.to_string())?
        else {
            return Err("not a result label".into());
        };
        let Some(want_id) = want.song_id else {
            return Ok(());
        };
        let rect = areas::result_title(fixture.width, fixture.height).ok_or("no area")?;
        let lines = ocr.read_lines(img, rect).map_err(|e| e.to_string())?;
        let candidates = songs.candidates(Some(want.chart), Some(want.level), want.notes);
        match match_text(&lines, candidates) {
            Some(m) if m.song.id == want_id => Ok(()),
            other => Err(format!(
                "{}: read {lines:?}, matched {:?}",
                want.title,
                other.map(|m| &m.song.title)
            )),
        }
    });
}
