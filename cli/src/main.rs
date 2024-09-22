mod client;
mod manifest;

use std::{
    collections::HashMap,
    fs::{self, Permissions},
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    str::FromStr,
};

use anyhow::{bail, Context};
use base64::{prelude::BASE64_STANDARD, Engine};
use clap::{command, ArgAction, CommandFactory, Parser, Subcommand};
use client::Client;
use colored::{Color, Colorize};
use dialoguer::{Confirm, Password};
use env_logger::fmt::Formatter;
use log::{error, info};
use manifest::Manifest;
use model::{GetInput, ListInput, PublishInput};
use serde::Deserialize;
use std::io::Write;

/// A personal package manager.
#[derive(Parser, Debug)]
#[command(version, about, max_term_width = 80)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Upload a package.
    Publish {
        /// The name of the package.
        name: String,
        /// The version of the package.
        version: String,
        /// The path to the package binary to upload.
        binary: PathBuf,
    },
    /// Install a package.
    Install {
        /// The identifier of the package.
        #[arg(value_name = "PACKAGE[@VERSION]")]
        id: Identifier,
        /// The version of the package.
        ///
        /// If not specified, the latest version of the package is installed.
        /// You can use this flag or specify a version in the identifier, but
        /// you cannot use both methods at the same time.
        #[arg(long)]
        version: Option<String>,
    },
    /// List available packages.
    List {
        /// List installed packages instead. (default: false)
        #[arg(long, default_value_t = false)]
        installed: bool,
    },
    /// Uninstall a package.
    Uninstall {
        /// The name of the package.
        ///
        /// If the name is "self" or "armory", this command will uninstall
        /// armory itself along with its associated metadata.
        name: String,
        /// Do not prompt for input.
        #[arg(long = "non-interactive",  default_value_t = true, action = ArgAction::SetFalse)]
        interactive: bool,
    },
    /// Set up registry credentials.
    Login,
}

/// A package identifier.
#[derive(Debug, Clone)]
struct Identifier {
    name: String,
    version: Option<String>,
}

impl FromStr for Identifier {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut parts = s.split("@");
        let name = parts
            .next()
            .expect("should be at least one part")
            .to_owned();
        let version = parts.next().map(|s| s.to_owned());
        if parts.count() > 0 {
            bail!("too many components in package identifier");
        }
        let identifier = Identifier { name, version };
        Ok(identifier)
    }
}

fn main() {
    init_logger();

    let cli = Cli::parse();

    let config = match Config::load() {
        Ok(config) => config,
        Err(e) => {
            error!("{e:?}");
            std::process::exit(1);
        }
    };

    let Some(command) = cli.command else {
        Cli::command().print_help().unwrap();
        std::process::exit(1);
    };

    let result = match command {
        Command::Publish {
            name,
            version,
            binary,
        } => publish(name, version, binary, config),
        Command::Install { id, version } => install(id, version, config),
        Command::List { installed } => list(config, installed),
        Command::Uninstall { name, interactive } => uninstall(name, interactive),
        Command::Login => login(),
    };

    if let Err(e) = result {
        error!("{e:?}");
        std::process::exit(1);
    }
}

/// Initialize the logger.
fn init_logger() {
    let format = |buf: &mut Formatter, record: &log::Record| {
        use log::Level::*;
        let level = {
            let color = match record.level() {
                Error => Color::Red,
                Warn => Color::Yellow,
                Info => Color::Blue,
                Debug => Color::Green,
                Trace => Color::Magenta,
            };

            let text = match record.level() {
                Warn => String::from("warning"),
                _ => record.level().to_string(),
            };

            text.to_string().to_lowercase().color(color).bold()
        };

        writeln!(buf, "{level}{} {}", ":".bold(), record.args())
    };

    env_logger::builder()
        .format(format)
        .filter_level(log::LevelFilter::Info)
        .init();
}

/// Application config file.
#[derive(Deserialize)]
struct ConfigFile {
    /// The password to use for authentication.
    password: String,
}

/// Application config.
struct Config {
    /// The URL of the registry.
    registry_url: String,
    /// The password to use for authentication.
    password: Option<String>,
}

impl Config {
    /// Loads config from the environment.
    fn load() -> anyhow::Result<Self> {
        let armory_home = dirs::home_dir()
            .expect("home directory should exist")
            .join(".armory");

        let config_file = armory_home.join("config.toml");

        let password = if config_file.exists() {
            let content = fs::read_to_string(config_file).context("failed to read config file")?;
            let config: ConfigFile =
                toml::from_str(&content).context("failed to parse config file")?;
            Some(config.password)
        } else {
            None
        };

        Ok(Self {
            #[cfg(debug_assertions)]
            registry_url: String::from("http://localhost:3000"),
            #[cfg(not(debug_assertions))]
            registry_url: String::from("https://armory.msmoiz.com"),
            password,
        })
    }
}

