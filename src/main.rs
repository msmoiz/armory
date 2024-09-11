use std::{
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::{bail, Context};
use axum::{extract::State, http::StatusCode, routing::post, Json, Router};
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

/// Publishes a package to the registry.
async fn publish(
    State(state): State<AppState>,
    Json(input): Json<PublishInput>,
) -> Result<Json<PublishOutput>, StatusCode> {
    info!("handling publish request");

    let Ok(content) = BASE64_STANDARD.decode(input.content.as_bytes()) else {
        return Err(StatusCode::BAD_REQUEST);
    };

    let artifact_path = state
        .armory_home
        .join("registry")
        .join(format!("{}-{}", input.name, input.version));

    if let Err(e) = fs::write(&artifact_path, content)
        .with_context(|| format!("failed to write artifact to {}", artifact_path.display()))
    {
        error!("internal failure: {e}");
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    };

    info!("published artifact to {}", artifact_path.display());

    Ok(Json(PublishOutput::default()))
}
