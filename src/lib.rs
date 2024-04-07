use std::{ops::Not, path::Path};

use anyhow::bail;
use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    #[arg(short, long)]
    pub path: String,
}

pub fn run(working_dir: &Path, args: Args) -> anyhow::Result<()> {
    let patch_workspace = working_dir.join(&args.path);

    if patch_workspace.is_dir().not() {
        bail!("relative path \"{}\" is not a directory", args.path);
    }

    let patch_manifest = patch_workspace.join("Cargo.toml");

    if patch_manifest.is_file().not() {
        bail!(
            "relative path \"{}\" does not contain a `Cargo.toml` file",
            args.path
        )
    }

    Ok(())
}
