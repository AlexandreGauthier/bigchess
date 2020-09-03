use crate::database::DatabaseRepr;
use crate::engine::EngineRepr;
use crate::errors::Error;
use crate::game::GameRepr;
use crate::state::StateHandle;

use std::io::Write;

use serde::{Deserialize, Serialize};
use serde_json;
use tokio::io;
use tokio::io::{AsyncBufReadExt, BufReader};

pub async fn handler(state: StateHandle) -> Result<(), Error> {
    let stdin = io::stdin();
    let mut stdin_lines = BufReader::new(stdin).lines();

    loop {
        let line = stdin_lines.next_line().await?.unwrap_or_default();

        let result = dispatch_line(&*line, &state);
        if let Err(err) = result {
            respond_with_error(err)
        }
    }
}

fn dispatch_line(line: &str, state: &StateHandle) -> Result<(), Error> {
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

fn respond_with_error(error: Error) {
    let response = Response {
        error: Some(error),
        changed_games: Vec::new(),
    };

    send_to_stdout(response);
}

pub fn respond_with_game(repr: GameRepr) {
    let mut changed_games = Vec::new();
    changed_games.push(repr);

    let response = Response {
        error: None,
        changed_games,
    };

    send_to_stdout(response);
}

pub fn respond_with_games(games: impl Iterator<Item = GameRepr>) {
    let response = Response {
        error: None,
        changed_games: games.collect(),
    };

    send_to_stdout(response);
}

pub fn send_to_stdout(response: Response) {
    let mut stdout = std::io::stdout();
    serde_json::to_writer(&mut stdout, &response)
        .expect("Unrecoverable error: could not serialize response object to stdout.");

    write!(&mut stdout, "\n")
        .and_then(|_| stdout.flush())
        .expect("Unrecoverable error: could not write to stdout.");
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
    error: Option<Error>,
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
