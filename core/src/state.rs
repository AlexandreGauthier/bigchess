use crate::api::{response_from_game, response_from_games, Response};
use crate::errors::{Error, ErrorType};
use crate::game::Game;

use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard, RwLock, RwLockWriteGuard};

type GameCell = Option<Mutex<Game>>;
type InnerState = HashMap<String, GameCell>;

pub struct StateHandle {
    inner: Arc<RwLock<InnerState>>,
}

impl StateHandle {
    pub fn play(&self, id: &String, from: String, to: String) -> Result<Response, Error> {
        self.game_operation(id, |game| game.play(&from, &to))
    }

    pub fn navigate_back(&self, id: &String, back: u16) -> Result<Response, Error> {
        self.game_operation(id, |game| {
            game.navigate_back(back);
            Ok(())
        })
    }

    pub fn get_all_games(&self) -> Result<Response, Error> {
        // self.state_operation returns a response with all state, so no extra operation is needed
        self.state_operation(|_| Ok(()))
    }

    pub fn new_game_default(&self, id: &String) -> Result<Response, Error> {
        self.state_operation(|state| {
            state.new_game_default(id)?;
            Ok(())
        })
    }

    /// Applies operation to a specific game located at `index`, responds with an error or with the modified game.
    fn game_operation<C>(&self, id: &String, closure: C) -> Result<Response, Error>
    where
        C: Fn(&mut MutexGuard<Game>) -> Result<(), Error>,
    {
        let read_guard = self.inner.read()?;
        let mut game_guard = read_guard.get_game(id)?;
        closure(&mut game_guard)?;

        Ok(response_from_game(id.clone(), game_guard.get_repr()))
    }

    /// Applies operation requiring access to the whole state. This is necessary to access all games or to add/delete a game.
    /// Returned response contains all games since all state has potentially been modified
    ///
    fn state_operation<C>(&self, closure: C) -> Result<Response, Error>
    where
        C: Fn(&mut RwLockWriteGuard<InnerState>) -> Result<(), Error>,
    {
        let mut guard = self.inner.write()?;
        closure(&mut guard)?;

        let all_games = guard
            .all_games()
            .map(|r| r.map(|(id, game)| (id, game.get_repr())));
        Ok(response_from_games(all_games)?)
    }
}

impl Default for StateHandle {
    fn default() -> StateHandle {
        let state = StateHandle {
            inner: Arc::new(RwLock::new(HashMap::new())),
        };
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
    fn get_game(&self, id: &String) -> Result<MutexGuard<Game>, Error>;
    fn all_games(&self) -> GamesIterator;
    fn close_game(&mut self, id: &String) -> Result<(), Error>;
    fn new_game_default(&mut self, id: &String) -> Result<(), Error>;
    fn new_game_fen(&mut self, id: &String, fen: String) -> Result<(), Error>;
}

impl StateOperations for InnerState {
    fn get_game(&self, id: &String) -> Result<MutexGuard<Game>, Error> {
        self.get(id)
            .ok_or(Error::new(ErrorType::BadHandle).with_id(id))?
            .as_ref()
            .ok_or(Error::new(ErrorType::StaleHandle).with_id(id))?
            .lock()
            .map_err(|_| Error::new(ErrorType::PoisonedHandle).with_id(id))
    }

    fn all_games(&self) -> GamesIterator {
        GamesIterator::from(self)
    }

    fn close_game(&mut self, id: &String) -> Result<(), Error> {
        let element = self
            .get_mut(id)
            .ok_or(Error::new(ErrorType::BadHandle).with_id(&id))?;

        match element {
            None => Err(Error::new(ErrorType::StaleHandle).with_id(&id)),
            Some(_) => {
                element.take();
                Ok(())
            }
        }
    }

    fn new_game_default(&mut self, id: &String) -> Result<(), Error> {
        let game = Some(Mutex::from(Game::default()));
        self.insert(id.clone(), game);
        Ok(())
    }

    fn new_game_fen(&mut self, id: &String, fen: String) -> Result<(), Error> {
        let game = Some(Mutex::from(Game::from_fen(fen)?));
        self.insert(id.clone(), game)
            .ok_or(Error::new(ErrorType::BadHandle).with_id(&id))?;
        Ok(())
    }
}

type HashMapIter<'a> = dyn Iterator<Item = (&'a String, &'a Option<Mutex<Game>>)> + 'a;
struct GamesIterator<'a> {
    hashmap_iter: Box<HashMapIter<'a>>,
    is_poisoned: bool,
}

impl<'a> Iterator for GamesIterator<'a> {
    type Item = Result<(String, MutexGuard<'a, Game>), Error>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.is_poisoned {
            return None;
        }

        match self.hashmap_iter.next() {
            None => None,
            Some((_, None)) => self.next(),
            Some((id, Some(mutex))) => match mutex.lock() {
                Ok(lock) => Some(Ok((id.clone(), lock))),
                Err(err) => {
                    self.is_poisoned = true;
                    Some(Err(err.into()))
                }
            },
        }
    }
}

impl<'a> From<&'a InnerState> for GamesIterator<'a> {
    fn from(hashmap: &'a InnerState) -> Self {
        GamesIterator {
            hashmap_iter: Box::from(hashmap.iter()),
            is_poisoned: false,
        }
    }
}

#[cfg(test)]
mod tests {}
