use crate::context;

use std::{path, path::Path};

use anyhow::{bail, Context as _};
use cargo_util_schemas::core::GitReference;
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

    let registry_table = create_subtable(patch_table, &registry, false)?;

    toml_edit::Table::insert(
        registry_table,
        name,
        source(working_dir, manifest_directory, mode),
    );

    Ok(manifest.to_string())
}

fn source(working_dir: &Path, manifest_directory: &Path, mode: &context::Mode) -> toml_edit::Item {
    let source = match mode {
        context::Mode::Path(relative_path) => {
            let path = if manifest_directory != working_dir {
                diff_paths(
                    path::absolute(&working_dir.join(relative_path)).unwrap(),
                    path::absolute(&manifest_directory).unwrap(),
                )
                .expect("both paths are absolute")
            } else {
                relative_path.into()
            };

            format!("{{ path = \"{}\" }}", path.display())
        }
        context::Mode::Git { url, reference } => {
            let reference = match reference {
                GitReference::DefaultBranch => String::new(),
                GitReference::Tag(tag) => format!(", tag = \"{tag}\""),
                GitReference::Rev(rev) => format!(", rev = \"{rev}\""),
                GitReference::Branch(branch) => format!(", branch = \"{branch}\""),
            };

            format!("{{ git = \"{url}\"{reference} }}")
        }
    };

    let Ok(new_patch) = source.parse::<toml_edit::Item>() else {
        todo!("We haven't escaped anything, so we can't be sure this will parse")
    };

    new_patch
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
