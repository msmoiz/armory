mod client;

use std::{
    fs::{self, Permissions},
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    str::FromStr,
};

use anyhow::{bail, Context};
use base64::{prelude::BASE64_STANDARD, Engine};
use clap::{command, CommandFactory, Parser, Subcommand};
use client::Client;
use colored::{Color, Colorize};
use env_logger::fmt::Formatter;
use log::{error, info};
use model::{GetInput, ListInput, PublishInput};
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
    List,
    /// Uninstall a package.
    Uninstall {
        /// The name of the package.
        name: String,
    },
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

    let config = Config::load();

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
        Command::List => list(config),
        Command::Uninstall { name } => uninstall(name),
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

/// Application config.
struct Config {
    /// The URL of the registry.
    registry_url: String,
}

impl Config {
    /// Loads config from the environment.
    fn load() -> Self {
        Self {
            #[cfg(debug_assertions)]
            registry_url: String::from("http://localhost:3000"),
            #[cfg(not(debug_assertions))]
            registry_url: String::from("https://armory.msmoiz.com"),
        }
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

    let client = Client::new(config.registry_url);
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

    let client = Client::new(config.registry_url);
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

    Ok(())
}

/// List available packages.
fn list(config: Config) -> anyhow::Result<()> {
    let input = ListInput {};
    let client = Client::new(config.registry_url);
    let output = client.list(input).context("'list' request failed")?;
    for package in output.packages {
        println!("    {package}")
    }
    Ok(())
}

/// Uninstall a package.
fn uninstall(name: String) -> anyhow::Result<()> {
    let armory_home = dirs::home_dir()
        .expect("home directory should exist")
        .join(".armory");

    let bin = armory_home.join("bin");

    let artifact_path = bin.join(&name);

    if !artifact_path.is_file() {
        error!("package '{name}' does not exist");
        return Ok(());
    }

    fs::remove_file(&artifact_path)
        .with_context(|| format!("failed to delete {}", artifact_path.display()))?;

    info!("deleted binary at {}", artifact_path.display());

    Ok(())
}
