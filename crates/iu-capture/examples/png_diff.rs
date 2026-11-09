//! Compares two PNGs inside a rectangle.
//!
//! cargo run --release -p iu-capture --example png_diff -- a.png b.png x y w h

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [a, b, rect @ ..] = args.as_slice() else {
        eprintln!("usage: png_diff a.png b.png x y w h");
        return;
    };
    let a = image::open(a).expect("open a").to_rgb8();
    let b = image::open(b).expect("open b").to_rgb8();
    let r: Vec<u32> = rect.iter().map(|s| s.parse().expect("number")).collect();
    let (x0, y0, w, h) = (r[0], r[1], r[2], r[3]);
    let (mut same, mut max, mut sum, mut hist) = (0u64, 0u8, 0u64, [0u64; 4]);
    for y in y0..y0 + h {
        for x in x0..x0 + w {
            let (pa, pb) = (a.get_pixel(x, y), b.get_pixel(x, y));
            let d = (0..3).map(|c| pa[c].abs_diff(pb[c])).max().unwrap();
            max = max.max(d);
            sum += u64::from(d);
            same += u64::from(d == 0);
            hist[match d {
                0 => 0,
                1..=2 => 1,
                3..=16 => 2,
                _ => 3,
            }] += 1;
        }
    }
    let n = f64::from(w * h);
    println!(
        "rect {x0},{y0} {w}x{h}: {:.3}% identical, max diff {max}, mean {:.4}; diff 0:{} 1-2:{} 3-16:{} >16:{}",
        same as f64 / n * 100.0,
        sum as f64 / n,
        hist[0],
        hist[1],
        hist[2],
        hist[3]
    );
}
