use std::fs;

use cargo_override::{run, Args};

use fake::{Fake, Faker};
use tempfile::TempDir;

#[test]
fn patch_path_exists() {
    let working_dir = TempDir::new().unwrap();
    let working_dir = working_dir.path();

    let patch_folder: String = Faker.fake();
    let patch_folder_path = working_dir.join(&patch_folder);

    fs::create_dir(patch_folder_path).expect("failed to create patch folder");

    run(working_dir, Args { path: patch_folder })
}

#[test]
#[should_panic]
fn patch_path_doesnt_exist() {
    let working_dir = TempDir::new().unwrap();
    let working_dir = working_dir.path();

    let patch_folder: String = Faker.fake();

    run(working_dir, Args { path: patch_folder })
}
