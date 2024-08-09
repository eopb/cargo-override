mod manifest;

use manifest::{Bin, Dependency, Header, Manifest};

use std::{
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
};

use cargo_override::{run, CargoInvocation, Cli, CARGO_TOML};

use fake::{Fake, Faker};
use googletest::{
    expect_that,
    matchers::{anything, displays_as, eq, err},
};
use tempfile::TempDir;
use test_case::test_case;

#[test_case("0.1.0", "0.0.2")]
#[test_case(">=1.2.3, <1.8.0", "1.2.3-alpha.1")]
#[googletest::test]
fn patch_version_incompatible(dependency_version: &str, patch_version: &str) {
    let working_dir = TempDir::new().unwrap();
    let working_dir = working_dir.path();

    let patch_crate_name = "redact";

    let patch_folder = patch_crate_name.to_string();
    let patch_folder_path = working_dir.join(patch_folder.clone());

    fs::create_dir(&patch_folder_path).expect("failed to create patch folder");

    let package_name = "package-name";
    let manifest_header = Header::basic(package_name);
    let manifest = Manifest::new(manifest_header)
        // Hack: cargo metadata fails if manifest doesn't contain [[bin]] or [lib] secion
        .add_bin(Bin::new(package_name, "src/main.rs"))
        .add_dependency(Dependency::new(patch_crate_name, dependency_version))
        .render();

    let working_dir_manifest_path = create_cargo_manifest(working_dir, &manifest);
    let _patch_manifest_path = create_cargo_manifest(
        &patch_folder_path,
        &Manifest::new(
            Header::basic(patch_crate_name)
                .name(patch_crate_name.to_owned())
                .version(patch_version.to_owned()),
        )
        .render(),
    );

    let manifest_before = fs::read_to_string(&working_dir_manifest_path).unwrap();

    let result = run(working_dir, override_path(patch_folder));

    expect_that!(result, err(anything()));

    let manifest_after = fs::read_to_string(working_dir_manifest_path).unwrap();

    expect_that!(manifest_before, eq(manifest_after));
}

#[test_case(None, None)]
#[test_case(Some("anyhow"), None)]
#[test_case(None, Some("0.1.0"))]
#[googletest::test]
fn missing_required_fields_on_patch(name: Option<&str>, version: Option<&str>) {
    let patch_crate_name = name.unwrap_or("anyhow");

    let [name, version] = [name, version].map(|option| option.map(str::to_owned));

    let working_dir = TempDir::new().unwrap();
    let working_dir = working_dir.path();

    let patch_folder = patch_crate_name.to_string();
    let patch_folder_path = working_dir.join(patch_folder.clone());

    fs::create_dir(&patch_folder_path).expect("failed to create patch folder");

    let package_name = "package-name";
    let manifest_header = Header::basic(package_name);
    let manifest = Manifest::new(manifest_header)
        // Hack: cargo metadata fails if manifest doesn't contain [[bin]] or [lib] secion
        .add_bin(Bin::new(package_name, "src/main.rs"))
        .add_dependency(Dependency::new(patch_crate_name, "1.0.86"))
        .render();

    let working_dir_manifest_path = create_cargo_manifest(working_dir, &manifest);
    let _patch_manifest_path = create_cargo_manifest(
        &patch_folder_path,
        &Manifest::new(Header::basic(patch_crate_name).name(name).version(version)).render(),
    );

    let manifest_before = fs::read_to_string(&working_dir_manifest_path).unwrap();

    let result = run(working_dir, override_path(patch_folder));
    expect_that!(result, err(anything()));

    let manifest_after = fs::read_to_string(working_dir_manifest_path).unwrap();

    expect_that!(manifest_before, eq(manifest_after));
}

#[googletest::test]
fn fail_patch_when_project_does_not_depend() {
    let working_dir = TempDir::new().unwrap();
    let working_dir = working_dir.path();

    let patch_crate_name = "anyhow";

    let patch_folder = patch_crate_name.to_string();
    let patch_folder_path = working_dir.join(patch_folder.clone());

    fs::create_dir(&patch_folder_path).expect("failed to create patch folder");

    let package_name = "package-name";
    let manifest_header = Header::basic(package_name);
    let manifest = Manifest::new(manifest_header)
        // Hack: cargo metadata fails if manifest doesn't contain [[bin]] or [lib] secion
        .add_bin(Bin::new(package_name, "src/main.rs"))
        .render();

    let working_dir_manifest_path = create_cargo_manifest(working_dir, &manifest);
    let _patch_manifest_path = create_cargo_manifest(
        &patch_folder_path,
        &Manifest::new(Header::basic(patch_crate_name)).render(),
    );

    let manifest_before = fs::read_to_string(&working_dir_manifest_path).unwrap();

    let result = run(working_dir, override_path(patch_folder));
    expect_that!(result, err(anything()));

    let manifest_after = fs::read_to_string(working_dir_manifest_path).unwrap();

    expect_that!(manifest_before, eq(manifest_after));
}

/// When we add a patch we want to make sure that we're actually depending on the dependency we're
/// patching.
#[googletest::test]
fn patch_exists_put_project_does_not_have_dep() {
    let working_dir = TempDir::new().unwrap();
    let working_dir = working_dir.path();

    let patch_folder = "u9KdJGBDefkZz";
    let patch_folder_path = working_dir.join(patch_folder);

    fs::create_dir(&patch_folder_path).expect("failed to create patch folder");

    let working_dir_manifest_path = create_cargo_manifest(
        working_dir,
        &Manifest::new(Header::basic("test-package")).render(),
    );
    let _patch_manifest_path = create_cargo_manifest(
        &patch_folder_path,
        &Manifest::new(Header::basic("patch-package")).render(),
    );

    let manifest_before = fs::read_to_string(&working_dir_manifest_path).unwrap();

    let result = run(working_dir, override_path(patch_folder));
    expect_that!(result, err(anything()));

    let manifest_after = fs::read_to_string(working_dir_manifest_path).unwrap();

    expect_that!(manifest_before, eq(manifest_after));
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

    let result = run(working_dir, override_path(patch_folder));

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

    let result = run(working_dir, override_path(patch_folder.clone()));

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

    let result = run(working_dir, override_path(patch_folder.clone()));

    expect_that!(
        result,
        err(displays_as(eq(format!(
            "relative path \"{}\" does not contain a `Cargo.toml` file",
            patch_folder
        ))))
    )
}

fn override_path(path: impl Into<String>) -> Cli {
    Cli {
        command: CargoInvocation::Override { path: path.into() },
    }
}
