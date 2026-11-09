//! Crops the same rectangle from several PNGs, scales it up (nearest
//! neighbour) and stacks the crops vertically.
//!
//! cargo run --release -p iu-capture --example crop_sheet -- out.png x y w h scale file...

use image::{RgbImage, imageops};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [out, x, y, w, h, scale, files @ ..] = args.as_slice() else {
        eprintln!("usage: crop_sheet out.png x y w h scale file...");
        return;
    };
    let n = |s: &String| s.parse::<u32>().expect("number");
    let (x, y, w, h, s) = (n(x), n(y), n(w), n(h), n(scale));
    let cols = std::env::var("COLS")
        .ok()
        .and_then(|c| c.parse().ok())
        .unwrap_or(1u32);
    let rows = (files.len() as u32).div_ceil(cols);
    let mut sheet = RgbImage::new((w * s + 4) * cols, (h * s + 4) * rows);
    for (i, f) in files.iter().enumerate() {
        let img = image::open(f).expect("open").to_rgb8();
        let crop = imageops::crop_imm(&img, x, y, w, h).to_image();
        let big = imageops::resize(&crop, w * s, h * s, imageops::FilterType::Nearest);
        let (cx, cy) = (i as u32 % cols, i as u32 / cols);
        imageops::replace(
            &mut sheet,
            &big,
            i64::from(cx * (w * s + 4)),
            i64::from(cy * (h * s + 4)),
        );
    }
    sheet.save(out).expect("save");
}
