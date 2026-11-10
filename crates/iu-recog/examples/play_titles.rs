//! Title masks of the play screen's info box: where the ink is in each
//! labelled play screenshot (to check the area holds the whole title line
//! and no artist), and the distances between them.
//!
//! cargo run --release -p iu-recog --example play_titles -- [extra.png layout]...

use iu_core::PlayLayout;
use iu_recog::title::{TitleMask, play_title_area};

fn main() {
    let mut masks: Vec<(String, PlayLayout, TitleMask)> = Vec::new();
    let mut add = |name: String, layout: PlayLayout, img: &image::RgbImage| {
        let Some(area) = play_title_area(img.width(), img.height(), layout) else {
            println!("{name}: no area");
            return;
        };
        let mask = TitleMask::from_image(img, area);
        // IU_ROWS=1: ink per screen row from y 30 to 130, across the area's
        // width (to place the title line).
        if std::env::var_os("IU_ROWS").is_some() {
            let tall = iu_recog::title::TitleArea {
                y: 30,
                height: 100,
                ..area
            };
            let m = TitleMask::from_image(img, tall);
            let rows: Vec<String> = (0..tall.height)
                .map(|y| {
                    let n = (0..tall.width).filter(|&x| m.get(x, y)).count();
                    format!("{}:{n}", y + 30)
                })
                .filter(|s| !s.ends_with(":0"))
                .collect();
            println!("{name} {layout}: {}", rows.join(" "));
        }
        // IU_DUMP=<dir>: the area and 40 px around it, to look at.
        if let Some(dir) = std::env::var_os("IU_DUMP") {
            let (y, h) = (area.y.saturating_sub(40), area.height + 80);
            let crop = image::imageops::crop_imm(img, area.x, y, area.width, h).to_image();
            let file = std::path::Path::new(&dir).join(format!(
                "{layout}_{}.png",
                name.rsplit(['/', '\\']).next().unwrap_or(&name)
            ));
            crop.save(file).expect("save");
        }
        let (mut x0, mut y0, mut x1, mut y1) = (u32::MAX, u32::MAX, 0, 0);
        for y in 0..area.height {
            for x in 0..area.width {
                if mask.get(x, y) {
                    x0 = x0.min(x);
                    y0 = y0.min(y);
                    x1 = x1.max(x);
                    y1 = y1.max(y);
                }
            }
        }
        println!(
            "{name:40} {layout:18} on {:5}  x {x0:3}..{x1:3}  y {y0:2}..{y1:2} (of {})",
            mask.count_ones(),
            area.height
        );
        masks.push((name, layout, mask));
    };
    for f in iu_fixtures::collect("play").unwrap_or_default() {
        let Some(label) = &f.label else { continue };
        let Ok(layout) = serde_json::from_value::<PlayLayout>(label["layout"].clone()) else {
            continue;
        };
        let img = f.load_rgb().expect("image");
        add(f.name(), layout, &img);
    }
    let args: Vec<String> = std::env::args().skip(1).collect();
    for pair in args.chunks(2) {
        let [file, layout] = pair else { break };
        let layout: PlayLayout = layout.parse().expect("layout");
        let img = image::open(file).expect("image").to_rgb8();
        add(file.clone(), layout, &img);
    }
    let mut closest = u32::MAX;
    for (i, (a, la, ma)) in masks.iter().enumerate() {
        for (b, lb, mb) in &masks[i + 1..] {
            if la != lb {
                continue;
            }
            let d = ma.distance(mb);
            println!("  {d:5}  {a} / {b}");
            if d > 0 {
                closest = closest.min(d);
            }
        }
    }
    println!("closest non-identical pair: {closest}");
}
