use failure::Fail;

#[derive(Debug, Fail)]
pub enum Error {
    #[fail(display = "Sqlite Error: {:?}", _0)]
    Sql(rusqlite::Error),
    #[fail(display = "Sqlite Connection Pool Error: {:?}", _0)]
    ConnectionPool(r2d2::Error),
    #[fail(display = "{:?}", _0)]
    Failure(failure::Error),
    #[fail(display = "Sqlite: Connection closed")]
    SqlNoConnection,
    #[fail(display = "Sqlite: Already open")]
    SqlAlreadyOpen,
    #[fail(display = "Sqlite: Failed to open")]
    SqlFailedToOpen,
}

pub type Result<T> = std::result::Result<T, Error>;

impl From<rusqlite::Error> for Error {
    fn from(err: rusqlite::Error) -> Error {
        Error::Sql(err)
    }
}

impl From<failure::Error> for Error {
    fn from(err: failure::Error) -> Error {
        Error::Failure(err)
    }
}

impl From<r2d2::Error> for Error {
    fn from(err: r2d2::Error) -> Error {
        Error::ConnectionPool(err)
    }
}
