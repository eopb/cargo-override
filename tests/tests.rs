use std::{
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
};

use cargo_override::{run, Args};

use fake::{Fake, Faker};
use googletest::{
    expect_that,
    matchers::{displays_as, eq, err, ok},
};
use tempfile::TempDir;

#[googletest::test]
fn patch_exists() {
    let working_dir = TempDir::new().unwrap();
    let working_dir = working_dir.path();

    let patch_folder: String = Faker.fake();
    let patch_folder_path = working_dir.join(&patch_folder);

    fs::create_dir(&patch_folder_path).expect("failed to create patch folder");

    let working_dir_manifest_path =
        create_cargo_manifest(working_dir, &manifest_header("test-package"));
    let _patch_manifest_path =
        create_cargo_manifest(&patch_folder_path, &manifest_header("patch-package"));

    let result = run(working_dir, Args { path: patch_folder });
    expect_that!(result, ok(eq(())));

    let manifest = fs::read_to_string(working_dir_manifest_path).unwrap();

    println!("***DEBUG: manifest content: {}", manifest);

    insta::assert_toml_snapshot!(manifest);
}

fn create_cargo_manifest(dir: &Path, content: &str) -> PathBuf {
    let manifest_path = dir.join("Cargo.toml");
    let mut manifest = File::create_new(&manifest_path).expect("failed to create manifest file");
    manifest
        .write_all(content.as_bytes())
        .expect("failed to write manifest file");
    manifest.flush().expect("failed to flush manifest file");
    manifest_path
}

fn manifest_header(crate_name: &str) -> String {
    format!(
        "[package]
name = \"{crate_name}\"
version = \"0.1.0\"
edition = \"2021\"
# See more keys and their definitions at https://doc.rust-lang.org/cargo/reference/manifest.html
"
    )
}

struct Patch<'a> {
    name: &'a str,
    path: &'a str,
}

struct PatchSet<'a> {
    registry: &'a str,
    patches: &'a [Patch<'a>],
}

impl<'a> PatchSet<'a> {
    fn format_1(&self) -> String {
        todo!()
        // [patch.crates-io]
        // uuid = { path = '../uuid' }
        // uuid = { path = '../uuid' }
    }
    fn format_2(&self) -> String {
        todo!()
        // [patch]
        // crates-io.uuid = { path = '../uuid' }
        // crates-io.uuid = { path = '../uuid' }
    }
}

#[googletest::test]
fn missing_manifest() {
    let working_dir = TempDir::new().unwrap();
    let working_dir = working_dir.path();

    let patch_folder: String = Faker.fake();
    let patch_folder_path = working_dir.join(&patch_folder);

    fs::create_dir(&patch_folder_path).expect("failed to create patch folder");

    let patch_manifest = patch_folder_path.join(&"Cargo.toml");

    File::create_new(&patch_manifest).expect("failed to create patch manifest file");

    let result = run(working_dir, Args { path: patch_folder });

    expect_that!(
        result,
        err(displays_as(eq(
            "the current working directory does not contain a `Cargo.toml` manifest",
        )))
    )
}

#[googletest::test]
fn patch_path_doesnt_exist() {
    let working_dir = TempDir::new().unwrap();
    let working_dir = working_dir.path();

    let patch_folder: String = Faker.fake();

    let result = run(
        working_dir,
        Args {
            path: patch_folder.clone(),
        },
    );

    expect_that!(
        result,
        err(displays_as(eq(format!(
            "relative path \"{}\" is not a directory",
            patch_folder
        ))))
    )
}

#[googletest::test]
fn patch_manifest_doesnt_exist() {
    let working_dir = TempDir::new().unwrap();
    let working_dir = working_dir.path();

    let patch_folder: String = Faker.fake();
    let patch_folder_path = working_dir.join(&patch_folder);

    fs::create_dir(patch_folder_path).expect("failed to create patch folder");

    let result = run(
        working_dir,
        Args {
            path: patch_folder.clone(),
        },
    );

    expect_that!(
        result,
        err(displays_as(eq(format!(
            "relative path \"{}\" does not contain a `Cargo.toml` file",
            patch_folder
        ))))
    )
}
