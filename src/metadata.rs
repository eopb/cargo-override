use std::{ops::Not, path::PathBuf};

use anyhow::{bail, Context};
use cargo::core::PackageIdSpec;
use semver::VersionReq;

/// For more information, see <https://doc.rust-lang.org/book/ch07-01-packages-and-crates.html>.
#[derive(Clone)]
pub struct Dependency {
    pub name: String,
    pub requirement: Option<VersionReq>,
    pub registry: Option<String>,
}

pub fn direct_dependencies(
    project_dir: impl Into<PathBuf>,
    locked: bool,
    offline: bool,
) -> Result<Vec<Dependency>, anyhow::Error> {
    let metadata = cargo_metadata(project_dir, locked, offline, false)?;

    let root_package = metadata.root_package().unwrap();

    Ok(root_package
        .dependencies
        .iter()
        .map(
            |cargo_metadata::Dependency {
                 name,
                 req,
                 registry,
                 ..
             }| Dependency {
                name: name.clone(),
                requirement: Some(req.clone()),
                registry: registry.clone(),
            },
        )
        .collect())
}

pub fn resolved_dependencies(
    project_dir: impl Into<PathBuf>,
    locked: bool,
    offline: bool,
) -> Result<Vec<Dependency>, anyhow::Error> {
    let metadata = cargo_metadata(project_dir, locked, offline, true)?;

    let Some(cargo_metadata::Resolve { nodes, .. }) = metadata.resolve else {
        bail!("failed to resolve transative dependencies")
    };

    Ok(nodes
        .iter()
        .map(|node| PackageIdSpec::parse(&node.id.repr))
        .flat_map(|package| package.ok())
        .flat_map(|package| {
            Some(Dependency {
                name: package.name().to_owned(),
                registry: Some(package.url()?.to_string()),
                requirement: None,
            })
        })
        .collect())
}

fn cargo_metadata(
    project_dir: impl Into<PathBuf>,
    locked: bool,
    offline: bool,
    include_deps: bool,
) -> anyhow::Result<cargo_metadata::Metadata> {
    let mut cmd = cargo_metadata::MetadataCommand::new();
    cmd.current_dir(project_dir);
    cmd.other_options(
        [
            locked.then_some("--locked"),
            offline.then_some("--offline"),
            include_deps.not().then_some("--no-deps"),
        ]
        .into_iter()
        .flatten()
        .map(str::to_owned)
        .collect::<Vec<_>>(),
    );
    cmd.exec().context("Unable to run `cargo metadata`")
}
