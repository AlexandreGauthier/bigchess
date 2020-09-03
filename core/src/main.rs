mod cli_arguments;
mod database;
mod engine;
mod errors;
mod game;
mod json_stdio;
mod state;

use errors::Error;

use state::StateHandle;
use tokio;

#[tokio::main]
async fn main() {
    let _opts = cli_arguments::parse();
    let state = StateHandle::default();
    let json_handler = json_stdio::handler(state.clone());

    let fatal_error = tokio::select! {
        r1 = json_handler => {r1},
    };

    exit_gracefully(fatal_error);
}

// TODO
fn exit_gracefully(_result: Result<(), Error>) {
    unimplemented!()
}
