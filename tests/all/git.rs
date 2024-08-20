//! Tests involving `--git` overrides

use super::create_cargo_manifest;
use super::manifest::{Dependency, Header, Manifest, Target};

use std::{env, path::Path};

use assert_cmd::Command;
use fs_err as fs;
use googletest::{expect_that, matchers::eq};
use tempfile::TempDir;

#[googletest::test]
fn git_patch() {
    let working_dir = TempDir::new().unwrap();
    let working_dir = working_dir.path();

    let package_name = "package-name";
    let manifest_header = Header::basic(package_name);
    let manifest = Manifest::new(manifest_header)
        .add_target(Target::bin(package_name, "src/main.rs"))
        .add_dependency(Dependency::new("redact", "0.1.0"))
        .render();

    let working_dir_manifest_path = create_cargo_manifest(working_dir, &manifest);

    let mut command = override_redact_crate(working_dir, |x| x);

    let assert = command.assert();

    let output = assert.get_output();

    let stdout = String::from_utf8(output.stdout.clone()).unwrap();
    let stderr = String::from_utf8(output.stderr.clone()).unwrap();

    assert.success();

    insta::assert_snapshot!(stdout, @"");
    insta::assert_snapshot!(stderr, @"");

    let manifest = fs::read_to_string(working_dir_manifest_path).unwrap();

    insta::assert_toml_snapshot!(manifest, @r###"
    '''
    [package]
    name = "package-name"
    version = "0.1.0"
    edition = "2021"

    # See more keys and their definitions at https://doc.rust-lang.org/cargo/reference/manifest.html

    [dependencies]
    redact = "0.1.0"

    [[bin]]
    name = "package-name"
    path = "src/main.rs"

    [patch.crates-io]
    redact = { git = "https://github.com/eopb/redact" }
    '''
    "###);
}

#[googletest::test]
fn git_patch_branch() {
    let working_dir = TempDir::new().unwrap();
    let working_dir = working_dir.path();

    let package_name = "package-name";
    let manifest_header = Header::basic(package_name);
    let manifest = Manifest::new(manifest_header)
        .add_target(Target::bin(package_name, "src/main.rs"))
        .add_dependency(Dependency::new("redact", "0.1.0"))
        .render();

    let working_dir_manifest_path = create_cargo_manifest(working_dir, &manifest);

    let mut command =
        override_redact_crate(working_dir, |command| command.arg("--branch").arg("main"));

    let assert = command.assert();

    let output = assert.get_output();

    let stdout = String::from_utf8(output.stdout.clone()).unwrap();
    let stderr = String::from_utf8(output.stderr.clone()).unwrap();

    assert.success();

    insta::assert_snapshot!(stdout, @"");
    insta::assert_snapshot!(stderr, @"");

    let manifest = fs::read_to_string(working_dir_manifest_path).unwrap();

    insta::assert_toml_snapshot!(manifest, @r###"
    '''
    [package]
    name = "package-name"
    version = "0.1.0"
    edition = "2021"

    # See more keys and their definitions at https://doc.rust-lang.org/cargo/reference/manifest.html

    [dependencies]
    redact = "0.1.0"

    [[bin]]
    name = "package-name"
    path = "src/main.rs"

    [patch.crates-io]
    redact = { git = "https://github.com/eopb/redact", branch = "main" }
    '''
    "###);
}

#[googletest::test]
fn git_patch_tag() {
    let working_dir = TempDir::new().unwrap();
    let working_dir = working_dir.path();

    let package_name = "package-name";
    let manifest_header = Header::basic(package_name);
    let manifest = Manifest::new(manifest_header)
        .add_target(Target::bin(package_name, "src/main.rs"))
        .add_dependency(Dependency::new("redact", "0.1.0"))
        .render();

    let working_dir_manifest_path = create_cargo_manifest(working_dir, &manifest);

    let mut command =
        override_redact_crate(working_dir, |command| command.arg("--tag").arg("v0.1.10"));

    let assert = command.assert();

    let output = assert.get_output();

    let stdout = String::from_utf8(output.stdout.clone()).unwrap();
    let stderr = String::from_utf8(output.stderr.clone()).unwrap();

    assert.success();

    insta::assert_snapshot!(stdout, @"");
    insta::assert_snapshot!(stderr, @"");

    let manifest = fs::read_to_string(working_dir_manifest_path).unwrap();

    insta::assert_toml_snapshot!(manifest, @r###"
    '''
    [package]
    name = "package-name"
    version = "0.1.0"
    edition = "2021"

    # See more keys and their definitions at https://doc.rust-lang.org/cargo/reference/manifest.html

    [dependencies]
    redact = "0.1.0"

    [[bin]]
    name = "package-name"
    path = "src/main.rs"

    [patch.crates-io]
    redact = { git = "https://github.com/eopb/redact", tag = "v0.1.10" }
    '''
    "###);
}

#[googletest::test]
fn git_patch_rev() {
    let working_dir = TempDir::new().unwrap();
    let working_dir = working_dir.path();

    let package_name = "package-name";
    let manifest_header = Header::basic(package_name);
    let manifest = Manifest::new(manifest_header)
        .add_target(Target::bin(package_name, "src/main.rs"))
        .add_dependency(Dependency::new("redact", "0.1.0"))
        .render();

    let working_dir_manifest_path = create_cargo_manifest(working_dir, &manifest);

    let mut command = override_redact_crate(working_dir, |command| {
        command
            .arg("--rev")
            .arg("931019c4d39af01a7ecfcb090f40f64bcfb1f295")
    });

    let assert = command.assert();

    let output = assert.get_output();

    let stdout = String::from_utf8(output.stdout.clone()).unwrap();
    let stderr = String::from_utf8(output.stderr.clone()).unwrap();

    assert.success();

    insta::assert_snapshot!(stdout, @"");
    insta::assert_snapshot!(stderr, @"");

    let manifest = fs::read_to_string(working_dir_manifest_path).unwrap();

    insta::assert_toml_snapshot!(manifest, @r###"
    '''
    [package]
    name = "package-name"
    version = "0.1.0"
    edition = "2021"

    # See more keys and their definitions at https://doc.rust-lang.org/cargo/reference/manifest.html

    [dependencies]
    redact = "0.1.0"

    [[bin]]
    name = "package-name"
    path = "src/main.rs"

    [patch.crates-io]
    redact = { git = "https://github.com/eopb/redact", rev = "931019c4d39af01a7ecfcb090f40f64bcfb1f295" }
    '''
    "###);
}

#[googletest::test]
fn git_patch_version_missmatch() {
    let working_dir = TempDir::new().unwrap();
    let working_dir = working_dir.path();

    let package_name = "package-name";
    let manifest_header = Header::basic(package_name);
    let manifest = Manifest::new(manifest_header)
        .add_target(Target::bin(package_name, "src/main.rs"))
        .add_dependency(Dependency::new("redact", "0.2.0"))
        .render();

    let manifest_path = create_cargo_manifest(working_dir, &manifest);

    let manifest_before = fs::read_to_string(&manifest_path).unwrap();

    let mut command = override_redact_crate(working_dir, |command| {
        command.arg("--tag").arg("0.1.0-pre0")
    });

    let assert = command.assert();

    let output = assert.get_output();

    let stdout = String::from_utf8(output.stdout.clone()).unwrap();
    let stderr = String::from_utf8(output.stderr.clone()).unwrap();

    assert.failure();

    insta::assert_snapshot!(stdout, @"");
    insta::assert_snapshot!(stderr, @r###"
    error: patch can not be applied becase version is incompatible
    "###);

    let manifest_after = fs::read_to_string(&manifest_path).unwrap();

    expect_that!(manifest_before, eq(manifest_after));
}

fn override_redact_crate(
    working_dir: &Path,
    args: impl Fn(&mut Command) -> &mut Command,
) -> Command {
    let mut cmd = Command::cargo_bin("cargo-override").unwrap();
    args(
        cmd.current_dir(working_dir)
            .arg("override")
            .arg("--git")
            .arg("https://github.com/eopb/redact"),
    )
    .arg("--frozen")
    .env_remove("RUST_BACKTRACE")
    .env("CARGO_HOME", working_dir);

    cmd
}
