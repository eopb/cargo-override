use std::{
    ops::Not,
    path::{Path, PathBuf},
};

use anyhow::bail;
use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    #[arg(short, long)]
    pub path: String,
}

pub fn run(working_dir: &Path, args: Args) -> anyhow::Result<()> {
    let _patch_manifest = patch_manifest(working_dir, &args.path)?;
    let _project_manifest = project_manifest(working_dir)?;

    Ok(())
}

fn patch_manifest(working_dir: &Path, patch_path: &str) -> anyhow::Result<PathBuf> {
    let patch_workspace = working_dir.join(&patch_path);

    if patch_workspace.is_dir().not() {
        bail!("relative path \"{}\" is not a directory", patch_path);
    }

    let patch_manifest = patch_workspace.join("Cargo.toml");

    if patch_manifest.is_file().not() {
        bail!(
            "relative path \"{}\" does not contain a `Cargo.toml` file",
            patch_path
        )
    }

    Ok(patch_manifest)
}

fn project_manifest(working_dir: &Path) -> anyhow::Result<PathBuf> {
    let project_manifest = working_dir.join(&"Cargo.toml");

    if project_manifest.is_file().not() {
        bail!("the current working directory does not contain a `Cargo.toml` manifest")
    }

    Ok(project_manifest)
}
