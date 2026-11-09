//! The annotator (developer screen): browse screenshots, label them, and
//! file them under `dev-capture\<w>x<h>\<screen>\` for `resgen` and the
//! regression tests.
//!
//! Sources: the dev-capture folders (images are moved between them) and the
//! game's F12 folder (images are copied; the game's folder is never
//! changed). Only files under these roots are read or written.

use std::path::{Path, PathBuf};

use iu_core::label::{self, DeadPoint, DjLevel, Label, Lamp, PlayLabel, ResultLabel, Screen};
use iu_core::songs::Chart;
use iu_recog::{PlayResources, ResultResources};
use serde::{Deserialize, Serialize};

use crate::engine::{PLAY_RESOURCES, RESULT_RESOURCES};

/// Sub-folders of `dev-capture\<w>x<h>\`, besides the screen kinds.
const UNSORTED: &str = "unsorted";
const DISCARDED: &str = "discarded";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Source {
    DevCapture,
    Game,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Shot {
    pub path: String,
    pub name: String,
    pub source: Source,
    /// dev-capture sub-folder (`unsorted`, `play`, …); `None` for the game
    /// folder.
    pub folder: Option<String>,
    pub width: u32,
    pub height: u32,
    pub labeled: bool,
    /// Seconds since the epoch, for sorting.
    pub modified: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Loaded {
    /// The saved label, if the image is already filed.
    pub label: Option<Label>,
    /// What the recognizer reads now (play screens only for now).
    pub suggestion: Option<Label>,
}

pub struct Annotator {
    dev_capture: PathBuf,
    game_dir: PathBuf,
    play: Vec<PlayResources>,
    result: Vec<ResultResources>,
}

impl Annotator {
    pub fn new(game_dir: PathBuf) -> Option<Self> {
        let dev_capture = dirs::picture_dir()?
            .join("INFINITAS Screenshots")
            .join("dev-capture");
        let play = PLAY_RESOURCES
            .iter()
            .filter_map(|j| PlayResources::from_json(j).ok())
            .collect();
        Some(Self {
            dev_capture,
            game_dir,
            play,
            result: RESULT_RESOURCES
                .iter()
                .filter_map(|j| ResultResources::from_json(j).ok())
                .collect(),
        })
    }

    pub fn list(&self) -> Vec<Shot> {
        let mut shots = Vec::new();
        for res_dir in read_dirs(&self.dev_capture) {
            for folder_dir in read_dirs(&res_dir) {
                let folder = file_name(&folder_dir);
                if folder == DISCARDED {
                    continue;
                }
                for png in pngs(&folder_dir) {
                    if let Some(shot) = shot(&png, Source::DevCapture, Some(folder.clone())) {
                        shots.push(shot);
                    }
                }
            }
        }
        for png in pngs(&self.game_dir) {
            if let Some(shot) = shot(&png, Source::Game, None) {
                shots.push(shot);
            }
        }
        shots.sort_by_key(|s| std::cmp::Reverse(s.modified));
        shots
    }

    /// The PNG bytes of an image under one of the roots.
    pub fn image(&self, path: &str) -> Result<Vec<u8>, String> {
        let path = self.checked(path)?;
        std::fs::read(path).map_err(|e| e.to_string())
    }

    pub fn load(&self, path: &str) -> Result<Loaded, String> {
        let path = self.checked(path)?;
        let label = self.saved_label(&path)?;
        let suggestion = image::open(&path)
            .ok()
            .map(|img| img.to_rgb8())
            .and_then(|img| self.suggest(&img));
        Ok(Loaded { label, suggestion })
    }

    /// Files the image under its screen kind with `label`; returns the new
    /// path. Game screenshots are copied, dev-capture images are moved.
    pub fn save(&self, path: &str, label: &Label) -> Result<String, String> {
        let src = self.checked(path)?;
        let (w, h) = image::image_dimensions(&src).map_err(|e| e.to_string())?;
        let dir = self
            .dev_capture
            .join(format!("{w}x{h}"))
            .join(label.screen().folder());
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let dest = dir.join(src.file_name().ok_or("no file name")?);
        if dest != src {
            if self.is_game(&src) {
                std::fs::copy(&src, &dest).map_err(|e| e.to_string())?;
            } else {
                std::fs::rename(&src, &dest).map_err(|e| e.to_string())?;
                let _ = std::fs::remove_file(src.with_extension("json"));
            }
        }
        let json = serde_json::to_string_pretty(&label.to_file_json()).expect("serializable");
        std::fs::write(dest.with_extension("json"), json + "\n").map_err(|e| e.to_string())?;
        Ok(dest.display().to_string())
    }

    /// Moves a dev-capture image (and its label) to `discarded\`, out of the
    /// list and the tests. Game screenshots cannot be discarded here.
    pub fn discard(&self, path: &str) -> Result<(), String> {
        let src = self.checked(path)?;
        if self.is_game(&src) {
            return Err("game screenshots are not changed by the annotator".into());
        }
        let res_dir = src
            .parent()
            .and_then(Path::parent)
            .ok_or("unexpected location")?;
        let dir = res_dir.join(DISCARDED);
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let name = src.file_name().ok_or("no file name")?;
        std::fs::rename(&src, dir.join(name)).map_err(|e| e.to_string())?;
        let _ = std::fs::remove_file(src.with_extension("json"));
        Ok(())
    }

    fn suggest(&self, img: &image::RgbImage) -> Option<Label> {
        self.suggest_play(img).or_else(|| self.suggest_result(img))
    }

    fn suggest_play(&self, img: &image::RgbImage) -> Option<Label> {
        let res = self
            .play
            .iter()
            .find(|r| (r.width, r.height) == img.dimensions())?;
        let anchor = res.detect(img)?;
        let judge = res.read_judge(img, &anchor).ok()?;
        Some(Label::Play(PlayLabel {
            layout: anchor.layout,
            judge,
        }))
    }

    /// Numbers and every recognized word; what is not recognized (song,
    /// classes never seen yet) gets a placeholder for the person to set.
    fn suggest_result(&self, img: &image::RgbImage) -> Option<Label> {
        let res = self
            .result
            .iter()
            .find(|r| (r.width, r.height) == img.dimensions())?;
        let side = res.detect(img)?;
        let r = res.read(img, side).ok()?;
        let sym = res.read_symbols(img, side);
        // The dead note equals PG+GR+GD+CB (PLAN 4.2); the measure is not
        // read yet.
        let dead = sym.dead.unwrap_or(false).then(|| DeadPoint {
            note: r.judge.pg + r.judge.gr + r.judge.gd + r.combo_break,
            measure: 0,
        });
        Some(Label::Result(ResultLabel {
            side: r.side,
            chart: sym.chart.unwrap_or(Chart::Spn),
            level: sym.level.unwrap_or(1),
            song_id: None,
            title: String::new(),
            notes: r.notes,
            lamp: sym.lamp.unwrap_or(Lamp::Clear),
            best_lamp: sym.best_lamp.flatten(),
            dj_level: sym.dj_level.unwrap_or(DjLevel::A),
            best_dj_level: sym.best_dj_level.flatten(),
            score: r.score,
            best_score: r.best_score,
            miss_count: r.miss_count,
            best_miss_count: r.best_miss_count,
            judge: r.judge,
            combo_break: r.combo_break,
            fast: r.fast,
            slow: r.slow,
            dead,
            use_option: sym.use_option.flatten(),
        }))
    }

    fn saved_label(&self, path: &Path) -> Result<Option<Label>, String> {
        if self.is_game(path) {
            return Ok(None);
        }
        let Some(screen) = path
            .parent()
            .and_then(|p| file_name(p).parse::<Screen>().ok())
        else {
            return Ok(None);
        };
        match std::fs::read_to_string(path.with_extension("json")) {
            Ok(json) => Label::from_file_json(screen, &json)
                .map(Some)
                .map_err(|e| format!("broken label: {e}")),
            Err(_) => Ok(None),
        }
    }

    fn is_game(&self, path: &Path) -> bool {
        canonical(&self.game_dir).is_some_and(|g| path.starts_with(g))
    }

    /// Resolves `path` and refuses anything outside the two roots, or not a
    /// PNG.
    fn checked(&self, path: &str) -> Result<PathBuf, String> {
        let p = canonical(Path::new(path)).ok_or("file not found")?;
        let inside = [&self.dev_capture, &self.game_dir]
            .into_iter()
            .filter_map(|r| canonical(r))
            .any(|root| p.starts_with(root));
        let png = p.extension().is_some_and(|e| e.eq_ignore_ascii_case("png"));
        if inside && png {
            Ok(p)
        } else {
            Err("not a screenshot in the annotator's folders".into())
        }
    }
}

pub fn check(label: &Label) -> Vec<String> {
    label::check(label)
}

fn canonical(p: &Path) -> Option<PathBuf> {
    dunce_canonicalize(p)
}

/// `canonicalize` without the `\\?\` prefix, so paths compare and display
/// like the ones built by hand.
fn dunce_canonicalize(p: &Path) -> Option<PathBuf> {
    let c = std::fs::canonicalize(p).ok()?;
    let s = c.to_string_lossy();
    Some(match s.strip_prefix(r"\\?\") {
        Some(rest) if !rest.starts_with("UNC") => PathBuf::from(rest),
        _ => c,
    })
}

fn read_dirs(dir: &Path) -> Vec<PathBuf> {
    std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect()
}

fn pngs(dir: &Path) -> Vec<PathBuf> {
    std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("png")))
        .collect()
}

fn file_name(p: &Path) -> String {
    p.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

fn shot(path: &Path, source: Source, folder: Option<String>) -> Option<Shot> {
    let (width, height) = image::image_dimensions(path).ok()?;
    let modified = std::fs::metadata(path)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map_or(0, |d| d.as_secs());
    let labeled = folder
        .as_deref()
        .is_some_and(|f| f != UNSORTED && path.with_extension("json").exists());
    Some(Shot {
        path: path.display().to_string(),
        name: file_name(path),
        source,
        folder,
        width,
        height,
        labeled,
        modified,
    })
}
