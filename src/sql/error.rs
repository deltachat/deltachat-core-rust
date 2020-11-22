#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Sqlite Error: {0:?}")]
    Sql(#[from] rusqlite::Error),
    #[error("Sqlite Connection Pool Error: {0:?}")]
    ConnectionPool(#[from] r2d2::Error),
    #[error("Sqlite: Connection closed")]
    SqlNoConnection,
    #[error("Sqlite: Already open")]
    SqlAlreadyOpen,
    #[error("Sqlite: Failed to open")]
    SqlFailedToOpen,
    #[error("{0}")]
    Io(#[from] std::io::Error),
    #[error("{0:?}")]
    BlobError(#[from] crate::blob::BlobError),
    #[error("{0}")]
    Other(#[from] anyhow::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
