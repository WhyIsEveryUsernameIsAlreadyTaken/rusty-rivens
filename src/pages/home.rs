use ascii::AsciiString;
use async_lock::Mutex;
use maud::{html, PreEscaped, DOCTYPE};
use std::{io::Cursor, ops::{Deref, DerefMut}, sync::Arc};
use tiny_http::{Response, StatusCode};

use crate::{http_client::{auth_state::AuthState, wfm_client::WFMClient}, server::{EditToggle, LOGGED_IN}, AppError};


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
    let height3 = format!("height: calc(126px + (2.2em * {}));", 3);
    let height2 = format!("height: calc(126px + (2.2em * {}));", 2);
    let height4 = format!("height: calc(126px + (2.2em * {}));", 4);
    html! {
        div class="row" {
            div class="cell" style=(height3) {
                div class="celltitle" {
                    "Torid Viva-concinak"
                }
                hr {}
                    p style="text-align: center;"{"+16.5% Heat"}
                    p style="text-align: center;"{"+16.5% Heat"}
                    p style="text-align: center; margin-block-start: 0; margin-block-end: 0;"{"+16.5% Heat"}
                div class="cellfooterdiv" {
                    div style="float: left;" {
                        button class="cellbutton" hx-post="/edit" hx-target="#edit_screen" hx-swap="outerHTML" {"Edit"}
                        button class="cellbutton" style="background-color: #ff4444;" {"Delete"}
                    }
                    img src="/wfm_favicon.ico" style="float: right; margin-left: 23px; padding-right: 13px;";
                }
            }
            div class="cell" style=(height2) {
                div class="celltitle" {
                    "Torid Viva-concinak"
                }
                hr {}
                    p style="text-align: center;"{"+16.5% Heat"}
                    p style="text-align: center; margin-block-start: 0; margin-block-end: 0;"{"+16.5% Heat"}
                div class="cellfooterdiv" {
                    div style="float: left;" {
                        button class="cellbutton" hx-post="/edit" hx-target="#edit_screen" hx-swap="outerHTML" {"Edit"}
                        button class="cellbutton" style="background-color: #ff4444;" {"Delete"}
                    }
                }
            }
            div class="cell" style=(height4) {
                div class="celltitle" {
                    "Torid Viva-concinak"
                }
                hr {}
                    p style="text-align: center;"{"+16.5% Heat"}
                    p style="text-align: center;"{"+16.5% Heat"}
                    p style="text-align: center;"{"+16.5% Heat"}
                    p style="text-align: center; margin-block-start: 0; margin-block-end: 0;"{"+16.5% Heat"}
                div class="cellfooterdiv" {
                    div style="float: left;" {
                        button class="cellbutton" hx-post="/edit" hx-target="#edit_screen" hx-swap="outerHTML" {"Edit"}
                        button class="cellbutton" style="background-color: #ff4444;" {"Delete"}
                    }
                }
            }
        }
        div id="edit_screen";
    }
}

pub fn uri_home() -> Response<Cursor<Vec<u8>>> {
    let pagecontent = rivens();
    tiny_http::Response::from_string(pagecontent.into_string()).with_header(tiny_http::Header {
        field: "Content-Type".parse().unwrap(),
        value: AsciiString::from_ascii("text/html; charset=utf8").unwrap(),
    })
}

pub fn uri_edit(edit_toggle: &mut EditToggle) ->  Response<Cursor<Vec<u8>>> {
    let pagecontent = if !edit_toggle.0 {
        edit_toggle.0 = true;
        html! {
            div id="edit_screen" style="display: block;" {
                div class="row_overlay" {
                    div id="edit_screen_gui" {
                        div style="flex-grow: 1;" {
                            div class="celltitle" {
                                "Edit Riven"
                            }
                            hr {}
                            p style="text-align: center;"{"+16.5% Heat"}
                            p style="text-align: center;"{"+16.5% Heat"}
                        }
                        div class="cellbuttondiv" style="padding-bottom: 13px;" {
                            button class="cellbutton" hx-post="/edit" hx-target="#edit_screen" hx-swap="outerHTML" {"Save"}
                            button class="cellbutton" hx-post="/edit" hx-target="#edit_screen" hx-swap="outerHTML" {"Cancel"}
                        }
                    }
                }

            }
        }
    } else {
        edit_toggle.0 = false;
        html! {
            div id="edit_screen" style="display: none;";
        }
    };
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
