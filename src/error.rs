use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Tracing error")]
    Logger(#[from] tracing::metadata::ParseLevelError),
    #[error("IO error")]
    IoError(#[from] std::io::Error),
    #[error("Serde error")]
    Serde(#[from] serde_json::Error),
    #[error("Config error")]
    Config(#[from] config::ConfigError),
    #[error("{0}")]
    Custom(String),
}

impl Error {
    pub fn custom<S: ToString>(msg: S) -> Self {
        Self::Custom(msg.to_string())
    }
}
