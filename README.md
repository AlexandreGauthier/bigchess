# bigchess

**WIP** Experimental chess GUI made made in Electron with a Rust backend.

## Roadmap
- [x] Basic playable chess board
- [ ] PGN-like game tree display
- [ ] UCI engine integration
- [ ] Manage game databases

## Design

Inspired by the [xi-editor](https://github.com/xi-editor/xi-editor), the program's logic is completely decoupled from the GUI. It consists of a backend written in Rust and an electron frontend. These components communicate in JSON through bigchess-core's standard input and output.

## Compiling

TODO
