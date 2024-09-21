use ascii::AsciiString;
use async_lock::Mutex;
use maud::{html, PreEscaped, DOCTYPE};
use std::{io::Cursor, ops::{Deref, DerefMut}, sync::Arc};
use tiny_http::{Response, StatusCode};

use crate::{http_client::{auth_state::AuthState, wfm_client::WFMClient}, server::{CurrentScreen, LOGGED_IN}, AppError};


pub fn uri_main(wfm: Arc<Mutex<WFMClient>>) -> Result<Response<Cursor<Vec<u8>>>, AppError> {
    let pagecontent = if LOGGED_IN.get().is_some() {
        html! {
            (DOCTYPE)
            head {
                (PreEscaped("<script src=\"/htmx.min.js\"></script>"))
                (PreEscaped("<link rel=\"stylesheet\" href=\"/styles.css\" />"))
                (PreEscaped("<script src=\"/htmx.min.js\"></script>"))
                body {
                    div hx-get="/home" hx-swap="outerHTML" hx-trigger="load";
                };
            }
        }
    } else {
        let valid = smolscale::block_on(async move {
            let wfm = wfm.lock().await;
            let wfm = wfm.deref();
            wfm.validate().await
        }).map_err(|e| e.prop("uri_main".into()))?;
        if valid {
            LOGGED_IN.set(true).unwrap();
            html! {
                (DOCTYPE)
                head {
                    (PreEscaped("<script src=\"/htmx.min.js\"></script>"))
                    (PreEscaped("<link rel=\"stylesheet\" href=\"/styles.css\" />"))
                    (PreEscaped("<script src=\"/htmx.min.js\"></script>"))
                    body {
                        div hx-get="/home" hx-swap="outerHTML" hx-trigger="load";
                    };
                }
            }
        } else {
            html! {
                (DOCTYPE)
                head {
                    (PreEscaped("<script src=\"/htmx.min.js\"></script>"))
                    (PreEscaped("<link rel=\"stylesheet\" href=\"/styles.css\" />"))
                    (PreEscaped("<script src=\"/htmx.min.js\"></script>"))
                    body {
                        div hx-get="/login" hx-swap="outerHTML" hx-trigger="load";
                    };
                }
            }
        }
    };

    Ok(tiny_http::Response::from_string(pagecontent.into_string()).with_header(tiny_http::Header {
        field: "Content-Type".parse().unwrap(),
        value: AsciiString::from_ascii("text/html; charset=utf8").unwrap(),
    }))
}

pub fn rivens() -> PreEscaped<String> {
    let height = format!("height: calc(126px + (2.2em * {}));", 2);
    html! {
        div class="row" {
            div class="cell" style=(height) {
                div class="celltitle" {
                    "Torid Viva-concinak"
                }
                hr {}
                    p style="text-align: center;"{"+16.5% Heat"}
                    p style="text-align: center; margin-block-start: 0; margin-block-end: 0;"{"+16.5% Heat"}
                div class="cellbuttondiv" {
                    button class="cellbutton" {"Edit"}
                    button class="cellbutton" style="background-color: #ff4444;" {"Delete"}
                }
            }
        }
    }
}

pub fn uri_home() -> Response<Cursor<Vec<u8>>> {
    let pagecontent = rivens();
    tiny_http::Response::from_string(pagecontent.into_string()).with_header(tiny_http::Header {
        field: "Content-Type".parse().unwrap(),
        value: AsciiString::from_ascii("text/html; charset=utf8").unwrap(),
    })
}

pub fn uri_unauthorized() -> Response<Cursor<Vec<u8>>> {
    let pagecontent = html! {
        (DOCTYPE)
        body {
            h2 {
                "401 Unauthorized"
            }
        }
    };

    tiny_http::Response::from_string(pagecontent.into_string())
        .with_header(tiny_http::Header {
            field: "Content-Type".parse().unwrap(),
            value: AsciiString::from_ascii("text/html; charset=utf8").unwrap(),
        })
        .with_status_code(StatusCode(403))
}

pub fn uri_not_found() -> Response<Cursor<Vec<u8>>> {
    let pagecontent = html! {
        (DOCTYPE)
        body {
            h2 {
                "404 Not Found"
            }
        }
    };

    tiny_http::Response::from_string(pagecontent.into_string())
        .with_header(tiny_http::Header {
            field: "Content-Type".parse().unwrap(),
            value: AsciiString::from_ascii("text/html; charset=utf8").unwrap(),
        })
        .with_status_code(StatusCode(404))
}
