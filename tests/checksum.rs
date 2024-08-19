use std::{collections::HashMap, path::Path};

use fs_err as fs;
use serde::Serialize;
use sha2::{Digest, Sha256};

#[derive(Serialize)]
pub struct Checksum {
    files: HashMap<String, String>,
}

impl Checksum {
    pub fn package_only_manifest(manifest: &str) -> Self {
        let hash = Sha256::digest(manifest.as_bytes());
        let hash = hex::encode(hash);

        Self {
            files: [("Cargo.toml".to_owned(), hash)].into_iter().collect(),
        }
    }
    pub fn write_to_dir(self, dir: &Path) {
        let file = dir.join(".cargo-checksum.json");

        let checksum = serde_json::to_string(&self).unwrap();

        fs::write(&file, checksum).expect("failed to write checksum");
    }
}
