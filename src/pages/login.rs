use std::{
    io::Cursor,
    ops::{Deref, DerefMut},
    rc::Rc,
    sync::Arc,
};

use ascii::AsciiString;
use async_lock::Mutex;
use maud::html;
use serde::Deserialize;
use tiny_http::{Header, Response, StatusCode};

use crate::{http_client::wfm_client::WFMClient, server::LOGGED_IN, AppError};

#[derive(Deserialize, Debug)]
struct Login {
    email: Arc<str>,
    password: Arc<str>,
}

pub fn uri_login() -> Response<Cursor<Vec<u8>>> {
    let pagecontent = html! {
        div id="login_screen" hx-trigger="LoginSuccess from:body" hx-swap="outerHTML" hx-get="/home" {
            div class="row" {
                img src="/logo.svg" class="logo";
            }
            div class="container" {
                form hx-put="/api/login" hx-target="#login_failed" {
                    div class="row" {
                        input
                            id="email-input"
                            type="email"
                            name="email"
                            placeholder="Email";
                    }
                        div class="row" {
                            input
                                id="password-input"
                                type="password"
                                name="password"
                                placeholder="Password";
                        }
                        div class="row" {
                            button type="submit" {"Login"}
                        }
                }
                p id="login_failed" style="text-align: center; color: red;" {b {""}}
            }
        }
    };
    tiny_http::Response::from_string(pagecontent.into_string()).with_header(tiny_http::Header {
        field: "Content-Type".parse().unwrap(),
        value: AsciiString::from_ascii("text/html; charset=utf8").unwrap(),
    })
}

// fn get_auth_state(
//     wfm: WFMClient,
// ) -> Result<bool, AppError> {
//     wfm.validate()
//         .map_err(|e| e.prop("Tauri CMD: get_auth_state".into()))
// }

pub fn uri_login_req(
    body: &str,
    wfm: Arc<Mutex<WFMClient>>,
) -> Result<Response<Cursor<Vec<u8>>>, AppError> {
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
        LOGGED_IN.set(authorized).unwrap();
        html! {""}
    };
    let r = tiny_http::Response::from_string(pagecontent.into_string()).with_header(
        tiny_http::Header {
            field: "Content-Type".parse().unwrap(),
            value: AsciiString::from_ascii("text/html; charset=utf8").unwrap(),
        },
    );
    if authorized {
        Ok(r.with_header(tiny_http::Header {
            field: "HX-Trigger".parse().unwrap(),
            value: AsciiString::from_ascii("LoginSuccess").unwrap(),
        }))
    } else {
        Ok(r)
    }
}
