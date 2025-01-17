use std::{fmt::Display, str::FromStr};

use anyhow::bail;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Error information.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ErrorInfo {
    pub code: String,
}

/// General errors that are shared across operations.
#[derive(Error, Debug)]
pub enum GeneralError {
    #[error("no password was provided")]
    PasswordMissing,
    #[error("password is invalid")]
    PasswordInvalid,
}

impl From<GeneralError> for ErrorInfo {
    fn from(value: GeneralError) -> Self {
        let code = match value {
            GeneralError::PasswordMissing => "password_missing",
            GeneralError::PasswordInvalid => "password_invalid",
        };

        ErrorInfo {
            code: code.to_owned(),
        }
    }
}

impl TryFrom<ErrorInfo> for GeneralError {
    type Error = anyhow::Error;

    fn try_from(value: ErrorInfo) -> Result<Self, Self::Error> {
        let code = value.code.as_ref();
        match code {
            "password_missing" => Ok(Self::PasswordMissing),
            "password_invalid" => Ok(Self::PasswordInvalid),
            _ => bail!("unrecognized error code: {code}"),
        }
    }
}

/// Target triple.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Triple {
    X86_64Linux,
    Aarch64Linux,
    X86_64Darwin,
    Aarch64Darwin,
    X86_64Windows,
    Aarch64Windows,
}

impl FromStr for Triple {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let triple = match s {
            "x86_64_linux" => Triple::X86_64Linux,
            "aarch64_linux" => Triple::Aarch64Linux,
            "x86_64_darwin" => Triple::X86_64Darwin,
            "aarch64_darwin" => Triple::Aarch64Darwin,
            "x86_64_windows" => Triple::X86_64Windows,
            "aarch64_windows" => Triple::Aarch64Windows,
            _ => bail!("unrecognized triple {s}"),
        };
        Ok(triple)
    }
}

impl Display for Triple {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let text = match self {
            Triple::X86_64Linux => "x86_64_linux",
            Triple::Aarch64Linux => "aarch64_linux",
            Triple::X86_64Darwin => "x86_64_darwin",
            Triple::Aarch64Darwin => "aarch64_darwin",
            Triple::X86_64Windows => "x86_64_windows",
            Triple::Aarch64Windows => "aarch64_windows",
        };

        write!(f, "{text}")
    }
}

/// Input for the publish operation.
#[derive(Serialize, Deserialize, Debug)]
pub struct PublishInput {
    pub name: String,
    pub version: String,
    pub triple: Triple,
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
    #[error("version already exists")]
    VersionExists,
    #[error("internal error")]
    InternalError,
}

impl From<PublishError> for ErrorInfo {
    fn from(value: PublishError) -> Self {
        let code = match value {
            PublishError::InvalidEncoding => "invalid_encoding",
            PublishError::VersionExists => "version_exists",
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
            "version_exists" => Ok(Self::VersionExists),
            "internal_error" => Ok(Self::InternalError),
            _ => bail!("unrecognized error code: {code}"),
        }
    }
}

/// Input for the get operation.
#[derive(Serialize, Deserialize, Debug)]
pub struct GetInput {
    pub name: String,
    pub version: Option<String>,
    pub triple: Triple,
}

/// Output for the get operation.
#[derive(Serialize, Deserialize, Debug)]
pub struct GetOutput {
    pub name: String,
    pub version: String,
    pub content: String,
}

/// Errors for the get operation.
#[derive(Error, Debug)]
pub enum GetError {
    #[error("package does not exist")]
    PackageNotFound,
    #[error("internal error")]
    InternalError,
}

impl From<GetError> for ErrorInfo {
    fn from(value: GetError) -> Self {
        let code = match value {
            GetError::PackageNotFound => "package_not_found",
            GetError::InternalError => "internal_error",
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
            "internal_error" => Ok(Self::InternalError),
            _ => bail!("unrecognized error code: {code}"),
        }
    }
}

/// Input for the get_info operation.
#[derive(Serialize, Deserialize, Debug)]
pub struct GetInfoInput {
    pub name: String,
    pub triple: Triple,
}

/// Output for the get_info operation.
#[derive(Serialize, Deserialize, Debug)]
pub struct GetInfoOutput {
    pub name: String,
    pub versions: Vec<String>,
}

/// Errors for the get_info operation.
#[derive(Error, Debug)]
pub enum GetInfoError {
    #[error("package does not exist")]
    PackageNotFound,
    #[error("internal error")]
    InternalError,
}

impl From<GetInfoError> for ErrorInfo {
    fn from(value: GetInfoError) -> Self {
        let code = match value {
            GetInfoError::PackageNotFound => "package_not_found",
            GetInfoError::InternalError => "internal_error",
        };

        ErrorInfo {
            code: code.to_owned(),
        }
    }
}

impl TryFrom<ErrorInfo> for GetInfoError {
    type Error = anyhow::Error;

    fn try_from(value: ErrorInfo) -> Result<Self, Self::Error> {
        let code = value.code.as_ref();
        match code {
            "package_not_found" => Ok(Self::PackageNotFound),
            "internal_error" => Ok(Self::InternalError),
            _ => bail!("unrecognized error code: {code}"),
        }
    }
}

/// Input for the list operation.
#[derive(Serialize, Deserialize, Debug)]
pub struct ListInput {
    pub triple: Triple,
}

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
