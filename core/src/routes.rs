use crate::state::StateHandle;
use crate::errors::Error;

use serde::Serialize;
use warp::http::StatusCode;
use warp::{filters::BoxedFilter, Filter};

pub type WarpReply = warp::reply::WithStatus<warp::reply::Json>;

// TODO: Transform to json-based api
pub fn config(state_handle: StateHandle) -> BoxedFilter<(impl warp::Reply,)> {
    let _state_handle = state_handle.clone();
    let play = warp::path!("play" / usize / String / String)
        .map(move |index, from, to| route_play(_state_handle.clone(), index, from, to));

    let _state_handle = state_handle.clone();
    let state = warp::path("state")
        .map(move || route_state(_state_handle.clone()));
    
    let _state_handle= state_handle.clone();
    let navigate_back = warp::path!("back" / usize /  u16)
        .map(move |index: usize, back: u16| route_navigate_back(_state_handle.clone(), index, back));

    (play.or(state).or(navigate_back)).and(warp::post()).boxed()
}

fn route_play(state: StateHandle, index: usize, from: String, to: String) -> WarpReply {
    let result = state.play(index, from, to);
    result_to_warp_reply(result)
}

fn route_navigate_back(state: StateHandle, index: usize, back: u16) -> WarpReply {
    let result = state.navigate_back(index, back);
    result_to_warp_reply(result)
}

fn route_state(state: StateHandle) -> WarpReply {
    let result = state.get_all_games();
    result_to_warp_reply(result)
}

fn ok_status(reply: warp::reply::Json) -> WarpReply {
    let ok = StatusCode::OK;
    warp::reply::with_status(reply, ok)
}

fn result_to_warp_reply(result: Result<impl Serialize, Error>) -> WarpReply {
    match result {
        Err(e) => e.into_warp_reply(),
        Ok(json_response) => into_warp_reply(&json_response)
    }
}

fn into_warp_reply(json: impl Serialize) -> WarpReply {
    ok_status(warp::reply::json(&json))
}
