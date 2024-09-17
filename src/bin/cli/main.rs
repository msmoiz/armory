use std::{
    fs::{self, Permissions},
    os::unix::fs::PermissionsExt,
    str::FromStr,
};

use anyhow::{bail, Context};
use base64::{prelude::BASE64_STANDARD, Engine};
use clap::{command, CommandFactory, Parser, Subcommand};
use colored::{Color, Colorize};
use env_logger::fmt::Formatter;
use log::{error, info};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use std::io::Write;

#[derive(Parser, Debug)]
#[command(version, about, max_term_width = 80)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Install a package.
    Install {
        /// The name of the package.
        name: String,
        /// The version of the package.
        version: String,
    },
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
        Command::Install { name, version } => install(name, version, config),
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
            registry_url: String::from("https://msmoiz.armory.com"),
        }
    }
}

pub mod header {
    /// Indicates the success or failure of an operation.
    ///
    /// Should be set to `true` or `false`.
    pub const OK: &'static str = "x-ok";
}

/// Error information.
#[derive(Serialize, Deserialize, Debug)]
struct ErrorInfo {
    code: String,
}

/// Input for the get operation.
#[derive(Serialize, Deserialize, Debug)]
struct GetInput {
    name: String,
    version: String,
}

/// Output for the get operation.
#[derive(Serialize, Deserialize, Debug)]
struct GetOutput {
    content: String,
}

/// Errors for the get operation.
enum GetError {
    PackageNotFound,
}

impl FromStr for GetError {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "package_not_found" => Ok(Self::PackageNotFound),
            _ => bail!("unsupported value: {s}"),
        }
    }
}

fn install(name: String, version: String, config: Config) -> anyhow::Result<()> {
    let client = Client::new();

    let input = GetInput {
        name: name.clone(),
        version: version.clone(),
    };

    let base_url = config.registry_url;

    let response = client
        .post(format!("{base_url}/get"))
        .json(&input)
        .send()
        .context("failed to send 'get' request")?;

    let ok = {
        let header = response.headers().get(header::OK).map(|v| v.to_str());
        match header {
            None => bail!("'ok' header is missing"),
            Some(Err(_)) => bail!("'ok' header is malformed"),
            Some(Ok(str)) => str == "true",
        }
    };

    if !ok {
        let error_info = response
            .json::<ErrorInfo>()
            .context("error message is malformed")?;

        let error = error_info
            .code
            .parse::<GetError>()
            .context("failed to parse error code")?;

        match error {
            GetError::PackageNotFound => bail!("package does not exist"),
        }
    }

    let output = response
        .json::<GetOutput>()
        .context("get response is malformed")?;

    let content = BASE64_STANDARD
        .decode(output.content)
        .context("package content is malformed")?;

    let armory_home = dirs::home_dir()
        .expect("home directory should exist")
        .join(".armory");

    let registry = armory_home.join("registry");
    fs::create_dir_all(&registry).context("failed to create registry directory")?;
    let artifact_path = registry.join(&format!("{name}-{version}"));
    fs::write(&artifact_path, &content).context("failed to store package in registry")?;

    info!("installed package to {}", artifact_path.display());

    let bin = armory_home.join("bin");
    fs::create_dir_all(&bin).context("failed to create bin directory")?;
    let artifact_path = bin.join(&format!("{name}"));
    fs::write(&artifact_path, &content).context("failed to store package in bin")?;
    fs::set_permissions(&artifact_path, Permissions::from_mode(0o700))
        .context("failed to set binary permissions")?;

    info!("installed binary to {}", artifact_path.display());

    Ok(())
}
