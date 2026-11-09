//! `resgen result`: result-screen caption mask, number positions per side,
//! and digit masks per font.

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{Context, bail, ensure};
use image::RgbImage;
use iu_core::Side;
use iu_core::label::{Label, ResultLabel, Screen};
use iu_recog::Mask;
use iu_recog::mask::Ink;
use iu_recog::result::{
    FieldPos, Font, NumberField, RESOURCE_VERSION, ResultResources, SideLayout, field_key,
};

/// `CLEAR TYPE` caption on the 1P panel (x, y, w, h), measured with the
/// `projection` example.
const CAPTION_1P: [u32; 4] = [41, 421, 138, 12];
/// The 2P panel is roughly the 1P one moved right by this much; exact
/// positions are searched within `SEARCH` px.
const SIDE_2P_SHIFT: u32 = 1350;
const SEARCH: i32 = 8;

/// Fonts: (name, cell width, height, pitch), measured.
const FONTS: [(&str, u32, u32, u32); 6] = [
    ("judge", 23, 15, 28),
    ("combo", 23, 15, 28),
    ("small", 14, 12, 17),
    ("big", 30, 20, 36),
    ("big-cyan", 30, 20, 36),
    ("notes", 17, 15, 21),
];

/// 1P positions (top-left of the ones cell), cells, font, ink — measured.
/// `shifts` says whether the field moves with the panel on 2P (the NOTES
/// count is in the shared title bar).
struct Seed {
    field: NumberField,
    x: u32,
    y: u32,
    cells: u32,
    font: &'static str,
    ink: Ink,
    shifts: bool,
}

const fn seed(field: NumberField, x: u32, y: u32, font: &'static str, ink: Ink) -> Seed {
    Seed {
        field,
        x,
        y,
        cells: 4,
        font,
        ink,
        shifts: true,
    }
}

const SEEDS: [Seed; 13] = [
    seed(NumberField::Pg, 476, 788, "judge", Ink::Bright),
    seed(NumberField::Gr, 476, 816, "judge", Ink::Bright),
    seed(NumberField::Gd, 476, 844, "judge", Ink::Bright),
    seed(NumberField::Bd, 476, 872, "judge", Ink::Bright),
    seed(NumberField::Pr, 476, 900, "judge", Ink::Bright),
    seed(NumberField::ComboBreak, 507, 981, "combo", Ink::Bright),
    seed(NumberField::Fast, 157, 979, "small", Ink::Bright),
    seed(NumberField::Slow, 157, 995, "small", Ink::Bright),
    seed(NumberField::Score, 478, 553, "big-cyan", Ink::Cyan),
    seed(NumberField::BestScore, 316, 553, "big", Ink::Bright),
    seed(NumberField::MissCount, 478, 621, "big-cyan", Ink::Cyan),
    seed(NumberField::BestMissCount, 316, 621, "big", Ink::Bright),
    Seed {
        shifts: false,
        ..seed(NumberField::Notes, 1066, 1039, "notes", Ink::Bright)
    },
];

struct Sample {
    name: String,
    image: RgbImage,
    label: ResultLabel,
}

impl Sample {
    fn value(&self, f: NumberField) -> Option<u32> {
        let l = &self.label;
        match f {
            NumberField::Pg => Some(l.judge.pg),
            NumberField::Gr => Some(l.judge.gr),
            NumberField::Gd => Some(l.judge.gd),
            NumberField::Bd => Some(l.judge.bd),
            NumberField::Pr => Some(l.judge.pr),
            NumberField::ComboBreak => Some(l.combo_break),
            NumberField::Fast => Some(l.fast),
            NumberField::Slow => Some(l.slow),
            NumberField::Score => Some(l.score),
            NumberField::BestScore => l.best_score,
            NumberField::MissCount => l.miss_count,
            NumberField::BestMissCount => l.best_miss_count,
            NumberField::Notes => Some(l.notes),
        }
    }
}

