const path = require('path')
const os = require('os')
const backend = require('./native')

const { app, BrowserWindow } = require('electron')

function createWindow () {
  let win = new BrowserWindow({
    width: 1000,
    height: 800,
    webPreferences: {
      nodeIntegration: true
    }
  })
  
  win.webContents.openDevTools()
  let port = backend.startBackend();
  console.log("Backend server listening at: " + port);

  win.webContents.executeJavaScript("const BACKEND_PORT = " + port+"; initBoardState();");
  win.loadFile('lib/main_window.html');
  win.blur()

}

function startBackend() {
}

app.setName("bigchess")
app.whenReady().then(createWindow)