use ascii::AsciiString;
use async_lock::Mutex;
use once_cell::sync::OnceCell;
use std::{io::Cursor, sync::{Arc, Mutex as StdMutex}, thread::{self, JoinHandle}};
use tiny_http::{Header, Response};

use crate::{http_client::{auth_state::AuthState, wfm_client::WFMClient}, pages::{home::{uri_forbidden, uri_home, uri_main, uri_not_found}, login::{uri_login, uri_login_req}}, resources::{uri_htmx, uri_logo, uri_styles}, AppError, STOPPED};

#[derive(Debug)]
struct User(Option<AsciiString>);
pub enum CurrentScreen {
    Login,
}

static USER: OnceCell<User> = OnceCell::new();
const ARRAY_REPEAT_VALUE: Option<JoinHandle<Result<(), AppError>>> = None;
pub static LOGGED_IN: OnceCell<bool> = OnceCell::new();

pub(crate) fn start_server() -> Result<(), AppError> {
    let s = tiny_http::Server::http("127.0.0.1:8000").unwrap();
    let current_screen = Arc::new(StdMutex::new(CurrentScreen::Login));
    println!("SERVER STARTED");
    if STOPPED.get().is_some() {
        return Ok(());
    }

    let auth_state = AuthState::setup().map_err(|e| e.prop("start_server".into()))?;

    let wfm_client = WFMClient::new(auth_state);
    let wfm_client = Arc::new(Mutex::new(wfm_client));


    let rq = s.recv().unwrap();
    let head = rq.headers().iter().find(|&v| v.field.equiv("User-Agent"));
    let head = head.unwrap().value.clone();
    USER.set(User(Some(head))).unwrap();
    // println!("received request! method: {:?}, url: {:?}",
    //     rq.method(),
    //     rq.url(),
    // );
    let rs = match_uri(
        rq.url(),
        None,
        rq.headers(),
        wfm_client.clone()).map_err(|e| e.prop("start_server".into()))?;
    rq.respond(rs).unwrap();

    loop {
        if let Some(mut rq) = s.try_recv().unwrap() {
            // println!("received request! method: {:?}, url: {:?}",
            //     rq.method(),
            //     rq.url(),
            // );
            if let User(Some(u)) = USER.get().unwrap() {
                if &rq.headers().iter().find(|&v| v.field.equiv("User-Agent")).unwrap().value != u {
                    let r = uri_forbidden();
                    rq.respond(r).unwrap();
                    continue;
                }
            }
            let mut body = String::new();
            rq.as_reader().read_to_string(&mut body).unwrap();
            let rs = match_uri(
                rq.url(),
                Some(body.as_str()),
                rq.headers(),
                wfm_client.clone(),
            ).map_err(|e| e.prop("start_server: spawn".into()))?;
            rq.respond(rs).unwrap();
        } else {
            if STOPPED.get() == Some(&true) {
                break;
            }
            continue;
        };
    }
    println!("SERVER CLOSED");
    Ok(())
}

fn match_uri(
    uri: &str,
    body: Option<&str>,
    _headers: &[Header],
    wfm: Arc<Mutex<WFMClient>>,
) -> Result<Response<Cursor<Vec<u8>>>, AppError> {
    match uri {
        "" | "/" => {
            uri_main(wfm)
        }
        "/htmx.min.js" => {
            Ok(uri_htmx())
        }
        "/styles.css" => {
            Ok(uri_styles())
        }
        "/api/login" => {
            uri_login_req(body.unwrap(), wfm.clone())
        }
        "/login" => {
            Ok(uri_login())
        }
        "/home" => {
            Ok(uri_home())
        }
        "/logo.svg" => {
            Ok(uri_logo())
        }
        _ => {
            Ok(uri_not_found())
        }
    }
}
