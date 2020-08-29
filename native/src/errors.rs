use crate::routes::WarpReply;

use std::fmt;
use std::sync::PoisonError;

use serde::Serialize;
use warp::http::StatusCode;
use shakmaty::IllegalMoveError;
use shakmaty::uci::ParseUciError;
use shakmaty::san::SanError;

#[derive(Debug)]
pub enum Error {
    IllegalMove(IllegalMoveError),
    ParseUci(ParseUciError),
    InvalidSan(SanError),
    StaleGameHandle(usize),
    BadGameHandle(usize),
    PoisonedMutex,
    TraverseDownBadLine,
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

impl From<SanError> for Error {
    fn from(e: SanError) -> Error {
        Error::InvalidSan(e)
    }
}


impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self {
            Error::IllegalMove(e) => {
               write!(f, "Recieved move request for an illegal move: {}).", e)
            },
            Error::ParseUci(e) => {
               write!(f, "'From' and 'to' parameters do not form a valid uci move: {}).", e)
            },
            Error::InvalidSan(e) => {
               write!(f, "'From' and 'to' parameters do not form a valid uci move: {}).", e)
            },
            Error::StaleGameHandle(handle) => {
                write!(f, "Tried to use stale game handle ({}).", handle) 
            },
            Error::BadGameHandle(handle) => {
                write!(f, "Mo game associated with handle {}.", handle) 
            },
            Error::TraverseDownBadLine => {
                write!(f, "Tried to access a non-existant line of a game tree.") 
            },
            Error::PoisonedMutex => {
                write!(f, "Program memory is corrupted because of a crash. (Poisoned mutex)") 
            }
        }
    }
}

#[derive(Serialize)]
struct ErrorJson {
    message: String,
    code: u16,
}


impl Error {
    pub fn into_warp_reply(self) -> WarpReply {
        let message = self.to_string();
        let code: u16 = match self {
            Error::ParseUci(_) => 400,
            Error::InvalidSan(_) => 400,
            Error::IllegalMove(_) => 400,
            Error::BadGameHandle(_) => 400,
            Error::StaleGameHandle(_) => 400,
            Error::TraverseDownBadLine => 500,
            Error::PoisonedMutex => 500,
        };

        generate_error_reply(message, code)
    }
}

fn generate_error_reply(message: String, code: u16) -> WarpReply {
    let json = ErrorJson { message, code };
    let json = warp::reply::json(&json);
    let code = StatusCode::from_u16(code).unwrap();
    warp::reply::with_status(json, code)
}
