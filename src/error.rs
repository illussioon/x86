use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum X86Error {
    #[error("I/O error while accessing {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("invalid image: {0}")]
    InvalidImage(String),
    #[error("invalid saved state: {0}")]
    InvalidState(String),
    #[error("unsupported format: {0}")]
    UnsupportedFormat(String),
    #[error("remote loading is disabled; enable the `remote` feature")]
    RemoteDisabled,
    #[error("remote request failed for {url}: {message}")]
    Remote { url: String, message: String },
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("operation is not available in the current backend: {0}")]
    BackendUnavailable(String),
}

pub type Result<T> = std::result::Result<T, X86Error>;

pub(crate) fn io_error(path: impl Into<PathBuf>, source: std::io::Error) -> X86Error {
    X86Error::Io {
        path: path.into(),
        source,
    }
}
