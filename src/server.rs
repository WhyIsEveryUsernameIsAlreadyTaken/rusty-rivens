use http_body_util::{BodyExt, Full};
use hyper::{body::{Bytes, Incoming}, service::service_fn, Request, Response};
use hyper_util::rt::{TokioExecutor, TokioIo};
use tokio::{net::TcpListener, runtime::Handle, sync::Mutex};
use tokio_rustls::{rustls::ServerConfig, TlsAcceptor};
use std::{fs, io::{self, BufReader}, net::{Ipv4Addr, SocketAddr}, sync::Arc, time::SystemTime};

pub type BoxBody = http_body_util::combinators::BoxBody<Bytes, hyper::Error>;

pub fn full<T: Into<Bytes>>(chunk: T) -> BoxBody {
    Full::new(chunk.into())
        .map_err(|never| match never {})
        .boxed()
}

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
    let addr = SocketAddr::new(Ipv4Addr::LOCALHOST.into(), 3000);

    let certfile = fs::File::open("sample.pem").expect("TwT");
    let mut buf_reader = BufReader::new(certfile);
    let certs = rustls_pemfile::certs(&mut buf_reader).map(|cert| cert.expect("failed to parse cert")).collect();
    let keyfile = fs::File::open("sample.rsa").expect("qwq");
    let mut buf_reader = BufReader::new(keyfile);
    let key = rustls_pemfile::private_key(&mut buf_reader).expect("failed to parse key").expect("no private key");

    let listener = TcpListener::bind(addr).await.unwrap();
    println!("Listening on https://{}", addr);

    let _ = rustls::crypto::ring::default_provider().install_default();

    let mut server_config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .expect("all this shit has to work, no exceptions");
    server_config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec(), b"http/1.0".to_vec()];
    let tls_acceptor = TlsAcceptor::from(Arc::new(server_config));

    loop {
        let (stream, addr) = listener.accept().await.unwrap();
        println!("connection accepted for {addr}");
        let tls_acceptor = tls_acceptor.clone();
        let server_state = server_state.clone();
        let service = service_fn(move |req| {
            let server_state = server_state.clone();
            async move {
                match_uri(req, server_state)
            }
        });
        tokio::spawn(async move {
            let stream = tls_acceptor.accept(stream).await.expect("FUCK YOU TLS");
            if let Err(err) = hyper_util::server::conn::auto::Builder::new(TokioExecutor::new())
                .serve_connection(TokioIo::new(stream), service).await {
                println!("Failed to serve connection for {addr}: {:#}", err)
            };
            println!("serving connection");
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
) -> Result<Response<BoxBody>, AppError> {
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

fn match_uri(mut req: Request<Incoming>, state: ServerState) -> Result<Response<BoxBody>, AppError> {
    let data = {
        tokio::task::block_in_place(|| {
            Handle::current().block_on(async {
                req.body_mut().collect().await.map_err(|e| AppError::new(e.to_string(), "handle_request".to_string()))
            })
        })
    }.unwrap();
    let data = data.to_bytes();

    println!("{}", req.uri().path());
    let uri = req.uri().path();
    let (root, other) = uri[1..].split_once('/').unwrap_or((&uri[1..], ""));

    let body = String::from_utf8(data.into()).map_err(|e| AppError::new(e.to_string(), "handle_request".to_string())).unwrap();
    let body = if body.len() != 0 {
        Some(body.as_str())
    } else {
        None
    };
    Ok(match root {
        "" | "/" => {
            uri_main(state.wfm.clone(), state.logged_in.clone()).unwrap()
        }
        "htmx.min.js" => {
            uri_htmx().unwrap()
        }
        "styles.css" => {
            uri_styles().unwrap()
        }
        "api" => {
            match_uri_api(other, body, state.wfm.clone(), state.logged_in.clone()).unwrap()
        }
        "login" => {
            uri_login()
        }
        "home" => {
            uri_home()
        }
        "edit" => {
            uri_edit(state.edit_toggle.clone())
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
    })
}
