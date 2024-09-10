pub mod checksum;
#[path = "cli.rs"]
mod cli_tests;
mod git;
pub mod manifest;

use checksum::Checksum;
use manifest::{Dependency, Header, Manifest, Target};

use std::{
    env,
    fs::File,
    io::Write,
    path,
    path::{Path, PathBuf},
};

use cargo_override::CARGO_TOML;

use assert_cmd::Command;
use fake::{Fake, Faker};
use fs_err as fs;
use googletest::{expect_eq, verify_eq, verify_that};
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
        working_dir,
        r#"
        [registries]
        private-registry = { index = "https://dl.cloudsmith.io/basic/private/registry/cargo/index.git" }

        [source."registry+https://dl.cloudsmith.io/basic/private/registry/cargo/index.git"]
        registry = "https://dl.cloudsmith.io/basic/private/registry/cargo/index.git"
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
            Dependency::new(intermediary_crate_name, "0.1.0").registry("private-registry"),
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
        let manifest =
            Manifest::new(manifest_header)
                .add_target(Target::lib(package_name, "src/lib.rs"))
                .add_dependency(Dependency::new(patch_crate_name, "1.0.86").registry_index(
                    "https://dl.cloudsmith.io/basic/private/registry/cargo/index.git",
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

    let mut command = override_path(&patch_folder, working_dir, |command| command);

    let assert = command.assert();

    let output = assert.get_output();

    let stdout = String::from_utf8(output.stdout.clone()).unwrap();
    let stderr = String::from_utf8(output.stderr.clone()).unwrap();

    assert.success();

    insta::assert_snapshot!(stdout, @"");
    insta::assert_snapshot!(stderr, @r###"
    Patched dependency "anyhow" on registry "private-registry"
    "###);

    let manifest = fs::read_to_string(working_dir_manifest_path).unwrap();

    insta::assert_toml_snapshot!(manifest, @r###"
    '''
    [package]
    name = "package_name"
    version = "0.1.0"
    edition = "2021"

    # See more keys and their definitions at https://doc.rust-lang.org/cargo/reference/manifest.html

    [dependencies]
    foo = { version = "0.1.0", registry = "private-registry" }

    [[bin]]
    name = "package_name"
    path = "src/main.rs"

    [patch.private-registry]
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
        working_dir,
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

    let mut command = override_path(&patch_folder, working_dir, |command| command);

    let assert = command.assert();

    let output = assert.get_output();

    let stdout = String::from_utf8(output.stdout.clone()).unwrap();
    let stderr = String::from_utf8(output.stderr.clone()).unwrap();

    assert.success();

    insta::assert_snapshot!(stdout, @"");
    insta::assert_snapshot!(stderr, @r###"
    Patched dependency "anyhow" on registry "crates-io"
    "###);

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

    let mut command = override_path(&patch_folder, working_dir, |command| command);

    let assert = command.assert();

    let output = assert.get_output();

    let stdout = String::from_utf8(output.stdout.clone()).unwrap();
    let stderr = String::from_utf8(output.stderr.clone()).unwrap();

    assert.success();

    insta::assert_snapshot!(stdout, @"");
    insta::assert_snapshot!(stderr, @r###"
    Patched dependency "anyhow" on registry "crates-io"
    "###);

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
fn patch_uses_workspace_version_inheritance() {
    let working_dir = TempDir::new().unwrap();
    let working_dir = working_dir.path();

    let workspace_folder = "workspace";
    let workspace_folder = working_dir.join(workspace_folder);

    let patch_crate_name = "anyhow";
    let patch_folder = patch_crate_name.to_string();
    let patch_folder_path = workspace_folder.join(patch_folder.clone());

    fs::create_dir(&workspace_folder).expect("failed to create patch folder");
    fs::create_dir(&patch_folder_path).expect("failed to create patch folder");

    let package_name = "package-name";
    let manifest_header = Header::basic(package_name);
    let manifest = Manifest::new(manifest_header)
        .add_target(Target::bin(package_name, "src/main.rs"))
        .add_dependency(Dependency::new(patch_crate_name, "1.0.86"))
        .render();

    let working_dir_manifest_path = create_cargo_manifest(working_dir, &manifest);
    let _workspace_manifest_path = create_cargo_manifest(
        &workspace_folder,
        r#"
        [workspace]
        members = ["anyhow"]
        [workspace.package]
        version = "1.0.87"
        "#,
    );
    let _patch_manifest_path = create_cargo_manifest(
        &patch_folder_path,
        r#"
        [package]
        name = "anyhow"
        version.workspace = true
        edition = "2021"

        [lib]
        name = "anyhow"
        path = "src/lib.rs"
        "#,
    );

    let mut command = override_path("workspace/anyhow", working_dir, |command| command);

    let assert = command.assert();

    let output = assert.get_output();

    let stdout = String::from_utf8(output.stdout.clone()).unwrap();
    let stderr = String::from_utf8(output.stderr.clone()).unwrap();

    assert.success();

    insta::assert_snapshot!(stdout, @"");
    insta::assert_snapshot!(stderr, @r###"
    Patched dependency "anyhow" on registry "crates-io"
    "###);

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
    anyhow = { path = "workspace/anyhow" }
    '''
    "###);
}

#[googletest::test]
fn project_is_workspace() {
    let working_dir = TempDir::new().unwrap();
    let working_dir = working_dir.path();

    let patch_crate_name = "anyhow";
    let patch_folder = patch_crate_name.to_string();

    let workspace_folder = "workspace";
    let workspace_folder = working_dir.join(workspace_folder);

    let project_folder = "subdir";
    let project_folder = workspace_folder.join(project_folder);
    let patch_folder_path = working_dir.join(patch_folder.clone());

    fs::create_dir(&patch_folder_path).expect("failed to create patch folder");
    fs::create_dir(&workspace_folder).expect("failed to create project folder");
    fs::create_dir(&project_folder).expect("failed to create project folder");

    let workspace_folder_manifest_path = create_cargo_manifest(
        &workspace_folder,
        r#"
        [workspace]
        members = ["subdir"]
        "#,
    );

    let package_name = "package-name";
    let manifest_header = Header::basic(package_name);
    let manifest = Manifest::new(manifest_header)
        .add_target(Target::bin(package_name, "src/main.rs"))
        .add_dependency(Dependency::new(patch_crate_name, "1.0.86"))
        .render();

    let _ = create_cargo_manifest(&project_folder, &manifest);
    let _patch_manifest_path = create_cargo_manifest(
        &patch_folder_path,
        &Manifest::new(Header::basic(patch_crate_name).version("1.1.5".to_owned()))
            .add_target(Target::lib(patch_crate_name, "src/lib.rs"))
            .render(),
    );

    let mut command = override_path(&patch_folder, working_dir, |command| {
        command
            .arg("--manifest-path")
            .arg(workspace_folder_manifest_path.as_os_str())
    });

    let assert = command.assert();

    let output = assert.get_output();

    let stdout = String::from_utf8(output.stdout.clone()).unwrap();
    let stderr = String::from_utf8(output.stderr.clone()).unwrap();

    assert.success();

    insta::assert_snapshot!(stdout, @"");
    insta::assert_snapshot!(stderr, @r###"
    Patched dependency "anyhow" on registry "crates-io"
    "###);

    let manifest = fs::read_to_string(workspace_folder_manifest_path).unwrap();

    insta::assert_toml_snapshot!(manifest, @r###"
    '''

            [workspace]
            members = ["subdir"]

    [patch.crates-io]
    anyhow = { path = "../anyhow" }
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

    let mut command = override_path(&patch_folder, working_dir, |command| {
        command
            .arg("--manifest-path")
            .arg(project_manifest_path.as_os_str())
    });

    let assert = command.assert();

    let output = assert.get_output();

    let stdout = String::from_utf8(output.stdout.clone()).unwrap();
    let stderr = String::from_utf8(output.stderr.clone()).unwrap();

    assert.success();

    insta::assert_snapshot!(stdout, @"");
    insta::assert_snapshot!(stderr, @r###"
    Patched dependency "anyhow" on registry "crates-io"
    "###);

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

    let mut command = override_path(
        path::absolute(patch_folder_path).unwrap().to_str().unwrap(),
        working_dir,
        |command| command,
    );

    let assert = command.assert();

    let output = assert.get_output();

    let stdout = String::from_utf8(output.stdout.clone()).unwrap();
    let stderr = String::from_utf8(output.stderr.clone()).unwrap();

    assert.success();

    insta::assert_snapshot!(stdout, @"");
    insta::assert_snapshot!(stderr, @r###"
    Patched dependency "anyhow" on registry "crates-io"
    "###);

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
        .add_target(Target::lib("patch_pacakge", "src/lib.rs"))
        .render(),
    );

    let manifest_before = fs::read_to_string(&working_dir_manifest_path).unwrap();

    let mut command = override_path(&patch_folder, working_dir, |command| command);

    let assert = command.assert();

    let output = assert.get_output();

    let stdout = String::from_utf8(output.stdout.clone()).unwrap();
    let stderr = String::from_utf8(output.stderr.clone()).unwrap();

    assert.failure();

    insta::allow_duplicates! {
        insta::assert_snapshot!(stdout, @"");
        insta::assert_snapshot!(stderr, @r###"
        error: patch could not be applied because version is incompatible
        "###
        );
    }

    let manifest_after = fs::read_to_string(working_dir_manifest_path).unwrap();

    expect_eq!(manifest_before, manifest_after);
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

    let mut command = override_path(&patch_folder, working_dir, |command| command);

    let assert = command.assert();

    assert.failure();

    let manifest_after = fs::read_to_string(working_dir_manifest_path).unwrap();

    expect_eq!(manifest_before, manifest_after);
}

/// When we add a patch we want to make sure that we're actually depending on the dependency we're
/// patching.
#[googletest::test]
fn patch_exists_put_project_does_not_depend_on_it() {
    let working_dir = TempDir::new().unwrap();
    let working_dir = working_dir.path();

    let patch_folder = "u9KdJGBDefkZz";
    let patch_folder_path = working_dir.join(patch_folder);

    fs::create_dir(&patch_folder_path).expect("failed to create patch folder");

    let working_dir_manifest_path = create_cargo_manifest(
        working_dir,
        &Manifest::new(Header::basic("test-package"))
            .add_target(Target::bin("test-package", "src/main.rs"))
            .render(),
    );
    let _patch_manifest_path = create_cargo_manifest(
        &patch_folder_path,
        &Manifest::new(Header::basic("patch_package"))
            .add_target(Target::lib("patch_pacakge", "src/lib.rs"))
            .render(),
    );

    let manifest_before = fs::read_to_string(&working_dir_manifest_path).unwrap();

    let mut command = override_path(patch_folder, working_dir, |command| command);

    let assert = command.assert();

    let output = assert.get_output();

    let stdout = String::from_utf8(output.stdout.clone()).unwrap();
    let stderr = String::from_utf8(output.stderr.clone()).unwrap();

    assert.failure();

    insta::assert_snapshot!(stdout, @"");
    insta::assert_snapshot!(stderr, @r###"
    error: Unable to find dependency on crate "patch_package"
    "###);

    let manifest_after = fs::read_to_string(working_dir_manifest_path).unwrap();

    expect_eq!(manifest_before, manifest_after);
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

    let mut command = override_path(&patch_folder, working_dir, |command| command);

    let assert = command.assert();

    let output = assert.get_output();

    let stdout = String::from_utf8(output.stdout.clone()).unwrap();
    let stderr = String::from_utf8(output.stderr.clone()).unwrap();

    assert.failure();

    insta::with_settings!({filters => vec![
        (r"tmp\/\.tmp.*\/", "[TEMPDIR]"),
        (r"private\/var\/.*\/\.tmp.*\/", "[TEMPDIR]"),
        (r"var\/.*\/\.tmp.*\/", "[TEMPDIR]"),
        (&patch_folder, "[PATCH]"),
    ]}, {
        insta::assert_snapshot!(stdout, @"");
        insta::assert_snapshot!(stderr, @r###"
        error: Unable to run `cargo metadata`

        Caused by:
            `cargo metadata` exited with an error: error: failed to parse manifest at `/[TEMPDIR]Cargo.toml`
            
            Caused by:
              virtual manifests must be configured with [workspace]
            
        "###);
    });
}

#[googletest::test]
fn patch_path_doesnt_exist() {
    let working_dir = TempDir::new().unwrap();
    let working_dir = working_dir.path();

    let patch_folder: String = Faker.fake();

    let mut command = override_path(&patch_folder, working_dir, |command| command);

    let assert = command.assert();

    let output = assert.get_output();

    let stdout = String::from_utf8(output.stdout.clone()).unwrap();
    let stderr = String::from_utf8(output.stderr.clone()).unwrap();

    assert.failure();

    insta::with_settings!({filters => vec![
        (r"tmp\/\.tmp.*\/", "[TEMPDIR]"),
        (r"var\/.*\/\.tmp.*\/", "[TEMPDIR]"),
        (&patch_folder, "[PATCH]"),
    ]}, {
        insta::assert_snapshot!(stdout, @"");
        insta::assert_snapshot!(stderr, @r###"
        error: Unable to run `cargo metadata`

        Caused by:
            0: failed to start `cargo metadata`: No such file or directory (os error 2)
            1: No such file or directory (os error 2)
        "###);
    });
}

#[googletest::test]
fn patch_manifest_doesnt_exist() {
    let working_dir = TempDir::new().unwrap();
    let working_dir = working_dir.path();

    let patch_folder: String = Faker.fake();
    let patch_folder_path = working_dir.join(&patch_folder);

    fs::create_dir(patch_folder_path).expect("failed to create patch folder");

    let mut command = override_path(&patch_folder, working_dir, |command| command);

    let assert = command.assert();

    let output = assert.get_output();

    let stdout = String::from_utf8(output.stdout.clone()).unwrap();
    let stderr = String::from_utf8(output.stderr.clone()).unwrap();

    assert.failure();

    insta::with_settings!({filters => vec![
        (r"tmp\/\.tmp.*\/", "[TEMPDIR]"),
        (r"private\/var\/.*\/\.tmp.*\/", "[TEMPDIR]"),
        (r"var\/.*\/\.tmp.*\/", "[TEMPDIR]"),
        (&patch_folder, "[PATCH]"),
    ]}, {
        insta::assert_snapshot!(stdout, @"");
        insta::assert_snapshot!(stderr, @r###"
        error: Unable to run `cargo metadata`

        Caused by:
            `cargo metadata` exited with an error: error: could not find `Cargo.toml` in `/[TEMPDIR][PATCH]` or any parent directory
            
        "###);
    });
}

fn basic_cargo_config(path: &Path) {
    write_cargo_config(
        path,
        r#"
        [registries]
        private-registry = { index = "https://dl.cloudsmith.io/basic/private/registry/cargo/index.git" }
        "#,
    )
}

#[cfg(feature = "failing_tests")]
fn basic_cargo_env_config(path: &Path) {
    write_cargo_config(
        path,
        r#"
        [env]
        CARGO_REGISTRIES_PRIVATE_REGISTRY_INDEX = "https://dl.cloudsmith.io/basic/private/registry/cargo/index.git"
        "#,
    )
}

#[cfg_attr(feature = "failing_tests", test_case(basic_cargo_env_config))]
#[test_case(basic_cargo_config)]
#[googletest::test]
fn patch_exists_alt_registry(setup: impl Fn(&Path)) {
    // let working_dir = TempDir::new().unwrap();
    let mut working_dir = tempfile::Builder::new();
    let working_dir = working_dir.keep(true).tempdir().unwrap();
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
        .add_dependency(Dependency::new(patch_crate_name, "1.0.86").registry("private-registry"))
        .render();

    let working_dir_manifest_path = create_cargo_manifest(working_dir, &manifest);
    let _patch_manifest_path = create_cargo_manifest(
        &patch_folder_path,
        &Manifest::new(Header::basic(patch_crate_name).version("1.1.5".to_owned()))
            .add_target(Target::lib(patch_crate_name, "src/lib.rs"))
            .render(),
    );

    let mut command = override_path(&patch_folder, working_dir, |command| command);

    let assert = command.assert();

    let output = assert.get_output();

    let stdout = String::from_utf8(output.stdout.clone()).unwrap();
    let stderr = String::from_utf8(output.stderr.clone()).unwrap();

    assert.success();

    insta::allow_duplicates! {
        insta::assert_snapshot!(stdout, @"");
        insta::assert_snapshot!(stderr, @r###"
        Patched dependency "anyhow" on registry "private-registry"
        "###);
    }

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
        anyhow = { version = "1.0.86", registry = "private-registry" }

        [[bin]]
        name = "package-name"
        path = "src/main.rs"

        [patch.private-registry]
        anyhow = { path = "anyhow" }
        '''
        "###);
    }
}

#[cfg_attr(feature = "failing_tests", test_case(basic_cargo_env_config))]
#[test_case(basic_cargo_config)]
#[googletest::test]
fn patch_registry_mismatch_fails(setup: impl Fn(&Path)) {
    let mut working_dir = tempfile::Builder::new();
    let working_dir = working_dir.keep(true).tempdir().unwrap();
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
        .add_dependency(Dependency::new(patch_crate_name, "1.0.86").registry("private-registry"))
        .render();

    let working_dir_manifest_path = create_cargo_manifest(working_dir, &manifest);
    let _patch_manifest_path = create_cargo_manifest(
        &patch_folder_path,
        &Manifest::new(Header::basic(patch_crate_name).version("1.1.5".to_owned()))
            .add_target(Target::lib(patch_crate_name, "src/lib.rs"))
            .render(),
    );

    let manifest_before = fs::read_to_string(&working_dir_manifest_path).unwrap();

    let mut command = override_path(&patch_folder, working_dir, |command| {
        command.arg("--registry").arg("another-registry")
    });

    let assert = command.assert();

    let output = assert.get_output();

    let stdout = String::from_utf8(output.stdout.clone()).unwrap();
    let stderr = String::from_utf8(output.stderr.clone()).unwrap();

    assert.failure();

    insta::allow_duplicates! {
        insta::with_settings!({filters => vec![
            (patch_folder.as_str(), "[PATCH]"),
        ]}, {
            insta::assert_snapshot!(stdout, @"");
            insta::assert_snapshot!(stderr, @r###"
            error: user provided registry `another-registry` with the `--registry` flag but dependency `[PATCH]` uses registry `private-registry`. 
                                 To use the registry, you passed, use `--force`
            "###);
        })
    };

    let manifest_after = fs::read_to_string(working_dir_manifest_path).unwrap();

    expect_eq!(manifest_before, manifest_after);
}

#[cfg_attr(feature = "failing_tests", test_case(basic_cargo_env_config))]
#[test_case(basic_cargo_config)]
#[googletest::test]
fn patch_registry_mismatch_force_succeeds(setup: impl Fn(&Path)) {
    let mut working_dir = tempfile::Builder::new();
    let working_dir = working_dir.keep(true).tempdir().unwrap();
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
        .add_dependency(Dependency::new(patch_crate_name, "1.0.86").registry("private-registry"))
        .render();

    let working_dir_manifest_path = create_cargo_manifest(working_dir, &manifest);
    let _patch_manifest_path = create_cargo_manifest(
        &patch_folder_path,
        &Manifest::new(Header::basic(patch_crate_name).version("1.1.5".to_owned()))
            .add_target(Target::lib(patch_crate_name, "src/lib.rs"))
            .render(),
    );

    let mut command = override_path(&patch_folder, working_dir, |command| {
        command
            .arg("--registry")
            .arg("another-registry")
            .arg("--force")
    });

    let assert = command.assert();

    let output = assert.get_output();

    let stdout = String::from_utf8(output.stdout.clone()).unwrap();
    let stderr = String::from_utf8(output.stderr.clone()).unwrap();

    assert.success();

    insta::allow_duplicates! {
        insta::assert_snapshot!(stdout, @"");
        insta::assert_snapshot!(stderr, @r###"
        Patched dependency "anyhow" on registry "another-registry"
        "###);
    }

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
        anyhow = { version = "1.0.86", registry = "private-registry" }

        [[bin]]
        name = "package-name"
        path = "src/main.rs"

        [patch.another-registry]
        anyhow = { path = "anyhow" }
        '''
        "###);
    }
}

#[googletest::test]
fn patch_exists_alt_registry_from_env() {
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
        .add_dependency(Dependency::new(patch_crate_name, "1.0.86").registry("private-registry"))
        .render();

    let working_dir_manifest_path = create_cargo_manifest(working_dir, &manifest);
    let _patch_manifest_path = create_cargo_manifest(
        &patch_folder_path,
        &Manifest::new(Header::basic(patch_crate_name).version("1.1.5".to_owned()))
            .add_target(Target::lib(patch_crate_name, "src/lib.rs"))
            .render(),
    );

    let mut command = override_path(&patch_folder, working_dir, |command| {
        command.env(
            "CARGO_REGISTRIES_PRIVATE_REGISTRY_INDEX",
            "https://dl.cloudsmith.io/basic/private/registry/cargo/index.git",
        )
    });

    let assert = command.assert();

    let output = assert.get_output();

    let stdout = String::from_utf8(output.stdout.clone()).unwrap();
    let stderr = String::from_utf8(output.stderr.clone()).unwrap();

    assert.success();

    insta::assert_snapshot!(stdout, @"");
    insta::assert_snapshot!(stderr, @r###"
    Patched dependency "anyhow" on registry "private-registry"
    "###);

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
        anyhow = { version = "1.0.86", registry = "private-registry" }

        [[bin]]
        name = "package-name"
        path = "src/main.rs"

        [patch.private-registry]
        anyhow = { path = "anyhow" }
        '''
        "###);
    }
}

fn write_cargo_config(path: &Path, toml: &str) {
    let cargo_config_dir = path.join(".cargo");

    fs::create_dir(&cargo_config_dir).expect("failed to create `.cargo` folder");

    let cargo_config = cargo_config_dir.join("config.toml");

    fs::write(&cargo_config, toml).expect("failed to write `.cargo/config.toml`");
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

fn override_path(
    path: &str,
    working_dir: &Path,
    args: impl Fn(&mut Command) -> &mut Command,
) -> Command {
    let mut cmd = Command::cargo_bin("cargo-override").unwrap();
    args(
        cmd.current_dir(working_dir)
            .arg("override")
            .arg("--path")
            .arg(path),
    )
    .env("CARGO_HOME", working_dir)
    .env_remove("RUST_BACKTRACE");

    cmd
}
