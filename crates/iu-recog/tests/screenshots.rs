//! Screenshot regression tests. They run only where the screenshot folder
//! exists (see `iu-fixtures`); elsewhere they print a notice and pass.

use std::path::PathBuf;

use iu_core::{Judge, PlayLayout};
use iu_recog::PlayResources;

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
