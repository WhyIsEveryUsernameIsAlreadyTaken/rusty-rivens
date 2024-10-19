use std::str::FromStr;

use ascii::AsciiString;
use http_body_util::Full;
use hyper::{body::Bytes, header::{HeaderValue, CONNECTION, SEC_WEBSOCKET_ACCEPT, SEC_WEBSOCKET_KEY, SEC_WEBSOCKET_VERSION, UPGRADE}, HeaderMap, Response, StatusCode};
use openssl::{base64, sha::sha1};

use crate::server::{full, BoxBody};


pub fn uri_rivens(headers: &HeaderMap) -> Response<BoxBody> {
    if headers.contains_key(CONNECTION) {
        let connection_header = headers[CONNECTION].clone();
        if connection_header.as_bytes() != "upgrade".as_bytes() {
            println!("Bad Upgrade header value: {:?}", connection_header.to_str());
            let mut res = Response::new(full(""));
            *res.status_mut() = StatusCode::BAD_REQUEST;
            return res;
        };
    }
    let (wsoc_key, _) = if headers.contains_key(UPGRADE) {
        let upgrade_header = headers[UPGRADE].clone();
        if upgrade_header.as_bytes() != "websocket".as_bytes() {
            println!("Bad protocol: {:?}", upgrade_header.to_str());
            let mut res = Response::new(full(""));
            *res.status_mut() = StatusCode::BAD_REQUEST;
            return res;
        };
        let wsoc_key = if headers.contains_key(SEC_WEBSOCKET_KEY) {
            headers[SEC_WEBSOCKET_KEY].clone()
        } else {
            let mut res = Response::new(full(""));
            *res.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
            return res;
        };
        let wsoc_ver = if headers.contains_key(SEC_WEBSOCKET_VERSION) {
            headers[SEC_WEBSOCKET_VERSION].clone()
        } else {
            let mut res = Response::new(full(""));
            *res.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
            return res;
        };
        (wsoc_key, wsoc_ver)
    } else {
            let mut res = Response::new(full(""));
            *res.status_mut() = StatusCode::BAD_REQUEST;
            return res;
    };

    let connection_header = "Upgrade".parse::<HeaderValue>().unwrap();
    let upgrade_header = "websocket".parse::<HeaderValue>().unwrap();
    let key = [wsoc_key.as_bytes(), b"258EAFA5-E914-47DA-95CA-C5AB0DC85B11"].concat();
    let key = sha1(&key);
    let key = base64::encode_block(&key);

    let wsoc_accept_header = format!("{key}").parse::<HeaderValue>().unwrap();

    let res = Response::builder()
        .header(CONNECTION, connection_header)
        .header(UPGRADE, upgrade_header)
        .header(SEC_WEBSOCKET_ACCEPT, wsoc_accept_header)
        .status(101)
        .body(full(""));
    match res {
        Ok(v) => v,
        Err(_) => {
            let mut res = Response::new(full(""));
            *res.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
            res
        },
    };
    todo!()
}
