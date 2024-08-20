pub mod registry;

mod metadata;
mod toml;

use std::path::{Path, PathBuf};

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

    let patch_manifest = metadata::crate_details(working_dir.join(&path), locked, offline)?;

    let project_manifest_path = project_manifest(working_dir)?;

    let project_deps = metadata::direct_dependencies(&working_dir, locked, offline)
        .context("failed to get dependencies for current project")?;

    let mut direct_deps = project_deps
        .iter()
        .filter(|dep| dep.name == patch_manifest.name)
        .peekable();

    let dependency = if direct_deps.peek().is_some() {
        if let Some(dep) = direct_deps.find(|dependency| match dependency.requirement {
            Some(ref req) => req.matches(&patch_manifest.version),
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
            .find(|dep| dep.name == patch_manifest.name)
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

    let project_manifest_toml = toml::patch_manifest(
        &project_manifest_content,
        &patch_manifest.name,
        &registry,
        &path,
    )?;

    fs::write(&project_manifest_path, project_manifest_toml.to_string())
        .context("failed to write patched `Cargo.toml` file")?;

    Ok(())
}

fn project_manifest(working_dir: &Path) -> anyhow::Result<PathBuf> {
    let manifest = metadata::workspace_root(working_dir, false, false)?.join(CARGO_TOML);

    debug_assert!(manifest.is_file(), "{:?} is not a file", manifest);

    Ok(manifest)
}
