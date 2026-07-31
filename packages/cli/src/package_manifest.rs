use std::{fs, path::PathBuf};

use anyhow::{bail, Context};
use model::Triple;
use serde::Deserialize;

/// Contains information needed to publish a package.
#[derive(Deserialize)]
pub struct PackageManifest {
    pub package: Package,
    pub targets: Vec<Target>,
}

/// A package description.
#[derive(Deserialize)]
pub struct Package {
    /// The name of the package.
    pub name: String,
    /// The version of the package.
    pub version: String,
}

/// A package target.
#[derive(Deserialize)]
pub struct Target {
    /// The platform that the package binary corresponds to.
    pub triple: Triple,
    /// The path to the package binary.
    pub path: PathBuf,
}

impl PackageManifest {
    /// Loads the package manifest from the current working directory.
    ///
    /// Returns an erorr if a manifest does not exist or cannot be loaded.
    pub fn load() -> anyhow::Result<Self> {
        let path = std::env::current_dir()?.join("armory.toml");
        if !path.exists() {
            bail!("no package manifest found in current directory");
        }
        let content = fs::read_to_string(path).context("failed to read manifest")?;
        let manifest = toml::from_str(&content).context("failed to parse manifest")?;
        Ok(manifest)
    }
}
