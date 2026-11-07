//! Screenshot regression-test harness.
//!
//! Game screenshots are copyrighted (Konami) and never go into the repo.
//! They live in the user's Pictures folder:
//!
//! ```text
//! <Pictures>\INFINITAS Screenshots\dev-capture\<w>x<h>\<screen>\*.png
//!                                                        \*.json  (label, optional)
//! ```
//!
//! - `IU_FIXTURES_DIR` overrides `<Pictures>\INFINITAS Screenshots`.
//! - When the folder is absent (CI, other machines) the tests print a
//!   notice and pass, so CI stays green without screenshots.
//! - `IU_REQUIRE_FIXTURES=1` turns "absent" into a failure, for local runs
//!   that must not skip silently.

use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

pub use image::RgbImage;

pub const ENV_DIR: &str = "IU_FIXTURES_DIR";
pub const ENV_REQUIRE: &str = "IU_REQUIRE_FIXTURES";
pub const DIR_NAME: &str = "INFINITAS Screenshots";
pub const DEV_CAPTURE: &str = "dev-capture";

/// The screenshot root, if it exists.
///
/// Panics if `IU_FIXTURES_DIR` points at a folder that does not exist:
/// that is a typo, not "no fixtures on this machine".
pub fn root() -> Option<PathBuf> {
    if let Some(dir) = std::env::var_os(ENV_DIR) {
        let dir = PathBuf::from(dir);
        assert!(
            dir.is_dir(),
            "{ENV_DIR} is set to {} but that folder does not exist",
            dir.display()
        );
        return Some(dir);
    }
    dirs::picture_dir()
        .map(|p| p.join(DIR_NAME))
        .filter(|p| p.is_dir())
}

/// One labelled (or unlabelled) screenshot.
#[derive(Debug, Clone)]
pub struct Fixture {
    pub path: PathBuf,
    pub width: u32,
    pub height: u32,
    /// Screen kind, i.e. the folder name (`play`, `result`, `select`…).
    pub screen: String,
    /// Contents of the sidecar `<name>.json`, if any. The label schema is
    /// defined by the annotator (P3a); tests deserialize what they need.
    pub label: Option<serde_json::Value>,
}

impl Fixture {
    pub fn name(&self) -> String {
        self.path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default()
    }

    pub fn load_rgb(&self) -> Result<RgbImage, String> {
        let img = image::open(&self.path)
            .map_err(|e| format!("cannot decode: {e}"))?
            .to_rgb8();
        if img.dimensions() != (self.width, self.height) {
            return Err(format!(
                "image is {}x{} but sits in the {}x{} folder",
                img.width(),
                img.height(),
                self.width,
                self.height
            ));
        }
        Ok(img)
    }
}

/// All fixtures for `screen` across every resolution folder, sorted by
/// path. `None` when there is no screenshot root on this machine.
pub fn collect(screen: &str) -> Option<Vec<Fixture>> {
    root().map(|root| collect_in(&root, screen))
}

/// Like [`collect`] but with an explicit root (used by this crate's tests).
pub fn collect_in(root: &Path, screen: &str) -> Vec<Fixture> {
    let mut out = Vec::new();
    let Ok(resolutions) = fs::read_dir(root.join(DEV_CAPTURE)) else {
        return out;
    };
    for res_dir in resolutions.flatten() {
        let Some((width, height)) = parse_resolution(&res_dir.file_name().to_string_lossy()) else {
            continue;
        };
        let Ok(files) = fs::read_dir(res_dir.path().join(screen)) else {
            continue;
        };
        for file in files.flatten() {
            let path = file.path();
            let is_png = path
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("png"));
            if !is_png {
                continue;
            }
            let label = fs::read_to_string(path.with_extension("json"))
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok());
            out.push(Fixture {
                path,
                width,
                height,
                screen: screen.to_owned(),
                label,
            });
        }
    }
    out.sort_by(|a, b| a.path.cmp(&b.path));
    out
}

fn parse_resolution(name: &str) -> Option<(u32, u32)> {
    let (w, h) = name.split_once('x')?;
    Some((w.parse().ok()?, h.parse().ok()?))
}

/// Runs `check` on every fixture of `screen` and reports **all** failures
/// at once (not just the first), so one run shows the whole picture.
///
/// Skips with a notice when there are no screenshots on this machine,
/// unless `IU_REQUIRE_FIXTURES=1`.
pub fn run<F>(screen: &str, mut check: F)
where
    F: FnMut(&Fixture, &RgbImage) -> Result<(), String>,
{
    let require = std::env::var_os(ENV_REQUIRE).is_some_and(|v| v == "1");
    let Some(fixtures) = collect(screen) else {
        assert!(
            !require,
            "{ENV_REQUIRE}=1 but no screenshot folder was found"
        );
        eprintln!("skipped: no screenshot folder (set {ENV_DIR} to run screenshot tests)");
        return;
    };
    if fixtures.is_empty() {
        assert!(
            !require,
            "{ENV_REQUIRE}=1 but there are no `{screen}` screenshots"
        );
        eprintln!("skipped: no `{screen}` screenshots under {DEV_CAPTURE}");
        return;
    }

    let mut failures = String::new();
    let mut failed = 0;
    for fixture in &fixtures {
        let result = fixture.load_rgb().and_then(|img| check(fixture, &img));
        if let Err(msg) = result {
            failed += 1;
            let _ = writeln!(failures, "  {}: {msg}", fixture.path.display());
        }
    }
    eprintln!(
        "`{screen}` screenshots: {} passed, {failed} failed",
        fixtures.len() - failed
    );
    assert!(
        failed == 0,
        "{failed} of {} `{screen}` screenshots failed:\n{failures}",
        fixtures.len()
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_resolution_folder_names() {
        assert_eq!(parse_resolution("1920x1080"), Some((1920, 1080)));
        assert_eq!(parse_resolution("1280x720"), Some((1280, 720)));
        assert_eq!(parse_resolution("results"), None);
        assert_eq!(parse_resolution("1920x"), None);
    }

    #[test]
    fn collects_pngs_with_optional_labels() {
        let root = std::env::temp_dir().join(format!("iu-fixtures-test-{}", std::process::id()));
        let dir = root.join(DEV_CAPTURE).join("4x2").join("play");
        fs::create_dir_all(&dir).unwrap();
        RgbImage::new(4, 2).save(dir.join("b.png")).unwrap();
        RgbImage::new(4, 2).save(dir.join("a.png")).unwrap();
        fs::write(dir.join("a.json"), r#"{ "pg": 1 }"#).unwrap();
        fs::write(dir.join("notes.txt"), "ignored").unwrap();
        fs::create_dir_all(root.join(DEV_CAPTURE).join("misc")).unwrap();

        let fixtures = collect_in(&root, "play");
        fs::remove_dir_all(&root).unwrap();

        let names: Vec<_> = fixtures.iter().map(Fixture::name).collect();
        assert_eq!(names, ["a.png", "b.png"]);
        assert_eq!((fixtures[0].width, fixtures[0].height), (4, 2));
        assert_eq!(fixtures[0].label.as_ref().unwrap()["pg"], 1);
        assert!(fixtures[1].label.is_none());
    }
}
