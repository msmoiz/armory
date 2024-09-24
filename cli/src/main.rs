mod client;
mod install_manifest;
mod package_manifest;
mod target;

use std::{
    collections::HashMap,
    fs::{self},
    str::FromStr,
};

use anyhow::{bail, Context};
use base64::{prelude::BASE64_STANDARD, Engine};
use clap::{command, ArgAction, CommandFactory, Parser, Subcommand};
use client::Client;
use colored::{Color, Colorize};
use dialoguer::{Confirm, Password};
use env_logger::fmt::Formatter;
use install_manifest::InstallManifest;
use log::{error, info};
use model::{GetInfoInput, GetInput, ListInput, PublishInput, Triple};
use package_manifest::PackageManifest;
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
    ///
    /// Package information is sourced from _armory.toml_, a package manifest
    /// file, located in the current directory. The file should contain a
    /// `package` section with `name` and `version`. It should also contain one
    /// or more [[target]] sections with a `triple` and the `path` to the binary for
    /// that triple.
    ///
    /// The following triples are supported:
    ///
    /// - x86_64_linux
    /// - aarch64_linux
    /// - x86_64_darwin
    /// - aarch64_darwin
    /// - x86_64_windows
    /// - aarch64_windows
    Publish {
        /// The target triple to publish.
        ///
        /// If there is only one target defined in the package manifest, this
        /// flag does not need to be specified. If there is more than target
        /// defined, this flag must be specified to select one.
        #[arg(long, value_name = "TARGET")]
        triple: Option<Triple>,
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
    ///
    /// This only shows packages that are available for the current platform.
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
        Command::Publish { triple } => publish(config, triple),
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
fn publish(config: Config, triple: Option<Triple>) -> anyhow::Result<()> {
    let PackageManifest { package, targets } =
        PackageManifest::load().context("failed to load package manifest")?;

    let target = match (targets.len(), triple) {
        (0, _) => bail!("there are not targets defined"),
        (1, _) => &targets[0],
        (_, None) => bail!(
            "no target selected; options: {:?}",
            targets
                .iter()
                .map(|t| format!("{}", t.triple))
                .collect::<Vec<_>>()
        ),
        (_, Some(triple)) => match targets.iter().find(|t| t.triple == triple) {
            Some(target) => target,
            None => bail!(
                "target not defined; options: {:?}",
                targets
                    .iter()
                    .map(|t| format!("{}", t.triple))
                    .collect::<Vec<_>>()
            ),
        },
    };

    if !target.path.is_file() {
        bail!("binary does not exist at {}", target.path.display());
    }

    info!(
        "publishing {}-{}-{} | binary: {}",
        package.name,
        package.version,
        target.triple,
        target.path.display()
    );

    let content = {
        let bytes = fs::read(&target.path).context("failed to load binary")?;
        let encoded = BASE64_STANDARD.encode(bytes);
        encoded
    };

    let input = PublishInput {
        name: package.name.clone(),
        version: package.version.clone(),
        triple: target.triple.clone(),
        content,
    };

    let client = Client::new(config.registry_url, config.password);
    client.publish(input).context("'publish' request failed")?;
    info!(
        "published {}-{}-{}",
        package.name, package.version, target.triple
    );

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

    let triple = target::triple()?;

    let input = GetInput {
        name,
        version,
        triple,
    };

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

    #[cfg(unix)]
    let artifact_path = bin.join(&format!("{}", output.name));

    #[cfg(windows)]
    let artifact_path = bin.join(&format!("{}.exe", output.name));

    if artifact_path.exists() {
        #[cfg(unix)]
        fs::remove_file(&artifact_path).context("failed to remove existing package")?;

        #[cfg(windows)]
        fs::rename(
            &artifact_path,
            artifact_path.with_file_name(&format!("old_{}", output.name)),
        )
        .context("failed to remove existing package")?;

        info!("deleted existing binary at {}", artifact_path.display());
    }

    fs::write(&artifact_path, &content).context("failed to store package in bin")?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&artifact_path, fs::Permissions::from_mode(0o700))
            .context("failed to set binary permissions")?;
    }

    info!("installed binary to {}", artifact_path.display());

    InstallManifest::load_or_create()
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
        let manifest = InstallManifest::load_or_create().context("failed to load manifest")?;
        println!("installed packages:");
        for package in manifest.packages() {
            println!("    {0: <20} {1: <10}", package.name, package.version)
        }
    } else {
        let triple = target::triple()?;
        let input = ListInput {
            triple: triple.clone(),
        };
        let client = Client::new(config.registry_url, config.password);
        let output = client.list(input).context("'list' request failed")?;
        println!("available packages:");
        for package in output.packages {
            let get_info_input = GetInfoInput {
                triple: triple.clone(),
                name: package.clone(),
            };

            let package = client
                .get_info(get_info_input)
                .with_context(|| format!("failed to fetch info for package {package}"))?;

            let latest_version = package
                .versions
                .iter()
                .max()
                .expect("should be at least one version");

            println!("    {0: <20} {1: <10}", package.name, latest_version)
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

    #[cfg(unix)]
    let artifact_path = bin.join(&name);

    #[cfg(windows)]
    let artifact_path = bin.join(&format!("{}.exe", name));

    if !artifact_path.is_file() {
        error!("package '{name}' does not exist");
        return Ok(());
    }

    fs::remove_file(&artifact_path)
        .with_context(|| format!("failed to delete {}", artifact_path.display()))?;

    info!("deleted binary at {}", artifact_path.display());

    InstallManifest::load_or_create()
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
