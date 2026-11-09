//! Generates recognition resources from our own labelled screenshots.
//!
//!   cargo run --release -p resgen -- play
//!
//! Reads `<fixtures>/dev-capture/<w>x<h>/play/*.png` with their `.json`
//! labels and writes `resources/<w>x<h>/play.json`. Fixtures are found the
//! same way as in the tests (`IU_FIXTURES_DIR` or Pictures).

mod play;

use std::path::PathBuf;

use anyhow::{Context, bail};

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let root = iu_fixtures::root().context("no screenshot folder (set IU_FIXTURES_DIR)")?;
    let out_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../resources");
    match args.first().map(String::as_str) {
        Some("play") => play::run(&root, &out_dir),
        _ => bail!("usage: resgen play"),
    }
}
