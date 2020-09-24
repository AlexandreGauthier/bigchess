use serde::Serialize;
use serde_json;

// Error types
use shakmaty::fen::ParseFenError;
use shakmaty::san::ParseSanError;
use shakmaty::san::SanError;
use shakmaty::uci::ParseUciError;
use shakmaty::IllegalMoveError;
use shakmaty::PositionError;
use std::{fmt::Display, sync::PoisonError};
use tokio::io;

#[derive(Debug)]
/// Custom error type
/// These errors end up being converted to ErrorRepr objects, wrapped inside Response objects, serialized to json and outputed to stdout.
pub struct Error {
    pub error_type: ErrorType,
    pub source: Option<Box<dyn std::error::Error + 'static>>,
    pub id: Option<String>,
}

impl Error {
    pub fn is_type(&self, error_type: ErrorType) -> bool {
        self.error_type == error_type
    }

    pub fn is_recoverable(&self) -> bool {
        !self.is_type(ErrorType::PoisonedHandle)
    }

    pub fn new(error_type: ErrorType) -> Error {
        Error {
            error_type,
            source: None,
            id: None,
        }
    }

    pub fn with_id(self, id: &String) -> Self {
        Error {
            error_type: self.error_type,
            source: self.source,
            id: Some(id.clone()),
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let source = match &self.source {
            None => String::from("No underlying error"),
            Some(e) => format!("{:?}", e),
        };
        write!(
            f,
            "{} Comes from {}",
            human_readable_message(&self.error_type),
            source
        )
    }
}

impl std::error::Error for Error {}

#[derive(Debug, Serialize, PartialEq)]
pub enum ErrorType {
    Deserialize,
    Parse,
    ChessRules,
    BadHandle,
    StaleHandle,
    PoisonedHandle,
    IO,
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct ErrorRepr {
    #[serde(rename = "type")]
    error_type: ErrorType,
    message: String,
    underlying_error: Option<String>,
    game_id: Option<String>,
}

impl From<Error> for ErrorRepr {
    fn from(e: Error) -> ErrorRepr {
        let message = human_readable_message(&e.error_type);
        ErrorRepr {
            error_type: e.error_type,
            underlying_error: Some(format!("{:?}", e.source)),
            game_id: e.id,
            message,
        }
    }
}

fn human_readable_message(err_type: &ErrorType) -> String {
    let message = match err_type {
        ErrorType::Deserialize => "Could not parse JSON from stdin.",
        ErrorType::Parse => "Could not parse given input.",
        ErrorType::ChessRules => "Unexpected illegal chess position. Unchecked chess information should be sent to a function expecting it.",
        ErrorType::BadHandle => "Tried to use an invalid handle to a game or the inner state.",
        ErrorType::StaleHandle => "Tried to use an expired handle to a game.",
        ErrorType::PoisonedHandle => "Unrecoverable error: A thread crashed while holding a lock to the program state.",
        ErrorType::IO => "IO operation failed."
    };

    String::from(message)
}

/// Generates error type chaining boilerplate.
/// ```
/// conversion_boilerplate!(ErrorType::Deserialize => {
///     serde_json::Error,
///     othercrate::DeserializationError
/// });
/// ```
/// would expand to
/// ```
///
///impl From<serde_json::Error> for Error {
///    fn from(e: serde_json::Error) -> Error {
///        Error {
///            error_type: ErrorType::Deserialize,
///            source: Some(Box::from(e)),
///            id: None
///        }
///    }
///}
///
///impl From<othercrate::DeserializationError> for Error {
///    fn from(e: othercrate::DeserializationError) -> Error {
///        Error {
///            error_type: ErrorType::Deserialize,
///            source: Some(Box::from(e)),
///            id: None
///        }
///    }
///}
/// ```
macro_rules! conversion_boilerplate {
    ( $( $t:path => [$( $e:ty ),+] ),+ ) => {
        $($(
                impl From<$e> for Error {
                    fn from(e: $e) -> Error {
                        Error {
                            error_type: $t,
                            source: Some(Box::from(e)),
                            id: None
                        }
                    }
                }
        )+)+
    };
}

/// Isn't constructed from the macro to discard the source, simplifying lifetime issues.
/// And access to the inner state is not necessary anyways since we panic after throwing this error.
impl<T> From<PoisonError<T>> for Error {
    fn from(_: PoisonError<T>) -> Error {
        Error {
            error_type: ErrorType::PoisonedHandle,
            source: None,
            id: None,
        }
    }
}

conversion_boilerplate! {
    ErrorType::Deserialize => [
        serde_json::Error
    ],

    ErrorType::Parse => [
        ParseUciError,
        ParseFenError,
        ParseSanError
    ],

    ErrorType::ChessRules => [
        IllegalMoveError,
        PositionError,
        SanError
    ],

    ErrorType::IO => [
    io::Error
    ]
}
