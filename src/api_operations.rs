use std::{io::Cursor, ops::DerefMut, sync::Arc};

use ascii::AsciiString;
use async_lock::Mutex;
use maud::html;
use serde::Deserialize;
use tiny_http::{Request, Response};

use crate::{http_client::wfm_client::WFMClient, AppError};

#[derive(Deserialize, Debug)]
struct Login {
    email: Arc<str>,
    password: Arc<str>,
}

pub fn uri_api_login(
    rq: Request,
    body: &str,
    wfm: Arc<Mutex<WFMClient>>,
    logged_in: &mut Option<bool>,
) -> Result<(), AppError> {
    let (email, password) = {
        let log =
            serde_urlencoded::from_str::<Login>(body).expect("bruh this aint urlencoded tf u doin");
        (log.email, log.password)
    };

    let status = smolscale::block_on(async move {
        let mut wfm = wfm.lock().await;
        let wfm = wfm.deref_mut();
        wfm.login(&email, &password).await
    }).map_err(|e| e.prop("uri_login_req".into()))?;

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

pub fn uri_api_delete_riven(rq: Request, id: &str) -> Result<(), AppError> {
    rq.respond(tiny_http::Response::empty(200)).map_err(|e| AppError::new(e.to_string(), "uri_api_delete_riven".to_string()))
}
