//! `resgen play`: JUDGE label mask, layout anchors and digit masks.

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{Context, bail, ensure};
use image::RgbImage;
use iu_core::{Judge, PlayLayout};
use iu_fixtures::Fixture;
use iu_recog::Mask;
use iu_recog::play::{
    DIGIT_COLUMNS, DigitGrid, JUDGE_ROWS, LayoutAnchor, PlayResources, RESOURCE_VERSION,
};

/// Where the JUDGE label is in the seed layout (SP 1P, graph inner), measured
/// on our screenshots with the `projection` example. Every other layout's
/// label position is found by searching.
const SEED_LAYOUT: PlayLayout = PlayLayout::Sp {
    side: iu_core::Side::P1,
    graph_inner: true,
};
const SEED_LABEL: [u32; 4] = [1014, 942, 73, 17];
/// Band searched for the label (the judge panel is always near the bottom).
const SEARCH_BAND: (u32, u32) = (840, 1080);
/// Measured digit grid: 11×11 cells, 14 px apart horizontally, 16 vertically.
const GRID: DigitGrid = DigitGrid {
    width: 11,
    height: 11,
    step_x: 14,
    step_y: 16,
};
/// Digit origin relative to the label. The panel is drawn mirrored in some
/// layouts (2P, DP info left), which moves the digits; the right offset is
/// picked per layout by checking it against the labels.
const DIGIT_OFFSETS: [[u32; 2]; 2] = [[35, 27], [18, 27]];

struct Sample {
    fixture: Fixture,
    image: RgbImage,
    layout: PlayLayout,
    judge: Judge,
}

pub fn run(root: &Path, out_dir: &Path) -> anyhow::Result<()> {
    let fixtures = iu_fixtures::collect_in(root, "play");
    let mut by_res: BTreeMap<(u32, u32), Vec<Sample>> = BTreeMap::new();
    for fixture in fixtures {
        let Some(label) = &fixture.label else {
            println!("skip (no label): {}", fixture.name());
            continue;
        };
        let layout: PlayLayout = serde_json::from_value(label["layout"].clone())
            .with_context(|| format!("{}: bad layout", fixture.name()))?;
        let judge: Judge = serde_json::from_value(label["judge"].clone())
            .with_context(|| format!("{}: bad judge", fixture.name()))?;
        let image = fixture.load_rgb().map_err(anyhow::Error::msg)?;
        by_res
            .entry((fixture.width, fixture.height))
            .or_default()
            .push(Sample {
                fixture,
                image,
                layout,
                judge,
            });
    }
    ensure!(!by_res.is_empty(), "no labelled play screenshots");

    for ((w, h), samples) in by_res {
        let res = build(w, h, &samples, root)?;
        let dir = out_dir.join(format!("{w}x{h}"));
        std::fs::create_dir_all(&dir)?;
        let path = dir.join("play.json");
        std::fs::write(&path, serde_json::to_string_pretty(&res)? + "\n")?;
        println!("wrote {}", path.display());
    }
    Ok(())
}

