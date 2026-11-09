//! Checks whether Spout frames are pixel-identical to the game's F12
//! screenshots (PLAN 4.2): keeps the last ~1 s of Spout frames, waits for
//! a new PNG in the screenshot folder, and compares it with each buffered
//! frame, reporting the closest one.
//!
//! cargo run --release -p iu-capture --example spout_vs_f12 -- [sender] [screenshot dir] [count]

use std::collections::{HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use image::RgbImage;
use iu_capture::FrameSource;
use iu_capture::spout::SpoutReceiver;

const BUFFER_FRAMES: usize = 30;

/// Recent Spout frames with the time each was captured.
type Frames = VecDeque<(Instant, RgbImage)>;

fn pngs(dir: &Path) -> HashSet<PathBuf> {
    std::fs::read_dir(dir)
        .map(|d| {
            d.flatten()
                .map(|e| e.path())
                .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("png")))
                .collect()
        })
        .unwrap_or_default()
}

struct Diff {
    identical: f64,
    max: u8,
    mean: [f64; 3],
}

fn diff(a: &RgbImage, b: &RgbImage) -> Option<Diff> {
    if a.dimensions() != b.dimensions() {
        return None;
    }
    let (mut same, mut max, mut sum) = (0u64, 0u8, [0u64; 3]);
    for (pa, pb) in a.pixels().zip(b.pixels()) {
        let mut equal = true;
        for c in 0..3 {
            let d = pa[c].abs_diff(pb[c]);
            max = max.max(d);
            sum[c] += u64::from(d);
            equal &= d == 0;
        }
        same += u64::from(equal);
    }
    let n = f64::from(a.width() * a.height());
    Some(Diff {
        identical: same as f64 / n * 100.0,
        max,
        mean: sum.map(|s| s as f64 / n),
    })
}

fn main() {
    let mut args = std::env::args().skip(1);
    let sender = args.next().unwrap_or_else(|| "Spout_OBS_Filter".to_owned());
    let dir = PathBuf::from(
        args.next()
            .unwrap_or_else(|| r"C:\Games\beatmania IIDX INFINITAS\screenshots".to_owned()),
    );
    let count: usize = args.next().and_then(|s| s.parse().ok()).unwrap_or(3);

    let mut rx = SpoutReceiver::new(&sender).expect("D3D11 device");
    let mut seen = pngs(&dir);
    let mut buffer: Frames = VecDeque::with_capacity(BUFFER_FRAMES);
    let deadline = Instant::now() + Duration::from_secs(1800);
    let mut done = 0;
    let mut pending: Vec<(PathBuf, Instant, Frames)> = Vec::new();
    println!(
        "ready: press F12 in game ({count} times); watching {}",
        dir.display()
    );

    while done < count && Instant::now() < deadline {
        match rx.next_frame() {
            Ok(Some(frame)) => {
                if buffer.len() == BUFFER_FRAMES {
                    buffer.pop_front();
                }
                buffer.push_back((Instant::now(), frame));
            }
            Ok(None) => {}
            Err(e) => {
                println!("spout error: {e}");
                std::thread::sleep(Duration::from_secs(1));
            }
        }

        let now = pngs(&dir);
        for new in now.difference(&seen) {
            // Freeze the frames around the key press; the game may need a
            // few seconds to finish writing the PNG.
            pending.push((
                new.clone(),
                Instant::now() + Duration::from_secs(10),
                buffer.clone(),
            ));
        }
        seen = now;

        pending.retain(|(path, give_up, frames)| {
            let Ok(shot) = image::open(path).map(|i| i.to_rgb8()) else {
                if Instant::now() > *give_up {
                    println!("{}: could not read the file", path.display());
                    done += 1;
                    return false;
                }
                return true;
            };
            report(path, &shot, frames);
            done += 1;
            false
        });
        std::thread::sleep(Duration::from_millis(30));
    }
}

fn report(path: &Path, shot: &RgbImage, frames: &Frames) {
    let name = path.file_name().unwrap().to_string_lossy();
    let best = frames
        .iter()
        .filter_map(|(t, f)| diff(shot, f).map(|d| (t, d)))
        .max_by(|a, b| a.1.identical.total_cmp(&b.1.identical));
    match best {
        Some((t, d)) => println!(
            "{name}: best of {} frames (captured {} ms ago): {:.4}% identical, max diff {}, mean diff R {:.4} G {:.4} B {:.4}",
            frames.len(),
            t.elapsed().as_millis(),
            d.identical,
            d.max,
            d.mean[0],
            d.mean[1],
            d.mean[2],
        ),
        None => println!("{name}: size differs from Spout frames"),
    }
}
