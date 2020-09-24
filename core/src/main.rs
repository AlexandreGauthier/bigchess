mod api;
mod cli_arguments;
mod database;
mod engine;
mod errors;
mod game;
mod state;
mod stdio;

use errors::Error;

use state::StateHandle;
use tokio;

#[tokio::main]
async fn main() {
    let _opts = cli_arguments::parse();
    let state = StateHandle::default();
    let stdio_handler = stdio::handler(state.clone());

    let fatal_error = tokio::select! {
        r1 = stdio_handler => {r1},
    };

    exit_gracefully(fatal_error);
}

// TODO
fn exit_gracefully(result: Result<(), Error>) {
    // Not sure how to shutdown if there's not an error
    // Maybe save current files, etc.
    let fatal_error = api::response_from_error(result.unwrap_err());
    stdio::send_to_stream(fatal_error, &mut std::io::stdout())
}
