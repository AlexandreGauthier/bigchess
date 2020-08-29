const {Chessground} = require('chessground')
const backend = require("./js/backend.js");

let audioPath = "./res/sounds/"
let audio = {
    move: new Audio(audioPath+"move.wav"),
    moveCheck: new Audio(audioPath+"move_check.wav"),
    takes: new Audio(audioPath+"takes.wav"),
    takesCheck: new Audio(audioPath+"takes_check.wav"),
    gameOver: new Audio(audioPath+"game_end.wav"),
}

var chessgroundConfig = {
    animation: {
        enabled: true,
        duration: 150,
    },
    movable: {
        free: false,
    },
    events: {
        move: boarduiMoveEvent
    },
}

const boardui = Chessground(document.getElementById("mainboard"), chessgroundConfig);

function objectToMap(object) {
    let map = new Map;
    for (key in object) {
        map.set(key, object[key]);
    }
    return map;
}

function boarduiMoveEvent(from, to) {
    backend.playMove(from, to, BACKEND_PORT, reloadui);
}

function moveBackOne() {
    backend.navigateBack(1, BACKEND_PORT, reloadui);
}

function initBoardState() {
    backend.getState(BACKEND_PORT, loadui);
}

function reloadui(response) {
    playMoveSound(response.is_check, response.is_takes);
    loadui(response)
}

function loadui(response) {
    let dests = objectToMap(response.available_moves);
    let fen = response.fen;
    boardui.cancelPremove();
    boardui.set({
        check: response.is_check,
        fen: fen,
        movable: {dests: dests}
    });
}

function playMoveSound(isCheck, isTakes) {
    let sound;
    if (isCheck && isTakes) {
        sound = audio.takesCheck;
    } else if (isCheck) {
        sound = audio.moveCheck;
    } else  if (isTakes){
        sound = audio.takes;
    } else {
        sound = audio.move;
    }

    sound.play()
}

window.ondragstart = function() { return false; } 

// Electron injects :
//    const BACKEND_PORT = {Ephemeral port at which the backend is listening};
//    initBoardState();
// after page load.