pub fn run(root: &Path, out_dir: &Path) -> anyhow::Result<()> {
    let mut by_res: BTreeMap<(u32, u32), Vec<Sample>> = BTreeMap::new();
    for fixture in iu_fixtures::collect_in(root, "result") {
        let Some(value) = &fixture.label else {
            println!("skip (no label): {}", fixture.name());
            continue;
        };
        let Label::Result(label) = Label::from_file_json(Screen::Result, &value.to_string())
            .with_context(|| format!("{}: bad label", fixture.name()))?
        else {
            unreachable!("result folder yields result labels");
        };
        let image = fixture.load_rgb().map_err(anyhow::Error::msg)?;
        by_res
            .entry((fixture.width, fixture.height))
            .or_default()
            .push(Sample {
                name: fixture.name().to_owned(),
                image,
                label,
            });
    }
    ensure!(!by_res.is_empty(), "no labelled result screenshots");
    for ((w, h), samples) in by_res {
        let res = build(w, h, &samples, root)?;
        let dir = out_dir.join(format!("{w}x{h}"));
        std::fs::create_dir_all(&dir)?;
        let path = dir.join("result.json");
        std::fs::write(&path, serde_json::to_string_pretty(&res)? + "\n")?;
        println!("wrote {}", path.display());
    }
    Ok(())
}

fn build(w: u32, h: u32, samples: &[Sample], root: &Path) -> anyhow::Result<ResultResources> {
    println!("== {w}x{h}: {} labelled result screenshots", samples.len());
    let fonts_geo: BTreeMap<&str, (u32, u32, u32)> =
        FONTS.iter().map(|&(n, w, h, p)| (n, (w, h, p))).collect();
    let p1: Vec<&Sample> = samples
        .iter()
        .filter(|s| s.label.side == Side::P1)
        .collect();
    let p2: Vec<&Sample> = samples
        .iter()
        .filter(|s| s.label.side == Side::P2)
        .collect();
    ensure!(!p1.is_empty(), "need 1P result samples");

    // 1. Caption mask from the 1P samples.
    let [cx, cy, cw, ch] = CAPTION_1P;
    let caption = majority(
        &p1.iter()
            .map(|s| Mask::from_image(&s.image, cx, cy, cw, ch))
            .collect::<Vec<_>>(),
    );

    // 2. 1P fields at the measured positions; every sample must fit.
    let mut sides = Vec::new();
    let mut p1_fields = BTreeMap::new();
    for seed in &SEEDS {
        let pos = field_pos(seed, seed.x, seed.y);
        let geo = fonts_geo[seed.font];
        if let Some(bad) = p1
            .iter()
            .find(|s| !shape_fits(&s.image, &pos, geo, s.value(seed.field)))
        {
            bail!(
                "{:?} does not fit the 1P position in {} (label or position wrong)",
                seed.field,
                bad.name
            );
        }
        p1_fields.insert(field_key(seed.field), pos);
    }

    // 3. Digit masks per font from the 1P samples.
    let fonts = fonts_from(&p1, &p1_fields, &fonts_geo)?;

    sides.push(SideLayout {
        side: Side::P1,
        caption: [cx, cy],
        fields: p1_fields,
    });

    // 4. 2P: search around the shifted 1P positions.
    if p2.is_empty() {
        println!("no 2P samples: 2P result screens will not be recognized yet");
    } else {
        let (_, x, y) = best_offset(cx + SIDE_2P_SHIFT, cy, |x, y| {
            p2.iter()
                .map(|s| caption.distance_at(&s.image, x, y))
                .sum::<u32>()
        });
        let mut fields = BTreeMap::new();
        for seed in &SEEDS {
            let sx = seed.x + if seed.shifts { SIDE_2P_SHIFT } else { 0 };
            let geo = fonts_geo[seed.font];
            let font = &fonts[seed.font];
            let (cost, fx, fy) = best_offset(sx, seed.y, |x, y| {
                let pos = field_pos(seed, x, y);
                p2.iter()
                    .map(|s| {
                        let v = s.value(seed.field);
                        if !shape_fits(&s.image, &pos, geo, v) {
                            return 10_000;
                        }
                        cells_of(&s.image, &pos, geo, v)
                            .iter()
                            .map(|(d, m)| m.distance(&font.digits[*d as usize]))
                            .sum()
                    })
                    .sum()
            });
            ensure!(cost < 10_000, "{:?}: no 2P position fits", seed.field);
            fields.insert(field_key(seed.field), field_pos(seed, fx, fy));
        }
        println!("2P caption at ({x}, {y})");
        sides.push(SideLayout {
            side: Side::P2,
            caption: [x, y],
            fields,
        });
    }

    // 5. Caption threshold between result and other screens.
    let worst = samples
        .iter()
        .map(|s| {
            let side = sides.iter().find(|l| l.side == s.label.side);
            side.map_or(u32::MAX, |l| {
                caption.distance_at(&s.image, l.caption[0], l.caption[1])
            })
        })
        .filter(|&d| d != u32::MAX)
        .max()
        .unwrap_or(0);
    let best_other = other_distance(root, w, h, &caption, &sides);
    println!("caption: worst result distance {worst}, best other-screen distance {best_other:?}");
    let caption_max_distance = match best_other {
        Some(n) => {
            ensure!(worst * 2 < n, "caption does not separate result screens");
            (worst + n) / 4
        }
        None => worst.max(20),
    };

    let res = ResultResources {
        version: RESOURCE_VERSION,
        width: w,
        height: h,
        caption,
        caption_max_distance,
        sides,
        fonts,
    };

    // 6. Read every sample back with the new resources.
    for s in samples {
        let side = res
            .detect(&s.image)
            .with_context(|| format!("{}: not detected as a result screen", s.name))?;
        ensure!(side.side == s.label.side, "{}: wrong side", s.name);
        for f in NumberField::ALL {
            let got = res
                .read_field(&s.image, side, f)
                .map_err(|e| anyhow::anyhow!("{}: {e}", s.name))?;
            ensure!(
                got == s.value(f),
                "{}: {f:?} read {got:?}, labelled {:?}",
                s.name,
                s.value(f)
            );
        }
    }
    println!("all {} samples read back correctly", samples.len());
    Ok(res)
}

