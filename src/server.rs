use ascii::AsciiString;
use dotenv::dotenv;
use once_cell::sync::OnceCell;
use tokio::sync::Mutex;
use std::{
    fs, io::{self}, sync::Arc, thread, time::SystemTime
};
use tiny_http::Request;

use crate::{
    api_operations::{uri_api_delete_riven, uri_api_login, uri_api_update_riven}, http_client::{auth_state::AuthState, qf_client::QFClient, wfm_client::WFMClient}, pages::{
        home::{uri_edit_cancel, uri_edit_open, uri_home, uri_main, uri_not_found, uri_unauthorized},
        login::uri_login,
    }, resources::{uri_htmx, uri_logo, uri_styles, uri_wfmlogo}, rivens::inventory::{database::InventoryDB, riven_lookop::RivenDataLookup}, websocket::start_websocket, AppError, STOPPED
};

#[derive(Debug)]
struct User(Option<AsciiString>);

static USER: OnceCell<User> = OnceCell::new();

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

pub async fn start_server() -> Result<(), AppError> {
    dotenv().expect("bleh");
    let s = tiny_http::Server::http("127.0.0.1:8000").unwrap();
    let mut logged_in: Option<bool> = None;
    println!("SERVER STARTED");
    if STOPPED.get().is_some() {
        return Ok(());
    }

    let auth_state = AuthState::setup().map_err(|e| e.prop("start_server".into()))?;
    let auth_state = Arc::new(Mutex::new(auth_state));

    let wfm_client = WFMClient::new(auth_state.clone());
    let wfm_client = Arc::new(Mutex::new(wfm_client));

    let qf_client = QFClient::new(auth_state.clone());
    let qf_client = Arc::new(Mutex::new(qf_client));

    let rq = s.recv().unwrap();
    let head = rq.headers().iter().find(|&v| v.field.equiv("User-Agent"));
    let head = head.unwrap().value.clone();
    USER.set(User(Some(head))).unwrap();
    println!(
        "received request! method: {:?}, url: {:?}",
        rq.method(),
        rq.url(),
    );
    let uri = rq.url().to_owned();
    handle_request(
        rq,
        uri.as_str(),
        wfm_client.clone(),
        qf_client.clone(),
        None,
        &mut logged_in,
    )
    .map_err(|e| e.prop("start_server".into()))?;

    let mut last_modified = LastModified(SystemTime::UNIX_EPOCH, SystemTime::UNIX_EPOCH);
    let db = InventoryDB::open("inventory.sqlite3")
        .map_err(|e| AppError::new(e.to_string(), "start_server: InventoryDB::open".to_string()))?;

    let db = Arc::new(Mutex::new(db));
    let lookup = Arc::new(RivenDataLookup::setup().await.expect(
        "FATAL: Could not retrieve riven lookup data"
    ));
    thread::spawn(|| start_websocket());

    let _ = tokio_rustls::rustls::crypto::ring::default_provider().install_default();

    loop {
        if let Some(mut rq) = s.try_recv().unwrap() {
            println!(
                "received request! method: {:?}, url: {:?}",
                rq.method(),
                rq.url(),
            );
            if let User(Some(u)) = USER.get().unwrap() {
                if &rq
                    .headers()
                    .iter()
                    .find(|&v| v.field.equiv("User-Agent"))
                    .unwrap()
                    .value
                    != u
                {
                    uri_unauthorized(rq).unwrap();
                    continue;
                }
            }
            let mut body = String::new();
            rq.as_reader().read_to_string(&mut body).unwrap();
            let uri = rq.url().to_owned();
            handle_request(
                rq,
                uri.as_str(),
                wfm_client.clone(),
                qf_client.clone(),
                Some(body.as_str()),
                &mut logged_in,
            )
            .map_err(|e| e.prop("start_server: spawn".into()))?;
        } else {
            if STOPPED.get() == Some(&true) {
                break;
            }
            continue;
        };

        // if last_modified.detect_file_change()
        //     .map_err(|e|
        //         AppError::new(
        //             e.to_string(),
        //             "start_server: detect_file_change".to_string()
        //         )
        // )? {
        //     smolscale::block_on({
        //         let lookup = lookup.clone();
        //         let db = db.clone();
        //         async move {
        //             sync_db(db, &lookup, None).await.unwrap()
        //         }
        //     });
        // }
    }
    println!("SERVER CLOSED");
    Ok(())
}

fn handle_request(
    rq: Request,
    uri: &str,
    wfm: Arc<Mutex<WFMClient>>,
    qf: Arc<Mutex<QFClient>>,
    body: Option<&str>,
    logged_in: &mut Option<bool>,
) -> Result<(), AppError> {
    let (root, other) = uri[1..].split_once('/').unwrap_or((&uri[1..], ""));
    let (root, _) = root.split_once('?').unwrap_or((root, ""));
    match root {
        "" | "/" => uri_main(rq, wfm, logged_in).map_err(|e| e.prop("handle_request".into())),
        "htmx.min.js" => {
            uri_htmx(rq).map_err(|e| AppError::new(e.to_string(), "handle_request".to_string()))
        }
        "styles.css" => {
            uri_styles(rq).map_err(|e| AppError::new(e.to_string(), "handle_request".to_string()))
        }
        "api" => match_uri_api(rq, other, body, wfm, qf, logged_in)
            .map_err(|e| e.prop("handle_request".into())),
        "login" => {
            uri_login(rq).map_err(|e| AppError::new(e.to_string(), "handle_request".to_string()))
        }
        "home" => {
            uri_home(rq).map_err(|e| AppError::new(e.to_string(), "handle_request".to_string()))
        }
        "edit_open" => uri_edit_open(rq, other)
            .map_err(|e| AppError::new(e.to_string(), "handle_request".to_string())),
        "edit_cancel" => uri_edit_cancel(rq)
            .map_err(|e| AppError::new(e.to_string(), "handle_request".to_string())),
        "logo.svg" => {
            uri_logo(rq).map_err(|e| AppError::new(e.to_string(), "handle_request".to_string()))
        }
        "wfm_favicon.ico" => {
            uri_wfmlogo(rq).map_err(|e| AppError::new(e.to_string(), "handle_request".to_string()))
        }
        _ => uri_not_found(rq)
            .map_err(|e| AppError::new(e.to_string(), "handle_request".to_string())),
    }
}

fn match_uri_api(
    rq: Request,
    uri: &str,
    body: Option<&str>,
    wfm: Arc<Mutex<WFMClient>>,
    qf: Arc<Mutex<QFClient>>,
    logged_in: &mut Option<bool>,
) -> Result<(), AppError> {
    let (root, other) = uri.split_once('/').unwrap_or((uri, ""));
    match root {
        "login" => uri_api_login(rq, body.unwrap(), wfm.clone(), qf, logged_in)
            .map_err(|e| e.prop("match_uri_api".into())),
        "delete_riven" => {
            uri_api_delete_riven(rq, other).map_err(|e| e.prop("match_uri_api".into()))
        }
        "update_riven" => {
            uri_api_update_riven(rq, other, body).map_err(|e| e.prop("match_uri_api".into()))
        }
        _ => uri_not_found(rq)
            .map_err(|e| AppError::new(e.to_string(), "handle_request".to_string())),
    }
}
