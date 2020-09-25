use crate::game::GameRepr;
use crate::{
    errors::{Error, ErrorRepr},
    state::StateHandle,
};

use serde::{Deserialize, Serialize};

/// Only returns Err(Error) when it is not recoverable
/// All other errors are returned in the form of Ok(Response)
pub fn dispatch_request(request: Request, state: &StateHandle) -> Result<Response, Error> {
    let result = match request {
        Request::Play(PlayArgs { id, from, to }) => state.play(&id, from, to),
        Request::NavigateBack(NavigateBackArgs { id, back }) => state.navigate_back(&id, back),
        Request::GetAllGames(_) => state.get_all_games(),
        Request::NewGame(NewGameArgs { id }) => state.new_game_default(&id),
    };

    handle_fatal_error(result)
}

pub fn response_from_error(error: Error) -> Response {
    Response {
        error: Some(error.into()),
        changed_games: Vec::new(),
    }
}

pub fn response_from_game(id: String, repr: GameRepr) -> Response {
    let mut changed_games = Vec::new();
    changed_games.push(ChangedGame { id, game: repr });

    Response {
        error: None,
        changed_games,
    }
}

/// Generates a response from an iterator of changed games
pub fn response_from_games(
    games: impl Iterator<Item = Result<(String, GameRepr), Error>>,
) -> Result<Response, Error> {
    let mut changed_games = Vec::new();
    for game in games {
        match game {
            Ok((id, repr)) => changed_games.push(ChangedGame { id, game: repr }),
            Err(err) => return Ok(handle_fatal_error(Err(err))?),
        }
    }

    Ok(Response {
        changed_games,
        error: None,
    })
}

/// Converts Err(Error) to Some(Response) as long as it is recoverable
pub fn handle_fatal_error(result: Result<Response, Error>) -> Result<Response, Error> {
    match result {
        Ok(response) => Ok(response),
        Err(err) if err.is_recoverable() => Ok(response_from_error(err)),
        Err(err) => Err(err),
    }
}

/// Response type to be serialized into JSON
#[derive(Serialize, Debug)]
#[serde(rename_all = "snake_case")]
pub struct Response {
    error: Option<ErrorRepr>,
    changed_games: Vec<ChangedGame>,
}

#[derive(Serialize, Debug)]
pub struct ChangedGame {
    id: String,
    game: GameRepr,
}

/// Request type into which JSON from stdin is deserialized
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "method", content = "params")]
pub enum Request {
    Play(PlayArgs),
    NavigateBack(NavigateBackArgs),
    GetAllGames(GetAllGamesArgs),
    NewGame(NewGameArgs),
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct PlayArgs {
    id: String,
    to: String,
    from: String,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct NavigateBackArgs {
    id: String,
    back: u16,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct GetAllGamesArgs {}

// TODO  more new game types (fen, pgn, path, etc.)
#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct NewGameArgs {
    id: String,
}
