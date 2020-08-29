use crate::errors::Error;

use shakmaty::uci::Uci;
use shakmaty::san::SanPlus;
use shakmaty::{Position};

pub use json::JsonResponse as JsonResponse;

#[derive(Default, Debug)]
pub struct Game {
    pub index: usize,
    game_info: GameInfo,
    current_line: Vec<SanPlus>,
    initial_position: shakmaty::Chess,
    game_tree: GameTree
}

#[derive(Default, Debug)]
struct GameTree {
    san: Option<SanPlus>,
    lines: Vec<GameTree>,
    annotation: Option<Annotation>,
    evaluation: Option<i16>,
}

impl Game {
    pub fn play(&mut self, from: String, to: String) -> Result<(), Error> {
        let san = self.find_or_create_branch(&from, &to, &self.current_line.clone())?;
        self.current_line.push(san);
        Ok(())
    }

    fn find_or_create_branch(&mut self, from: &String, to: &String, line: &Vec<SanPlus>) -> Result<SanPlus, Error> {

        let branch = traverse_down(&mut self.game_tree, line.as_slice())?;
        let pos = shakmaty_position(&self.initial_position, line);
        let mov = fromto_to_move(from, to, &pos)?;
        let san = SanPlus::from_move(pos, &mov);

        let existing_branch = branch.lines.iter().position(|elem| {elem.san.as_ref() == Some(&san)});
        if existing_branch.is_none() {
            insert_branch(&mut branch.lines, san.clone());
        }
        Ok(san)
    }

    pub fn navigate_back(&mut self, back: u16) {
        let new_length = self.current_line.len().saturating_sub(back as usize);
        self.current_line.truncate(new_length);
    }

    pub fn generate_json(&self) -> json::JsonResponse {
        json::generate_json(self)
    }

} 

fn traverse_down<'a>(tree: &'a mut GameTree, line: &[SanPlus]) -> Result<&'a mut GameTree, Error> {
    match line.split_first() {
        None => Ok(tree),
        Some((san, tail)) => {
            let child = tree.lines.iter_mut().find(|pos| {pos.san.as_ref() == Some(san)});
            match child {
                None => Err(Error::TraverseDownBadLine),
                Some(game) => traverse_down(game, tail)
            }
        }
    }
}

fn shakmaty_position<'a, I>(starting_position: &shakmaty::Chess, line: I) -> shakmaty::Chess
where I: IntoIterator<Item=&'a SanPlus>,
{
    let mut position = starting_position.clone();
    for san in line {
        let m = san.san.to_move(&position).expect("Tried to compute an invalid line");
        position.play_unchecked(&m);
    }
    position
}

fn fromto_to_move (from: &String, to: &String, pos: &shakmaty::Chess) -> Result<shakmaty::Move, Error> {
    let m = format!("{}{}", from, to).parse::<Uci>()?;
    Ok(m.to_move(pos)?)
}

fn san_to_move(san: &SanPlus, pos: &shakmaty::Chess) -> Result<shakmaty::Move, Error> {
    Ok(san.san.to_move(pos)?)
}

fn insert_branch(vec: &mut Vec<GameTree>, san: SanPlus) {
    vec.push(GameTree{
        san: Some(san),
        lines: Vec::new(),
        annotation: None,
        evaluation: None
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

mod json {
    use crate::game;
    use serde::Serialize;
    use shakmaty::{Position, san::SanPlus};
    use std::collections::HashMap;

    #[derive(Serialize, Debug)]
    pub struct JsonResponse {
        pub code: u16,
        pub index: usize,
        pub available_moves: HashMap<String, Vec<String>>,
        pub fen: String,
        pub is_takes: bool,
        pub is_check: bool,
    }


    pub fn generate_json(game: &game::Game) -> JsonResponse {
        let (maybe_last, current_pos) = last_and_current_position(game);
        
        JsonResponse {
            code: 200,
            index: game.index,
            available_moves: dbg!(available_moves(&current_pos)),
            fen: dbg!(fen(&current_pos)),
            is_takes: dbg!(is_takes(maybe_last)),
            is_check: dbg!(current_pos.is_check())
        }
    }

    fn last_and_current_position(game: &game::Game) -> (Option<(SanPlus, shakmaty::Chess)>, shakmaty::Chess) {
        match game.current_line.split_last() {
            Some((last_move, line)) => {
                let last_pos = game::shakmaty_position(&game.initial_position, line);
                let current_pos = game::shakmaty_position(&last_pos, std::iter::once(last_move));
                (Some((last_move.clone(), last_pos)), current_pos)
                 
            },
            None => (None, game.initial_position.clone())
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
            Some((san, pos)) => {
                game::san_to_move(&san, &pos).unwrap().is_capture()
            }
        }
    }
}
