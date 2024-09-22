use std::{fs, path::PathBuf};

use anyhow::Context;
use serde::{Deserialize, Serialize};

/// Record of installed packages.
///
/// The manifest is stored at ~/.armory/installed.toml.
#[derive(Default, Serialize, Deserialize, Clone)]
pub struct Manifest {
    packages: Vec<PackageRecord>,
}

/// Record of an installed package.
#[derive(Serialize, Deserialize, Clone)]
pub struct PackageRecord {
    /// The name of the package.
    pub name: String,
    /// The version of the package.
    pub version: String,
}

impl Manifest {
    /// Returns the path to the manifest file.
    fn path() -> PathBuf {
        dirs::home_dir()
            .expect("home dir should exist")
            .join(".armory")
            .join("installed.toml")
    }

    /// Loads the manifest from disk or creates one if it does not exist.
    pub fn load_or_create() -> anyhow::Result<Self> {
        if let Ok(content) = fs::read_to_string(Self::path()) {
            let manifest: Self =
                toml::from_str(&content).context("failed to parse manifest file")?;
            return Ok(manifest);
        }
        let manifest = Self::default();
        return Ok(manifest);
    }

    /// List installed packages.
    pub fn packages(&self) -> &[PackageRecord] {
        &self.packages
    }

    /// Adds a package to the manifest.
    pub fn add_package(&mut self, name: String, version: String) {
        self.packages.push(PackageRecord { name, version });
    }

    /// Removes a package from the manifest.
    ///
    /// This method is idempotent and will not fail if the package has already
    /// been removed or is not present in the manifest.
    pub fn remove_package(&mut self, name: String) {
        if let Some(pos) = self
            .packages
            .iter()
            .position(|package| package.name == name)
        {
            self.packages.remove(pos);
        }
    }

    /// Saves the manifest to disk.
    pub fn save(&self) -> anyhow::Result<()> {
        let mut manifest = self.clone();
        manifest.packages.sort_by(|a, b| a.name.cmp(&b.name));
        let content = toml::to_string(&manifest)?;
        fs::write(Self::path(), content).context("failed to write file")
    }
}
