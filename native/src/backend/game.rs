use crate::backend::errors::Error;

use shakmaty::uci::Uci;
use shakmaty::san::SanPlus;
use shakmaty::{Position};

use std::sync::{Arc, Mutex, MutexGuard, RwLock, RwLockReadGuard, RwLockWriteGuard};

pub use json::JsonResponse as JsonResponse;

type InnerState = Vec<Option<Box<Mutex<Game>>>>;

pub struct StateHandle {
    inner: Arc<RwLock<InnerState>>
}

impl StateHandle {
    pub fn play(&self, index: usize, from: String, to:String) -> Result<JsonResponse, Error> {
        let read_lock = self.read()?;
        let mut game_lock = read_lock.get_game(index)?;
        game_lock.play(from, to)
            .map(|res| game_lock.generate_json())
    }

    pub fn navigate_back(&self, index: usize, back: u16) -> Result<JsonResponse, Error> {
        let read_lock = self.read()?;
        let mut game_lock = read_lock.get_game(index)?;

        game_lock.navigate_back(back);
        Ok(game_lock.generate_json())
    }

    pub fn get_all_games(&self) -> Result<Vec<JsonResponse>, Error> {
        let read_lock = self.read()?;
        let responses = read_lock.all_games()
            .map(|game| game.generate_json())
            .collect::<Vec<JsonResponse>>();

        Ok(responses)
    }

    pub fn new_game_default(&self) -> Result<JsonResponse, Error> {
        let mut write_lock = self.write()?;
        let index = write_lock.new_game_default();
        drop(write_lock);

        let read_lock = self.read()?;
        let game = read_lock.get_game(index)?;
        Ok(game.generate_json())
    }

    fn read(&self) -> Result<RwLockReadGuard<InnerState>, Error> {
        self.inner.read().map_err(|_| Error::PoisonedMutex)
    }

    fn write(&self) -> Result<RwLockWriteGuard<InnerState>, Error> {
        self.inner.write().map_err(|_| Error::PoisonedMutex)
    }
}

impl Default for StateHandle {
    fn default() -> StateHandle {
        let state =  StateHandle {
            inner: Arc::new(RwLock::new(Vec::new()))
        };
    let _ = state.new_game_default();
    state
    }
}

impl Clone for StateHandle {
    fn clone(&self) -> StateHandle {
        StateHandle {
            inner: Arc::clone(&self.inner)
        }
    }
}

