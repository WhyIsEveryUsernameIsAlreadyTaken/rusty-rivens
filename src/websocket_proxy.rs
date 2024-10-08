use std::{io, str::FromStr};

use ascii::AsciiString;
use tiny_http::{Header, Request, Response};


pub fn uri_rivens(rq: Request) -> io::Result<()> {
    if let Some(connection_header) = rq.headers().iter().find(|&h| h.field.equiv("Connection")) {
        let upgrade_value = if let Ok(upgrade) =  AsciiString::from_str("upgrade") {
            upgrade
        } else {
            return rq.respond(Response::empty(500));
        };
        if !connection_header.value.eq_ignore_ascii_case(&upgrade_value) {
            return rq.respond(Response::empty(426));
        };
    }
    let (wsoc_key, wsoc_ver) = if let Some(upgrade_header) = rq.headers().iter().find(|&h| h.field.equiv("Upgrade")) {
        let protocol = if let Ok(p) =  AsciiString::from_str("websocket") {
            p
        } else {
            return rq.respond(Response::empty(500));
        };
        if !upgrade_header.value.eq_ignore_ascii_case(&protocol) {
            return rq.respond(Response::empty(426));
        };
        let connection_header = if let Ok(h) = Header::from_str("Connecton: upgrade") {
            h
        } else {
            return rq.respond(Response::empty(500));
        };
        let upgrade_header = if let Ok(h) = Header::from_str("Upgrade: websocket") {
            h
        } else {
            return rq.respond(Response::empty(500));
        };
        let wsoc_key = if let Some(wsoc_key_header) = rq.headers().iter().find(|&h| h.field.equiv("Sec-WebSocket-Key")) {
            wsoc_key_header.value
        } else {
            todo!()
        };
        let wsoc_key = if let Some(wsoc_key_header) = rq.headers().iter().find(|&h| h.field.equiv("Sec-WebSocket-Key")) {
            wsoc_key_header.value
        } else {
            todo!()
        };
        rq.respond(Response::empty(101)
            .with_header(connection_header)
            .with_header(upgrade_header)
        )?;
        (wsoc_key, wsoc_key)
    } else {
        todo!()
    };
    todo!()
}
