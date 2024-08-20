//! For creating `.cargo-checksum.json` files, which are required by cargo before accepting vendored dependencies

use std::{collections::HashMap, path::Path};

use fs_err as fs;
use serde::Serialize;
use sha2::{Digest, Sha256};

#[derive(Serialize)]
pub struct Checksum {
    files: HashMap<String, String>,
    // For some reason we're able to get away not specifying a `package` checksum.
    // This is very kind of cargo, becuase I have no idea how to calculate it 🫠
}

impl Checksum {
    /// Calculate the checksum for a package that contains nothing other than a single `Cargo.toml` file
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
