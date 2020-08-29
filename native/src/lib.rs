mod backend;

use neon::prelude::*;

fn js_start_backend(mut cx: FunctionContext) -> JsResult<JsNumber> {
    let port = backend::start();
    Ok(cx.number(port))
}

register_module!(mut cx, {
    let _ = cx.export_function("startBackend", js_start_backend);
    Ok(())
});
