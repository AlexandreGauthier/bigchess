use crate::errors::Error;

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use shakmaty::san::SanPlus;
use shakmaty::uci::Uci;
use shakmaty::Position;

#[derive(Default, Debug)]
pub struct Game {
    /// Index if the game in state's inner Vec
    pub index: usize,
    /// Textual information about the game.
    game_info: GameInfo,
    /// List of san moves leading to the current position (e4 e5 Nf3 nc6 ...)
    current_line: Vec<SanPlus>,
    /// Initial game state, the starting chess position or loaded from an fen.
    initial_position: shakmaty::Chess,
    /// Tree of moves played or analysed during the game.
    game_tree: GameTree,
}

#[derive(Default, Debug)]
struct GameTree {
    /// Standard algebraic notation for the current move. Is `None` if this GameTree represents the starting position.
    san: Option<SanPlus>,
    /// `lines[0]` represents the main line, `lines[1..n]` are sidelines.
    lines: Vec<GameTree>,
    /// Move annotation like ?? for blunders and ! for critical moves.
    annotation: Option<Annotation>,
    /// Engine evaluation in tenths of pawns (evaluation = +10 -> 1 pawn advantage for white);
    evaluation: Option<i16>,
}

impl Game {
    pub fn play(&mut self, from: String, to: String) -> Result<(), Error> {
        let san = self.find_or_create_branch(&from, &to, &self.current_line.clone())?;
        self.current_line.push(san);
        Ok(())
    }

    fn find_or_create_branch(
        &mut self,
        from: &String,
        to: &String,
        line: &Vec<SanPlus>,
    ) -> Result<SanPlus, Error> {
        let branch = traverse_down(&mut self.game_tree, line.as_slice())?;
        let pos = shakmaty_position(&self.initial_position, line);
        let mov = fromto_to_move(from, to, &pos)?;
        let san = SanPlus::from_move(pos, &mov);

        let existing_branch = branch
            .lines
            .iter()
            .position(|elem| elem.san.as_ref() == Some(&san));

        if existing_branch.is_none() {
            insert_branch(&mut branch.lines, san.clone());
        }
        Ok(san)
    }

    pub fn navigate_back(&mut self, back: u16) {
        let new_length = self.current_line.len().saturating_sub(back as usize);
        self.current_line.truncate(new_length);
    }

    pub fn get_repr(&self) -> GameRepr {
        let (maybe_last, current_position) = last_and_current_position(self);
        GameRepr {
            index: self.index,
            available_moves: available_moves(&current_position),
            fen: fen(&current_position),
            is_takes: is_takes(maybe_last),
            is_check: current_position.is_check(),
        }
    }

    pub fn from_fen(fen_string: String) -> Result<Game, Error> {
        let mut game = Game::default();
        let setup: shakmaty::fen::Fen = fen_string.parse()?;
        game.initial_position = setup.position()?;
        Ok(game)
    }
}

fn traverse_down<'a>(tree: &'a mut GameTree, line: &[SanPlus]) -> Result<&'a mut GameTree, Error> {
    match line.split_first() {
        None => Ok(tree),
        Some((san, tail)) => {
            let child = tree
                .lines
                .iter_mut()
                .find(|pos| pos.san.as_ref() == Some(san));
            match child {
                None => Err(Error::TraverseDownBadLine),
                Some(game) => traverse_down(game, tail),
            }
        }
    }
}

fn shakmaty_position<'a, I>(starting_position: &shakmaty::Chess, line: I) -> shakmaty::Chess
where
    I: IntoIterator<Item = &'a SanPlus>,
{
    let mut position = starting_position.clone();
    for san in line {
        let m = san
            .san
            .to_move(&position)
            .expect("Tried to compute an invalid line");
        position.play_unchecked(&m);
    }
    position
}

fn fromto_to_move(
    from: &String,
    to: &String,
    pos: &shakmaty::Chess,
) -> Result<shakmaty::Move, Error> {
    let m = format!("{}{}", from, to).parse::<Uci>()?;
    Ok(m.to_move(pos)?)
}

fn san_to_move(san: &SanPlus, pos: &shakmaty::Chess) -> Result<shakmaty::Move, Error> {
    Ok(san.san.to_move(pos)?)
}

fn insert_branch(vec: &mut Vec<GameTree>, san: SanPlus) {
    vec.push(GameTree {
        san: Some(san),
        lines: Vec::new(),
        annotation: None,
        evaluation: None,
    });
}

// TODO
#[derive(Debug)]
enum Annotation {}

// TODO
#[derive(Default, Debug, PartialEq, Eq)]
struct Player {}

// TODO
#[derive(Default, Debug, PartialEq, Eq)]
struct Lichess {}

#[derive(Default, Debug, PartialEq, Eq)]
struct GameInfo {
    players: (Option<Player>, Option<Player>),
    game_title: String,
    lichess: Option<Lichess>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GameRepr {
    pub index: usize,
    pub available_moves: HashMap<String, Vec<String>>,
    pub fen: String,
    pub is_takes: bool,
    pub is_check: bool,
}

fn last_and_current_position(game: &Game) -> (Option<(SanPlus, shakmaty::Chess)>, shakmaty::Chess) {
    match game.current_line.split_last() {
        Some((last_move, line)) => {
            let last_pos = shakmaty_position(&game.initial_position, line);
            let current_pos = shakmaty_position(&last_pos, std::iter::once(last_move));
            (Some((last_move.clone(), last_pos)), current_pos)
        }
        None => (None, game.initial_position.clone()),
    }
}

fn available_moves(position: &shakmaty::Chess) -> HashMap<String, Vec<String>> {
    let mut map = HashMap::with_capacity(32);
    let legal_moves = position.legals();
    for m in legal_moves {
        insert_move(&m, &mut map);
    }
    map
}

fn insert_move(chess_move: &shakmaty::Move, map: &mut HashMap<String, Vec<String>>) {
    let from = chess_move.from().unwrap().to_string();
    let to = chess_move.to().to_string();
    insert_from_to(from, to, map);
}

fn insert_from_to(from: String, to: String, map: &mut HashMap<String, Vec<String>>) {
    match map.get_mut(&from) {
        None => {
            let mut vec = Vec::with_capacity(30);
            vec.push(to);
            map.insert(from, vec);
        }
        Some(vec) => {
            vec.push(to);
        }
    };
}

fn fen(pos: &shakmaty::Chess) -> String {
    shakmaty::fen::fen(pos).to_string()
}

fn is_takes(maybe_last: Option<(SanPlus, shakmaty::Chess)>) -> bool {
    match maybe_last {
        None => false,
        Some((san, pos)) => san_to_move(&san, &pos).unwrap().is_capture(),
    }
}
