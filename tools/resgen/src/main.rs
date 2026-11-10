//! Generates recognition resources from our own labelled screenshots.
//!
//!   cargo resources                         (= resgen all, see .cargo/config.toml)
//!   cargo run --release -p resgen -- all    check labels, generate everything, run the tests
//!   cargo run --release -p resgen -- play   only resources/<w>x<h>/play.json
//!   cargo run --release -p resgen -- labels only check the labels
//!
//! Reads `<fixtures>/dev-capture/<w>x<h>/<screen>/*.png` with their `.json`
//! labels (written by the app's annotator). Fixtures are found the same way
//! as in the tests (`IU_FIXTURES_DIR` or Pictures).

mod play;
mod result;
mod result_symbols;
mod titles;

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, bail};
use iu_core::label::{self, Label, Screen};
use iu_core::songs::SongList;

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let root = iu_fixtures::root().context("no screenshot folder (set IU_FIXTURES_DIR)")?;
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let out_dir = repo.join("resources");
    match args.first().map(String::as_str) {
        Some("play") => play::run(&root, &out_dir),
        Some("result") => result::run(&root, &out_dir),
        Some("titles") => titles::run(&root, &out_dir),
        Some("labels") => check_labels(&root),
        Some("all") => {
            check_labels(&root)?;
            play::run(&root, &out_dir)?;
            result::run(&root, &out_dir)?;
            titles::run(&root, &out_dir)?;
            run_tests(&repo)
        }
        _ => bail!("usage: resgen all | play | result | titles | labels"),
    }
}

/// Every label must parse for its folder's screen kind and pass the
/// consistency rules; a broken label would silently teach the wrong thing.
fn check_labels(root: &Path) -> anyhow::Result<()> {
    let mut checked = 0;
    let mut bad = Vec::new();
    for screen in Screen::ALL {
        for fixture in iu_fixtures::collect_in(root, screen.folder()) {
            let Some(value) = &fixture.label else {
                continue;
            };
            checked += 1;
            let name = fixture.path.display();
            match Label::from_file_json(screen, &value.to_string()) {
                Ok(l) => {
                    for problem in label::check(&l).into_iter().chain(against_song_list(&l)) {
                        bad.push(format!("{name}: {problem}"));
                    }
                }
                Err(e) => bad.push(format!("{name}: unreadable label: {e}")),
            }
        }
    }
    println!("labels: {checked} checked, {} problems", bad.len());
    for b in &bad {
        println!("  {b}");
    }
    if bad.is_empty() {
        Ok(())
    } else {
        bail!("fix the labels above (in the app's annotator) first")
    }
}

/// A label naming a song must agree with the song list on the chart's level
/// and note count: catches mislabels and song-list errors alike.
fn against_song_list(l: &Label) -> Vec<String> {
    static SONGS: std::sync::OnceLock<SongList> = std::sync::OnceLock::new();
    let songs = SONGS.get_or_init(|| {
        SongList::from_json(include_str!("../../../resources/songs.json"))
            .expect("resources/songs.json is valid")
    });
    let (song_id, chart, level, notes) = match l {
        Label::Result(r) => (&r.song_id, r.chart, r.level, Some(r.notes)),
        Label::Select(s) => (&s.song_id, s.chart, s.level, None),
        _ => return Vec::new(),
    };
    let Some(id) = song_id else {
        return Vec::new();
    };
    let Some(song) = songs.songs.iter().find(|s| &s.id == id) else {
        return vec![format!("song id {id} is not in the song list")];
    };
    let Some(info) = song.charts.get(&chart) else {
        return vec![format!("{}: no {chart} chart in the song list", song.title)];
    };
    let mut out = Vec::new();
    if info.level != level {
        out.push(format!(
            "{} {chart}: level {level}, song list says {}",
            song.title, info.level
        ));
    }
    if let (Some(n), Some(listed)) = (notes, info.notes)
        && n != listed
    {
        out.push(format!(
            "{} {chart}: NOTES {n}, song list says {listed}",
            song.title
        ));
    }
    out
}

/// The screenshot regression tests, required to find the fixtures.
fn run_tests(repo: &Path) -> anyhow::Result<()> {
    println!("running the screenshot regression tests…");
    let status = Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".into()))
        .args(["test", "--release", "-p", "iu-recog", "-p", "iu-core"])
        .env("IU_REQUIRE_FIXTURES", "1")
        .current_dir(repo)
        .status()
        .context("cannot run cargo test")?;
    if !status.success() {
        bail!("regression tests failed");
    }
    println!("all done: resources regenerated and tests passed");
    Ok(())
}