fn field_pos(seed: &Seed, x: u32, y: u32) -> FieldPos {
    FieldPos {
        x,
        y,
        cells: seed.cells,
        font: seed.font.to_owned(),
        ink: seed.ink,
    }
}

/// Cells of a right-aligned number: (digit, mask), ones first.
fn cells_of(
    img: &RgbImage,
    pos: &FieldPos,
    geo: (u32, u32, u32),
    value: Option<u32>,
) -> Vec<(u32, Mask)> {
    let (w, h, pitch) = geo;
    let Some(v) = value else { return Vec::new() };
    v.to_string()
        .bytes()
        .rev()
        .enumerate()
        .map(|(i, b)| {
            let x = pos.x - i as u32 * pitch;
            (
                u32::from(b - b'0'),
                Mask::from_image_ink(img, x, pos.y, w, h, pos.ink),
            )
        })
        .collect()
}

/// Digit cells have ink, and the cell left of the number is blank. Fields
/// labelled `None` ("—") are not checked.
fn shape_fits(img: &RgbImage, pos: &FieldPos, geo: (u32, u32, u32), value: Option<u32>) -> bool {
    let (w, h, pitch) = geo;
    let Some(v) = value else { return true };
    let len = v.to_string().len() as u32;
    let ink = |i: u32| {
        pos.x
            .checked_sub(i * pitch)
            .map(|x| Mask::from_image_ink(img, x, pos.y, w, h, pos.ink).count_ones())
    };
    (0..len).all(|i| ink(i).is_some_and(|n| n > 2))
        && (len >= pos.cells || ink(len).is_none_or(|n| n <= 2))
}

