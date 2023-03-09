use serenity::Error as SerenityError;
use sqlx::migrate::MigrateError;
use sqlx::Error as DbError;
use std;
use std::fmt::Error as FmtError;
use std::fmt::{self, Display};
use std::num::ParseIntError;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    Message(String),
    IlleagalNumberOfGuesses(char),
    ParseError(String),
    DatabaseError(DbError),
    DatabaseMigrationError(MigrateError),
    SerenityError(SerenityError),
    FmtError(FmtError),
    UnknownCommand,
}

impl From<FmtError> for Error {
    fn from(v: FmtError) -> Self {
        Self::FmtError(v)
    }
}

impl From<SerenityError> for Error {
    fn from(v: SerenityError) -> Self {
        Self::SerenityError(v)
    }
}

impl From<MigrateError> for Error {
    fn from(v: MigrateError) -> Self {
        Self::DatabaseMigrationError(v)
    }
}

impl Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Message(msg) => formatter.write_str(msg),
            Error::IlleagalNumberOfGuesses(x) => {
                formatter.write_fmt(format_args!("Illeagal number of guesses: {x}"))
            }
            Error::ParseError(err) => formatter.write_str(&err),
            Error::DatabaseError(err) => formatter.write_str(&err.to_string()),
            Error::DatabaseMigrationError(err) => formatter.write_str(&err.to_string()),
            Error::SerenityError(err) => formatter.write_str(&err.to_string()),
            Error::FmtError(err) => formatter.write_str(&err.to_string()),
            Error::UnknownCommand => formatter.write_str("Unknown command recieved."),
        }
    }
}

impl From<ParseIntError> for Error {
    fn from(v: ParseIntError) -> Self {
        Self::ParseError(v.to_string())
    }
}

impl From<DbError> for Error {
    fn from(err: DbError) -> Self {
        Self::DatabaseError(err)
    }
}

impl std::error::Error for Error {}
