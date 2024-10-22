use core::panic;
use std::{io::{self, Write}, net::TcpListener, sync::Arc};

use maud::html;
use tungstenite::{handshake::{machine::{HandshakeMachine, StageResult}, server::NoCallback, MidHandshake}, protocol::Role, HandshakeError, ServerHandshake};
use crate::pages::home::rivens;

pub fn start_websocket() {
    let server = TcpListener::bind("localhost:8069").expect("could not bind to port: ");
    loop {
        let (stream, _addr) = server.accept().expect("could not accept connection");

        let mut wsoc_connection = tungstenite::accept(stream).expect("this should accept");
        println!("handshake complete");
        let rivens = rivens();
        let msg = html! {
            div id="riven-table" class="row" hx-swap-oob="beforeend" {
                (rivens)
            }
        };
        wsoc_connection.send(msg.into_string().into()).unwrap();
        wsoc_connection.close(None).unwrap();
    }
}
