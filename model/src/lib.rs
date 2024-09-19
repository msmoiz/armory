use anyhow::bail;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Error information.
#[derive(Serialize, Deserialize, Debug)]
pub struct ErrorInfo {
    pub code: String,
}

/// Input for the publish operation.
#[derive(Serialize, Deserialize, Debug)]
pub struct PublishInput {
    pub name: String,
    pub version: String,
    pub content: String,
}

/// Output for the publish operation.
#[derive(Serialize, Deserialize, Debug)]
pub struct PublishOutput {}

/// Errors for the publish operation.
#[derive(Error, Debug)]
pub enum PublishError {
    #[error("content is not encoded correctly")]
    InvalidEncoding,
    #[error("internal error")]
    InternalError,
}

impl From<PublishError> for ErrorInfo {
    fn from(value: PublishError) -> Self {
        let code = match value {
            PublishError::InvalidEncoding => "invalid_encoding",
            PublishError::InternalError => "internal_error",
        };

        ErrorInfo {
            code: code.to_owned(),
        }
    }
}

impl TryFrom<ErrorInfo> for PublishError {
    type Error = anyhow::Error;

    fn try_from(value: ErrorInfo) -> Result<Self, Self::Error> {
        let code = value.code.as_ref();
        match code {
            "invalid_encoding" => Ok(Self::InvalidEncoding),
            "internal_error" => Ok(Self::InternalError),
            _ => bail!("unrecognized error code: {code}"),
        }
    }
}

/// Input for the get operation.
#[derive(Serialize, Deserialize, Debug)]
pub struct GetInput {
    pub name: String,
    pub version: String,
}

/// Output for the get operation.
#[derive(Serialize, Deserialize, Debug)]
pub struct GetOutput {
    pub content: String,
}

/// Errors for the get operation.
#[derive(Error, Debug)]
pub enum GetError {
    #[error("package does not exist")]
    PackageNotFound,
}

impl From<GetError> for ErrorInfo {
    fn from(value: GetError) -> Self {
        let code = match value {
            GetError::PackageNotFound => "package_not_found",
        };

        ErrorInfo {
            code: code.to_owned(),
        }
    }
}

impl TryFrom<ErrorInfo> for GetError {
    type Error = anyhow::Error;

    fn try_from(value: ErrorInfo) -> Result<Self, Self::Error> {
        let code = value.code.as_ref();
        match code {
            "package_not_found" => Ok(Self::PackageNotFound),
            _ => bail!("unrecognized error code: {code}"),
        }
    }
}

/// Input for the list operation.
#[derive(Serialize, Deserialize, Debug)]
pub struct ListInput {}

/// Output for the list operation.
#[derive(Serialize, Deserialize, Debug)]
pub struct ListOutput {
    pub packages: Vec<String>,
}

/// Errors for the list operation.
#[derive(Error, Debug)]
pub enum ListError {
    #[error("internal error")]
    InternalError,
}

impl TryFrom<ErrorInfo> for ListError {
    type Error = anyhow::Error;

    fn try_from(value: ErrorInfo) -> Result<Self, Self::Error> {
        let code = value.code.as_ref();
        match code {
            "internal_error" => Ok(Self::InternalError),
            _ => bail!("unrecognized error code: {code}"),
        }
    }
}

impl From<ListError> for ErrorInfo {
    fn from(value: ListError) -> Self {
        let code = match value {
            ListError::InternalError => "internal_error",
        };

        ErrorInfo {
            code: code.to_owned(),
        }
    }
}
