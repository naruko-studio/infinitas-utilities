//! Prints which rows/columns of a rectangle contain "white" pixels
//! (min channel > threshold), as runs, to find glyph cells.
//!
//! cargo run --release -p iu-capture --example projection -- file.png x y w h [threshold]

fn runs(v: &[u32], base: u32) -> String {
    let mut out = Vec::new();
    let mut start = None;
    for (i, &c) in v.iter().chain(std::iter::once(&0)).enumerate() {
        match (start, c > 0) {
            (None, true) => start = Some(i),
            (Some(s), false) => {
                out.push(format!("{}-{}", base + s as u32, base + i as u32 - 1));
                start = None;
            }
            _ => {}
        }
    }
    out.join(" ")
}

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let img = image::open(&a[0]).expect("open").to_rgb8();
    let n = |i: usize| a[i].parse::<u32>().expect("number");
    let (x0, y0, w, h) = (n(1), n(2), n(3), n(4));
    let t: u8 = a.get(5).map_or(150, |s| s.parse().unwrap());
    let white = |x, y| {
        let p: &image::Rgb<u8> = img.get_pixel(x, y);
        p.0.iter().all(|&c| c > t)
    };
    let rows: Vec<u32> = (y0..y0 + h)
        .map(|y| (x0..x0 + w).filter(|&x| white(x, y)).count() as u32)
        .collect();
    let cols: Vec<u32> = (x0..x0 + w)
        .map(|x| (y0..y0 + h).filter(|&y| white(x, y)).count() as u32)
        .collect();
    println!("rows: {}", runs(&rows, y0));
    println!("cols: {}", runs(&cols, x0));
}
