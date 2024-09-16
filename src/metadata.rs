use crate::context;

use std::{ops::Not, path::PathBuf};

use anyhow::{bail, Context as _};
use cargo::core::PackageIdSpec;
use semver::{Version, VersionReq};
use url::Url;

#[derive(Clone)]
pub struct Crate {
    pub name: String,
    pub version: Version,
}

pub fn crate_details(
    project_dir: impl Into<PathBuf>,
    cargo: context::Cargo,
) -> Result<Crate, anyhow::Error> {
    let project_dir = project_dir.into();

    let metadata = cargo_metadata(&project_dir, cargo, false)?;

    let root_packages = metadata.workspace_default_packages();

    let package = match root_packages[..] {
        [] => {
            bail!("no package found in directory \"{project_dir:?}\"")
        }
        [_, _, ..] => {
            bail!("multiple candidate packages found in directory \"{project_dir:?}\"")
        }
        [package] => package,
    };

    Ok(Crate {
        name: package.name.clone(),
        version: package.version.clone(),
    })
}

pub fn workspace_root(
    project_dir: impl Into<PathBuf>,
    cargo: context::Cargo,
) -> Result<PathBuf, anyhow::Error> {
    let metadata = cargo_metadata(project_dir, cargo, false)?;

    Ok(metadata.workspace_root.into())
}

#[derive(Debug, Clone)]
pub struct Dependency {
    pub name: String,
    pub requirement: Option<VersionReq>,
    pub registry: Option<String>,
}

pub fn direct_dependencies(
    project_dir: impl Into<PathBuf>,
    cargo: context::Cargo,
) -> Result<Vec<Dependency>, anyhow::Error> {
    let metadata = cargo_metadata(project_dir, cargo, false)?;

    Ok(metadata
        .packages
        .into_iter()
        .flat_map(|package| package.dependencies)
        .map(
            |cargo_metadata::Dependency {
                 name,
                 req,
                 registry,
                 source,
                 ..
             }| {
                let source_url = source
                    .as_ref()
                    .and_then(|url_str| Url::parse(url_str.as_str()).ok());

                Dependency {
                    name: name.clone(),
                    requirement: Some(req.clone()),
                    registry: source_url
                        .and_then(|url| {
                            if is_git_url(&url) {
                                Some(as_repo_url(&url).to_owned())
                            } else {
                                None
                            }
                        })
                        .or(registry.clone()),
                }
            },
        )
        .collect())
}

// Url format: "git+<ssh|https>://<git@|www.>github.com/repo_path<?branch>"
//             -> "<ssh|https>://<git@|www.>github.com/repo_path"
fn as_repo_url(url: &Url) -> String {
    let (_, repo_path) = url.as_str().split_once('+').unwrap();
    let repo = repo_path.split('?').next().unwrap();
    repo.to_owned()
}

fn is_git_url(url: &Url) -> bool {
    url.scheme()
        .split_once('+')
        .map_or(false, |(scheme, _)| scheme == "git")
}

pub fn resolved_dependencies(
    project_dir: impl Into<PathBuf>,
    cargo: context::Cargo,
) -> Result<Vec<Dependency>, anyhow::Error> {
    let metadata = cargo_metadata(project_dir, cargo, true)?;

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
    context::Cargo { locked, offline }: context::Cargo,
    include_deps: bool,
) -> anyhow::Result<cargo_metadata::Metadata> {
    let mut cmd = cargo_metadata::MetadataCommand::new();
    cmd.current_dir(project_dir);
    cmd.other_options(
        [
            locked.then_some("--locked"),
            offline.then_some("--offline"),
            include_deps.not().then_some("--no-deps"),
            Some("--color"),
            Some("never"),
        ]
        .into_iter()
        .flatten()
        .map(str::to_owned)
        .collect::<Vec<_>>(),
    );
    cmd.exec().context("Unable to run `cargo metadata`")
}
