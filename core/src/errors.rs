use serde::{Serialize, Serializer};
use serde_json;

use shakmaty::fen::ParseFenError;
use shakmaty::san::SanError;
use shakmaty::uci::ParseUciError;
use shakmaty::IllegalMoveError;
use shakmaty::PositionError;
use std::sync::PoisonError;
use tokio::io;

#[derive(Debug)]
pub enum Error {
    IllegalMove(IllegalMoveError),
    ParseUci(ParseUciError),
    ParseFen(ParseFenError),
    InvalidPosition(PositionError),
    InvalidSan(SanError),
    StaleGameHandle(usize),
    BadGameHandle(usize),
    PoisonedMutex,
    TraverseDownBadLine,
    IO(io::Error),
    Deserialize(serde_json::Error),
}

// TODO
impl Serialize for Error {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&*format!(
            "ERROR SERIALIZATION NOT YET IMPLEMENTED -- {:?}",
            self
        ))
    }
}

impl From<IllegalMoveError> for Error {
    fn from(e: IllegalMoveError) -> Error {
        Error::IllegalMove(e)
    }
}

impl<T> From<PoisonError<T>> for Error {
    fn from(_: PoisonError<T>) -> Error {
        Error::PoisonedMutex
    }
}

impl From<ParseUciError> for Error {
    fn from(e: ParseUciError) -> Error {
        Error::ParseUci(e)
    }
}

impl From<ParseFenError> for Error {
    fn from(e: ParseFenError) -> Error {
        Error::ParseFen(e)
    }
}

impl From<PositionError> for Error {
    fn from(e: PositionError) -> Error {
        Error::InvalidPosition(e)
    }
}

impl From<SanError> for Error {
    fn from(e: SanError) -> Error {
        Error::InvalidSan(e)
    }
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Error {
        Error::IO(e)
    }
}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Error {
        Error::Deserialize(e)
    }
}
