use std::{ops::DerefMut, sync::Arc};

use ascii::AsciiString;
use maud::html;
use serde::Deserialize;
use tiny_http::Request;
use tokio::sync::Mutex;

use crate::{block_in_place, http_client::{qf_client::QFClient, wfm_client::WFMClient}, rivens::inventory::riven_lookop::RivenDataLookup, server::RIVEN_LOOKUP, AppError};

#[derive(Deserialize, Debug)]
struct Login {
    email: Arc<str>,
    password: Arc<str>,
}

pub fn uri_api_login(
    rq: Request,
    body: &str,
    wfm: Arc<Mutex<WFMClient>>,
    qf: Arc<Mutex<QFClient>>,
    logged_in: &mut Option<bool>,
) -> Result<(), AppError> {
    let (email, password) = {
        let log =
            serde_urlencoded::from_str::<Login>(body).expect("bruh this aint urlencoded tf u doin");
        (log.email, log.password)
    };

    let (status, id, check_code, ingame_name) = block_in_place!(async move {
        let mut wfm = wfm.lock().await;
        let wfm = wfm.deref_mut();
        wfm.login(&email, &password).await
    }).map_err(|e| e.prop("uri_login_req".into()))?;

    block_in_place!(async {
        let qf = qf.clone();
        let mut qf = qf.lock().await;
        let qf = qf.deref_mut();
        qf.login(id, check_code, ingame_name).await
    }).map_err(|e| e.prop("uri_login_req".into()))?;

    let lookup = block_in_place!( async { RivenDataLookup::setup(qf.clone()).await }).expect(
        "FATAL: Could not retrieve riven lookup data"
    );
    RIVEN_LOOKUP.set(lookup).expect("FATAL: Could not store riven lookup data in memory");

    // for testing
    let authorized = status.code == 200;

    let pagecontent = if !authorized {
        html! {p id="login_failed" style="text-align: center; color: red;" {b {"Login Failed, Please try again"}}}
    } else {
        *logged_in = Some(authorized);
        html! {""}
    };
    let r = tiny_http::Response::from_string(pagecontent.into_string()).with_header(
        tiny_http::Header {
            field: "Content-Type".parse().unwrap(),
            value: AsciiString::from_ascii("text/html; charset=utf8").unwrap(),
        },
    );
    if authorized {
        rq.respond(r.with_header(tiny_http::Header {
            field: "HX-Trigger".parse().unwrap(),
            value: AsciiString::from_ascii("LoginSuccess").unwrap(),
        })).map_err(|e| AppError::new(e.to_string(), "uri_api_login".to_string()))
    } else {
        rq.respond(r).map_err(|e| AppError::new(e.to_string(), "uri_api_login".to_string()))
    }
}

pub fn uri_api_delete_riven(rq: Request, _id: &str) -> Result<(), AppError> {
    rq.respond(tiny_http::Response::empty(200)).map_err(|e| AppError::new(e.to_string(), "uri_api_delete_riven".to_string()))
}

#[derive(Debug, Deserialize)]
struct EditOptions {
    price: i64,
    visible: bool,
    description: String,
}

pub fn uri_api_update_riven(rq: Request, mult: bool, _id: &str, body: Option<&str>) -> Result<(), AppError> {
    if let Some(body) = body {
        let v = body.find("visible=").unwrap();
        let d = body.find("description=").unwrap();
        let left = &body[..v];
        let middle = &body[v..d];
        let middle = middle.replace("on", "true").replace("off", "false");
        let right = &body[d..];
        let mut body = left.to_string();
        body.push_str(&middle);
        body.push_str(right);
        let body = serde_urlencoded::from_str::<EditOptions>(body.as_str()).expect("bruh this aint urlencoded tf u doin");
        println!("{body:#?}");
    };
    rq.respond(tiny_http::Response::empty(200)).map_err(|e| AppError::new(e.to_string(), "uri_api_update_riven".to_string()))
}
