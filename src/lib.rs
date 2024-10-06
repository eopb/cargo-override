pub mod cli;
mod git;
pub mod registry;

mod context;
mod metadata;
mod toml;

pub use cli::{CargoInvocation, Cli};
pub use context::Context;
use context::ContextBuilder;

use std::path::Path;

use anyhow::{bail, ensure, Context as _};

pub static DEFAULT_REGISTRY: &str = "crates-io";
pub static DEFAULT_REGISTRY_URL: &str = "https://github.com/rust-lang/crates.io-index";
pub static CARGO_TOML: &str = "Cargo.toml";

pub fn run(working_dir: &Path, args: Cli) -> anyhow::Result<()> {
    let context = ContextBuilder::try_from(args)?.build(working_dir)?;

    match context.operation {
        context::Operation::Override { .. } => add_override(context),
        context::Operation::Remove { .. } => remove_override(context),
    }
}

fn add_override(
    Context {
        cargo,
        manifest_path,
        manifest_dir,
        working_dir,
        registry_hint,
        operation,
        force,
    }: Context,
) -> anyhow::Result<()> {
    let mode = match operation {
        context::Operation::Override { mode } => mode,
        _ => unreachable!(),
    };

    let path = match &mode {
        context::Mode::Path(ref path) => working_dir.join(path),
        context::Mode::Git { url, reference } => {
            git::get_source(working_dir, url, reference.clone())?
        }
    };

    let patch_manifest = metadata::crate_details(&path, cargo)?;

    let project_deps = metadata::direct_dependencies(manifest_dir.as_path(), cargo)
        .context("failed to get dependencies for current project")?;

    let mut direct_deps = project_deps
        .iter()
        .filter(|dep| dep.name == patch_manifest.name)
        .peekable();

    let dependency = if direct_deps.peek().is_some() {
        direct_deps
            .find(|dep| {
                dep.requirement
                    .as_ref()
                    .is_some_and(|req| req.matches(&patch_manifest.version) || force)
            })
            .context("patch could not be applied because version is incompatible")?
    } else {
        let resolved_deps = metadata::resolved_dependencies(manifest_dir.as_path(), cargo)
            .context("failed to get dependencies for current project")?;

        &resolved_deps
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
        dependency.registry.as_deref()
    };

    let registry = if let Some(registry_url) = &dependency_registry {
        let registry_guess = registry::get_registry_name_from_url(manifest_dir, registry_url)
            .context("failed to guess registry")?;

        match (registry_hint.to_owned(), registry_guess) {
            (Some(registry), None) => registry,
            (None, Some(registry)) => registry,
            (Some(registry_flag), Some(registry_guess)) if registry_guess == registry_flag => {
                registry_guess
            }
            (Some(registry_flag), Some(registry_guess)) => {
                ensure!(
                    force,
                    "user provided registry `{}` with the `--registry` flag \
                     but dependency `{}` \
                     uses registry `{}`. 
                     To use the registry, you passed, use `--force`",
                    registry_flag,
                    dependency.name,
                    registry_guess
                );
                registry_flag
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

    let project_path = {
        let mut manifest_path = manifest_path.clone();
        manifest_path.pop();
        manifest_path
    };

    let mut manifest = toml::Manifest::new(&manifest_path)?;
    manifest.add_patch(
        working_dir,
        &project_path,
        &registry,
        &patch_manifest.name,
        &mode,
    )?;
    manifest.write()?;

    eprintln!(
        "Patched dependency \"{}\" on registry \"{registry}\"",
        patch_manifest.name
    );

    Ok(())
}

fn remove_override(
    Context {
        manifest_path,
        manifest_dir: _,
        working_dir: _,
        cargo: _,
        force: _,
        operation,
        registry_hint: _,
    }: Context,
) -> anyhow::Result<()> {
    let package = match operation {
        context::Operation::Remove { name } => name,
        _ => unreachable!(),
    };

    let mut manifest = toml::Manifest::new(&manifest_path)?;
    let success = manifest.remove_patch(package.as_str())?;
    manifest.write()?;

    if success {
        eprintln!("Removed package patch \"{}\"", package);
    }

    Ok(())
}
