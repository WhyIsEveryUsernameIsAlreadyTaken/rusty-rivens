use ascii::AsciiString;
use maud::{html, PreEscaped, DOCTYPE};
use std::{
    io::{self},
    ops::DerefMut,
    sync::Arc,
};
use tiny_http::{Request, Response, StatusCode};
use tokio::sync::Mutex;

use crate::{
    block_in_place,
    http_client::{qf_client::QFClient, wfm_client::WFMClient},
    rivens::inventory::riven_lookop::RivenDataLookup,
    server::RIVEN_LOOKUP,
    AppError,
};

pub fn uri_main(
    rq: Request,
    wfm: Arc<Mutex<WFMClient>>,
    qf: Arc<Mutex<QFClient>>,
    logged_in: &mut Option<bool>,
) -> Result<(), AppError> {
    let pagecontent = if logged_in.is_some() {
        html! {
            (DOCTYPE)
            head {
                (PreEscaped("<script src=\"/htmx.min.js\"></script>"))
                (PreEscaped("<link rel=\"stylesheet\" href=\"/styles.css\" />"))
                (PreEscaped("<script src=\"https://unpkg.com/htmx.org@1.9.12/dist/ext/ws.js\"></script>"))
                body {
                    div hx-get="/home" hx-swap="outerHTML" hx-trigger="load";
                };
            }
        }
    } else {
        let valid = block_in_place!(async move {
            let mut wfm = wfm.lock().await;
            let wfm = wfm.deref_mut();
            wfm.validate().await
        })
        .map_err(|e| e.prop("uri_main".into()))?;
        if valid {
            *logged_in = Some(true);
            let lookup = block_in_place!(async { RivenDataLookup::setup(qf).await })
                .expect("FATAL: Could not retrieve riven lookup data");
            RIVEN_LOOKUP
                .set(lookup)
                .expect("FATAL: Could not store riven lookup data in memory");
            html! {
                (DOCTYPE)
                head {
                    (PreEscaped("<script src=\"/htmx.min.js\"></script>"))
                    (PreEscaped("<link rel=\"stylesheet\" href=\"/styles.css\" />"))
                    (PreEscaped("<script src=\"https://unpkg.com/htmx.org@1.9.12/dist/ext/ws.js\"></script>"))
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
                    (PreEscaped("<script src=\"https://unpkg.com/htmx.org@1.9.12/dist/ext/ws.js\"></script>"))
                    body {
                        div hx-get="/login" hx-swap="outerHTML" hx-trigger="load";
                    };
                }
            }
        }
    };

    rq.respond(
        tiny_http::Response::from_string(pagecontent.into_string()).with_header(
            tiny_http::Header {
                field: "Content-Type".parse().unwrap(),
                value: AsciiString::from_ascii("text/html; charset=utf8").unwrap(),
            },
        ),
    )
    .map_err(|e| AppError::new(e.to_string(), "uri_main".to_string()))
}

pub fn uri_home(rq: Request) -> io::Result<()> {
    let pagecontent = html! {
    div id="screen" style="justify-content: center;" {
        div hx-ext="ws" ws-connect="ws://localhost:8069"
            div id="riven-table" class="row" {
            }
        }
    };
    rq.respond(
        tiny_http::Response::from_string(pagecontent.into_string()).with_header(
            tiny_http::Header {
                field: "Content-Type".parse().unwrap(),
                value: AsciiString::from_ascii("text/html; charset=utf8").unwrap(),
            },
        ),
    )
}

pub fn uri_edit_cancel(rq: Request) -> io::Result<()> {
    rq.respond(Response::empty(200))
}

pub fn uri_edit_open(rq: Request, oid: &str) -> io::Result<()> {
    let title = format!("Edit <RIVENNAME>");
    let post_url = format!("/api/update_single_riven/{oid}");

    let pagecontent = html! {
        div id="edit_screen" style="display: block;" {
            div class="row_overlay" {
                div id="edit_screen_gui" {
                    form hx-post=(post_url) hx-target="#edit_screen" hx-swap="outerHTML swap:.08s"  {
                        div style="flex-grow: 1;" {
                            div class="celltitle" {
                                (title)
                            }
                            hr {}
                            div {
                                label for="price-input" style="padding-right: 13px; padding-left: 13px;" {"Price"}
                                input
                                    id="price-input"
                                    style="font-size: 0.8em;"
                                    type="number"
                                    min="10"
                                    max="100000"
                                    value="10"
                                    name="price";
                            }
                            div style="display: flex; flex-wrap: wrap; padding-top: 15px" {
                                label for="visible-toggle" style="padding-right: 13px; padding-left: 13px;" {"Visible"}
                                label class="switch" {
                                    input
                                        id="visible-toggle"
                                        type="checkbox"
                                        checked
                                        name="visible";
                                    span class="slider";
                                }
                            }
                            div style="display: flex; flex-direction: column;" {
                                textarea
                                    type="text"
                                    name="description"
                                    placeholder="Description (Optional)"
                                    rows="4"
                                    resize="none"
                                    maxlength="200" {""}
                            }
                        }
                        div style="padding-bottom: 13px;" {
                            button
                                class="cellbutton"
                                type="submit"
                                style="background-color: #7bdaff;"
                                {"Save"}

                            button class="cellbutton" hx-delete="/edit_cancel" hx-target="#edit_screen" hx-swap="outerHTML swap:.08s" {"Cancel"}
                            button class="cellbutton" style="float: right; margin-right: 13px" {"Blacklist"}
                        }
                    }
                }
            }
        }
    };
    rq.respond(
        tiny_http::Response::from_string(pagecontent.into_string()).with_header(
            tiny_http::Header {
                field: "Content-Type".parse().unwrap(),
                value: AsciiString::from_ascii("text/html; charset=utf8").unwrap(),
            },
        ),
    )
}

pub fn uri_unauthorized(rq: Request) -> io::Result<()> {
    let pagecontent = html! {
        (DOCTYPE)
        body {
            h2 {
                "401 Unauthorized"
            }
        }
    };

    rq.respond(
        tiny_http::Response::from_string(pagecontent.into_string())
            .with_header(tiny_http::Header {
                field: "Content-Type".parse().unwrap(),
                value: AsciiString::from_ascii("text/html; charset=utf8").unwrap(),
            })
            .with_status_code(StatusCode(403)),
    )
}

pub fn uri_not_found(rq: Request) -> io::Result<()> {
    let pagecontent = html! {
        (DOCTYPE)
        body {
            h2 {
                "404 Not Found"
            }
        }
    };

    rq.respond(
        tiny_http::Response::from_string(pagecontent.into_string())
            .with_header(tiny_http::Header {
                field: "Content-Type".parse().unwrap(),
                value: AsciiString::from_ascii("text/html; charset=utf8").unwrap(),
            })
            .with_status_code(StatusCode(404)),
    )
}
