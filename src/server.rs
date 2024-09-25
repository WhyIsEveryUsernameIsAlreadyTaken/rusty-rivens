use ascii::AsciiString;
use async_lock::Mutex;
use once_cell::sync::OnceCell;
use std::{fs, io::{self, Cursor}, sync::{Arc, Mutex as StdMutex}, thread::JoinHandle, time::SystemTime};
use tiny_http::{Header, Response};

use crate::{http_client::{auth_state::AuthState, wfm_client::WFMClient}, pages::{home::{uri_home, uri_main, uri_not_found, uri_unauthorized}, login::{uri_login, uri_login_req}}, resources::{uri_htmx, uri_logo, uri_styles}, rivens::inventory::{database::InventoryDB, inventory_sync::sync_db, riven_lookop::RivenDataLookup}, AppError, STOPPED};

#[derive(Debug)]
struct User(Option<AsciiString>);
pub enum CurrentScreen {
    Login,
}

static USER: OnceCell<User> = OnceCell::new();
pub static LOGGED_IN: OnceCell<bool> = OnceCell::new();

struct LastModified(SystemTime, SystemTime);

impl LastModified {
    fn detect_file_change(&mut self) -> io::Result<bool> {
        let attrs = fs::metadata("dummy.txt")?;
        self.1 = attrs.modified().unwrap();
        if self.1 != self.0 {
            self.0 = self.1;
            return Ok(true);
        }
        Ok(false)
    }
}

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

    let mut last_modified = LastModified(SystemTime::UNIX_EPOCH, SystemTime::UNIX_EPOCH);
    let db = InventoryDB::open("inventory.sqlite")
        .map_err(
            |e| AppError::new(e.to_string(), "start_server: InventoryDB::open".to_string())
        )?;

    let db = Arc::new(Mutex::new(db));
    let lookup = Arc::new(RivenDataLookup::setup().unwrap());

    loop {
        if let Some(mut rq) = s.try_recv().unwrap() {
            // println!("received request! method: {:?}, url: {:?}",
            //     rq.method(),
            //     rq.url(),
            // );
            if let User(Some(u)) = USER.get().unwrap() {
                if &rq.headers().iter().find(|&v| v.field.equiv("User-Agent")).unwrap().value != u {
                    let r = uri_unauthorized();
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

        if last_modified.detect_file_change()
            .map_err(|e|
                AppError::new(
                    e.to_string(),
                    "start_server: detect_file_change".to_string()
                )
        )? {
            smolscale::block_on({
                let lookup = lookup.clone();
                let db = db.clone();
                async move {
                    sync_db(db, &lookup, None).await.unwrap()
                }
            });
        }
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

