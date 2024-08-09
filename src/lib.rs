use std::{
    fs,
    ops::Not,
    path::{Path, PathBuf},
};

use anyhow::{bail, Context};
use cargo_metadata::Dependency;
use clap::Parser;

pub static DEFAULT_REGISTRY: &str = "crates-io";
pub static CARGO_TOML: &str = "Cargo.toml";

#[derive(Parser, Debug)]
#[command(bin_name = "cargo")]
pub struct Cli {
    #[command(subcommand)]
    pub command: CargoInvocation,
}

#[derive(Parser, Debug)]
pub enum CargoInvocation {
    #[command(name = "override")]
    Override {
        #[arg(short, long)]
        path: String,
    },
}

pub fn run(working_dir: &Path, args: Cli) -> anyhow::Result<()> {
    let Cli {
        command: CargoInvocation::Override { path },
    } = args;

    let patch_manifest_path = patch_manifest(working_dir, &path)?;

    let project_manifest_path = project_manifest(working_dir)?;

    let project_manifest_content =
        fs::read_to_string(&project_manifest_path).context("failed to read patch manifest")?;

    let mut project_manifest_toml: toml_edit::DocumentMut = project_manifest_content
        .parse()
        .context("patch manifest contains invalid toml")?;

    let project_manifest_table = project_manifest_toml.as_table_mut();

    let project_patch_table = create_subtable(project_manifest_table, "patch", true)?;

    let project_patch_overrides_table =
        create_subtable(project_patch_table, DEFAULT_REGISTRY, false)?;

    let Ok(new_patch) = format!("{{ path = \"{}\" }}", path).parse::<toml_edit::Item>() else {
        todo!("We haven't escaped the path so we can't be sure this will parse")
    };

    let patch_manifest_content =
        fs::read_to_string(patch_manifest_path).context("failed to read patch manifest")?;

    let patch_manifest_toml: toml_edit::DocumentMut = patch_manifest_content
        .parse()
        .context("patch manifest contains invalid toml")?;

    let patch_name = patch_manifest_toml
        .get("package")
        .context("patch manifest missing `package`")?
        .get("name")
        .context("patch manifest missing `package.name`")?
        .as_str()
        .context("patch manifest `package.name` is not a string")?;

    let project_deps = get_project_dependencies(&project_manifest_path)?;

    if let Some(dependeny) = project_deps.iter().find(|dep| dep.name == patch_name) {
        println!(
            "patch dependency '{}' version requirement: '{}' found in project dependencies",
            dependeny.name, dependeny.req
        );
    };

    toml_edit::Table::insert(project_patch_overrides_table, patch_name, new_patch);

    // TODO: handle error
    let _ = fs::write(&project_manifest_path, project_manifest_toml.to_string());

    Ok(())
}

fn get_project_dependencies(
    project_manifest_path: &PathBuf,
) -> Result<Vec<Dependency>, anyhow::Error> {
    let mut cmd = cargo_metadata::MetadataCommand::new();
    cmd.manifest_path(project_manifest_path);
    let metadata = cmd.exec().context("Unable to run `cargo metadata`")?;

    let root_package = metadata.root_package().unwrap();

    Ok(root_package.dependencies.clone())
}

fn create_subtable<'a>(
    table: &'a mut toml_edit::Table,
    name: &str,
    dotted: bool,
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

    subtable.set_dotted(dotted);

    Ok(subtable)
}

fn patch_manifest(working_dir: &Path, patch_path: &str) -> anyhow::Result<PathBuf> {
    let patch_workspace = working_dir.join(patch_path);

    if patch_workspace.is_dir().not() {
        bail!("relative path \"{}\" is not a directory", patch_path);
    }

    let patch_manifest_path = patch_workspace.join(CARGO_TOML);

    if patch_manifest_path.is_file().not() {
        bail!("relative path \"{patch_path}\" does not contain a `{CARGO_TOML}` file")
    }

    Ok(patch_manifest_path)
}

fn project_manifest(working_dir: &Path) -> anyhow::Result<PathBuf> {
    let project_manifest = working_dir.join(CARGO_TOML);

    if project_manifest.is_file().not() {
        bail!("the current working directory does not contain a `{CARGO_TOML}` manifest")
    }

    Ok(project_manifest)
}
