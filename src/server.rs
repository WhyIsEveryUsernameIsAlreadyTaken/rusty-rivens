use ascii::AsciiString;
use dotenv::dotenv;
use once_cell::sync::OnceCell;
use std::{ops::DerefMut, sync::Arc, thread};
use tiny_http::{Request, Server};
use tokio::{select, sync::{broadcast::{self, Receiver}, Mutex}};

use crate::{
    api_operations::{uri_api_blacklist_riven, uri_api_delete_riven, uri_api_login, uri_api_update_riven}, http_client::{auth_state::AuthState, qf_client::QFClient, wfm_client::WFMClient}, pages::{
        home::{
            uri_edit_cancel, uri_edit_open, uri_home, uri_main, uri_not_found, uri_unauthorized,
        },
        login::uri_login,
    }, resources::{uri_htmx, uri_logo, uri_styles, uri_wfmlogo}, rivens::inventory::riven_lookop::RivenDataLookup, websocket::start_websocket, AppError, StopSignal
};

#[derive(Debug)]
struct User(AsciiString);

static USER: OnceCell<User> = OnceCell::new();
pub static RIVEN_LOOKUP: OnceCell<RivenDataLookup> = OnceCell::new();

async fn recv_request(server: &Server) -> tiny_http::Request {
    server.recv().unwrap()
}

struct ServerState<'a> {
    server: &'a Server,
    stop_receiver: Receiver<StopSignal>,
    wfm_client: Arc<Mutex<WFMClient>>,
    qf_client: Arc<Mutex<QFClient>>,
    logged_in: Option<bool>,
}

pub async fn start_server(stop_receiver: Receiver<StopSignal>) -> Result<(), AppError> {
    dotenv().expect("FATAL: Could not load envvars from `.env`");
    let server = tiny_http::Server::http("127.0.0.1:8000").unwrap();
    let logged_in: Option<bool> = None;
    println!("SERVER STARTED");

    let auth_state = AuthState::setup().map_err(|e| e.prop("start_server".into()))?;
    let auth_state = Arc::new(Mutex::new(auth_state));

    let wfm_client = WFMClient::new(auth_state.clone(), stop_receiver.resubscribe());
    let wfm_client = Arc::new(Mutex::new(wfm_client));

    let qf_client = QFClient::new(auth_state, stop_receiver.resubscribe());
    let qf_client = Arc::new(Mutex::new(qf_client));

    let receiver_clone = stop_receiver.resubscribe();
    let server_state = ServerState {
        server: &server,
        stop_receiver,
        wfm_client,
        qf_client,
        logged_in,
    };
    let server_state = Arc::new(Mutex::new(server_state));

    handle_request(server_state.clone(), false).await.map_err(|e| e.prop("start_server".into()))?;

    thread::spawn(move || start_websocket(receiver_clone));

    let _ = tokio_rustls::rustls::crypto::ring::default_provider().install_default();

    handle_request(server_state, true).await.map_err(|e| e.prop("start_server".into()))?;
    Ok(())
}

async fn handle_request<'a>(server_state: Arc<Mutex<ServerState<'a>>>, main_loop: bool) -> Result<(), AppError> {
    let mut server_state_mutex = server_state.lock().await;
    let server_state = server_state_mutex.deref_mut();
    while main_loop || server_state.logged_in.is_none() {
        select! {
            mut rq = recv_request(server_state.server) => {
                println!(
                    "received request! method: {:?}, url: {:?}",
                    rq.method(),
                    rq.url(),
                );
                if let Some(User(user)) = USER.get() {
                    if &rq
                        .headers()
                        .iter()
                        .find(|&v| v.field.equiv("User-Agent"))
                        .unwrap()
                    .value
                    != user
                    {
                        uri_unauthorized(rq).unwrap();
                        continue;
                    }
                } else {
                    let head = rq.headers().iter().find(|&v| v.field.equiv("User-Agent"));
                    let head = head.unwrap().value.clone();
                    USER.set(User(head)).unwrap();
                }
                let mut body = String::new();
                rq.as_reader().read_to_string(&mut body).unwrap();
                let uri = rq.url().to_owned();
                match_request(
                    rq,
                    uri.as_str(),
                    server_state.wfm_client.clone(),
                    server_state.qf_client.clone(),
                    Some(body.as_str()),
                    &mut server_state.logged_in,
                )
                    .map_err(|e| e.prop("start_server: spawn".into()))?;
            }
                _ = server_state.stop_receiver.recv() => {println!("SERVER CLOSED"); break;}
        }
    }
    Ok(())
}

fn match_request(
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
        "" | "/" => uri_main(rq, wfm, qf, logged_in).map_err(|e| e.prop("handle_request".into())),
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
    let (other, _) = other.split_once('?').unwrap_or((other, ""));
    match root {
        "login" => uri_api_login(rq, body.unwrap(), wfm.clone(), qf, logged_in)
            .map_err(|e| e.prop("match_uri_api".into())),
        "delete_riven" => {
            uri_api_delete_riven(rq, other).map_err(|e| e.prop("match_uri_api".into()))
        }
        "blacklist_riven" => {
            uri_api_blacklist_riven(rq, other).map_err(|e| e.prop("match_uri_api".into()))
        }
        "update_single_riven" => {
            uri_api_update_riven(rq, false, other, body).map_err(|e| e.prop("match_uri_api".into()))
        }
        "update_mult_riven" => {
            uri_api_update_riven(rq, true, other, body).map_err(|e| e.prop("match_uri_api".into()))
        }
        _ => uri_not_found(rq)
            .map_err(|e| AppError::new(e.to_string(), "handle_request".to_string())),
    }
}
