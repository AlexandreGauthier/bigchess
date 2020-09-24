use crate::api::*;
use crate::errors::Error;
use crate::state::StateHandle;

use std::fmt::Debug;
use std::io::Write;

use serde_json;
use tokio::io;
use tokio::io::{AsyncBufReadExt, BufReader};

pub async fn handler(state: StateHandle) -> Result<(), Error> {
    let mut stdout = std::io::stdout();
    let stdin = io::stdin();
    let mut stdin_lines = BufReader::new(stdin).lines();

    send_initial_message(&state, &mut stdout)?;

    loop {
        let new_line = stdin_lines.next_line().await?.unwrap_or_default();
        let response = dispatch(&*new_line, &state)?;
        send_to_stream(response, &mut stdout);
    }
}

fn send_initial_message<W: Write + Debug>(
    state: &StateHandle,
    stream: &mut W,
) -> Result<(), Error> {
    let message = state.get_all_games()?;
    send_to_stream(message, stream);
    Ok(())
}

/// Only returns Err(Error) when it is not recoverable
/// All other errors are returned in the form of Ok(Response)
fn dispatch(line: &str, state: &StateHandle) -> Result<Response, Error> {
    match serde_json::from_str(line) {
        Ok(request) => dispatch_request(request, state),
        Err(err) => return Ok(response_from_error(err.into())),
    }
}

pub fn send_to_stream<W: Write + Debug>(response: Response, mut stream: W) {
    serde_json::to_writer(&mut stream, &response)
        .expect("Unrecoverable error: could not serialize response object to stdout.");

    write!(&mut stream, "\n")
        .and_then(|_| stream.flush())
        .expect(&*format!(
            "Unrecoverable error: could not write to stream {:?}",
            stream,
        ));
}
