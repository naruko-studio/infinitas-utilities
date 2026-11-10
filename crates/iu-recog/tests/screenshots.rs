//! Screenshot regression tests. They run only where the screenshot folder
//! exists (see `iu-fixtures`); elsewhere they print a notice and pass.

use std::path::PathBuf;

use iu_core::label::{Label, Screen};
use iu_core::songs::SongList;
use iu_core::{Judge, PlayLayout};
use iu_recog::result::ResultSymbols;
use iu_recog::title::{TitleMatch, TitleResources, identify};
use iu_recog::{PlayResources, ResultReading, ResultResources};

fn play_resources(width: u32, height: u32) -> Result<PlayResources, String> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../resources")
        .join(format!("{width}x{height}"))
        .join("play.json");
    let json = std::fs::read_to_string(&path)
        .map_err(|e| format!("no play resources for {width}x{height}: {e}"))?;
    PlayResources::from_json(&json).map_err(|e| e.to_string())
}

/// Every labelled play screenshot: right layout and exact judge counts.
#[test]
fn play_screens_read_correctly() {
    iu_fixtures::run("play", |fixture, img| {
        let label = fixture.label.as_ref().ok_or("no label")?;
        let want_layout: PlayLayout =
            serde_json::from_value(label["layout"].clone()).map_err(|e| e.to_string())?;
        let want_judge: Judge =
            serde_json::from_value(label["judge"].clone()).map_err(|e| e.to_string())?;
        let res = play_resources(fixture.width, fixture.height)?;
        let anchor = res.detect(img).ok_or("not detected as a play screen")?;
        if anchor.layout != want_layout {
            return Err(format!("layout {} (want {want_layout})", anchor.layout));
        }
        let judge = res.read_judge(img, &anchor).map_err(|e| e.to_string())?;
        if judge != want_judge {
            return Err(format!("judge {judge:?} (want {want_judge:?})"));
        }
        Ok(())
    });
}

/// No other screen may be mistaken for a play screen.
#[test]
fn other_screens_are_not_play() {
    for screen in ["result", "select", "option", "other"] {
        iu_fixtures::run(screen, |fixture, img| {
            let res = play_resources(fixture.width, fixture.height)?;
            match res.detect(img) {
                None => Ok(()),
                Some(a) => Err(format!("detected as play screen ({})", a.layout)),
            }
        });
    }
}

fn result_resources(width: u32, height: u32) -> Result<ResultResources, String> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../resources")
        .join(format!("{width}x{height}"))
        .join("result.json");
    let json = std::fs::read_to_string(&path)
        .map_err(|e| format!("no result resources for {width}x{height}: {e}"))?;
    ResultResources::from_json(&json).map_err(|e| e.to_string())
}

/// Every labelled result screenshot: right side and every number exact.
#[test]
fn result_screens_read_correctly() {
    iu_fixtures::run("result", |fixture, img| {
        let label = fixture.label.as_ref().ok_or("no label")?;
        let Label::Result(want) =
            Label::from_file_json(Screen::Result, &label.to_string()).map_err(|e| e.to_string())?
        else {
            return Err("not a result label".into());
        };
        let res = result_resources(fixture.width, fixture.height)?;
        let side = res.detect(img).ok_or("not detected as a result screen")?;
        let got = res.read(img, side).map_err(|e| e.to_string())?;
        let expected = ResultReading {
            side: want.side,
            judge: want.judge,
            combo_break: want.combo_break,
            fast: want.fast,
            slow: want.slow,
            score: want.score,
            best_score: want.best_score,
            miss_count: want.miss_count,
            best_miss_count: want.best_miss_count,
            notes: want.notes,
        };
        if got != expected {
            return Err(format!("read {got:?}\n want {expected:?}"));
        }
        let symbols = res.read_symbols(img, side);
        let want_symbols = ResultSymbols {
            lamp: Some(want.lamp),
            best_lamp: Some(want.best_lamp),
            dj_level: Some(want.dj_level),
            best_dj_level: Some(want.best_dj_level),
            chart: Some(want.chart),
            level: Some(want.level),
            dead: Some(want.dead.is_some()),
            use_option: Some(want.use_option.clone()),
        };
        if symbols != want_symbols {
            return Err(format!("symbols {symbols:?}\n want {want_symbols:?}"));
        }
        Ok(())
    });
}

/// Every labelled result screenshot with a song: the chart, level and NOTES
/// count plus the title mask name the right song.
#[test]
fn result_titles_are_recognized() {
    let songs = SongList::from_json(include_str!("../../../resources/songs.json"))
        .expect("bundled song list");
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
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../resources")
            .join(format!("{}x{}", fixture.width, fixture.height))
            .join("titles.json");
        let json = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
        let res = TitleResources::from_json(&json).map_err(|e| e.to_string())?;
        let mask = res.mask(img).ok_or("wrong size")?;
        let candidates: Vec<&str> = songs
            .candidates(Some(want.chart), Some(want.level), want.notes)
            .into_iter()
            .map(|s| s.id.as_str())
            .collect();
        match identify(&mask, &res.titles, &candidates, res.max_distance) {
            TitleMatch::OnlyCandidate(id) | TitleMatch::Recognized { song_id: id, .. }
                if id == want_id =>
            {
                Ok(())
            }
            other => Err(format!(
                "{}: got {other:?} among {} candidates",
                want.title,
                candidates.len()
            )),
        }
    });
}

/// No other screen may be mistaken for a result screen.
#[test]
fn other_screens_are_not_result() {
    for screen in ["play", "select", "option", "other"] {
        iu_fixtures::run(screen, |fixture, img| {
            let res = result_resources(fixture.width, fixture.height)?;
            match res.detect(img) {
                None => Ok(()),
                Some(s) => Err(format!("detected as result screen ({:?})", s.side)),
            }
        });
    }
}

/// Every labelled play screenshot: the info box's title line lies wholly
/// inside its area (ink, but none on the area's edge rows).
#[test]
fn play_title_lines_fit_their_area() {
    use iu_recog::title::{TitleMask, play_title_area};
    iu_fixtures::run("play", |fixture, img| {
        let label = fixture.label.as_ref().ok_or("no label")?;
        let layout: PlayLayout =
            serde_json::from_value(label["layout"].clone()).map_err(|e| e.to_string())?;
        let area = play_title_area(fixture.width, fixture.height, layout)
            .ok_or("no play title area for this resolution")?;
        let mask = TitleMask::from_image(img, area);
        if mask.count_ones() < 40 {
            return Err(format!("only {} ink pixels", mask.count_ones()));
        }
        let edge = [0, 1, area.height - 2, area.height - 1];
        for y in edge {
            if (0..area.width).any(|x| mask.get(x, y)) {
                return Err(format!("ink on edge row {y} of {}", area.height));
            }
        }
        Ok(())
    });
}
