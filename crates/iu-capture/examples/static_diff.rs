//! Compares an F12 screenshot with live Spout frames on the pixels that
//! stay constant across ~2 s of Spout frames (animated backgrounds and
//! semi-transparent panels are excluded). Use while the game shows the same
//! screen as the screenshot.
//!
//! cargo run --release -p iu-capture --example static_diff -- shot.png [sender]

use std::time::{Duration, Instant};

use iu_capture::FrameSource;
use iu_capture::spout::SpoutReceiver;

fn main() {
    let mut args = std::env::args().skip(1);
    let shot = image::open(args.next().expect("shot.png"))
        .expect("open screenshot")
        .to_rgb8();
    let sender = args.next().unwrap_or_else(|| "Spout_OBS_Filter".to_owned());

    let mut rx = SpoutReceiver::new(&sender).expect("D3D11 device");
    let first = loop {
        if let Some(f) = rx.next_frame().expect("spout frame") {
            break f;
        }
    };
    assert_eq!(first.dimensions(), shot.dimensions(), "size differs");
    let mut stable = vec![true; (first.width() * first.height()) as usize];
    let (start, mut frames) = (Instant::now(), 1);
    while start.elapsed() < Duration::from_secs(2) {
        if let Some(f) = rx.next_frame().expect("spout frame") {
            frames += 1;
            for ((s, a), b) in stable.iter_mut().zip(first.pixels()).zip(f.pixels()) {
                *s &= a == b;
            }
        }
        std::thread::sleep(Duration::from_millis(30));
    }

    let (mut n, mut same, mut max) = (0u64, 0u64, 0u8);
    let mut hist = [0u64; 4];
    let mut signed = [0i64; 3];
    for ((s, a), b) in stable.iter().zip(first.pixels()).zip(shot.pixels()) {
        if !*s {
            continue;
        }
        n += 1;
        let d = (0..3).map(|c| a[c].abs_diff(b[c])).max().unwrap();
        for c in 0..3 {
            signed[c] += i64::from(a[c]) - i64::from(b[c]);
        }
        max = max.max(d);
        same += u64::from(d == 0);
        hist[match d {
            0 => 0,
            1 => 1,
            2..=8 => 2,
            _ => 3,
        }] += 1;
    }
    let total = u64::from(first.width() * first.height());
    println!(
        "{frames} Spout frames; {n} static pixels ({:.1}% of screen)",
        n as f64 / total as f64 * 100.0
    );
    println!(
        "vs screenshot: {:.3}% identical, max diff {max}; diff 0:{} 1:{} 2-8:{} >8:{}",
        same as f64 / n.max(1) as f64 * 100.0,
        hist[0],
        hist[1],
        hist[2],
        hist[3]
    );
    let nn = n.max(1) as f64;
    println!(
        "mean signed (Spout - shot): R {:+.4} G {:+.4} B {:+.4}",
        signed[0] as f64 / nn,
        signed[1] as f64 / nn,
        signed[2] as f64 / nn
    );
}
