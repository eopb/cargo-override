use crate::context;

use std::{
    iter::FromIterator,
    path::{self, Path, PathBuf},
};

use anyhow::{bail, Context};
use cargo_util_schemas::core::GitReference;
use fs_err as fs;
use pathdiff::diff_paths;

pub struct Manifest {
    path: PathBuf,
    manifest: toml_edit::DocumentMut,
}

impl Manifest {
    pub fn new(path: impl Into<PathBuf>) -> anyhow::Result<Self> {
        let path = path.into();
        let content = Self::read(path.as_path())?;
        let manifest = Self::parse(content)?;
        Ok(Self { path, manifest })
    }

    fn read(path: &Path) -> anyhow::Result<String> {
        fs::read_to_string(path).context("failed to read patch manifest")
    }

    fn parse(content: String) -> anyhow::Result<toml_edit::DocumentMut> {
        content
            .parse()
            .context("patch manifest contains invalid toml")
    }

    pub fn write(self) -> anyhow::Result<()> {
        fs::write(&self.path, self.manifest.to_string())
            .context("failed to write patched `Cargo.toml` file")
    }

    pub fn add_patch(
        &mut self,
        working_dir: &Path,
        manifest_directory: &Path,
        registry: &str,
        name: &str,
        mode: &context::Mode,
    ) -> anyhow::Result<()> {
        let manifest_table = self.as_table_mut();

        Self::add_patch_to_manifest(
            manifest_table,
            working_dir,
            manifest_directory,
            registry,
            name,
            mode,
        )
    }

    pub fn remove_patch(&mut self, name: &str) -> anyhow::Result<bool> {
        let manifest_table = self.as_table_mut();

        Self::remove_patch_from_manifest(manifest_table, name)
    }

    fn add_patch_to_manifest(
        manifest_table: &mut toml_edit::Table,
        working_dir: &Path,
        manifest_directory: &Path,
        registry: &str,
        name: &str,
        mode: &context::Mode,
    ) -> anyhow::Result<()> {
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

        Ok(())
    }

    fn remove_patch_from_manifest(
        manifest_table: &mut toml_edit::Table,
        name: &str,
    ) -> anyhow::Result<bool> {
        let mut removed = false;
        if let Some(patch_table) = manifest_table.get_mut("patch") {
            let mut to_remove_registry = None;

            let patch_table = patch_table.as_table_mut().unwrap();
            for (registry_name, patch_table_item) in patch_table.iter_mut() {
                let registry_table = patch_table_item.as_table_mut().unwrap();
                if registry_table.remove(name).is_some() {
                    removed = true;

                    if registry_table.is_empty() {
                        to_remove_registry = Some(registry_name.to_owned());
                    }

                    // We can stop searching, it should be only one patch per package name.
                    break;
                }
            }

            // TODO: somehow it removes the comment in the manifest file -> see test
            //       Removes a comment in the final toml file when using the tool as well
            //       Maybe it thinks the comment refers to the table.
            //       Reason: sees a comment before a table as a decor which belongs to it
            //       Solution: don't remove if there is any comment in front of the table?
            //       Solution2: toml_edit should only take direct attached comments as prefix?
            //       On the other hand it wouldn't be a problem if we add the patch section at the end,
            //            there shouldn't be any comments before it, which do not belong to it.
            //
            // If the patch table is empty afterwards, will remove the patch table automatically as well.
            if let Some(registry_name) = to_remove_registry {
                patch_table.remove(registry_name.as_str());
            }
        }

        Ok(removed)
    }

    fn as_table_mut(&mut self) -> &mut toml_edit::Table {
        self.manifest.as_table_mut()
    }
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

#[cfg(test)]
mod test {
    use super::*;

    fn modify_manifest<T>(
        manifest: &str,
        f: impl FnOnce(&mut toml_edit::Table) -> anyhow::Result<T>,
    ) -> anyhow::Result<(String, T)> {
        let mut manifest: toml_edit::DocumentMut = manifest.parse().unwrap();
        let table = manifest.as_table_mut();
        let res = f(table)?;
        Ok((manifest.to_string(), res))
    }

    const TEST_MANIFEST: &str = r###"[package]
name = "package-name"
version = "0.1.0"
edition = "2021"

# See more keys and their definitions at https://doc.rust-lang.org/cargo/reference/manifest.html

[dependencies]
anyhow = "1.0.40"
pathdiff = "0.2.1"
custom-package = { git = "https://link/to/crate" }

[patch.crates-io]
anyhow = { git = "https://github.com/dtolnay/anyhow.git" }
anyhow-dev = { path = "../path/to/anyhow" }

# This is a patch for a custom package
[patch."https://link/to/crate"]
custom-package = { path = "../path/to/crate" }
"###;

