//! Reads the text lines in a rectangle of a PNG with Windows OCR, with
//! where each line is on the screen.
//!
//! cargo run -p iu-ocr --example ocr -- file.png x y w h [scale]

use iu_ocr::{Ocr, Rect};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [file, x, y, w, h, rest @ ..] = args.as_slice() else {
        return Err("usage: ocr file.png x y w h [scale]".into());
    };
    let n = |s: &String| s.parse::<u32>();
    let rect = Rect {
        x: n(x)?,
        y: n(y)?,
        width: n(w)?,
        height: n(h)?,
    };
    let scale = rest.first().map(n).transpose()?.unwrap_or(1);
    let img = image::open(file)?.to_rgb8();
    let ocr = Ocr::new()?;
    for l in ocr.read_placed(&img, rect, scale)? {
        println!(
            "({:4.0},{:4.0}) {:4.0}x{:3.0}  {}",
            l.x, l.y, l.width, l.height, l.text
        );
    }
    Ok(())
}
