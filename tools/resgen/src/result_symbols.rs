//! Result-screen symbol fields: lamps, DJ LEVEL, chart, SP/DP, DEAD POINT,
//! USE OPTION. Each is a fixed region matched as one whole mask per class
//! (word); classes come from the labels, masks by majority vote.

use std::collections::BTreeMap;

use anyhow::ensure;
use iu_core::Side;
use iu_core::label::ResultLabel;
use iu_recog::Mask;
use iu_recog::mask::Ink;
use iu_recog::result::{ResultSymbols, SideLayout, SymbolPos, SymbolSet, symbol};

use crate::result::Sample;

/// (set, width, height, ink), measured with the `projection` example.
const SETS: [(&str, u32, u32, Ink); 6] = [
    ("lamp", 149, 18, Ink::Vivid),
    ("dj", 80, 34, Ink::Vivid),
    ("chart", 150, 19, Ink::Vivid),
    ("style", 72, 43, Ink::Bright),
    ("deadPoint", 136, 14, Ink::Bright),
    ("useOption", 125, 19, Ink::Bright),
];

/// (field, set, 1P x, y, moves with the panel on 2P). Fields in the shared
/// title bar stay put.
const FIELDS: [(&str, &str, u32, u32, bool); 8] = [
    (symbol::LAMP, "lamp", 363, 418, true),
    (symbol::BEST_LAMP, "lamp", 201, 418, true),
    (symbol::DJ_LEVEL, "dj", 398, 476, true),
    (symbol::BEST_DJ_LEVEL, "dj", 236, 476, true),
    (symbol::CHART, "chart", 748, 1037, false),
    (symbol::STYLE, "style", 924, 1033, false),
    (symbol::DEAD_POINT, "deadPoint", 606, 290, true),
    (symbol::USE_OPTION, "useOption", 176, 335, true),
];

/// 2P positions are searched this far around 1P + 1350 horizontally (the
/// USE OPTION banner, for one, is only +1324), and ±4 vertically.
const SHIFT_2P: u32 = 1350;
const SEARCH_X: i32 = 40;
const SEARCH_Y: i32 = 4;

/// The class a label implies for a field.
fn class(l: &ResultLabel, field: &str) -> String {
    let key = |v: serde_json::Value| v.as_str().unwrap_or_default().to_owned();
    let none = || symbol::NONE.to_owned();
    match field {
        symbol::LAMP => key(serde_json::to_value(l.lamp).expect("serializable")),
        symbol::BEST_LAMP => {
            l.best_lamp.map_or_else(
                none,
                |v| key(serde_json::to_value(v).expect("serializable")),
            )
        }
        symbol::DJ_LEVEL => key(serde_json::to_value(l.dj_level).expect("serializable")),
        symbol::BEST_DJ_LEVEL => l.best_dj_level.map_or_else(none, |v| {
            key(serde_json::to_value(v).expect("serializable"))
        }),
        symbol::CHART => {
            let c = l.chart.key();
            format!("{}{}", &c[2..], l.level)
        }
        symbol::STYLE => l.chart.key()[..2].to_ascii_lowercase(),
        symbol::DEAD_POINT => if l.dead.is_some() {
            "present"
        } else {
            symbol::NONE
        }
        .to_owned(),
        symbol::USE_OPTION => l.use_option.clone().unwrap_or_else(none),
        _ => unreachable!("unknown symbol field {field}"),
    }
}

/// What reading a labelled screenshot must give.
pub fn expected(l: &ResultLabel) -> ResultSymbols {
    ResultSymbols {
        lamp: Some(l.lamp),
        best_lamp: Some(l.best_lamp),
        dj_level: Some(l.dj_level),
        best_dj_level: Some(l.best_dj_level),
        chart: Some(l.chart),
        level: Some(l.level),
        dead: Some(l.dead.is_some()),
        use_option: Some(l.use_option.clone()),
    }
}

fn region(s: &Sample, set: (u32, u32, Ink), x: u32, y: u32) -> Mask {
    Mask::from_image_ink(&s.image, x, y, set.0, set.1, set.2)
}

