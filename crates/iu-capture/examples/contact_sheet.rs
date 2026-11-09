//! Tiles PNGs from a folder into one overview image (sorted by name,
//! left to right, top to bottom) and prints the index → file mapping.
//!
//! cargo run --release -p iu-capture --example contact_sheet -- <dir> <out.png> [cols] [tile width]

use image::{RgbImage, imageops};

fn main() {
    let mut args = std::env::args().skip(1);
    let dir = args.next().expect("dir");
    let out = args.next().expect("out.png");
    let cols: u32 = args.next().and_then(|s| s.parse().ok()).unwrap_or(6);
    let tw: u32 = args.next().and_then(|s| s.parse().ok()).unwrap_or(320);
    let th = tw * 9 / 16;

    let mut files: Vec<_> = std::fs::read_dir(&dir)
        .expect("read dir")
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("png")))
        .collect();
    files.sort();
    let rows = (files.len() as u32).div_ceil(cols);
    let mut sheet = RgbImage::new(cols * (tw + 4), rows * (th + 4));
    for (i, path) in files.iter().enumerate() {
        let img = image::open(path).expect("open").to_rgb8();
        let small = imageops::thumbnail(&img, tw, th);
        let (x, y) = ((i as u32 % cols) * (tw + 4), (i as u32 / cols) * (th + 4));
        imageops::replace(&mut sheet, &small, i64::from(x), i64::from(y));
        println!("{i:3} {}", path.file_name().unwrap().to_string_lossy());
    }
    sheet.save(out).expect("save");
}
