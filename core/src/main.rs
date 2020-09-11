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
fn exit_gracefully(result: Result<(), Error>) {
    // Not sure how to shutdown if there's not an error
    // Maybe save current files, etc.
    let fatal_error = json_stdio::response_from_error(result.unwrap_err());
    json_stdio::send_to_stream(fatal_error, &mut std::io::stdout())
}
