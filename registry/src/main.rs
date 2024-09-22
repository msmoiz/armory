use std::{
    collections::HashSet,
    env::VarError,
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::{bail, Context};
use axum::{
    extract::{DefaultBodyLimit, Request, State},
    http::{HeaderMap, HeaderValue},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::post,
    Json, Router,
};
use base64::prelude::*;
use model::{
    ErrorInfo, GeneralError, GetError, GetInfoError, GetInfoInput, GetInfoOutput, GetInput,
    GetOutput, ListError, ListInput, ListOutput, PublishError, PublishInput, PublishOutput,
};
use serde::Serialize;
use tracing::{error, info, warn};
use tracing_subscriber::fmt;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .event_format(fmt::format().compact())
        .init();

    info!("starting server");

    let Some(armory_home) = dirs::home_dir().map(|home| home.join("armory")) else {
        bail!("unable to determine armory home");
    };

    info!(r#"armory_home = "{}""#, armory_home.display());

    create_armory_dirs(&armory_home).context("failed to create armory directories")?;

    let password = load_password()?;
    let state = AppState {
        armory_home: Arc::new(armory_home),
        password,
    };

    let app = Router::new()
        .route("/publish", post(publish))
        .route("/get", post(get))
        .route("/get-info", post(get_info))
        .route("/list", post(list))
        .with_state(state.clone())
        .layer(middleware::from_fn_with_state(state, authentication))
        .layer(DefaultBodyLimit::max(1024 * 1024 * 100));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;

    info!("listening on port 3000");

    axum::serve(listener, app).await?;

    Ok(())
}

/// Loads the registry password from the environment.
fn load_password() -> anyhow::Result<Option<String>> {
    return match std::env::var("ARMORY_PASSWORD") {
        Ok(password) => Ok(Some(password)),
        Err(VarError::NotPresent) => {
            warn!("no password configured; set ARMORY_PASSWORD to set a password");
            Ok(None)
        }
        Err(VarError::NotUnicode(_)) => bail!("password is not valid unicode"),
    };
}

fn create_armory_dirs(armory_home: &Path) -> anyhow::Result<()> {
    fs::create_dir_all(armory_home).context("failed to create armory home directory")?;
    fs::create_dir_all(armory_home.join("registry"))
        .context("failed to create armory registry directory")?;
    Ok(())
}

/// Global application state.
#[derive(Clone, Debug)]
struct AppState {
    /// Home directory of the application.
    armory_home: Arc<PathBuf>,
    /// Registry password.
    password: Option<String>,
}

pub mod header {
    /// Indicates the success or failure of an operation.
    ///
    /// Should be set to `true` or `false`.
    pub const OK: &'static str = "x-ok";
    /// The password to use for authentication.
    pub const PASSWORD: &'static str = "x-password";
}

/// An authentication layer.
async fn authentication(state: State<AppState>, request: Request, next: Next) -> Response {
    if let Some(password) = state.password.as_ref() {
        let provided = request
            .headers()
            .get(header::PASSWORD)
            .and_then(|v| v.to_str().ok());

        if provided.is_none() {
            return Error(GeneralError::PasswordMissing).into_response();
        }

        if provided.unwrap() != password {
            return Error(GeneralError::PasswordInvalid).into_response();
        }
    }
    let response = next.run(request).await;
    response
}

/// Output response.
///
/// Sets the `x-ok` header to `true` and serializes the body to JSON. The output
/// value must implement `Serialize`.
struct Output<T>(T);

impl<T> IntoResponse for Output<T>
where
    T: Serialize,
{
    fn into_response(self) -> Response {
        let headers = {
            let mut map = HeaderMap::new();
            map.insert(header::OK, HeaderValue::from_static("true"));
            map
        };

        (headers, Json(self.0)).into_response()
    }
}

/// Error response.
///
/// Sets the `x-ok` header to `false` and serializes the body to JSON. The error
/// value must implement `Into<ErrorInfo>`.
struct Error<T>(T);

impl<T> IntoResponse for Error<T>
where
    T: Into<ErrorInfo>,
{
    fn into_response(self) -> Response {
        let headers = {
            let mut map = HeaderMap::new();
            map.insert(header::OK, HeaderValue::from_static("false"));
            map
        };

        let error_info: ErrorInfo = self.0.into();

        (headers, Json(error_info)).into_response()
    }
}

/// Publishes a package to the registry.
async fn publish(
    State(state): State<AppState>,
    Json(input): Json<PublishInput>,
) -> Result<Output<PublishOutput>, Error<PublishError>> {
    info!("handling publish request");

    let Ok(content) = BASE64_STANDARD.decode(input.content.as_bytes()) else {
        return Err(Error(PublishError::InvalidEncoding));
    };

    let artifact_path = state
        .armory_home
        .join("registry")
        .join(format!("{}-{}", input.name, input.version));

    if let Err(e) = fs::write(&artifact_path, content)
        .with_context(|| format!("failed to write artifact to {}", artifact_path.display()))
    {
        error!("internal failure: {e}");
        return Err(Error(PublishError::InternalError));
    };

    info!("published artifact to {}", artifact_path.display());

    Ok(Output(PublishOutput {}))
}

async fn get(
    State(state): State<AppState>,
    Json(input): Json<GetInput>,
) -> Result<Output<GetOutput>, Error<GetError>> {
    info!("handling get request");

    let registry = state.armory_home.join("registry");

    let version = match input.version {
        Some(version) => version,
        None => {
            let entries = match fs::read_dir(&registry).context("failed to read registry") {
                Ok(entries) => entries,
                Err(e) => {
                    error!("internal failure: {e}");
                    return Err(Error(GetError::InternalError));
                }
            };

            let version = entries
                .filter_map(Result::ok)
                .map(|e| e.file_name().to_string_lossy().to_string())
                .filter(|e| {
                    let mut parts = e.split('-').collect::<Vec<_>>();
                    parts.pop(); // discard version
                    parts.join("-") == input.name
                })
                .max_by(|x, y| x.split("-").last().cmp(&y.split("-").last()))
                .map(|e| e.split("-").last().unwrap().to_owned());

            match version {
                Some(version) => version,
                None => return Err(Error(GetError::PackageNotFound)),
            }
        }
    };

    let artifact_path = registry.join(format!("{}-{version}", input.name));

    let Ok(bytes) = fs::read(&artifact_path) else {
        return Err(Error(GetError::PackageNotFound));
    };

    let content = BASE64_STANDARD.encode(bytes);

    Ok(Output(GetOutput {
        name: input.name,
        version,
        content,
    }))
}

async fn get_info(
    State(state): State<AppState>,
    Json(input): Json<GetInfoInput>,
) -> Result<Output<GetInfoOutput>, Error<GetInfoError>> {
    info!("handling get info request");

    let registry = state.armory_home.join("registry");

    let entries = match fs::read_dir(&registry).context("failed to read registry") {
        Ok(entries) => entries,
        Err(e) => {
            error!("internal failure: {e}");
            return Err(Error(GetInfoError::InternalError));
        }
    };

    let versions = entries
        .filter_map(Result::ok)
        .map(|e| e.file_name().to_string_lossy().to_string())
        .filter(|e| {
            let mut parts = e.split('-').collect::<Vec<_>>();
            parts.pop(); // discard version
            parts.join("-") == input.name
        })
        .map(|e| e.split("-").last().unwrap().to_owned())
        .collect::<Vec<_>>();

    if versions.is_empty() {
        return Err(Error(GetInfoError::PackageNotFound));
    }

    Ok(Output(GetInfoOutput {
        name: input.name,
        versions,
    }))
}

async fn list(
    State(state): State<AppState>,
    Json(_): Json<ListInput>,
) -> Result<Output<ListOutput>, Error<ListError>> {
    info!("handling list request");

    let registry = state.armory_home.join("registry");

    let entries = match fs::read_dir(registry).context("failed to read registry") {
        Ok(entries) => entries,
        Err(e) => {
            error!("internal failure: {e}");
            return Err(Error(ListError::InternalError));
        }
    };

    let mut packages = HashSet::<String>::new();
    for entry in entries {
        let entry = match entry.context("failed to read registry entry") {
            Ok(entry) => entry,
            Err(e) => {
                error!("internal failure: {e}");
                return Err(Error(ListError::InternalError));
            }
        };

        let path = entry.path();
        if path.is_file() {
            let name = path.file_name().expect("path should point to a file");
            let name = name.to_str().expect("name should be valid unicode");
            let mut parts: Vec<_> = name.split('-').collect();
            parts.pop(); // discard the version
            packages.insert(parts.join("-"));
        }
    }

    Ok(Output(ListOutput {
        packages: packages.into_iter().collect(),
    }))
}
