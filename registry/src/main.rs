use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::{bail, Context};
use axum::{
    extract::{DefaultBodyLimit, State},
    http::{HeaderMap, HeaderValue},
    response::{IntoResponse, Response},
    routing::post,
    Json, Router,
};
use base64::prelude::*;
use model::{
    ErrorInfo, GetError, GetInput, GetOutput, ListError, ListInput, ListOutput, PublishError,
    PublishInput, PublishOutput,
};
use serde::Serialize;
use tracing::{error, info};
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

    let state = AppState {
        armory_home: Arc::new(armory_home),
    };

    let app = Router::new()
        .route("/publish", post(publish))
        .route("/get", post(get))
        .route("/list", post(list))
        .with_state(state)
        .layer(DefaultBodyLimit::max(1024 * 1024 * 100));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;

    info!("listening on port 3000");

    axum::serve(listener, app).await?;

    Ok(())
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
}

pub mod header {
    /// Indicates the success or failure of an operation.
    ///
    /// Should be set to `true` or `false`.
    pub const OK: &'static str = "x-ok";
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

    let artifact_path = state
        .armory_home
        .join("registry")
        .join(format!("{}-{}", input.name, input.version));

    let Ok(bytes) = fs::read(&artifact_path) else {
        return Err(Error(GetError::PackageNotFound));
    };

    let content = BASE64_STANDARD.encode(bytes);

    Ok(Output(GetOutput { content }))
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