fn fonts_from(
    samples: &[&Sample],
    fields: &BTreeMap<String, FieldPos>,
    geo: &BTreeMap<&str, (u32, u32, u32)>,
) -> anyhow::Result<BTreeMap<String, Font>> {
    let mut out = BTreeMap::new();
    for &(name, w, h, pitch) in &FONTS {
        let mut cells: Vec<Vec<Mask>> = vec![Vec::new(); 10];
        for s in samples {
            for seed in SEEDS.iter().filter(|seed| seed.font == name) {
                let pos = &fields[&field_key(seed.field)];
                for (d, m) in cells_of(&s.image, pos, geo[name], s.value(seed.field)) {
                    cells[d as usize].push(m);
                }
            }
        }
        let mut digits: Vec<Option<Mask>> = cells
            .iter()
            .map(|c| (!c.is_empty()).then(|| majority(c)))
            .collect();
        // Seven-segment digits: 9 is 6 turned around, and vice versa.
        for (missing, from) in [(9, 6), (6, 9)] {
            if digits[missing].is_none()
                && let Some(m) = digits[from].clone()
            {
                println!("font {name}: no {missing} in the samples; using {from} rotated 180°");
                digits[missing] = Some(rotate180(&m));
            }
        }
        let missing: Vec<usize> = (0..10).filter(|&d| digits[d].is_none()).collect();
        ensure!(
            missing.is_empty(),
            "font {name}: no samples of digits {missing:?}; label more screenshots"
        );
        let digits: Vec<Mask> = digits.into_iter().map(Option::unwrap).collect();
        let within = cells
            .iter()
            .zip(&digits)
            .flat_map(|(c, m)| c.iter().map(move |x| x.distance(m)))
            .max()
            .unwrap_or(0);
        let between = (0..10)
            .flat_map(|a| (a + 1..10).map(move |b| (a, b)))
            .map(|(a, b)| digits[a].distance(&digits[b]))
            .min()
            .expect("10 digits");
        let counts: Vec<String> = cells.iter().map(|c| c.len().to_string()).collect();
        println!(
            "font {name:<5} {w}x{h}/{pitch}: samples [{}], in-class ≤ {within}, closest pair {between}",
            counts.join(" ")
        );
        ensure!(
            within * 2 < between,
            "font {name}: digits not separable (in-class {within} vs between {between})"
        );
        out.insert(
            name.to_owned(),
            Font {
                width: w,
                height: h,
                pitch,
                digits,
                max_distance: between / 2 - 1,
            },
        );
    }
    Ok(out)
}

/// Lowest-cost (cost, x, y) within ±SEARCH of (x, y); ties go to the
/// nearest position.
fn best_offset(x: u32, y: u32, cost: impl Fn(u32, u32) -> u32) -> (u32, u32, u32) {
    let mut best = (u32::MAX, u32::MAX, x, y);
    for dy in -SEARCH..=SEARCH {
        for dx in -SEARCH..=SEARCH {
            let (Some(px), Some(py)) = (x.checked_add_signed(dx), y.checked_add_signed(dy)) else {
                continue;
            };
            let c = cost(px, py);
            let dist = (dx.abs() + dy.abs()) as u32;
            if (c, dist) < (best.0, best.1) {
                best = (c, dist, px, py);
            }
        }
    }
    (best.0, best.2, best.3)
}

fn rotate180(m: &Mask) -> Mask {
    let (w, h) = (m.width(), m.height());
    let bits = (0..h)
        .flat_map(|y| (0..w).map(move |x| (x, y)))
        .map(|(x, y)| m.get(w - 1 - x, h - 1 - y))
        .collect();
    Mask::new(w, h, bits)
}

fn majority(masks: &[Mask]) -> Mask {
    let (w, h) = (masks[0].width(), masks[0].height());
    let bits = (0..h)
        .flat_map(|y| (0..w).map(move |x| (x, y)))
        .map(|(x, y)| masks.iter().filter(|m| m.get(x, y)).count() * 2 > masks.len())
        .collect();
    Mask::new(w, h, bits)
}

fn other_distance(
    root: &Path,
    w: u32,
    h: u32,
    caption: &Mask,
    sides: &[SideLayout],
) -> Option<u32> {
    let mut best: Option<u32> = None;
    for screen in ["play", "select", "option", "other"] {
        for f in iu_fixtures::collect_in(root, screen) {
            if (f.width, f.height) != (w, h) {
                continue;
            }
            let Ok(img) = f.load_rgb() else { continue };
            for s in sides {
                let d = caption.distance_at(&img, s.caption[0], s.caption[1]);
                best = Some(best.map_or(d, |b| b.min(d)));
            }
        }
    }
    best
}
