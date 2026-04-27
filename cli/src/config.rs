use confique::Config as Confique;
use confique::Layer;

use crate::dirs;

/// Application config.
#[derive(Confique)]
pub struct Config {
    /// The URL of the registry.
    #[config(env = "ARMORY_URL")]
    pub registry_url: String,
    /// The password to use for authentication.
    #[config(env = "ARMORY_PASSWORD")]
    pub password: Option<String>,
}

impl Config {
    /// Loads config from various sources.
    ///
    /// Values are sourced in order of priority from: environment variables, a
    /// config file, and default values. Values found in a higher priority
    /// source override those found in a lower priority source. Returns an error
    /// if loading fails.
    pub fn load() -> anyhow::Result<Self> {
        Config::builder()
            .env()
            .file(dirs::armory_home().join("config.toml"))
            .preloaded(fallback())
            .load()
            .map_err(|e| e.into())
    }
}

type ConfigLayer = <Config as Confique>::Layer;

/// Creates a fallback layer.
///
/// This is used to set fallback values for required fields in situations where
/// a default expression will not suffice. confique does not support passing
/// functions to the `default` attribute for individual config fields so this is
/// a workaround. See: https://github.com/LukasKalbertodt/confique/issues/15.
fn fallback() -> ConfigLayer {
    ConfigLayer {
        #[cfg(debug_assertions)]
        registry_url: Some(String::from("http://localhost:3000")),
        #[cfg(not(debug_assertions))]
        registry_url: Some(String::from("https://armory.msmoiz.com")),

        ..ConfigLayer::default_values()
    }
}