pub mod header {
    /// Indicates the success or failure of an operation.
    ///
    /// Should be set to `true` or `false`.
    pub const OK: &'static str = "x-ok";
}

/// Publish a package.
fn publish(name: String, version: String, binary: PathBuf, config: Config) -> anyhow::Result<()> {
    if !binary.is_file() {
        bail!("{binary:?} is not a file");
    }

    let content = {
        let bytes = fs::read(binary).context("failed to read binary file")?;
        let encoded = BASE64_STANDARD.encode(bytes);
        encoded
    };

    let input = PublishInput {
        name: name.clone(),
        version: version.clone(),
        content,
    };

    let client = Client::new(config.registry_url, config.password);
    client.publish(input).context("'publish' request failed")?;
    info!("published {name}-{version}");

    Ok(())
}

/// Install a package.
fn install(id: Identifier, version: Option<String>, config: Config) -> anyhow::Result<()> {
    let name = id.name;

    if id.version.is_some() && version.is_some() {
        error!("version specified multiple times");
        return Ok(());
    }

    let version = id.version.or(version);

    let input = GetInput { name, version };

    let client = Client::new(config.registry_url, config.password);
    let output = client.get(input).context("'get' request failed")?;

    let content = BASE64_STANDARD
        .decode(output.content)
        .context("package content is malformed")?;

    let armory_home = dirs::home_dir()
        .expect("home directory should exist")
        .join(".armory");

    let registry = armory_home.join("registry");
    fs::create_dir_all(&registry).context("failed to create registry directory")?;
    let artifact_path = registry.join(&format!("{}-{}", output.name, output.version));
    fs::write(&artifact_path, &content).context("failed to store package in registry")?;

    info!("installed package to {}", artifact_path.display());

    let bin = armory_home.join("bin");
    fs::create_dir_all(&bin).context("failed to create bin directory")?;
    let artifact_path = bin.join(&format!("{}", output.name));
    if artifact_path.exists() {
        fs::remove_file(&artifact_path).context("failed to remove existing package")?;
        info!("deleted existing binary at {}", artifact_path.display());
    }
    fs::write(&artifact_path, &content).context("failed to store package in bin")?;
    fs::set_permissions(&artifact_path, Permissions::from_mode(0o700))
        .context("failed to set binary permissions")?;

    info!("installed binary to {}", artifact_path.display());

    Manifest::load_or_create()
        .and_then(|mut manifest| {
            manifest.add_package(output.name, output.version);
            manifest.save()
        })
        .context("failed to update manifest")?;

    Ok(())
}

/// List available packages.
fn list(config: Config, installed: bool) -> anyhow::Result<()> {
    if installed {
        let manifest = Manifest::load_or_create().context("failed to load manifest")?;
        println!("installed packages:");
        for package in manifest.packages() {
            println!("    {0: <20} {1: <10}", package.name, package.version)
        }
    } else {
        let input = ListInput {};
        let client = Client::new(config.registry_url, config.password);
        let output = client.list(input).context("'list' request failed")?;
        println!("available packages:");
        for package in output.packages {
            println!("    {package}")
        }
    }

    Ok(())
}

/// Uninstall a package.
fn uninstall(name: String, interactive: bool) -> anyhow::Result<()> {
    let armory_home = dirs::home_dir()
        .expect("home directory should exist")
        .join(".armory");

    if name == "self" || name == "armory" {
        let confirm = if interactive {
            Confirm::new().with_prompt("uninstall armory?").interact()?
        } else {
            true
        };

        if !confirm {
            info!("uninstall aborted");
            return Ok(());
        }

        fs::remove_dir_all(armory_home).context("failed to delete armory home")?;
        info!("uninstalled armory");
        return Ok(());
    }

    let bin = armory_home.join("bin");

    let artifact_path = bin.join(&name);

    if !artifact_path.is_file() {
        error!("package '{name}' does not exist");
        return Ok(());
    }

    fs::remove_file(&artifact_path)
        .with_context(|| format!("failed to delete {}", artifact_path.display()))?;

    info!("deleted binary at {}", artifact_path.display());

    Manifest::load_or_create()
        .and_then(|mut manifest| {
            manifest.remove_package(&name);
            manifest.save()
        })
        .context("failed to update manifest")?;

    Ok(())
}

/// Set up registry credentials.
fn login() -> anyhow::Result<()> {
    let armory_home = dirs::home_dir()
        .expect("home directory should exist")
        .join(".armory");

    let config_file = armory_home.join("config.toml");

    let password = Password::new()
        .with_prompt("enter your password")
        .interact()?;

    let config = HashMap::from([("password", password)]);

    fs::write(&config_file, toml::to_string_pretty(&config)?)
        .context("failed to save config file")?;

    info!("credentials saved at {}", config_file.display());

    Ok(())
}
