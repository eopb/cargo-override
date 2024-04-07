use std::env::current_dir;

use cargo_override::{run, Args};

use clap::Parser;

fn main() {
    let args = Args::parse();

    if let Err(e) = run(&current_dir().unwrap(), args) {
        eprintln!("error: {e}");
    }
}
