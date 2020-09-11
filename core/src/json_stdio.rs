use crate::database::DatabaseRepr;
use crate::engine::EngineRepr;
use crate::errors::{Error, ErrorRepr};
use crate::game::GameRepr;
use crate::state::StateHandle;

use std::fmt::Debug;
use std::io::Write;

use serde::{Deserialize, Serialize};
use serde_json;
use tokio::io;
use tokio::io::{AsyncBufReadExt, BufReader};

pub async fn handler(state: StateHandle) -> Result<(), Error> {
    let mut stdout = std::io::stdout();
    let stdin = io::stdin();
    let mut stdin_lines = BufReader::new(stdin).lines();

    loop {
        let new_line = stdin_lines.next_line().await?.unwrap_or_default();
        let response = dispatch_line(&*new_line, &state)?;
        send_to_stream(response, &mut stdout);
    }
}

fn dispatch_line(line: &str, state: &StateHandle) -> Result<Response, Error> {
    let request = serde_json::from_str(line)?;
    match request {
        Request::Play(PlayArgs { index, from, to }) => state.play(index, from, to),
        Request::NavigateBack(NavigateBackArgs { index, back }) => state.navigate_back(index, back),
        Request::GetAllGames => state.get_all_games(),
        Request::NewGame(t) => match t {
            NewGameType::Default => state.new_game_default(),
            NewGameType::FromFen(fen) => state.new_game_fen(fen),
        },
    }
}

pub fn response_from_error(error: Error) -> Response {
    Response {
        error: Some(error.into()),
        changed_games: Vec::new(),
    }
}

pub fn response_from_game(repr: GameRepr) -> Response {
    let mut changed_games = Vec::new();
    changed_games.push(repr);

    Response {
        error: None,
        changed_games,
    }
}

pub fn response_from_games(games: impl Iterator<Item = GameRepr>) -> Response {
    Response {
        error: None,
        changed_games: games.collect(),
    }
}

pub fn send_to_stream<W: Write + Debug>(response: Response, mut stream: W) {
    serde_json::to_writer(&mut stream, &response)
        .expect("Unrecoverable error: could not serialize response object to stdout.");

    write!(&mut stream, "\n")
        .and_then(|_| stream.flush())
        .expect(&*format!(
            "Unrecoverable error: could not write to stream {:?}",
            stream,
        ));
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "method", content = "params")]
enum Request {
    Play(PlayArgs),
    NavigateBack(NavigateBackArgs),
    GetAllGames,
    NewGame(NewGameType),
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "snake_case")]
pub struct Response {
    error: Option<ErrorRepr>,
    changed_games: Vec<GameRepr>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct PlayArgs {
    index: usize,
    to: String,
    from: String,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct NavigateBackArgs {
    index: usize,
    back: u16,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
enum NewGameType {
    Default,
    FromFen(String),
}
