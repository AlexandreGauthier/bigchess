use crate::errors;
use crate::errors::{Error, ErrorType};
use crate::game::Game;
use crate::json_stdio::{response_from_game, response_from_games, Response};
use std::sync::{Arc, Mutex, MutexGuard, RwLock, RwLockWriteGuard};

type InnerState = Vec<Option<Box<Mutex<Game>>>>;

pub struct StateHandle {
    inner: Arc<RwLock<InnerState>>,
}

impl StateHandle {
    pub fn play(&self, index: usize, from: String, to: String) -> Result<Response, Error> {
        self.game_operation(index, |game| game.play(&from, &to))
    }

    pub fn navigate_back(&self, index: usize, back: u16) -> Result<Response, Error> {
        self.game_operation(index, |game| {
            game.navigate_back(back);
            Ok(())
        })
    }

    pub fn get_all_games(&self) -> Result<Response, Error> {
        // self.state_operation returns a response with all state, so no extra operation is needed
        self.state_operation(|_| Ok(()))
    }

    pub fn new_game_default(&self) -> Result<Response, Error> {
        self.state_operation(|state| {
            state.new_game_default();
            Ok(())
        })
    }

    pub fn new_game_fen(&self, fen: String) -> Result<Response, Error> {
        self.state_operation(|state| {
            state.new_game_fen(fen.clone());
            Ok(())
        })
    }

    /// Applies operation to a specific game located at `index`, responds with an error or with the modified game.
    fn game_operation<C>(&self, index: usize, closure: C) -> Result<Response, Error>
    where
        C: Fn(&mut MutexGuard<Game>) -> Result<(), Error>,
    {
        let read_guard = self.inner.read()?;
        let mut game_guard = read_guard.get_game(index)?;
        closure(&mut game_guard)?;

        Ok(response_from_game(game_guard.get_repr()))
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

        let all_games = guard.all_games().map(|game| game.get_repr());
        Ok(response_from_games(all_games))
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
            .ok_or(errors::empty(ErrorType::BadHandle))?
            .as_ref()
            .ok_or(errors::empty(ErrorType::StaleHandle))?
            .lock()
            .map_err(|_| errors::empty(ErrorType::PoisonedHandle))
    }

    fn all_games(&self) -> GamesIterator {
        GamesIterator {
            target: &self,
            index: 0,
        }
    }

    fn close_game(&mut self, index: usize) -> Result<(), Error> {
        let element = self
            .get_mut(index)
            .ok_or(errors::empty(ErrorType::BadHandle))?;

        match element {
            None => Err(errors::empty(ErrorType::StaleHandle)),
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

fn insert_game(vec: &mut InnerState, mut game: Game) -> usize {
    let index = vec.len();
    game.index = index;
    let element = Some(Box::new(Mutex::new(game)));
    vec.push(element);

    index
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
            Ok(lock) => Some(lock),
            Err(e) => match e.error_type {
                ErrorType::StaleHandle => self.next(),
                _ => None,
            },
        }
    }
}

#[cfg(test)]
mod tests {}
