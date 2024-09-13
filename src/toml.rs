use crate::context;

use std::{iter::FromIterator, path, path::Path};

use anyhow::{bail, Context as _};
use cargo_util_schemas::core::GitReference;
use fs_err as fs;
use pathdiff::diff_paths;

pub fn patch_manifest(
    working_dir: &Path,
    manifest: &str,
    manifest_directory: &Path,
    name: &str,
    registry: &str,
    mode: &context::Mode,
) -> anyhow::Result<String> {
    let mut manifest: toml_edit::DocumentMut = manifest
        .parse()
        .context("patch manifest contains invalid toml")?;

    let manifest_table = manifest.as_table_mut();

    let patch_table = create_subtable(manifest_table, "patch", true)?;

    let registry_table = create_subtable(patch_table, registry, false)?;

    toml_edit::Table::insert(
        registry_table,
        name,
        toml_edit::Item::Value(toml_edit::Value::InlineTable(source(
            working_dir,
            manifest_directory,
            mode,
        ))),
    );

    Ok(manifest.to_string())
}

fn source(
    working_dir: &Path,
    manifest_directory: &Path,
    mode: &context::Mode,
) -> toml_edit::InlineTable {
    match mode {
        context::Mode::Path(relative_path) => {
            let attempt_to_canonicalize =
                |path: &Path| fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());

            let [manifest_directory, working_dir] =
                [manifest_directory, working_dir].map(attempt_to_canonicalize);

            let path = if manifest_directory != working_dir {
                diff_paths(
                    path::absolute(working_dir.join(relative_path)).unwrap(),
                    path::absolute(manifest_directory).unwrap(),
                )
                .expect("both paths are absolute")
            } else {
                relative_path.into()
            };

            let path = path
                .as_os_str()
                .to_str()
                .expect("path must be utf8 unicode");

            toml_edit::InlineTable::from_iter([("path", path)])
        }
        context::Mode::Git { url, reference } => {
            let reference = match reference {
                GitReference::DefaultBranch => None,
                GitReference::Tag(tag) => Some(("tag", tag.as_str())),
                GitReference::Rev(rev) => Some(("rev", rev.as_str())),
                GitReference::Branch(branch) => Some(("branch", branch.as_str())),
            };

            toml_edit::InlineTable::from_iter(
                [Some(("git", url.as_str())), reference]
                    .into_iter()
                    .flatten(),
            )
        }
    }
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
