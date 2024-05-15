use std::{
    fs,
    ops::Not,
    path::{Path, PathBuf},
};

use anyhow::{bail, Context};
use clap::Parser;

static DEFAULT_REGISTRY: &str = "crates-io";

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    #[arg(short, long)]
    pub path: String,
}

pub fn run(working_dir: &Path, args: Args) -> anyhow::Result<()> {
    let _patch_manifest_path = patch_manifest(working_dir, &args.path)?;

    let project_manifest_path = project_manifest(working_dir)?;

    let project_manifest_content =
        fs::read_to_string(&project_manifest_path).context("failed to read patch manifest")?;

    let mut project_manifest_toml: toml_edit::DocumentMut = project_manifest_content
        .parse()
        .context("patch manifest contains invalid toml")?;

    let project_manifest_table = project_manifest_toml.as_table_mut();

    let project_patch_table = create_subtable(project_manifest_table, "patch")?;

    let mut project_patch_overrides_table = create_subtable(project_patch_table, DEFAULT_REGISTRY)?;

    let Ok(new_patch) = format!("{{ path= \"{}\" }}", args.path).parse::<toml_edit::Item>() else {
        todo!("We haven't escaped the path so we can't be sure this will parse")
    };

    toml_edit::Table::insert(&mut project_patch_overrides_table, "anyhow", new_patch);

    // TODO: handle error
    let _ = fs::write(&project_manifest_path, project_manifest_toml.to_string());

    Ok(())
}

fn create_subtable<'a>(
    table: &'a mut toml_edit::Table,
    name: &str,
) -> anyhow::Result<&'a mut toml_edit::Table> {
    let existing = &mut table[name];

    if existing.is_none() {
        // If the table does not exist, create it
        *existing = toml_edit::Item::Table(toml_edit::Table::new());
    }

    // TODO: in the future we may be able to do cool things with miette
    let _span = existing.span();

    let Some(subtable) = existing.as_table_mut() else {
        bail!("{name} already exists but is not a table")
    };

    Ok(subtable)
}

fn patch_manifest(working_dir: &Path, patch_path: &str) -> anyhow::Result<PathBuf> {
    let patch_workspace = working_dir.join(&patch_path);

    if patch_workspace.is_dir().not() {
        bail!("relative path \"{}\" is not a directory", patch_path);
    }

    let patch_manifest_path = patch_workspace.join("Cargo.toml");

    if patch_manifest_path.is_file().not() {
        bail!(
            "relative path \"{}\" does not contain a `Cargo.toml` file",
            patch_path
        )
    }

    Ok(patch_manifest_path)
}

fn project_manifest(working_dir: &Path) -> anyhow::Result<PathBuf> {
    let project_manifest = working_dir.join(&"Cargo.toml");

    if project_manifest.is_file().not() {
        bail!("the current working directory does not contain a `Cargo.toml` manifest")
    }

    Ok(project_manifest)
}
