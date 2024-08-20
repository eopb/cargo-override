mod checksum;
mod manifest;

use checksum::Checksum;
use manifest::{Dependency, Header, Manifest, Target};

use std::{
    env,
    fs::File,
    io::Write,
    path,
    path::{Path, PathBuf},
};

use cargo_override::{run, CargoInvocation, Cli, CARGO_TOML};

use assert_cmd::Command;
use fake::{Fake, Faker};
use fs_err as fs;
use googletest::{
    expect_that,
    matchers::{anything, displays_as, eq, err, ok},
};
use tempfile::TempDir;
use test_case::test_case;

#[googletest::test]
fn patch_transative_on_regisrty() {
    let working_dir = TempDir::new().unwrap();
    let working_dir = working_dir.path();

    let patch_crate_name = "anyhow";
    let intermediary_crate_name = "foo";
    let patch_folder = patch_crate_name.to_string();
    let patch_folder_path = working_dir.join(patch_folder.clone());

    fs::create_dir(&patch_folder_path).expect("failed to create patch folder");

    write_cargo_config(
        &working_dir,
        r#"
        [registries]
        truelayer-rustlayer = { index = "https://dl.cloudsmith.io/basic/truelayer/rustlayer/cargo/index.git" }

        [source."registry+https://dl.cloudsmith.io/basic/truelayer/rustlayer/cargo/index.git"]
        registry = "https://dl.cloudsmith.io/basic/truelayer/rustlayer/cargo/index.git"
        replace-with = "vendored-sources"

        [source.crates-io]
        replace-with = "vendored-sources"

        [source.vendored-sources]
        directory = "vendor"
        "#,
    );

    let package_name = "package_name";
    let manifest_header = Header::basic(package_name);
    let manifest = Manifest::new(manifest_header)
        .add_target(Target::bin(package_name, "src/main.rs"))
        .add_dependency(
            Dependency::new(intermediary_crate_name, "0.1.0").registry("truelayer-rustlayer"),
        )
        .render();

    let working_dir_manifest_path = create_cargo_manifest(working_dir, &manifest);
    let _patch_manifest_path = create_cargo_manifest(
        &patch_folder_path,
        &Manifest::new(Header::basic(patch_crate_name).version("1.1.5".to_owned()))
            .add_target(Target::lib(package_name, "src/lib.rs"))
            .render(),
    );

    let vendor_dir = working_dir.join("vendor");

    fs::create_dir(&vendor_dir).expect("failed to create vendor folder");

    {
        let vendored_itermediray_crate = vendor_dir.join(intermediary_crate_name);

        fs::create_dir(&vendored_itermediray_crate).expect("failed to create vendor folder");

        let manifest_header = Header::basic(intermediary_crate_name);
        let manifest = Manifest::new(manifest_header)
            .add_target(Target::lib(package_name, "src/lib.rs"))
            .add_dependency(Dependency::new(patch_crate_name, "1.0.86").registry_index(
                "https://dl.cloudsmith.io/basic/truelayer/rustlayer/cargo/index.git",
            ))
            .render();

        let _ = create_cargo_manifest(&vendored_itermediray_crate, &manifest);
        let checksum = Checksum::package_only_manifest(&manifest);
        checksum.write_to_dir(&vendored_itermediray_crate);
    }
    {
        let vendored_transative_crate = vendor_dir.join("anyhow");

        fs::create_dir(&vendored_transative_crate).expect("failed to create vendor folder");

        let manifest_header = Header::basic("anyhow").version("1.0.86".to_owned());
        let manifest = Manifest::new(manifest_header)
            .add_target(Target::lib(package_name, "src/lib.rs"))
            .render();

        let _ = create_cargo_manifest(&vendored_transative_crate, &manifest);
        let checksum = Checksum::package_only_manifest(&manifest);
        checksum.write_to_dir(&vendored_transative_crate);
    }

    let result = run(
        working_dir,
        Cli {
            command: CargoInvocation::Override {
                path: Some(patch_folder.to_owned().into()),
                frozen: false,
                locked: false,
                offline: true,
                no_deps: false,
                registry: None,
                branch: None,
                rev: None,
                tag: None,
                git: None,
                manifest_path: None,
            },
        },
    );

    expect_that!(result, ok(eq(())));

    let manifest = fs::read_to_string(working_dir_manifest_path).unwrap();

    insta::assert_toml_snapshot!(manifest, @r###"
    '''
    [package]
    name = "package_name"
    version = "0.1.0"
    edition = "2021"

    # See more keys and their definitions at https://doc.rust-lang.org/cargo/reference/manifest.html

    [dependencies]
    foo = { version = "0.1.0", registry = "truelayer-rustlayer" }

    [[bin]]
    name = "package_name"
    path = "src/main.rs"

    [patch.truelayer-rustlayer]
    anyhow = { path = "anyhow" }
    '''
    "###);
}

#[googletest::test]
fn patch_transative() {
    let working_dir = tempfile::Builder::new().keep(true).tempdir().unwrap();
    let working_dir = working_dir.path();

    let patch_crate_name = "anyhow";
    let intermediary_crate_name = "foo";
    let patch_folder = patch_crate_name.to_string();
    let patch_folder_path = working_dir.join(patch_folder.clone());

    fs::create_dir(&patch_folder_path).expect("failed to create patch folder");

    write_cargo_config(
        &working_dir,
        r#"
        [source.crates-io]
        replace-with = "vendored-sources"

        [source.vendored-sources]
        directory = "vendor"
        "#,
    );

    let package_name = "package_name";
    let manifest_header = Header::basic(package_name);
    let manifest = Manifest::new(manifest_header)
        .add_target(Target::bin(package_name, "src/main.rs"))
        .add_dependency(Dependency::new(intermediary_crate_name, "0.1.0"))
        .render();

    let working_dir_manifest_path = create_cargo_manifest(working_dir, &manifest);
    let _patch_manifest_path = create_cargo_manifest(
        &patch_folder_path,
        &Manifest::new(Header::basic(patch_crate_name).version("1.1.5".to_owned()))
            .add_target(Target::lib(package_name, "src/lib.rs"))
            .render(),
    );

    let vendor_dir = working_dir.join("vendor");

    fs::create_dir(&vendor_dir).expect("failed to create vendor folder");

    {
        let vendored_itermediray_crate = vendor_dir.join(intermediary_crate_name);

        fs::create_dir(&vendored_itermediray_crate).expect("failed to create vendor folder");

        let manifest_header = Header::basic(intermediary_crate_name);
        let manifest = Manifest::new(manifest_header)
            .add_target(Target::lib(package_name, "src/lib.rs"))
            .add_dependency(Dependency::new(patch_crate_name, "1.0.86"))
            .render();

        let _ = create_cargo_manifest(&vendored_itermediray_crate, &manifest);
        let checksum = Checksum::package_only_manifest(&manifest);
        checksum.write_to_dir(&vendored_itermediray_crate);
    }
    {
        let vendored_transative_crate = vendor_dir.join("anyhow");

        fs::create_dir(&vendored_transative_crate).expect("failed to create vendor folder");

        let manifest_header = Header::basic("anyhow").version("1.0.86".to_owned());
        let manifest = Manifest::new(manifest_header)
            .add_target(Target::lib(package_name, "src/lib.rs"))
            .render();

        let _ = create_cargo_manifest(&vendored_transative_crate, &manifest);
        let checksum = Checksum::package_only_manifest(&manifest);
        checksum.write_to_dir(&vendored_transative_crate);
    }

    let result = run(
        working_dir,
        Cli {
            command: CargoInvocation::Override {
                path: Some(patch_folder.to_owned().into()),
                frozen: false,
                locked: false,
                offline: true,
                no_deps: false,
                registry: None,
                branch: None,
                rev: None,
                tag: None,
                git: None,
                manifest_path: None,
            },
        },
    );

    expect_that!(result, ok(eq(())));

    let manifest = fs::read_to_string(working_dir_manifest_path).unwrap();

    insta::assert_toml_snapshot!(manifest, @r###"
    '''
    [package]
    name = "package_name"
    version = "0.1.0"
    edition = "2021"

    # See more keys and their definitions at https://doc.rust-lang.org/cargo/reference/manifest.html

    [dependencies]
    foo = "0.1.0"

    [[bin]]
    name = "package_name"
    path = "src/main.rs"

    [patch.crates-io]
    anyhow = { path = "anyhow" }
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

    let mut cmd = Command::cargo_bin("cargo-override").unwrap();
    _ = cmd
        .current_dir(working_dir)
        .arg("override")
        .arg("--git")
        .arg("https://github.com/eopb/redact")
        .arg("--branch")
        .arg("main")
        .arg("--frozen")
        .arg("--no-deps")
        .env("CARGO_HOME", working_dir)
        .env("exit", "42")
        .assert()
        .success();

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

    let mut cmd = Command::cargo_bin("cargo-override").unwrap();
    _ = cmd
        .current_dir(working_dir)
        .arg("override")
        .arg("--git")
        .arg("https://github.com/eopb/redact")
        .arg("--tag")
        .arg("v0.1.10")
        .arg("--frozen")
        .arg("--no-deps")
        .env("CARGO_HOME", working_dir)
        .env("exit", "42")
        .assert()
        .success();

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

    let mut cmd = Command::cargo_bin("cargo-override").unwrap();
    _ = cmd
        .current_dir(working_dir)
        .arg("override")
        .arg("--git")
        .arg("https://github.com/eopb/redact")
        .arg("--rev")
        .arg("931019c4d39af01a7ecfcb090f40f64bcfb1f295")
        .arg("--frozen")
        .arg("--no-deps")
        .env("CARGO_HOME", working_dir)
        .env("exit", "42")
        .assert()
        .success();

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

    let _ = create_cargo_manifest(working_dir, &manifest);

    let mut cmd = Command::cargo_bin("cargo-override").unwrap();
    _ = cmd
        .current_dir(working_dir)
        .arg("override")
        .arg("--git")
        .arg("https://github.com/eopb/redact")
        .arg("--tag")
        .arg("0.1.0-pre0")
        .arg("--frozen")
        .arg("--no-deps")
        .env("CARGO_HOME", working_dir)
        .env("exit", "42")
        .assert()
        .failure();
}

#[googletest::test]
fn patch_exists() {
    let working_dir = TempDir::new().unwrap();
    let working_dir = working_dir.path();

    let patch_crate_name = "anyhow";
    let patch_folder = patch_crate_name.to_string();
    let patch_folder_path = working_dir.join(patch_folder.clone());

    fs::create_dir(&patch_folder_path).expect("failed to create patch folder");

    let package_name = "package-name";
    let manifest_header = Header::basic(package_name);
    let manifest = Manifest::new(manifest_header)
        .add_target(Target::bin(package_name, "src/main.rs"))
        .add_dependency(Dependency::new(patch_crate_name, "1.0.86"))
        .render();

    let working_dir_manifest_path = create_cargo_manifest(working_dir, &manifest);
    let _patch_manifest_path = create_cargo_manifest(
        &patch_folder_path,
        &Manifest::new(Header::basic(patch_crate_name).version("1.1.5".to_owned()))
            .add_target(Target::lib(patch_crate_name, "src/lib.rs"))
            .render(),
    );

    let result = run(working_dir, override_path(&patch_folder));
    expect_that!(result, ok(eq(())));

    let manifest = fs::read_to_string(working_dir_manifest_path).unwrap();

    insta::assert_toml_snapshot!(manifest, @r###"
    '''
    [package]
    name = "package-name"
    version = "0.1.0"
    edition = "2021"

    # See more keys and their definitions at https://doc.rust-lang.org/cargo/reference/manifest.html

    [dependencies]
    anyhow = "1.0.86"

    [[bin]]
    name = "package-name"
    path = "src/main.rs"

    [patch.crates-io]
    anyhow = { path = "anyhow" }
    '''
    "###);
}

#[googletest::test]
fn patch_manifest_in_subdir() {
    let working_dir = TempDir::new().unwrap();
    let working_dir = working_dir.path();

    let patch_crate_name = "anyhow";
    let patch_folder = patch_crate_name.to_string();
    let project_folder = "subdir";
    let project_folder = working_dir.join(project_folder);
    let patch_folder_path = working_dir.join(patch_folder.clone());

    fs::create_dir(&patch_folder_path).expect("failed to create patch folder");
    fs::create_dir(&project_folder).expect("failed to create project folder");

    let package_name = "package-name";
    let manifest_header = Header::basic(package_name);
    let manifest = Manifest::new(manifest_header)
        .add_target(Target::bin(package_name, "src/main.rs"))
        .add_dependency(Dependency::new(patch_crate_name, "1.0.86"))
        .render();

    let project_manifest_path = create_cargo_manifest(&project_folder, &manifest);
    let _patch_manifest_path = create_cargo_manifest(
        &patch_folder_path,
        &Manifest::new(Header::basic(patch_crate_name).version("1.1.5".to_owned()))
            .add_target(Target::lib(patch_crate_name, "src/lib.rs"))
            .render(),
    );

    let result = run(
        working_dir,
        Cli {
            command: CargoInvocation::Override {
                path: Some(patch_folder.to_owned().into()),
                frozen: true,
                locked: false,
                offline: false,
                no_deps: true,
                registry: None,
                branch: None,
                rev: None,
                tag: None,
                git: None,
                manifest_path: Some(project_manifest_path.clone().try_into().unwrap()),
            },
        },
    );
    expect_that!(result, ok(eq(())));

    let manifest = fs::read_to_string(project_manifest_path).unwrap();

    insta::assert_toml_snapshot!(manifest, @r###"
    '''
    [package]
    name = "package-name"
    version = "0.1.0"
    edition = "2021"

    # See more keys and their definitions at https://doc.rust-lang.org/cargo/reference/manifest.html

    [dependencies]
    anyhow = "1.0.86"

    [[bin]]
    name = "package-name"
    path = "src/main.rs"

    [patch.crates-io]
    anyhow = { path = "../anyhow" }
    '''
    "###);
}

#[googletest::test]
fn patch_absolute_path() {
    let working_dir = TempDir::new().unwrap();
    let working_dir = working_dir.path();

    let patch_crate_name = "anyhow";
    let patch_folder = patch_crate_name.to_string();
    let patch_folder_path = working_dir.join(patch_folder.clone());

    fs::create_dir(&patch_folder_path).expect("failed to create patch folder");

    let package_name = "package-name";
    let manifest_header = Header::basic(package_name);
    let manifest = Manifest::new(manifest_header)
        .add_target(Target::bin(package_name, "src/main.rs"))
        .add_dependency(Dependency::new(patch_crate_name, "1.0.86"))
        .render();

    let working_dir_manifest_path = create_cargo_manifest(working_dir, &manifest);
    let _patch_manifest_path = create_cargo_manifest(
        &patch_folder_path,
        &Manifest::new(Header::basic(patch_crate_name).version("1.1.5".to_owned()))
            .add_target(Target::lib(patch_crate_name, "src/lib.rs"))
            .render(),
    );

    let result = run(
        working_dir,
        override_path(path::absolute(patch_folder_path).unwrap().to_str().unwrap()),
    );
    expect_that!(result, ok(eq(())));

    let manifest = fs::read_to_string(working_dir_manifest_path).unwrap();

    insta::with_settings!({filters => vec![
        (r"tmp\/\.tmp.*\/", "[TEMPDIR]"),
        (r"var\/.*\/\.tmp.*\/", "[TEMPDIR]"),
    ]}, {
        insta::assert_toml_snapshot!(manifest, @r###"
        '''
        [package]
        name = "package-name"
        version = "0.1.0"
        edition = "2021"

        # See more keys and their definitions at https://doc.rust-lang.org/cargo/reference/manifest.html

        [dependencies]
        anyhow = "1.0.86"

        [[bin]]
        name = "package-name"
        path = "src/main.rs"

        [patch.crates-io]
        anyhow = { path = "/[TEMPDIR]anyhow" }
        '''
        "###);
    });
}

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
        .add_target(Target::bin(package_name, "src/main.rs"))
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

    let result = run(working_dir, override_path(&patch_folder));

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
        .add_target(Target::bin(package_name, "src/main.rs"))
        .add_dependency(Dependency::new(patch_crate_name, "1.0.86"))
        .render();

    let working_dir_manifest_path = create_cargo_manifest(working_dir, &manifest);
    let _patch_manifest_path = create_cargo_manifest(
        &patch_folder_path,
        &Manifest::new(Header::basic(patch_crate_name).name(name).version(version)).render(),
    );

    let manifest_before = fs::read_to_string(&working_dir_manifest_path).unwrap();

    let result = run(working_dir, override_path(&patch_folder));
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
        .add_target(Target::bin(package_name, "src/main.rs"))
        .render();

    let working_dir_manifest_path = create_cargo_manifest(working_dir, &manifest);
    let _patch_manifest_path = create_cargo_manifest(
        &patch_folder_path,
        &Manifest::new(Header::basic(patch_crate_name)).render(),
    );

    let manifest_before = fs::read_to_string(&working_dir_manifest_path).unwrap();

    let result = run(working_dir, override_path(&patch_folder));
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

    let result = run(working_dir, override_path(&patch_folder));
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

    let result = run(working_dir, override_path(&patch_folder));

    expect_that!(result, err(displays_as(anything())))
}

#[googletest::test]
fn patch_path_doesnt_exist() {
    let working_dir = TempDir::new().unwrap();
    let working_dir = working_dir.path();

    let patch_folder: String = Faker.fake();

    let result = run(working_dir, override_path(&patch_folder.clone()));

    expect_that!(result, err(displays_as(anything())))
}

#[googletest::test]
fn patch_manifest_doesnt_exist() {
    let working_dir = TempDir::new().unwrap();
    let working_dir = working_dir.path();

    let patch_folder: String = Faker.fake();
    let patch_folder_path = working_dir.join(&patch_folder);

    fs::create_dir(patch_folder_path).expect("failed to create patch folder");

    let result = run(working_dir, override_path(&patch_folder.clone()));

    expect_that!(result, err(displays_as(anything())))
}

fn write_cargo_config(path: &Path, toml: &str) {
    let cargo_config_dir = path.join(".cargo");

    fs::create_dir(&cargo_config_dir).expect("failed to create `.cargo` folder");

    let cargo_config = cargo_config_dir.join("config.toml");

    fs::write(&cargo_config, toml).expect("failed to write `.cargo/config.toml`");
}
fn basic_cargo_config(path: &Path) {
    write_cargo_config(
        &path,
        r#"
        [registries]
        truelayer-rustlayer = { index = "https://dl.cloudsmith.io/basic/truelayer/rustlayer/cargo/index.git" }
        "#,
    )
}

#[cfg(feature = "failing_tests")]
fn basic_cargo_env_config(path: &Path) {
    write_cargo_config(
        &path,
        r#"
        [env]
        CARGO_REGISTRIES_TRUELAYER_RUSTLAYER_INDEX = "https://dl.cloudsmith.io/basic/truelayer/rustlayer/cargo/index.git"
        "#,
    )
}

#[cfg_attr(feature = "failing_tests", test_case(basic_cargo_env_config))]
#[test_case(basic_cargo_config)]
#[googletest::test]
fn patch_exists_alt_registry(setup: impl Fn(&Path)) {
    // let working_dir = TempDir::new().unwrap();
    let mut working_dir = tempfile::Builder::new();
    let working_dir = (&mut working_dir).keep(true).tempdir().unwrap();
    let working_dir = working_dir.path();

    setup(working_dir);

    let patch_crate_name = "anyhow";
    let patch_folder = patch_crate_name.to_string();
    let patch_folder_path = working_dir.join(patch_folder.clone());

    fs::create_dir(&patch_folder_path).expect("failed to create patch folder");

    let package_name = "package-name";
    let manifest_header = Header::basic(package_name);
    let manifest = Manifest::new(manifest_header)
        .add_target(Target::bin(package_name, "src/main.rs"))
        .add_dependency(Dependency::new(patch_crate_name, "1.0.86").registry("truelayer-rustlayer"))
        .render();

    let working_dir_manifest_path = create_cargo_manifest(working_dir, &manifest);
    let _patch_manifest_path = create_cargo_manifest(
        &patch_folder_path,
        &Manifest::new(Header::basic(patch_crate_name).version("1.1.5".to_owned()))
            .add_target(Target::lib(patch_crate_name, "src/lib.rs"))
            .render(),
    );

    let result = run(working_dir, override_path(&patch_folder));
    expect_that!(result, ok(eq(())));

    let manifest = fs::read_to_string(working_dir_manifest_path).unwrap();

    insta::allow_duplicates! {
        insta::assert_toml_snapshot!(manifest, @r###"
        '''
        [package]
        name = "package-name"
        version = "0.1.0"
        edition = "2021"

        # See more keys and their definitions at https://doc.rust-lang.org/cargo/reference/manifest.html

        [dependencies]
        anyhow = { version = "1.0.86", registry = "truelayer-rustlayer" }

        [[bin]]
        name = "package-name"
        path = "src/main.rs"

        [patch.truelayer-rustlayer]
        anyhow = { path = "anyhow" }
        '''
        "###);
    }
}

#[googletest::test]
fn patch_exists_alt_registry_from_env() {
    // let working_dir = TempDir::new().unwrap();
    let mut working_dir = tempfile::Builder::new();
    let working_dir = (&mut working_dir).keep(true).tempdir().unwrap();
    let working_dir = working_dir.path();

    let patch_crate_name = "anyhow";
    let patch_folder = patch_crate_name.to_string();
    let patch_folder_path = working_dir.join(patch_folder.clone());

    fs::create_dir(&patch_folder_path).expect("failed to create patch folder");

    let package_name = "package-name";
    let manifest_header = Header::basic(package_name);
    let manifest = Manifest::new(manifest_header)
        .add_target(Target::bin(package_name, "src/main.rs"))
        .add_dependency(Dependency::new(patch_crate_name, "1.0.86").registry("truelayer-rustlayer"))
        .render();

    let working_dir_manifest_path = create_cargo_manifest(working_dir, &manifest);
    let _patch_manifest_path = create_cargo_manifest(
        &patch_folder_path,
        &Manifest::new(Header::basic(patch_crate_name).version("1.1.5".to_owned()))
            .add_target(Target::lib(patch_crate_name, "src/lib.rs"))
            .render(),
    );

    let mut cmd = Command::cargo_bin("cargo-override").unwrap();

    _ = cmd
        .current_dir(working_dir)
        .arg("override")
        .arg("--path")
        .arg(patch_folder)
        .arg("--frozen")
        .arg("--no-deps")
        .env(
            "CARGO_REGISTRIES_TRUELAYER_RUSTLAYER_INDEX",
            "https://dl.cloudsmith.io/basic/truelayer/rustlayer/cargo/index.git",
        )
        .env("exit", "42")
        .assert()
        .success();

    let manifest = fs::read_to_string(working_dir_manifest_path).unwrap();

    insta::allow_duplicates! {
        insta::assert_toml_snapshot!(manifest, @r###"
        '''
        [package]
        name = "package-name"
        version = "0.1.0"
        edition = "2021"

        # See more keys and their definitions at https://doc.rust-lang.org/cargo/reference/manifest.html

        [dependencies]
        anyhow = { version = "1.0.86", registry = "truelayer-rustlayer" }

        [[bin]]
        name = "package-name"
        path = "src/main.rs"

        [patch.truelayer-rustlayer]
        anyhow = { path = "anyhow" }
        '''
        "###);
    }
}

fn override_path(path: &str) -> Cli {
    Cli {
        command: CargoInvocation::Override {
            path: Some(path.to_owned().into()),
            frozen: true,
            locked: false,
            offline: false,
            no_deps: true,
            registry: None,
            branch: None,
            rev: None,
            tag: None,
            git: None,
            manifest_path: None,
        },
    }
}
