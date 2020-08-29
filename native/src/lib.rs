pub mod database;
pub mod state;
pub mod errors;
pub mod game;
pub mod routes;
pub mod backend_thread;

use neon::prelude::*;

fn js_start_backend(mut cx: FunctionContext) -> JsResult<JsNumber> {
    let port = backend_thread::start();
    Ok(cx.number(port))
}

register_module!(mut cx, {
    let _ = cx.export_function("startBackend", js_start_backend);
    Ok(())
});
