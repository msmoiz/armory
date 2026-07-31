use std::{fs, io, path::PathBuf};

use anyhow::{anyhow, Context};

/// Stores a package in the cache.
///
/// Returns the path to the cached artifact.
pub fn put(name: &str, version: &str, content: &[u8]) -> anyhow::Result<PathBuf> {
    let filename = format!("{}-{}", name, version);
    let path = crate::dirs::armory_cache().join(filename);
    fs::create_dir_all(crate::dirs::armory_cache()).context("failed to create cache dir")?;
    fs::write(&path, content).with_context(|| format!("failed to cache content at {path:?}"))?;
    Ok(path)
}

/// Loads a package from the cache.
///
/// Returns the package content if it is cached.
pub fn get(name: &str, version: &str) -> anyhow::Result<Option<Vec<u8>>> {
    let filename = format!("{}-{}", name, version);
    let path = crate::dirs::armory_cache().join(filename);
    match fs::read(path) {
        Ok(content) => Ok(Some(content)),
        Err(e) if matches!(e.kind(), io::ErrorKind::NotFound) => Ok(None),
        Err(e) => Err(anyhow!(e).context("failed to read cached content")),
    }
}
