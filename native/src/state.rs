use std::sync::{Arc, Mutex, MutexGuard, RwLock, RwLockReadGuard, RwLockWriteGuard};
use crate::game::{Game, JsonResponse};
use crate::errors::Error;

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
