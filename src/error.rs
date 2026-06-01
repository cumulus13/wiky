//! Error types for wiky.

use thiserror::Error;

/// All errors that can occur in wiky.
#[derive(Debug, Error)]
pub enum WikiError {
    /// Network or HTTP error when calling the Wikipedia API.
    #[error("network error: {0}")]
    Network(#[from] reqwest::Error),

    /// JSON parse error.
    #[error("parse error: {0}")]
    Parse(#[from] serde_json::Error),

    /// No article found for the given query.
    #[error("article not found: '{0}'")]
    NotFound(String),

    /// Wikipedia API returned an unexpected response.
    #[error("API error: {0}")]
    Api(String),

    /// Config file error.
    #[error("config error: {0}")]
    Config(String),

    /// Invalid hex color string.
    #[error("invalid color '{0}': expected #RRGGBB or #RGB")]
    InvalidColor(String),

    /// IO error (reading/writing files).
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    /// Generic error wrapper.
    #[error("{0}")]
    Other(String),
}

impl From<confy::ConfyError> for WikiError {
    fn from(e: confy::ConfyError) -> Self {
        WikiError::Config(e.to_string())
    }
}
