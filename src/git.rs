use std::{
    io,
    path::{Path, PathBuf},
};

use anyhow::{bail, Context};
use cargo::{
    core::{shell::Shell, GitReference, SourceId},
    sources::git::GitSource,
    util::{cache_lock::CacheLockMode, context::GlobalContext},
};
use home::cargo_home_with_cwd;
use url::Url;

pub fn get_source(
    working_dir: &Path,
    url: &Url,
    reference: GitReference,
) -> anyhow::Result<PathBuf> {
    let shell = Shell::from_write(Box::new(io::sink()));

    let global_context = GlobalContext::new(
        shell,
        working_dir.to_path_buf(),
        cargo_home_with_cwd(working_dir).unwrap(),
    );

    let package_lock = global_context
        .acquire_package_cache_lock(CacheLockMode::DownloadExclusive)
        .unwrap();

    let mut git_source =
        GitSource::new(SourceId::for_git(url, reference).unwrap(), &global_context)
            .with_context(|| format!("failed to download git source. Is \"{url}\" a valid URL?"))?;

    let packages = git_source.read_packages().with_context(|| {
        format!("failed to read packages from git source. Does \"{url}\" contain a crate?")
    })?;

    drop(package_lock);

    match packages[..] {
        [] => {
            bail!("git repo {url} does not expose any crates")
        }
        [_, _, ..] => {
            bail!("multiple candiate packages found in git repo {url}")
        }
        [ref package] => Ok(package.root().to_path_buf()),
    }
}
