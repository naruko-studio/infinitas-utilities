//! Prints the RGB values of a rectangle, one row per line (for checking
//! thresholds by eye).
//!
//! cargo run --release -p iu-capture --example pixels -- file.png x y w h [step]

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [file, x, y, w, h, rest @ ..] = args.as_slice() else {
        eprintln!("usage: pixels file.png x y w h [step]");
        return;
    };
    let n = |s: &String| s.parse::<u32>().expect("number");
    let (x, y, w, h) = (n(x), n(y), n(w), n(h));
    let step = rest.first().map_or(1, n).max(1);
    let img = image::open(file).expect("image").to_rgb8();
    for py in (y..y + h).step_by(step as usize) {
        let row: Vec<String> = (x..x + w)
            .step_by(step as usize)
            .map(|px| {
                let [r, g, b] = img.get_pixel(px, py).0;
                format!("{r:3},{g:3},{b:3}")
            })
            .collect();
        println!("{py:4}: {}", row.join(" "));
    }
}
