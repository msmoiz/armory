use std::{
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::{bail, Context};
use axum::{
    extract::State,
    http::{HeaderMap, HeaderValue},
    response::{IntoResponse, Response},
    routing::post,
    Json, Router,
};
use base64::prelude::*;
use serde::{Deserialize, Serialize};
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
        .with_state(state);

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

/// Error body content.
#[derive(Serialize)]
struct Error {
    code: &'static str,
}

impl Error {
    /// Creates a new error from an error code.
    fn new(code: &'static str) -> Self {
        Self { code }
    }
}

/// Input for the publish operation.
#[derive(Deserialize, Debug)]
struct PublishInput {
    name: String,
    version: String,
    content: String,
}

/// Output for the publish operation.
#[derive(Serialize, Default)]
struct PublishOutput {}

/// Errors for the publish operation.
enum PublishError {
    InvalidEncoding,
    InternalError,
}

impl IntoResponse for PublishError {
    fn into_response(self) -> Response {
        let code = match self {
            PublishError::InvalidEncoding => "invalid_encoding",
            PublishError::InternalError => "internal_error",
        };

        let headers = {
            let mut map = HeaderMap::new();
            map.insert(header::OK, HeaderValue::from_static("false"));
            map
        };

        (headers, Json(Error::new(code))).into_response()
    }
}

/// Publishes a package to the registry.
async fn publish(
    State(state): State<AppState>,
    Json(input): Json<PublishInput>,
) -> Result<Output<PublishOutput>, PublishError> {
    info!("handling publish request");

    let Ok(content) = BASE64_STANDARD.decode(input.content.as_bytes()) else {
        return Err(PublishError::InvalidEncoding);
    };

    let artifact_path = state
        .armory_home
        .join("registry")
        .join(format!("{}-{}", input.name, input.version));

    if let Err(e) = fs::write(&artifact_path, content)
        .with_context(|| format!("failed to write artifact to {}", artifact_path.display()))
    {
        error!("internal failure: {e}");
        return Err(PublishError::InternalError);
    };

    info!("published artifact to {}", artifact_path.display());

    Ok(Output(PublishOutput::default()))
}
