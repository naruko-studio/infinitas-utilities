//! `resgen titles`: title-line masks of the labelled result screenshots,
//! one per song (the sample closest to the song's other samples), and the
//! distance under which a title counts as recognized.

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{Context, ensure};
use iu_core::label::{Label, Screen};
use iu_recog::title::{KnownTitle, RESOURCE_VERSION, TitleArea, TitleMask, TitleResources};

/// The title line under the score panels, measured with `crop_sheet`: the
/// text sits between the score panels' bottom and the artist line, centred
/// on the screen. Wide enough for the longest titles seen.
const AREA: TitleArea = TitleArea {
    x: 560,
    y: 962,
    width: 800,
    height: 34,
};

/// (song ID, title as labelled, mask).
type Sample = (String, String, TitleMask);

pub fn run(root: &Path, out_dir: &Path) -> anyhow::Result<()> {
    let mut by_res: BTreeMap<(u32, u32), Vec<Sample>> = BTreeMap::new();
    for fixture in iu_fixtures::collect_in(root, Screen::Result.folder()) {
        let Some(value) = &fixture.label else {
            continue;
        };
        let Ok(Label::Result(l)) = Label::from_file_json(Screen::Result, &value.to_string()) else {
            continue;
        };
        let Some(id) = l.song_id else {
            continue;
        };
        let img = fixture
            .load_rgb()
            .map_err(anyhow::Error::msg)
            .with_context(|| fixture.name())?;
        by_res
            .entry((fixture.width, fixture.height))
            .or_default()
            .push((id, l.title, TitleMask::from_image(&img, AREA)));
    }
    for ((width, height), samples) in by_res {
        let res = build(width, height, &samples)?;
        let dir = out_dir.join(format!("{width}x{height}"));
        std::fs::create_dir_all(&dir)?;
        let path = dir.join("titles.json");
        let mut json = serde_json::to_string_pretty(&res)?;
        json.push('\n');
        std::fs::write(&path, json)?;
        println!(
            "wrote {} ({} titles from {} samples)",
            path.display(),
            res.titles.len(),
            samples.len()
        );
    }
    Ok(())
}

fn build(width: u32, height: u32, samples: &[Sample]) -> anyhow::Result<TitleResources> {
    dump(samples);
    // Same song: how far apart its own samples are. Different songs: how
    // close the nearest other song comes.
    let mut within = 0;
    let mut between = u32::MAX;
    let mut closest = (String::new(), String::new());
    for (i, (id_a, title_a, a)) in samples.iter().enumerate() {
        for (id_b, title_b, b) in &samples[i + 1..] {
            let d = a.distance(b);
            if id_a == id_b {
                within = within.max(d);
            } else if d < between {
                between = d;
                closest = (title_a.clone(), title_b.clone());
            }
        }
    }
    println!(
        "titles {}x{}: same song ≤ {within}, different songs ≥ {between} ({} / {})",
        AREA.width, AREA.height, closest.0, closest.1
    );
    ensure!(
        within < between,
        "title masks do not separate the songs ({within} ≥ {between})"
    );

    // One mask per song: the sample with the smallest worst distance to
    // the song's other samples.
    let mut songs: BTreeMap<&str, Vec<&TitleMask>> = BTreeMap::new();
    for (id, _, m) in samples {
        songs.entry(id).or_default().push(m);
    }
    let titles = songs
        .into_iter()
        .map(|(id, masks)| {
            let medoid = masks
                .iter()
                .min_by_key(|a| masks.iter().map(|b| a.distance(b)).max())
                .expect("at least one sample");
            KnownTitle {
                song_id: id.to_owned(),
                mask: (*medoid).clone(),
            }
        })
        .collect();
    Ok(TitleResources {
        version: RESOURCE_VERSION,
        width,
        height,
        area: AREA,
        max_distance: if between == u32::MAX {
            within.max(1) * 2
        } else {
            (within + between) / 2
        },
        titles,
    })
}

/// Debug: `IU_TITLE_DUMP=<dir>` writes each sample's mask as a PNG.
fn dump(samples: &[Sample]) {
    let Some(dir) = std::env::var_os("IU_TITLE_DUMP") else {
        return;
    };
    for (i, (_, title, m)) in samples.iter().enumerate() {
        let bytes = m.to_bytes();
        let img = image::GrayImage::from_fn(m.width(), m.height(), |x, y| {
            let n = (y * m.width() + x) as usize;
            image::Luma([if bytes[n / 8] >> (n % 8) & 1 == 1 {
                255
            } else {
                0
            }])
        });
        let safe: String = title
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { '_' })
            .collect();
        let name = format!("{i:02}_{safe}.png");
        let _ = img.save(std::path::Path::new(&dir).join(name));
    }
}
