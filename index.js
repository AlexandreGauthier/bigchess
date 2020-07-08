const path = require('path')
const os = require('os')

const { app, BrowserWindow } = require('electron')


function createWindow () {
  let win = new BrowserWindow({
    width: 1000,
    height: 600,
    webPreferences: {
      nodeIntegration: true
    }
  })

  // and load the index.html of the app.
  win.loadFile('lib/main_window.html')
}

app.whenReady().then(createWindow)