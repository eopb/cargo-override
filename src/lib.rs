mod metadata;
pub mod registry;

use std::{
    ops::Not,
    path::{Path, PathBuf},
};

use anyhow::{bail, Context};
use clap::Parser;
use fs_err as fs;

pub static DEFAULT_REGISTRY: &str = "crates-io";
pub static DEFAULT_REGISTRY_URL: &str = "https://github.com/rust-lang/crates.io-index";
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
        #[arg(long)]
        registry: Option<String>,
        /// Assert that `Cargo.lock` will remain unchanged
        #[arg(long)]
        locked: bool,
        /// Run without accessing the network
        #[arg(long)]
        offline: bool,
        /// Equivalent to specifying both --locked and --offline
        #[arg(long)]
        frozen: bool,
        #[arg(long, hide = true)]
        no_deps: bool,
    },
}

pub fn run(working_dir: &Path, args: Cli) -> anyhow::Result<()> {
    let Cli {
        command:
            CargoInvocation::Override {
                path,
                locked,
                offline,
                frozen,
                registry,
                no_deps,
            },
    } = args;

    // `--frozen` implies `--locked` and `--offline`
    let [locked, offline] = [locked, offline].map(|f| f || frozen);

    let patch_manifest_path = patch_manifest(working_dir, &path)?;

    let project_manifest_path = project_manifest(working_dir)?;

    let patch_manifest_content =
        fs::read_to_string(patch_manifest_path).context("failed to read patch manifest")?;

    let patch_manifest_toml: toml_edit::DocumentMut = patch_manifest_content
        .parse()
        .context("patch manifest contains invalid toml")?;

    let patch_manifest_details =
        ManifestDetails::read(&patch_manifest_toml).context("failed to get details for patch")?;

    let project_deps = metadata::direct_dependencies(&working_dir, locked, offline)
        .context("failed to get dependencies for current project")?;

    let mut direct_deps = project_deps
        .iter()
        .filter(|dep| dep.name == patch_manifest_details.name)
        .peekable();

    let dependency = if direct_deps.peek().is_some() {
        if let Some(dep) = direct_deps.find(|dependency| match dependency.requirement {
            Some(ref req) => req.matches(&patch_manifest_details.version),
            None => false,
        }) {
            dep.clone()
        } else {
            bail!("patch can not be applied becase version is incompatible")
        }
    } else {
        if no_deps {
            bail!("dependency can not be found");
        }
        let resolved_deps = metadata::resolved_dependencies(&working_dir, locked, offline)
            .context("failed to get dependencies for current project")?;

        resolved_deps
            .into_iter()
            .find(|dep| dep.name == patch_manifest_details.name)
            .context("dep can not be found")?
    };

    let dependency_registry = if dependency.registry == Some(DEFAULT_REGISTRY_URL.to_owned()) {
        None
    } else {
        dependency.registry
    };

    let registry = if let Some(registry_url) = &dependency_registry {
        let registry_guess =
            registry::get_registry_name_from_url(working_dir.to_path_buf(), registry_url)
                .context("failed to guess registry")?;

        match (registry.to_owned(), registry_guess) {
            (Some(registry), None) => registry,
            (None, Some(registry)) => registry,
            (Some(registry_flag), Some(registry_guess)) if registry_guess == registry_flag => {
                registry_guess
            }
            (Some(registry_flag), Some(registry_guess)) => {
                // TODO: force is unimplemented
                bail!(
                    "user provided registry `{}` with the `--registry` flag \
                     but dependency `{}` \
                     uses registry `{}`. 
                     To use the registry, you passed, use `--force`",
                    registry_flag,
                    dependency.name,
                    registry_guess
                )
            }
            (None, None) => bail!(
                "unable to determine registry name for `{}`
                 provide it using the `--registry` flag",
                registry_url
            ),
        }
    } else {
        if let Some(registry) = registry {
            if registry != DEFAULT_REGISTRY {
                bail!(
                    "user provided registry `{}` with the `--registry` flag \
                     but dependency `{}` \
                     uses the default registry `{}`",
                    registry,
                    dependency.name,
                    DEFAULT_REGISTRY,
                )
            };
        }
        DEFAULT_REGISTRY.to_owned()
    };

    let project_manifest_content =
        fs::read_to_string(&project_manifest_path).context("failed to read patch manifest")?;

    let mut project_manifest_toml: toml_edit::DocumentMut = project_manifest_content
        .parse()
        .context("patch manifest contains invalid toml")?;

    let project_manifest_table = project_manifest_toml.as_table_mut();

    let project_patch_table = create_subtable(project_manifest_table, "patch", true)?;

    let project_patch_overrides_table = create_subtable(project_patch_table, &registry, false)?;

    let Ok(new_patch) = format!("{{ path = \"{}\" }}", path).parse::<toml_edit::Item>() else {
        todo!("We haven't escaped the path so we can't be sure this will parse")
    };

    toml_edit::Table::insert(
        project_patch_overrides_table,
        patch_manifest_details.name,
        new_patch,
    );

    fs::write(&project_manifest_path, project_manifest_toml.to_string())
        .context("failed to write patched `Cargo.toml` file")?;

    Ok(())
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

struct ManifestDetails<'a> {
    name: &'a str,
    version: semver::Version,
}

impl<'a> ManifestDetails<'a> {
    fn read(document: &'a toml_edit::DocumentMut) -> anyhow::Result<Self> {
        let package = document
            .get("package")
            .context("manifest missing `package`")?;
        Ok({
            Self {
                name: package
                    .get("name")
                    .context("manifest missing `package.name`")?
                    .as_str()
                    .context("manifest `package.name` is not a string")?,
                version: package
                    .get("version")
                    .context("manifest missing `package.version`")?
                    .as_str()
                    .context("manifest `package.version` is not a string")?
                    .parse()
                    .context("manifest `package.version` is not valid semver")?,
            }
        })
    }
}
