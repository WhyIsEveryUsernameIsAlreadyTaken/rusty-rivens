use ascii::AsciiString;
use http_body_util::Full;
use hyper::{body::Bytes, header::{HeaderValue, CONTENT_TYPE}, Request, Response, StatusCode};
use tokio::{runtime::Handle, sync::Mutex};
use maud::{html, PreEscaped, DOCTYPE};
use serde_json::from_str;
use std::{io, ops::{Deref, DerefMut}, str::FromStr, sync::Arc, u8};

use crate::{http_client::wfm_client::WFMClient, rivens::inventory::convert_raw_inventory::Item, AppError};


pub fn uri_main(wfm: Arc<Mutex<WFMClient>>, logged_in: Arc<Mutex<Option<bool>>>) -> Result<Response<Full<Bytes>>, AppError> {
    let mut logged_in = logged_in.blocking_lock();
    let logged_in = logged_in.deref_mut();
    let pagecontent = if logged_in.is_some() {
        html! {
            (DOCTYPE)
            head {
                (PreEscaped("<script src=\"/htmx.min.js\"></script>"))
                (PreEscaped("<link rel=\"stylesheet\" href=\"/styles.css\" />"))
                body {
                    div hx-get="/home" hx-swap="outerHTML" hx-trigger="load";
                };
            }
        }
    } else {
        let valid = {
            let mut wfm = wfm.blocking_lock();
            let wfm = wfm.deref_mut();
            tokio::task::block_in_place(move || {
                Handle::current().block_on(async move {
                    wfm.validate().await
                })
            })
        }.map_err(|e| e.prop("uri_main".into()))?;
        if valid {
            *logged_in = Some(true);
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
    let charset = "text/html; charset=utf8".parse::<HeaderValue>().unwrap();
    Ok(match Response::builder()
        .header(CONTENT_TYPE, charset)
        .body(Full::new(Bytes::from(pagecontent.into_string()))) {
        Ok(v) => v,
        Err(_) => {
            let mut res = Response::new(Full::new(Bytes::new()));
            *res.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
            res
        },
    })
}

pub fn rivens() -> PreEscaped<String> {
    let rivens_data = include_str!("../../rivenData.json");
    let mut rivens: Vec<Item> = from_str(rivens_data).unwrap();
    rivens.sort_by(|a, b| a.attributes.len().cmp(&b.attributes.len()));
    let pagecontent = rivens.iter().fold(PreEscaped::default(),|acc, riven| {
        let title = format!("{} {}", riven.weapon_name, riven.name);
        let stats = riven.attributes.iter().fold(PreEscaped::default(), |acc, attr|{
            let stat = if attr.positive {
                format!("+{} {}", attr.value, attr.short_string)
            } else {
                format!("{} {}", attr.value, attr.short_string)
            };
            html! {
                (acc)
                p style="text-align: center; margin: 10px;"{(stat)}
            }
        });
        let oid = riven.oid.clone();
        let uri = format!("/api/delete_riven/{oid}");

        // let height = format!("height: calc(126px + (2.2em * {}));", riven.attributes.len());
        html! {
            (acc)
            div class="cell" id=(oid) {
                div class="celltitle" {
                    (title)
                }
                hr style="width: 100%";
                div style="flex-grow: 1"{
                    (stats)
                }
                div class="cellfooterdiv" {
                    div style="float: left;" {
                        button class="cellbutton" hx-post="/edit" hx-target="#edit_screen" hx-swap="outerHTML" {"Edit"}
                        button class="cellbutton" hx-delete=(uri) hx-target="closest .cell" hx-swap="outerHTML" style="background-color: #ff4444;" {"Delete"}
                    }
                    // img src="/wfm_favicon.ico" style="float: right; margin-left: 23px; padding-right: 13px;";
                }
            }
        }
    });
    html! {
        div style="justify-content: center;" {
            div class="row" {
                (pagecontent)
            }
        }
        div id="edit_screen";
    }
}

pub fn uri_home() -> Response<Full<Bytes>> {
    let pagecontent = rivens();
    let cc = "text/html; charset=utf8".parse::<HeaderValue>().unwrap();
    match Response::builder()
        .header(CONTENT_TYPE, cc)
        .body(Full::new(Bytes::from(pagecontent.into_string()))) {
        Ok(v) => v,
        Err(_) => {
            let mut res = Response::new(Full::new(Bytes::new()));
            *res.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
            res
        },
    }
}

pub fn uri_edit(edit_toggle: Arc<Mutex<bool>>) -> Response<Full<Bytes>> {
    let mut edit_toggle = edit_toggle.blocking_lock();
    let edit_toggle = edit_toggle.deref_mut();
    let pagecontent = if !*edit_toggle {
        *edit_toggle = true;
        html! {
            div id="edit_screen" style="display: block;" {
                div class="row_overlay" {
                    div id="edit_screen_gui" {
                        div style="flex-grow: 1;" {
                            div class="celltitle" {
                                "Edit Riven"
                            }
                            hr {}
                        }
                        div style="padding-bottom: 13px;" {
                            button class="cellbutton" hx-post="/edit" hx-target="#edit_screen" hx-swap="outerHTML" {"Save"}
                            button class="cellbutton" hx-post="/edit" hx-target="#edit_screen" hx-swap="outerHTML" {"Cancel"}
                        }
                    }
                }
            }
        }
    } else {
        *edit_toggle = false;
        html! {
            div id="edit_screen" style="display: none;";
        }
    };
    let cc = "text/html; charset=utf8".parse::<HeaderValue>().unwrap();
    match Response::builder()
        .header(CONTENT_TYPE, cc)
        .body(Full::new(Bytes::from(pagecontent.into_string())))
    {
        Ok(v) => v,
        Err(_) => {
            let mut res = Response::new(Full::new(Bytes::new()));
            *res.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
            res
        },
    }
}

pub async fn uri_unauthorized() -> Response<Full<Bytes>> {
    let pagecontent = html! {
        (DOCTYPE)
        body {
            h2 {
                "401 Unauthorized"
            }
        }
    };

    let cc = "text/html; charset=utf8".parse::<HeaderValue>().unwrap();
    match Response::builder()
        .header(CONTENT_TYPE, cc)
        .status(401)
        .body(Full::new(Bytes::from(pagecontent.into_string())))
    {
        Ok(v) => v,
        Err(_) => {
            let mut res = Response::new(Full::new(Bytes::new()));
            *res.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
            res
        },
    }
}

pub fn uri_not_found() -> Response<Full<Bytes>> {
    let pagecontent = html! {
        (DOCTYPE)
        body {
            h2 {
                "404 Not Found"
            }
        }
    };

    let cc = "text/html; charset=utf8".parse::<HeaderValue>().unwrap();
    match Response::builder()
        .header(CONTENT_TYPE, cc)
        .status(404)
        .body(Full::new(Bytes::from(pagecontent.into_string())))
    {
        Ok(v) => v,
        Err(_) => {
            let mut res = Response::new(Full::new(Bytes::new()));
            *res.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
            res
        },
    }
}
