use crate::errors::Error;
use crate::game::Game;
use crate::json_stdio;
use std::sync::{Arc, Mutex, MutexGuard, RwLock, RwLockReadGuard, RwLockWriteGuard};

type InnerState = Vec<Option<Box<Mutex<Game>>>>;

pub struct StateHandle {
    inner: Arc<RwLock<InnerState>>,
}

impl StateHandle {
    pub fn play(&self, index: usize, from: String, to: String) -> Result<(), Error> {
        let read_lock = self.read()?;
        let mut game_lock = read_lock.get_game(index)?;

        game_lock.play(from, to)?;
        respond_with_game(&game_lock);
        Ok(())
    }

    pub fn navigate_back(&self, index: usize, back: u16) -> Result<(), Error> {
        let read_lock = self.read()?;
        let mut game_lock = read_lock.get_game(index)?;

        game_lock.navigate_back(back);
        respond_with_game(&game_lock);
        Ok(())
    }

    pub fn get_all_games(&self) -> Result<(), Error> {
        let read_lock = self.read()?;
        let games = read_lock.all_games();

        respond_with_games(games);
        Ok(())
    }

    pub fn new_game_default(&self) -> Result<(), Error> {
        let mut write_lock = self.write()?;
        let index = write_lock.new_game_default();
        let game = write_lock.get_game(index)?;

        respond_with_game(&game);
        Ok(())
    }

    pub fn new_game_fen(&self, fen: String) -> Result<(), Error> {
        let mut write_lock = self.write()?;
        let index = write_lock.new_game_fen(fen)?;
        let game = write_lock.get_game(index)?;

        respond_with_game(&game);
        Ok(())
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
        let state = StateHandle {
            inner: Arc::new(RwLock::new(Vec::new())),
        };
        let _ = state.new_game_default();
        state
    }
}

impl Clone for StateHandle {
    fn clone(&self) -> StateHandle {
        StateHandle {
            inner: Arc::clone(&self.inner),
        }
    }
}

trait StateOperations {
    fn get_game(&self, index: usize) -> Result<MutexGuard<Game>, Error>;
    fn all_games(&self) -> GamesIterator;
    fn close_game(&mut self, index: usize) -> Result<(), Error>;
    fn new_game_default(&mut self) -> usize;
    fn new_game_fen(&mut self, fen: String) -> Result<usize, Error>;
}

impl StateOperations for InnerState {
    fn get_game(&self, index: usize) -> Result<MutexGuard<Game>, Error> {
        self.get(index)
            .ok_or(Error::BadGameHandle(index))?
            .as_ref()
            .ok_or(Error::StaleGameHandle(index))?
            .lock()
            .map_err(|_| Error::PoisonedMutex)
    }

    fn all_games(&self) -> GamesIterator {
        GamesIterator {
            target: &self,
            index: 0,
        }
    }

    fn close_game(&mut self, index: usize) -> Result<(), Error> {
        let element = self.get_mut(index).ok_or(Error::BadGameHandle(index))?;

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

    fn new_game_fen(&mut self, fen: String) -> Result<usize, Error> {
        let game = Game::from_fen(fen)?;
        let index = insert_game(self, game);
        Ok(index)
    }
}

struct GamesIterator<'a> {
    target: &'a InnerState,
    index: usize,
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
            }
            Err(e) => match e {
                Error::PoisonedMutex => panic! {"Iterated over corrupted state!"},
                Error::StaleGameHandle(_) => self.next(),
                _ => None,
            },
        }
    }
}

fn respond_with_game(game_lock: &MutexGuard<Game>) {
    let repr = game_lock.get_repr();
    json_stdio::respond_with_game(repr);
}

fn respond_with_games(games: GamesIterator) {
    let game_reprs = games.map(|game| game.get_repr());
    json_stdio::respond_with_games(game_reprs);
}

fn insert_game(vec: &mut Vec<Option<Box<Mutex<Game>>>>, mut game: Game) -> usize {
    let index = vec.len();
    game.index = index;
    let element = Some(Box::new(Mutex::new(game)));
    vec.push(element);
    index
}
