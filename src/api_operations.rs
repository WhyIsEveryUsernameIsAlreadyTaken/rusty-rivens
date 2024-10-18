use std::{ops::DerefMut, sync::Arc};

use http_body_util::Full;
use hyper::{body::Bytes, header::{HeaderValue, CONTENT_TYPE}, Response, StatusCode};
use maud::html;
use serde::Deserialize;
use tokio::{runtime::Handle, sync::Mutex, task};

use crate::{http_client::wfm_client::WFMClient, AppError};

#[derive(Deserialize, Debug)]
struct Login {
    email: Arc<str>,
    password: Arc<str>,
}

pub fn uri_api_login(
    body: &str,
    wfm: Arc<Mutex<WFMClient>>,
    logged_in: Arc<Mutex<Option<bool>>>,
) -> Result<Response<Full<Bytes>>, AppError> {
    let (email, password) = {
        let log =
            serde_urlencoded::from_str::<Login>(body).expect("bruh this aint urlencoded tf u doin");
        (log.email, log.password)
    };

    let status = {
        let mut wfm = wfm.blocking_lock();
        let wfm = wfm.deref_mut();
        task::block_in_place(move || {
            Handle::current().block_on(async move {
                wfm.login(&email, &password).await
            })
        })
    }.map_err(|e| e.prop("uri_login_req".into()))?;

    // for testing
    let authorized = status.code == 200;

    let mut logged_in = logged_in.blocking_lock();
    let logged_in = logged_in.deref_mut();
    let pagecontent = if !authorized {
        html! {p id="login_failed" style="text-align: center; color: red;" {b {"Login Failed, Please try again"}}}
    } else {
        *logged_in = Some(authorized);
        html! {""}
    };
    let cc = "text/html; charset=utf8".parse::<HeaderValue>().unwrap();
    let r = Response::builder()
        .header(CONTENT_TYPE, cc);
    let ls = "LoginSuccess".parse::<HeaderValue>().unwrap();
    let r = if authorized {
        r.header("HX-Trigger", ls)
    } else {
        r
    };
    let r = r.body(Full::new(Bytes::from(pagecontent.into_string())));
    Ok(match r {
        Ok(v) => v,
        Err(_) => {
            let mut resp = Response::new(Full::new(Bytes::new()));
            *resp.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
            resp
        },
    })
}

pub fn uri_api_delete_riven(_id: &str) -> Response<Full<Bytes>> {
    Response::new(Full::new(Bytes::new()))
}
