pub mod database;
pub mod errors;
pub mod game;
pub mod routes;

use std::net::SocketAddrV4;
use std::sync::mpsc;
use std::thread;

pub fn start() -> u16 {
    let (send_bound_port, recv_bound_port) = mpsc::channel::<u16>();
    thread::spawn(move || bind_server(send_bound_port));
    recv_bound_port.recv().unwrap()
}

#[tokio::main]
async fn bind_server(send_port: mpsc::Sender<u16>) {
    let game_state = game::StateHandle::default();
    let routes = routes::config(game_state);
    let address = "127.0.0.1:0".parse::<SocketAddrV4>().unwrap();

    let (address, server) = warp::serve(routes).bind_ephemeral(address);

    send_port.send(address.port()).unwrap();
    server.await;
}
