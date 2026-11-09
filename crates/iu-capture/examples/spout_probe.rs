//! Lists Spout senders, grabs frames from one and saves the last as PNG.
//!
//! cargo run -p iu-capture --example spout_probe -- [sender] [out.png] [frames]

use std::time::{Duration, Instant};

use iu_capture::FrameSource;
use iu_capture::spout::{SpoutReceiver, list_senders, sender_info};

fn main() {
    let mut args = std::env::args().skip(1);
    let sender = args.next().unwrap_or_else(|| "Spout_OBS_Filter".to_owned());
    let out = args.next().unwrap_or_else(|| "spout_probe.png".to_owned());
    let frames: u32 = args.next().and_then(|s| s.parse().ok()).unwrap_or(60);

    println!("senders: {:?}", list_senders());
    println!("info for {sender}: {:?}", sender_info(&sender));

    let mut rx = SpoutReceiver::new(&sender).expect("D3D11 device");
    let start = Instant::now();
    let (mut got, mut polls, mut last) = (0u32, 0u32, None);
    let mut read_time = Duration::ZERO;
    while got < frames && start.elapsed() < Duration::from_secs(10) {
        polls += 1;
        let t = Instant::now();
        match rx.next_frame() {
            Ok(Some(frame)) => {
                read_time += t.elapsed();
                got += 1;
                last = Some(frame);
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(2)),
            Err(e) => {
                println!("error: {e}");
                return;
            }
        }
    }
    let secs = start.elapsed().as_secs_f64();
    println!(
        "{got} new frames in {secs:.2}s ({:.1} fps), {polls} polls, avg read {:.2} ms",
        f64::from(got) / secs,
        read_time.as_secs_f64() * 1000.0 / f64::from(got.max(1))
    );
    if let Some(frame) = last {
        frame.save(&out).expect("save png");
        println!("saved {}x{} to {out}", frame.width(), frame.height());
    }
}
