//! Cuts a template rectangle out of one PNG and finds its best match in a
//! search band of other PNGs (binarized: min channel > 128, Hamming distance).
//!
//! cargo run --release -p iu-capture --example find_template -- tpl.png tx ty tw th band_y band_h file...

fn bin(img: &image::RgbImage, x: u32, y: u32) -> bool {
    img.get_pixel(x, y).0.iter().all(|&c| c > 128)
}

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let n = |i: usize| a[i].parse::<u32>().expect("number");
    let tpl_img = image::open(&a[0]).expect("open template").to_rgb8();
    let (tx, ty, tw, th, by, bh) = (n(1), n(2), n(3), n(4), n(5), n(6));
    let tpl: Vec<bool> = (0..th)
        .flat_map(|y| (0..tw).map(move |x| (x, y)))
        .map(|(x, y)| bin(&tpl_img, tx + x, ty + y))
        .collect();
    let ones = tpl.iter().filter(|&&b| b).count();
    println!("template {tw}x{th}, {ones} white px");
    for f in &a[7..] {
        let img = image::open(f).expect("open").to_rgb8();
        let (w, _) = img.dimensions();
        let b: Vec<bool> = (by..by + bh)
            .flat_map(|y| (0..w).map(move |x| (x, y)))
            .map(|(x, y)| bin(&img, x, y))
            .collect();
        let mut best = (u32::MAX, 0, 0);
        for y in 0..bh - th {
            for x in 0..w - tw {
                let mut d = 0;
                'outer: for j in 0..th {
                    let row = ((y + j) * w + x) as usize;
                    for i in 0..tw {
                        d += u32::from(b[row + i as usize] != tpl[(j * tw + i) as usize]);
                        if d >= best.0 {
                            break 'outer;
                        }
                    }
                }
                if d < best.0 {
                    best = (d, x, by + y);
                }
            }
        }
        let name = std::path::Path::new(f)
            .file_name()
            .unwrap()
            .to_string_lossy();
        println!("{name}: dist {:4} at ({}, {})", best.0, best.1, best.2);
    }
}
