use neon::prelude::*;

fn talk_to_me(mut cx: FunctionContext) -> JsResult<JsNumber> {
        let x = cx.argument::<JsNumber>(0)?.value();
        Ok(cx.number(x*(100 as f64)))
}

register_module!(mut cx, {
    cx.export_function("talkToMe", talk_to_me)
});

enum PieceType {
        King,
        Queen,
        Rook,
        Bishop,
        Knight,
        Pawn
}

enum PieceColor {
        White,
        Black
}

struct Piece {
        ptype: PieceType,
        color: PieceColor
}