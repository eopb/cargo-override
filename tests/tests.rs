use std::{fs, fs::File, io::Write};

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

    let patch_manifest = patch_folder_path.join("Cargo.toml");
    let _patch_manifest =
        File::create_new(&patch_manifest).expect("failed to create patch manifest file");

    let manifest = working_dir.join("Cargo.toml");
    {
        let mut manifest = File::create_new(&manifest).expect("failed to create manifest file");
        manifest
            .write_all(include_bytes!("fixture/basic.toml"))
            .expect("failed to write manifest file");
    }

    let result = run(working_dir, Args { path: patch_folder });
    expect_that!(result, ok(eq(())))
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
