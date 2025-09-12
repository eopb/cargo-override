use std::path::{Path, PathBuf};

use crate::{cli, metadata, CARGO_TOML};

use anyhow::bail;
use camino::Utf8PathBuf;
use cargo_util_schemas::core::GitReference;
use url::Url;

pub struct Context<'a> {
    pub cargo: Cargo,

    pub registry_hint: Option<String>,

    pub manifest_path: PathBuf,

    pub manifest_dir: PathBuf,

    pub working_dir: &'a Path,

    pub operation: Operation,

    pub force: bool,
}

#[derive(Copy, Clone, Default)]
pub struct Cargo {
    pub locked: bool,
    pub offline: bool,
}

impl Cargo {
    pub fn new(locked: bool, offline: bool, frozen: bool) -> Self {
        let [locked, offline] = [locked, offline].map(|f| f || frozen);
        Self { locked, offline }
    }
}

pub enum Operation {
    Override { mode: Mode },
    Remove { name: String },
}

pub enum Mode {
    Path(Utf8PathBuf),
    Git { url: Url, reference: GitReference },
}

pub struct ContextBuilder {
    cargo: Cargo,
    registry_hint: Option<String>,
    manifest_path: Option<Utf8PathBuf>,
    operation: Operation,
    force: bool,
}

impl ContextBuilder {
    pub fn new(operation: Operation) -> Self {
        Self {
            cargo: Cargo::default(),
            registry_hint: None,
            manifest_path: None,
            operation,
            force: false,
        }
    }

    pub fn cargo(mut self, cargo: Cargo) -> Self {
        self.cargo = cargo;
        self
    }

    pub fn registry_hint(mut self, registry_hint: Option<String>) -> Self {
        self.registry_hint = registry_hint;
        self
    }

    pub fn manifest_path(mut self, manifest_path: Option<Utf8PathBuf>) -> Self {
        self.manifest_path = manifest_path;
        self
    }

    pub fn force(mut self, force: bool) -> Self {
        self.force = force;
        self
    }

    pub fn build(self, working_dir: &Path) -> anyhow::Result<Context<'_>> {
        let (manifest_dir, manifest_path) =
            compute_manifest_paths(working_dir, self.cargo, self.manifest_path)?;

        Ok(Context {
            cargo: self.cargo,
            registry_hint: self.registry_hint,
            manifest_path,
            manifest_dir,
            working_dir,
            operation: self.operation,
            force: self.force,
        })
    }
}

impl TryFrom<cli::Cli> for ContextBuilder {
    type Error = anyhow::Error;

    fn try_from(cli: cli::Cli) -> Result<Self, Self::Error> {
        match cli.command {
            cli::CargoInvocation::Override(cli::Override {
                locked,
                offline,
                frozen,
                registry,
                manifest_path,
                source: cli::Source { path, git },
                git: cli::Git { branch, tag, rev },
                force,
            }) => {
                let mode = match (git, path) {
                    (Some(git), None) => Mode::Git {
                        url: git,
                        reference: {
                            match (branch, tag, rev) {
                                (None, None, None) => GitReference::DefaultBranch,
                                (Some(branch), None, None) => GitReference::Branch(branch),
                                (None, Some(tag), None) => GitReference::Tag(tag),
                                (None, None, Some(rev)) => GitReference::Rev(rev),
                                _ => bail!("multiple git identifiers used. Only use one of `--branch`, `--tag` or `--rev`")
                            }
                        },
                    },
                    (None, Some(path)) => Mode::Path(path),
                    (Some(_), Some(_)) => {
                        bail!("`--git` can not bot set at the same time as `--path`")
                    }
                    (None, None) => {
                        bail!("specify a package to patch with using `--path` or `--git`")
                    }
                };

                let context = ContextBuilder::new(Operation::Override { mode })
                    .registry_hint(registry)
                    .manifest_path(manifest_path)
                    .force(force)
                    .cargo(Cargo::new(locked, offline, frozen));

                Ok(context)
            }
            cli::CargoInvocation::RmOverride(cli::RmOverride {
                package,
                manifest_path,
                locked,
            }) => {
                let context = ContextBuilder::new(Operation::Remove { name: package })
                    .manifest_path(manifest_path)
                    .cargo(Cargo::new(locked, false, false));

                Ok(context)
            }
        }
    }
}

fn compute_manifest_paths(
    working_dir: &Path,
    cargo: Cargo,
    manifest_path: Option<Utf8PathBuf>,
) -> anyhow::Result<(PathBuf, PathBuf)> {
    let manifest_dir = manifest_path.map(|mut path| {
        path.pop();
        path
    });

    let manifest_dir = manifest_dir
        .as_ref()
        .map(|path| path.as_path().as_std_path())
        .unwrap_or(working_dir);

    let manifest_path = project_manifest(manifest_dir, cargo)?;

    Ok((manifest_dir.to_owned(), manifest_path))
}

fn project_manifest(manifest_path: &Path, cargo: Cargo) -> anyhow::Result<PathBuf> {
    let manifest = metadata::workspace_root(manifest_path, cargo)?.join(CARGO_TOML);

    debug_assert!(manifest.is_file(), "{:?} is not a file", manifest);

    Ok(manifest)
}
