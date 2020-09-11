use crate::errors::{Error, ErrorType};

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use shakmaty::san::{San, SanPlus};
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
    pub fn play(&mut self, from: &String, to: &String) -> Result<(), Error> {
        let san = self.find_or_create_branch(&from, &to, &self.current_line.clone())?;
        self.current_line.push(san);
        Ok(())
    }

    #[allow(dead_code)]
    pub fn play_san(&mut self, san: String) -> Result<(), Error> {
        let parsed_san: San = san.parse()?;
        let current_position = self.current_position();
        let mov = parsed_san.to_move(&current_position)?;
        self.play(&mov.from().unwrap().to_string(), &mov.to().to_string())
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

    pub fn current_position(&self) -> shakmaty::Chess {
        shakmaty_position(&self.initial_position, &self.current_line)
    }

    pub fn current_fen(&self) -> String {
        fen(&self.current_position())
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
                None => Err(Error {
                    error_type: ErrorType::ChessRules,
                    source: None,
                }),
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn play() {
        let mut game = Game::default();

        // Scholar's mate
        for (from, to) in &[
            ("e2", "e4"),
            ("e7", "e5"),
            ("f1", "c4"),
            ("b8", "c6"),
            ("d1", "h5"),
            ("g8", "f6"),
            ("h5", "f7"),
        ] {
            game.play(&String::from(from.to_owned()), &String::from(to.to_owned()))
                .unwrap();
        }
        let (_, current_pos) = last_and_current_position(&game);
        assert!(current_pos.is_checkmate());
        assert_eq!(
            &*fen(&current_pos),
            "r1bqkb1r/pppp1Qpp/2n2n2/4p3/2B1P3/8/PPPP1PPP/RNB1K1NR b KQkq - 0 4"
        )
    }

    #[test]
    fn navigate_back() {
        let mut game = Game::default();

        // Opera game: Paul Morphy - Duke of Brunswick and Count Isouard (1858)
        let opera_game = vec![
            "e4", "e5", "Nf3", "d6", "d4", "Bg4", "dxe5", "Bxf3", "Qxf3", "dxe5", "Bc4", "Nf6",
            "Qb3", "Qe7", "Nc3", "c6", "Bg5", "b5", "Nxb5", "cxb5", "Bxb5+", "Nbd7", "O-O-O",
            "Rd8", "Rxd7", "Rxd7", "Rd1", "Qe6", "Bxd7+", "Nxd7", "Qb8+", "Nxb8", "Rd8#",
        ];
        for san in opera_game {
            let parsed: SanPlus = san.parse().unwrap();
            game.play_san(String::from(parsed.san.to_string())).unwrap();
        }

        assert_eq!(
            (&game).current_fen(),
            "1n1Rkb1r/p4ppp/4q3/4p1B1/4P3/8/PPP2PPP/2K5 b k - 1 17"
        );

        // Should not change position
        game.navigate_back(0);
        assert_eq!(
            (&game).current_fen(),
            "1n1Rkb1r/p4ppp/4q3/4p1B1/4P3/8/PPP2PPP/2K5 b k - 1 17"
        );

        game.navigate_back(1);
        assert_eq!(
            (&game).current_fen(),
            "1n2kb1r/p4ppp/4q3/4p1B1/4P3/8/PPP2PPP/2KR4 w k - 0 17"
        );

        game.navigate_back(5);
        assert_eq!(
            (&game).current_fen(),
            "4kb1r/p2rqppp/5n2/1B2p1B1/4P3/1Q6/PPP2PPP/2KR4 b k - 1 14"
        );

        game.navigate_back(1000);
        assert_eq!(game.current_fen(), Game::default().current_fen());
    }

    #[test]
    // TODO - incomplete
    fn game_repr() {
        // Botvinik - Capablanca (1937)
        //
        let fen = String::from("r3r1k1/p2q1ppp/np3n2/3p4/P1pP4/2PQP3/1B2NPPP/R4RK1 w - - 0 15");
        let game = Game::from_fen(fen).unwrap();
        let g = game.get_repr();
        assert_eq!(g.index, 0);
        assert_eq!(g.is_check, false);
        assert_eq!(g.is_takes, false)
    }
}
