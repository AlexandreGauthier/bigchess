const {Net} = require("electron")

function localhostRequest(port, path, callback) {
    let url = `http://127.0.0.1:${port}${path}`;
    let Http = new XMLHttpRequest();
    Http.onload = function() {
        let json = JSON.parse(this.response) 
        if (json.code != 200) {
            console.error(`Recieved code ${json.code} on request to backend: ${url}`);
            console.error(json);
        } else {
            callback(json);
        }
    };
    Http.open("POST", url);
    Http.send();
}

function playMove(from, to, port, callback) {
    let path = `/play/${from}/${to}`;
    localhostRequest(port, path, callback);
}

function getState(port, callback) {
    let path = "/state";
    localhostRequest(port, path, callback);
}

function navigateBack(n, port, callback) {
    let path = `/back/${n}`;
    localhostRequest(port, path, callback);
}

exports.playMove = playMove;
exports.getState = getState;
exports.navigateBack = navigateBack;