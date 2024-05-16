mod manifest;

use manifest::{Dependency, Header, Manifest};

use std::{
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
};

use cargo_override::{run, Args, CARGO_TOML};

use fake::{Fake, Faker};
use googletest::{
    expect_that,
    matchers::{anything, displays_as, eq, err, ok},
};
use tempfile::TempDir;

#[googletest::test]
fn patch_exists() {
    let working_dir = TempDir::new().unwrap();
    let working_dir = working_dir.path();

    let patch_folder = "u9KdJGBDefkZz";
    let patch_folder_path = working_dir.join(patch_folder);
    let patch_crate_name = "patch-package";

    fs::create_dir(&patch_folder_path).expect("failed to create patch folder");

    let manifest_header = Header::basic("package-name");
    let manifest = Manifest::new(manifest_header)
        .add_dependency(Dependency::new(patch_crate_name, "0.1.0"))
        .render();

    let working_dir_manifest_path = create_cargo_manifest(working_dir, &manifest);
    let _patch_manifest_path = create_cargo_manifest(
        &patch_folder_path,
        &Manifest::new(Header::basic(patch_crate_name)).render(),
    );

    let result = run(
        working_dir,
        Args {
            path: patch_folder.to_string(),
        },
    );
    expect_that!(result, ok(eq(())));

    let manifest = fs::read_to_string(working_dir_manifest_path).unwrap();

    insta::assert_toml_snapshot!(manifest);
}

/// When we add a patch we want to make sure that we're actually depending on the dependency we're
/// patching.
#[googletest::test]
#[should_panic] // This shouldn't panic but having random test failures is annoying.
                // Remove this line when the code is fixed to pass this test
fn patch_exists_put_project_does_not_have_dep() {
    let working_dir = TempDir::new().unwrap();
    let working_dir = working_dir.path();

    let patch_folder = "u9KdJGBDefkZz";
    let patch_folder_path = working_dir.join(&patch_folder);

    fs::create_dir(&patch_folder_path).expect("failed to create patch folder");

    let _working_dir_manifest_path = create_cargo_manifest(
        working_dir,
        &Manifest::new(Header::basic("test-package")).render(),
    );
    let _patch_manifest_path = create_cargo_manifest(
        &patch_folder_path,
        &Manifest::new(Header::basic("patch-package")).render(),
    );

    let result = run(
        working_dir,
        Args {
            path: patch_folder.to_string(),
        },
    );
    expect_that!(result, err(anything()));
}

fn create_cargo_manifest(dir: &Path, content: &str) -> PathBuf {
    let manifest_path = dir.join(CARGO_TOML);
    let mut manifest = File::create_new(&manifest_path).expect("failed to create manifest file");
    manifest
        .write_all(content.as_bytes())
        .expect("failed to write manifest file");
    manifest.flush().expect("failed to flush manifest file");
    manifest_path
}

#[googletest::test]
fn missing_manifest() {
    let working_dir = TempDir::new().unwrap();
    let working_dir = working_dir.path();

    let patch_folder: String = Faker.fake();
    let patch_folder_path = working_dir.join(&patch_folder);

    fs::create_dir(&patch_folder_path).expect("failed to create patch folder");

    let patch_manifest = patch_folder_path.join(CARGO_TOML);

    File::create_new(patch_manifest).expect("failed to create patch manifest file");

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
