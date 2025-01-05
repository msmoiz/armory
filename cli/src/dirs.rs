use std::path::PathBuf;

/// Returns the Armory home directory.
///
/// It is located at ~/.armory.
pub fn armory_home() -> PathBuf {
    dirs::home_dir()
        .expect("home directory should exist")
        .join(".armory")
}

/// Returns the Armory cache directory.
///
/// It is located at ~/.armory/cache
pub fn armory_cache() -> PathBuf {
    armory_home().join("cache")
}
