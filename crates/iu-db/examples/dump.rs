//! Prints one game day of `data.db` (default: the app's database, today).
//!
//! cargo run -p iu-db --example dump -- [YYYY-MM-DD] [path to data.db]

use std::path::PathBuf;

use chrono::{Local, Utc};
use iu_core::day::{default_day_start, game_day};
use iu_db::{DATABASE_FILE_NAME, Store};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let day = match args.next() {
        Some(d) => d.parse()?,
        None => game_day(Utc::now(), default_day_start()),
    };
    let path = match args.next() {
        Some(p) => PathBuf::from(p),
        None => dirs_data()?.join(DATABASE_FILE_NAME),
    };
    let store = Store::open(&path)?;
    println!("{} — game day {day}", path.display());
    for p in store.plays_on(day)? {
        let j = p.judge;
        println!(
            "{} – {}  {:<18}  PG {:>4} GR {:>4} GD {:>4} BD {:>4} PR {:>4} CB {:>4}  notes {:>4}",
            p.started_at.with_timezone(&Local).format("%H:%M:%S"),
            p.finished_at.with_timezone(&Local).format("%H:%M:%S"),
            p.layout,
            j.pg,
            j.gr,
            j.gd,
            j.bd,
            j.pr,
            j.cb,
            j.notes_hit()
        );
    }
    let s = store.day_summary(day)?;
    println!(
        "total: {} plays, {} notes, {:?}",
        s.plays,
        s.judge.notes_hit(),
        s.judge
    );
    Ok(())
}

/// `%APPDATA%\studio.naruko.infinitas-utilities` (Tauri's app data dir).
fn dirs_data() -> Result<PathBuf, &'static str> {
    std::env::var_os("APPDATA")
        .map(|d| PathBuf::from(d).join("studio.naruko.infinitas-utilities"))
        .ok_or("APPDATA is not set")
}