    #[test]
    fn test_patch_manifest_add() {
        let (manifest_after_adding, _) = modify_manifest(TEST_MANIFEST, |manifest_table| {
            Manifest::add_patch_to_manifest(
                manifest_table,
                Path::new("/path/to/working/dir/"),
                Path::new("/path/to/working/dir/"),
                "crates-io",
                "pathdiff",
                &context::Mode::Path("../path/to/pathdiff".into()),
            )
        })
        .unwrap();

        insta::assert_toml_snapshot!(manifest_after_adding, @r###"
        '''
        [package]
        name = "package-name"
        version = "0.1.0"
        edition = "2021"

        # See more keys and their definitions at https://doc.rust-lang.org/cargo/reference/manifest.html

        [dependencies]
        anyhow = "1.0.40"
        pathdiff = "0.2.1"
        custom-package = { git = "https://link/to/crate" }

        [patch.crates-io]
        anyhow = { git = "https://github.com/dtolnay/anyhow.git" }
        anyhow-dev = { path = "../path/to/anyhow" }
        pathdiff = { path = "../path/to/pathdiff" }

        # This is a patch for a custom package
        [patch."https://link/to/crate"]
        custom-package = { path = "../path/to/crate" }
        '''
        "###);
    }

    #[test]
    fn test_patch_manifest_remove() {
        let (manifest_after_removing, removed) = modify_manifest(TEST_MANIFEST, |manifest_table| {
            Manifest::remove_patch_from_manifest(manifest_table, "custom-package")
        })
        .unwrap();

        assert!(removed);

        insta::assert_toml_snapshot!(manifest_after_removing, @r###"
        '''
        [package]
        name = "package-name"
        version = "0.1.0"
        edition = "2021"

        # See more keys and their definitions at https://doc.rust-lang.org/cargo/reference/manifest.html
        
        [dependencies]
        anyhow = "1.0.40"
        pathdiff = "0.2.1"
        custom-package = { git = "https://link/to/crate" }

        [patch.crates-io]
        anyhow = { git = "https://github.com/dtolnay/anyhow.git" }
        anyhow-dev = { path = "../path/to/anyhow" }
        '''
        "###);

        let (manifest_after_removing, removed) = modify_manifest(TEST_MANIFEST, |manifest_table| {
            Manifest::remove_patch_from_manifest(manifest_table, "anyhow-dev")
        })
        .unwrap();

        assert!(removed);

        insta::assert_toml_snapshot!(manifest_after_removing, @r###"
        '''
        [package]
        name = "package-name"
        version = "0.1.0"
        edition = "2021"

        # See more keys and their definitions at https://doc.rust-lang.org/cargo/reference/manifest.html
        
        [dependencies]
        anyhow = "1.0.40"
        pathdiff = "0.2.1"
        custom-package = { git = "https://link/to/crate" }

        [patch.crates-io]
        anyhow = { git = "https://github.com/dtolnay/anyhow.git" }

        # This is a patch for a custom package
        [patch."https://link/to/crate"]
        custom-package = { path = "../path/to/crate" }
        '''
        "###);
    }

    #[test]
    fn test_patch_manifest_remove_patch_not_exists() {
        let (manifest_after_removing, removed) = modify_manifest(TEST_MANIFEST, |manifest_table| {
            Manifest::remove_patch_from_manifest(manifest_table, "non-existing-package")
        })
        .unwrap();

        assert!(!removed);

        insta::assert_toml_snapshot!(manifest_after_removing, @r###"
        '''
        [package]
        name = "package-name"
        version = "0.1.0"
        edition = "2021"

        # See more keys and their definitions at https://doc.rust-lang.org/cargo/reference/manifest.html

        [dependencies]
        anyhow = "1.0.40"
        pathdiff = "0.2.1"
        custom-package = { git = "https://link/to/crate" }

        [patch.crates-io]
        anyhow = { git = "https://github.com/dtolnay/anyhow.git" }
        anyhow-dev = { path = "../path/to/anyhow" }

        # This is a patch for a custom package
        [patch."https://link/to/crate"]
        custom-package = { path = "../path/to/crate" }
        '''
        "###);
    }

    #[test]
    fn test_patch_manifest_remove_with_comment() {
        // illustrates the problem with removing a patch from a manifest with a comment
        let manifest_with_comment = r###"[package]
name = "package-name"
version = "0.1.0"
edition = "2021"

# See more keys and their definitions at https://doc.rust-lang.org/cargo/reference/manifest.html

[patch.crates-io]
anyhow = { path = "../path/to/anyhow" }
"###;

        let (manifest_after_removing, removed) =
            modify_manifest(manifest_with_comment, |manifest_table| {
                Manifest::remove_patch_from_manifest(manifest_table, "anyhow")
            })
            .unwrap();

        assert!(removed);

        insta::assert_toml_snapshot!(manifest_after_removing, @r###"
        '''
        [package]
        name = "package-name"
        version = "0.1.0"
        edition = "2021"
        '''
        "###);
    }
}