pub fn build(
    samples: &[Sample],
    sides: &mut [SideLayout],
) -> anyhow::Result<BTreeMap<String, SymbolSet>> {
    let geo: BTreeMap<&str, (u32, u32, Ink)> = SETS
        .iter()
        .map(|&(n, w, h, ink)| (n, (w, h, ink)))
        .collect();
    let on =
        |side: Side| -> Vec<&Sample> { samples.iter().filter(|s| s.label.side == side).collect() };
    let (p1, p2) = (on(Side::P1), on(Side::P2));

    // 1P positions are measured.
    let mut pos: BTreeMap<(Side, &str), (u32, u32)> = FIELDS
        .iter()
        .map(|&(f, _, x, y, _)| ((Side::P1, f), (x, y)))
        .collect();

    // Masks from 1P, then 2P positions by matching them, then masks again
    // with 2P included (it brings classes 1P lacks, e.g. assist-clear).
    let mut sets = masks(&geo, &pos, &p1)?;
    if !p2.is_empty() {
        for &(field, set, x, y, moves) in &FIELDS {
            let g = geo[set];
            let classes = &sets[set];
            let sx = if moves { x + SHIFT_2P } else { x };
            let mut best = (u32::MAX, u32::MAX, sx, y);
            let reach = if moves { SEARCH_X } else { 0 };
            for dy in -SEARCH_Y..=SEARCH_Y {
                for dx in -reach..=reach {
                    let (Some(px), Some(py)) =
                        (sx.checked_add_signed(dx), y.checked_add_signed(dy))
                    else {
                        continue;
                    };
                    let cost: u32 = p2
                        .iter()
                        .filter_map(|s| {
                            let m = classes.get(&class(&s.label, field))?;
                            Some(m.distance(&region(s, g, px, py)))
                        })
                        .sum();
                    let near = (dx.abs() + dy.abs()) as u32;
                    if (cost, near) < (best.0, best.1) {
                        best = (cost, near, px, py);
                    }
                }
            }
            pos.insert((Side::P2, field), (best.2, best.3));
        }
        sets = masks(&geo, &pos, samples.iter().collect::<Vec<_>>().as_slice())?;
    }

    for side in sides.iter_mut() {
        for &(field, set, ..) in &FIELDS {
            if let Some(&(x, y)) = pos.get(&(side.side, field)) {
                side.symbols.insert(
                    field.to_owned(),
                    SymbolPos {
                        x,
                        y,
                        set: set.to_owned(),
                    },
                );
            }
        }
    }

    // Thresholds: every set must separate its classes.
    let mut out = BTreeMap::new();
    for (name, classes) in sets {
        let (w, h, ink) = geo[name.as_str()];
        let within = within_distance(&name, &geo, &pos, samples, &classes);
        let between = classes
            .values()
            .enumerate()
            .flat_map(|(i, a)| classes.values().skip(i + 1).map(move |b| a.distance(b)))
            .min()
            .unwrap_or(w * h);
        let names: Vec<&String> = classes.keys().collect();
        println!(
            "symbols {name:<9} {w}x{h}: {} classes {names:?}, in-class ≤ {within}, closest pair {between}",
            classes.len()
        );
        ensure!(
            within * 2 < between,
            "symbols {name}: classes not separable (in-class {within} vs between {between})"
        );
        out.insert(
            name.clone(),
            SymbolSet {
                width: w,
                height: h,
                ink,
                classes,
                max_distance: (within + between) / 2,
            },
        );
    }
    Ok(out)
}

type Positions<'a> = BTreeMap<(Side, &'a str), (u32, u32)>;

fn masks(
    geo: &BTreeMap<&str, (u32, u32, Ink)>,
    pos: &Positions,
    samples: &[&Sample],
) -> anyhow::Result<BTreeMap<String, BTreeMap<String, Mask>>> {
    let mut collected: BTreeMap<String, BTreeMap<String, Vec<Mask>>> = BTreeMap::new();
    for s in samples {
        for &(field, set, ..) in &FIELDS {
            let Some(&(x, y)) = pos.get(&(s.label.side, field)) else {
                continue;
            };
            collected
                .entry(set.to_owned())
                .or_default()
                .entry(class(&s.label, field))
                .or_default()
                .push(region(s, geo[set], x, y));
        }
    }
    Ok(collected
        .into_iter()
        .map(|(set, classes)| {
            let masks = classes
                .into_iter()
                .map(|(c, ms)| (c, majority(&ms)))
                .collect();
            (set, masks)
        })
        .collect())
}

fn within_distance(
    set: &str,
    geo: &BTreeMap<&str, (u32, u32, Ink)>,
    pos: &Positions,
    samples: &[Sample],
    classes: &BTreeMap<String, Mask>,
) -> u32 {
    let mut worst = 0;
    for s in samples {
        for &(field, fset, ..) in FIELDS.iter().filter(|f| f.1 == set) {
            let Some(&(x, y)) = pos.get(&(s.label.side, field)) else {
                continue;
            };
            if let Some(m) = classes.get(&class(&s.label, field)) {
                let d = m.distance(&region(s, geo[fset], x, y));
                worst = worst.max(d);
            }
        }
    }
    worst
}

fn majority(masks: &[Mask]) -> Mask {
    let (w, h) = (masks[0].width(), masks[0].height());
    let bits = (0..h)
        .flat_map(|y| (0..w).map(move |x| (x, y)))
        .map(|(x, y)| masks.iter().filter(|m| m.get(x, y)).count() * 2 > masks.len())
        .collect();
    Mask::new(w, h, bits)
}