fn build(w: u32, h: u32, samples: &[Sample], root: &Path) -> anyhow::Result<PlayResources> {
    println!("== {w}x{h}: {} labelled play screenshots", samples.len());

    // 1. JUDGE label mask: majority vote over the seed-layout samples.
    let [lx, ly, lw, lh] = SEED_LABEL;
    let seeds: Vec<Mask> = samples
        .iter()
        .filter(|s| s.layout == SEED_LAYOUT)
        .map(|s| Mask::from_image(&s.image, lx, ly, lw, lh))
        .collect();
    ensure!(!seeds.is_empty(), "need at least one {SEED_LAYOUT} sample");
    let label = majority(&seeds);
    println!("label mask {lw}x{lh}, {} white px", label.count_ones());

    // 2. Find the label in every sample; one position per layout.
    let mut anchors: BTreeMap<String, LayoutAnchor> = BTreeMap::new();
    let mut worst_play = 0;
    for s in samples {
        let (dist, x, y) = search(&label, &s.image);
        worst_play = worst_play.max(dist);
        let anchor = anchors
            .entry(s.layout.name().to_owned())
            .or_insert(LayoutAnchor {
                layout: s.layout,
                label: [x, y],
                digits: [0, 0],
            });
        if anchor.label != [x, y] {
            bail!(
                "{}: label at ({x}, {y}) but other {} samples have it at {:?}",
                s.fixture.name(),
                s.layout,
                anchor.label
            );
        }
    }

    // 3. Digit origin per layout: the offset whose blank/non-blank cells
    //    match the labelled numbers in every sample of that layout.
    for anchor in anchors.values_mut() {
        let layout_samples: Vec<&Sample> = samples
            .iter()
            .filter(|s| s.layout == anchor.layout)
            .collect();
        let offset = DIGIT_OFFSETS
            .iter()
            .find(|off| {
                let origin = [anchor.label[0] + off[0], anchor.label[1] + off[1]];
                layout_samples
                    .iter()
                    .all(|s| digit_shape_matches(&s.image, origin, s.judge))
            })
            .with_context(|| format!("no digit offset fits the {} samples", anchor.layout))?;
        anchor.digits = [anchor.label[0] + offset[0], anchor.label[1] + offset[1]];
        println!(
            "{:<18} label {:?} digits {:?} ({} samples)",
            anchor.layout.name(),
            anchor.label,
            anchor.digits,
            layout_samples.len()
        );
    }

    // 4. Digit masks: majority vote over every labelled cell.
    let mut cells: Vec<Vec<Mask>> = vec![Vec::new(); 10];
    for s in samples {
        let anchor = &anchors[s.layout.name()];
        for (row, value) in (0..JUDGE_ROWS).zip(s.judge.to_array()) {
            for (column, digit) in digit_columns(value) {
                cells[digit as usize].push(cell(&s.image, anchor.digits, row, column));
            }
        }
    }
    for (d, c) in cells.iter().enumerate() {
        ensure!(
            !c.is_empty(),
            "no samples of digit {d}; label more screenshots"
        );
    }
    let digits: Vec<Mask> = cells.iter().map(|c| majority(c)).collect();

    // 5. Thresholds from the data: the worst in-class distance must stay
    //    well below the closest pair of different digits.
    let within = cells
        .iter()
        .zip(&digits)
        .flat_map(|(c, m)| c.iter().map(move |x| x.distance(m)))
        .max()
        .unwrap_or(0);
    let between = (0..10)
        .flat_map(|a| (a + 1..10).map(move |b| (a, b)))
        .map(|(a, b)| (digits[a].distance(&digits[b]), a, b))
        .min()
        .expect("10 digits");
    for (d, c) in cells.iter().enumerate() {
        print!("{d}:{} ", c.len());
    }
    println!(
        "\nworst in-class distance {within}, closest digits {}/{} at {}",
        between.1, between.2, between.0
    );
    ensure!(
        within * 2 < between.0,
        "digits are not separable enough (in-class {within} vs between {})",
        between.0
    );
    let digit_max_distance = between.0 / 2 - 1;

    // Label threshold: between the worst play sample and the best non-play
    // screenshot at any anchor.
    let best_nonplay = nonplay_distance(root, w, h, &label, anchors.values());
    println!("label: worst play distance {worst_play}, best non-play distance {best_nonplay:?}");
    let label_max_distance = match best_nonplay {
        Some(n) => {
            ensure!(
                worst_play * 2 < n,
                "label does not separate play from other screens"
            );
            (worst_play + n) / 4
        }
        None => worst_play.max(20),
    };
    println!("thresholds: label <= {label_max_distance}, digit <= {digit_max_distance}");

    Ok(PlayResources {
        version: RESOURCE_VERSION,
        width: w,
        height: h,
        judge_label: label,
        label_max_distance,
        layouts: anchors.into_values().collect(),
        digit_grid: GRID,
        digits,
        digit_max_distance,
    })
}

/// (column, digit) pairs of a right-aligned number in the 4-column grid.
fn digit_columns(value: u32) -> Vec<(u32, u32)> {
    let s = value.to_string();
    let start = DIGIT_COLUMNS - s.len() as u32;
    s.bytes()
        .enumerate()
        .map(|(i, b)| (start + i as u32, u32::from(b - b'0')))
        .collect()
}

fn cell(img: &RgbImage, origin: [u32; 2], row: u32, column: u32) -> Mask {
    Mask::from_image(
        img,
        origin[0] + column * GRID.step_x,
        origin[1] + row * GRID.step_y,
        GRID.width,
        GRID.height,
    )
}

/// Every labelled digit cell has ink and every leading cell is blank.
fn digit_shape_matches(img: &RgbImage, origin: [u32; 2], judge: Judge) -> bool {
    (0..JUDGE_ROWS).zip(judge.to_array()).all(|(row, value)| {
        let used: Vec<u32> = digit_columns(value).iter().map(|&(c, _)| c).collect();
        (0..DIGIT_COLUMNS).all(|column| {
            let ink = cell(img, origin, row, column).count_ones() > 2;
            ink == used.contains(&column)
        })
    })
}

fn majority(masks: &[Mask]) -> Mask {
    let (w, h) = (masks[0].width(), masks[0].height());
    let bits = (0..h)
        .flat_map(|y| (0..w).map(move |x| (x, y)))
        .map(|(x, y)| masks.iter().filter(|m| m.get(x, y)).count() * 2 > masks.len())
        .collect();
    Mask::new(w, h, bits)
}

/// Best (distance, x, y) of `label` in the search band.
fn search(label: &Mask, img: &RgbImage) -> (u32, u32, u32) {
    let (y0, y1) = SEARCH_BAND;
    let mut best = (u32::MAX, 0, 0);
    for y in y0..y1.saturating_sub(label.height()) {
        for x in 0..img.width() - label.width() {
            let d = label.distance_at(img, x, y);
            if d < best.0 {
                best = (d, x, y);
            }
        }
    }
    best
}

fn nonplay_distance<'a>(
    root: &Path,
    w: u32,
    h: u32,
    label: &Mask,
    anchors: impl Iterator<Item = &'a LayoutAnchor> + Clone,
) -> Option<u32> {
    let mut best: Option<u32> = None;
    for screen in ["result", "select", "option", "other"] {
        for f in iu_fixtures::collect_in(root, screen) {
            if (f.width, f.height) != (w, h) {
                continue;
            }
            let Ok(img) = f.load_rgb() else { continue };
            for a in anchors.clone() {
                let d = label.distance_at(&img, a.label[0], a.label[1]);
                best = Some(best.map_or(d, |b| b.min(d)));
            }
        }
    }
    best
}