trait ReadOperations<'a> {
    fn get_game(&'a self, index: usize) -> Result<MutexGuard<'a, Game>, Error>;
    fn all_games(&'a self) -> GamesIterator<'a>;
}

impl<'a> ReadOperations<'a> for RwLockReadGuard<'a, InnerState> {
    fn get_game(&'a self, index: usize) -> Result<MutexGuard<'a, Game>, Error> {
        self.get(index)
            .ok_or(Error::BadGameHandle(index))?.as_ref()
            .ok_or(Error::StaleGameHandle(index))?
            .lock().map_err(|_| Error::PoisonedMutex)
    }

    fn all_games(&'a self) -> GamesIterator<'a> {
        GamesIterator {
            target: &self,
            index: 0
        }
    } 
}



pub trait WriteOperations {
    fn close_game(&mut self, index: usize) -> Result<(), Error>;
    fn new_game_default(&mut self) -> usize;
}

impl<'a> WriteOperations for RwLockWriteGuard<'a, InnerState> {
    fn close_game(&mut self, index: usize) -> Result<(), Error> {
        let element = self.get_mut(index)
            .ok_or(Error::BadGameHandle(index))?;

        match element {
            None => Err(Error::StaleGameHandle(index)),
            Some(_) => {
                element.take();
                Ok(())
            }
        }
    }

    fn new_game_default(&mut self) -> usize {
        let game = Game::default();
        insert_game(self, game)
    }
}

/// Iterator over every game in the state.
/// Skips over deleted games.
struct GamesIterator<'a> {
    target: &'a RwLockReadGuard<'a, InnerState>,
    index: usize
}

impl<'a> Iterator for GamesIterator<'a> {
    type Item = MutexGuard<'a, Game>;
    fn next(&mut self) -> Option<Self::Item> {
        let result = self.target.get_game(self.index);
        self.index += 1;

        match result {
            Ok(lock) => {
                self.index += 1;
                Some(lock)
            },
            Err(e) => match e {
                Error::PoisonedMutex => panic!{"Iterated over corrupted state! Error: {}", e},
                Error::StaleGameHandle(_) => self.next(),
                _ => None
            }
        }
    }
}

fn insert_game(vec: &mut Vec<Option<Box<Mutex<Game>>>>, mut game: Game) -> usize {
    let index = vec.len();
    game.index = index;
    let element = Some(Box::new(Mutex::new(game)));
    vec.push(element);
    index
}

#[derive(Default, Debug)]
pub struct Game {
    index: usize,
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

    pub fn generate_json(&self) -> JsonResponse {
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

// TODO (Maybe in other module)
pub struct EngineConfig {}

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
    use crate::backend::game;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test] 
    fn state_handle_() {
        let state_handle = StateHandle::default();
    }

    #[test]
    fn default_state_values() {
        let state_handle = StateHandle::default();
        let mut state = state_handle.lock();

        let mut game_tree_lock = state.game_tree.lock();
        assert_eq!(game_tree_lock.children.len(), 0);
        game_tree_lock.children.push(Branch::starting_position());
        drop(game_tree_lock);

        let current_pos_lock = state.current_position.lock();
        assert_eq!(current_pos_lock.children.len(), 1);
        assert_eq!(shakmaty::fen::epd(&current_pos_lock.position),
           "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq -");
        assert_eq!(state.game_info, GameInfo::default());

    }

    #[test]
    fn state_play() {
        let state_handle = StateHandle::default();
        let mut state = state_handle.lock();
        let result = state.play("e2".to_string(), "e4".to_string());

        assert!(result.is_ok());

        let game_tree_lock = state.game_tree.lock();
        assert_eq!(game_tree_lock.children.len(), 1);

        let child_lock = game_tree_lock.children[0].lock();
        assert!(state.current_position.inner.try_lock().is_err());
        assert_eq!(child_lock.position.turn(), shakmaty::Color::Black);

    }

    #[test]
    fn state_navigate_back() {
        let state_handle = StateHandle::default();
        let mut state = state_handle.lock();

        for moves in &[("e2", "e4"), ("e7", "e5"), ("g1", "f3"),  ("b8", "c6"), ("f1", "c4")] {
            let result = state.play(moves.0.to_string(), moves.1.to_string());
            assert!(result.is_ok());
        }

        state.navigate_back(1);
        let lock = state.current_position.lock();
        assert_eq!(lock.san, "Nc6");
        drop(lock);

        state.navigate_back(0);
        let lock = state.current_position.lock();
        assert_eq!(lock.san, "Nc6");
        drop(lock);

        state.navigate_back(3);
        let lock = state.current_position.lock();
        assert_eq!(lock.san, "e4");
        drop(lock);

        state.navigate_back(100);
        let lock = state.current_position.lock();
        assert!(lock.san.is_empty());
    }

    #[test]
    fn main_line_current_pos() {
        let state_handle = StateHandle::default();
        let mut state = state_handle.lock();

        let _ = state.play("e2".to_string(), "e4".to_string());
        state.navigate_back(1);
        let _ = state.play("e2".to_string(), "e3".to_string());
        state.navigate_back(1);
        let _ = state.play("e2".to_string(), "e4".to_string());
        let _ = state.play("e7".to_string(), "e5".to_string());
        state.navigate_back(2);
        let _ = state.play("g1".to_string(), "f3".to_string());

        assert_eq!(state.current_position.lock().san, "Nf3".to_string());
        assert_eq!(state.main_line.lock().san, "e5".to_string());
    }
}

