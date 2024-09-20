use anyhow::{anyhow, Context};
use model::{
    ErrorInfo, GeneralError, GetError, GetInput, GetOutput, ListError, ListInput, ListOutput,
    PublishError, PublishInput, PublishOutput,
};
use reqwest::blocking::Client as HttpClient;
use serde::{de::DeserializeOwned, Serialize};
use thiserror::Error;

mod header {
    /// Indicates the success or failure of an operation.
    ///
    /// Should be set to `true` or `false`.
    pub const OK: &'static str = "x-ok";
    /// The password to use for authentication.
    pub const PASSWORD: &'static str = "x-password";
}

/// Client errors.
#[derive(Error, Debug)]
pub enum Error<T> {
    /// A transport error.
    ///
    /// This covers all errors that might arise during transmission of the
    /// request such as issues with the input/output format, inability to reach
    /// the server, missing headers, and so forth.
    #[error("transport error: {0:?}")]
    Transport(anyhow::Error),
    /// A general error that can occur for any operation.
    ///
    /// This includes authentication and authorization errors and other errors
    /// that are not tied to a specific operation.
    #[error("{0}")]
    General(GeneralError),
    /// An error specific to the requested operation.
    ///
    /// This covers substantive errors returned by the server and is only
    /// returned when the request successfully reaches the server and a response
    /// is returned.
    #[error("{0}")]
    Specific(T),
}

/// A client for the armory registry.
pub struct Client {
    registry_url: String,
    password: Option<String>,
    client: HttpClient,
}

impl Client {
    /// Creates a new client.
    pub fn new(registry_url: String, password: Option<String>) -> Self {
        Self {
            registry_url,
            password,
            client: HttpClient::new(),
        }
    }

    /// Sends a request.
    fn send<Input, Output, Err>(&self, path: &str, input: Input) -> Result<Output, Error<Err>>
    where
        Input: Serialize,
        Output: DeserializeOwned,
        Err: TryFrom<ErrorInfo, Error = anyhow::Error>,
    {
        let url = format!("{}{path}", self.registry_url);

        let mut request = self.client.post(url).json(&input);

        if let Some(password) = self.password.as_ref() {
            request = request.header(header::PASSWORD, password);
        }

        let response = request
            .send()
            .context("failed to send request")
            .map_err(|e| Error::Transport(e))?;

        let ok = {
            let header = response.headers().get(header::OK).map(|v| v.to_str());
            match header {
                None => return Err(Error::Transport(anyhow!("'ok' response header is missing"))),
                Some(Err(_)) => {
                    return Err(Error::Transport(anyhow!(
                        "'ok' response header is malformed"
                    )))
                }
                Some(Ok(str)) => str == "true",
            }
        };

        if !ok {
            let error_info = response
                .json::<ErrorInfo>()
                .context("error message is malformed")
                .map_err(|e| Error::Transport(e))?;

            if let Ok(error) = TryInto::<GeneralError>::try_into(error_info.clone()) {
                return Err(Error::General(error));
            }

            let error: Err = error_info
                .try_into()
                .context("failed to parse error code")
                .map_err(|e| Error::Transport(e))?;

            return Err(Error::Specific(error));
        }

        let output = response
            .json::<Output>()
            .context("output is malformed")
            .map_err(|e| Error::Transport(e))?;

        Ok(output)
    }

    /// Publishes a package to the registry.
    pub fn publish(&self, input: PublishInput) -> Result<PublishOutput, Error<PublishError>> {
        self.send("/publish", input)
    }

    /// Gets a package from the registry.
    pub fn get(&self, input: GetInput) -> Result<GetOutput, Error<GetError>> {
        self.send("/get", input)
    }

    /// Lists packages in the registry.
    pub fn list(&self, input: ListInput) -> Result<ListOutput, Error<ListError>> {
        self.send("/list", input)
    }
}
