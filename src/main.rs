use cargo_override::{run, Args};
use clap::Parser;

fn main() {
    let args = Args::parse();

    run(args);
}
