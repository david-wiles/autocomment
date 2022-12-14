use std::fmt::Debug;
use thiserror::Error;
use crate::Error::AutocommentError;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Error occurred: {0}")]
    AutocommentError(String),

    #[error(transparent)]
    SerdeJsonError(#[from] serde_json::Error),

    #[error(transparent)]
    SerdeYamlError(#[from] serde_yaml::Error),

    #[error(transparent)]
    ReqwestError(#[from] reqwest::Error),

    #[error(transparent)]
    FsError(#[from] std::io::Error),
}

impl From<String> for Error {
    fn from(cause: String) -> Self {
        AutocommentError(cause)
    }
}
