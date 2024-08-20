pub mod cli;
mod git;
pub mod registry;

mod context;
mod metadata;
mod toml;

pub use cli::{CargoInvocation, Cli};
pub use context::Context;

use std::{
    ops::Not,
    path::{Path, PathBuf},
};

use anyhow::{bail, Context as _};
use fs_err as fs;

pub static DEFAULT_REGISTRY: &str = "crates-io";
pub static DEFAULT_REGISTRY_URL: &str = "https://github.com/rust-lang/crates.io-index";
pub static CARGO_TOML: &str = "Cargo.toml";

pub fn run(working_dir: &Path, args: Cli) -> anyhow::Result<()> {
    let Context {
        cargo,
        manifest_path,
        registry_hint,
        mode,
    } = args.try_into()?;

    let path = match &mode {
        context::Mode::Path(ref path) => working_dir.join(path),
        context::Mode::Git { url, reference } => {
            git::get_source(working_dir, url, reference.clone())?
        }
    };

    let manifest_path = manifest_path.map(|mut path| {
        path.pop();
        path
    });

    let manifest_path = manifest_path
        .as_ref()
        .map(|path| path.as_path().as_std_path())
        .unwrap_or(working_dir);

    let patch_manifest = metadata::crate_details(&path, cargo)?;

    let project_manifest_path = project_manifest(manifest_path, cargo)?;

    let project_deps = metadata::direct_dependencies(manifest_path, cargo)
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
        if cargo.include_deps.not() {
            bail!("dependency can not be found");
        }
        let resolved_deps = metadata::resolved_dependencies(manifest_path, cargo)
            .context("failed to get dependencies for current project")?;

        resolved_deps
            .into_iter()
            .find(|dep| dep.name == patch_manifest.name)
            .with_context(|| {
                format!(
                    "Unable to find dependency on crate \"{}\"",
                    patch_manifest.name
                )
            })?
    };

    let dependency_registry = if dependency.registry == Some(DEFAULT_REGISTRY_URL.to_owned()) {
        None
    } else {
        dependency.registry
    };

    let registry = if let Some(registry_url) = &dependency_registry {
        let registry_guess =
            registry::get_registry_name_from_url(manifest_path.to_path_buf(), registry_url)
                .context("failed to guess registry")?;

        match (registry_hint.to_owned(), registry_guess) {
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
        if let Some(registry) = registry_hint {
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

    let project_path = {
        let mut manifest_path = project_manifest_path.clone();
        manifest_path.pop();
        manifest_path
    };

    let project_manifest_toml = toml::patch_manifest(
        working_dir,
        &project_manifest_content,
        &project_path,
        &patch_manifest.name,
        &registry,
        &mode,
    )?;

    fs::write(&project_manifest_path, &project_manifest_toml)
        .context("failed to write patched `Cargo.toml` file")?;

    Ok(())
}

fn project_manifest(manifest_path: &Path, cargo: context::Cargo) -> anyhow::Result<PathBuf> {
    let manifest =
        metadata::workspace_root(manifest_path, cargo.include_deps(false))?.join(CARGO_TOML);

    debug_assert!(manifest.is_file(), "{:?} is not a file", manifest);

    Ok(manifest)
}
