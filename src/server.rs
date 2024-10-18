use hyper::body::{Bytes, Incoming};
use tokio::{net::TcpListener, runtime::Handle, sync::Mutex};
use std::{fs, future::Future, io, net::SocketAddr, pin::Pin, sync::Arc, time::SystemTime};
use http_body_util::{BodyExt, Full};
use hyper::server::conn::http2;
use hyper::service::Service;
use hyper::{Request, Response};

use hyper_util::rt::{TokioExecutor, TokioIo};

use crate::{
    api_operations::{uri_api_delete_riven, uri_api_login}, http_client::{
        auth_state::AuthState, wfm_client::WFMClient
    }, pages::{
        home::{
            uri_edit, uri_home, uri_main, uri_not_found
        },
        login::uri_login
    }, resources::{
        uri_htmx,
        uri_logo,
        uri_styles,
        uri_wfmlogo
    }, rivens::inventory::database::InventoryDB, websocket_proxy::uri_rivens, AppError
};

#[derive(Debug, Clone)]
struct ServerState {
    wfm: Arc<Mutex<WFMClient>>,
    edit_toggle: Arc<Mutex<bool>>,
    logged_in: Arc<Mutex<Option<bool>>>,
}

impl Service<Request<Incoming>> for ServerState {
    type Response = Response<Full<Bytes>>;
    type Error = hyper::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn call(&self, mut req: Request<Incoming>) -> Self::Future {
        let data = {
            tokio::task::block_in_place(|| {
                Handle::current().block_on(async {
                    req.body_mut().collect().await.map_err(|e| AppError::new(e.to_string(), "handle_request".to_string()))
                })
            })
        }.unwrap();
        let data = data.to_bytes();

        let uri = req.uri().path();
        let (root, other) = uri[1..].split_once('/').unwrap_or((&uri[1..], ""));

        let body = String::from_utf8(data.into()).map_err(|e| AppError::new(e.to_string(), "handle_request".to_string())).unwrap();
        let body = if body.len() != 0 {
            Some(body.as_str())
        } else {
            None
        };
        let res = match root {
            "" | "/" => {
                uri_main(self.wfm.clone(), self.logged_in.clone()).unwrap()
            }
            "htmx.min.js" => {
                uri_htmx().unwrap()
            }
            "styles.css" => {
                uri_styles().unwrap()
            }
            "api" => {
                match_uri_api(other, body, self.wfm.clone(), self.logged_in.clone()).unwrap()
            }
            "login" => {
                uri_login()
            }
            "home" => {
                uri_home()
            }
            "edit" => {
                uri_edit(self.edit_toggle.clone())
            }
            "logo.svg" => {
                uri_logo().unwrap()
            }
            "wfm_favicon.ico" => {
                uri_wfmlogo().unwrap()
            }
            "rivens" => {
                uri_rivens(req.headers())
            }
            _ => {
                uri_not_found()
            }
        };
        Box::pin(async { Ok(res) })
    }
}

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

pub async fn start_server() {
    let edit_toggle = false;
    let logged_in: Option<bool> = None;
    println!("SERVER STARTED");

    let auth_state = AuthState::setup().unwrap();
    let wfm_client = WFMClient::new(auth_state);
    let wfm_client = Arc::new(Mutex::new(wfm_client));

    let mut last_modified = LastModified(SystemTime::UNIX_EPOCH, SystemTime::UNIX_EPOCH);
    let db = InventoryDB::open("inventory.sqlite3").unwrap();

    let db = Arc::new(Mutex::new(db));
    // let lookup = Arc::new(RivenDataLookup::setup().unwrap());

    let server_state = ServerState {
        wfm: wfm_client,
        edit_toggle: Arc::new(Mutex::new(edit_toggle)),
        logged_in: Arc::new(Mutex::new(logged_in)),
    };
    let addr: SocketAddr = ([127, 0, 0, 1], 3000).into();

    let listener = TcpListener::bind(addr).await.unwrap();
    println!("Listening on http://{}", addr);

    loop {
        let (stream, _) = listener.accept().await.unwrap();
        println!("connection accepted");
        let io = TokioIo::new(stream);
        let server_state = server_state.clone();
        tokio::task::spawn(async move {
            if let Err(err) = http2::Builder::new(TokioExecutor::new()).serve_connection(io, server_state).await { // ????????????????????????????????????
                println!("Failed to serve connection: {:?}", err);
            }
        });
    }

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
    // println!("SERVER CLOSED");
}

fn match_uri_api(
    uri: &str,
    body: Option<&str>,
    wfm: Arc<Mutex<WFMClient>>,
    logged_in: Arc<Mutex<Option<bool>>>
) -> Result<Response<Full<Bytes>>, AppError> {
    let (root, other) = uri.split_once('/').unwrap_or((uri, ""));
    match root {
        "login" => {
            uri_api_login(body.unwrap(), wfm.clone(), logged_in).map_err(|e| e.prop("match_uri_api".into()))
        }
        "delete_riven" => {
            Ok(uri_api_delete_riven(other))
        }
        _ => {
            Ok(uri_not_found())
        }
    }
}
