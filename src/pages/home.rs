use ascii::AsciiString;
use async_lock::Mutex;
use maud::{html, PreEscaped, DOCTYPE};
use serde_json::from_str;
use std::{io::{self}, ops::Deref, sync::Arc};
use tiny_http::{Request, Response, StatusCode};

use crate::{http_client::wfm_client::WFMClient, rivens::inventory::convert_raw_inventory::{Item, Units}, AppError};


pub fn uri_main(rq: Request, wfm: Arc<Mutex<WFMClient>>, logged_in: &mut Option<bool>) -> Result<(), AppError> {
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
        let valid = smolscale::block_on(async move {
            let wfm = wfm.lock().await;
            let wfm = wfm.deref();
            wfm.validate().await
        }).map_err(|e| e.prop("uri_main".into()))?;
        if valid {
            *logged_in = Some(true);
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

    rq.respond(tiny_http::Response::from_string(pagecontent.into_string()).with_header(tiny_http::Header {
        field: "Content-Type".parse().unwrap(),
        value: AsciiString::from_ascii("text/html; charset=utf8").unwrap(),
    })).map_err(|e| AppError::new(e.to_string(), "uri_main".to_string()))
}

pub fn rivens() -> PreEscaped<String> {
    let rivens_data = include_str!("../../rivenData.json");
    let mut rivens: Vec<Item> = from_str(rivens_data).unwrap();
    rivens.sort_by(|a, b| a.attributes.len().cmp(&b.attributes.len()));
    let pagecontent = rivens.iter().fold(PreEscaped::default(),|acc, riven| {
        let title = format!("{} {}", riven.weapon_name, riven.name);
        let stats = riven.attributes.iter().fold(PreEscaped::default(), |acc, attr|{
            let stat = match attr.units {
                Units::Percent => {
                    if attr.positive {
                        format!("+{}% {}", attr.value, attr.short_string)
                    } else {
                        format!("-{}% {}", attr.value, attr.short_string)
                    }
                },
                Units::Multiply => format!("x{} {}", attr.value, attr.short_string),
                Units::Seconds => {
                    if attr.positive {
                        format!("+{}s {}", attr.value, attr.short_string)
                    } else {
                        format!("-{}s {}", attr.value, attr.short_string)
                    }
                },
                Units::Null => format!("{} {}", attr.value, attr.short_string),
            };
            html! {
                (acc)
                p style="text-align: center; margin: 10px;"{(stat)}
            }
        });
        let oid = riven.oid.clone();
        let id = format!("a{oid}");
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
                        button class="cellbutton" hx-post="/edit" hx-target="#screen" hx-swap="beforeend" {"Edit"}
                        button class="cellbutton" hx-delete=(uri) hx-target="closest .cell" hx-swap="outerHTML" style="background-color: #ff4444;" {"Delete"}
                    }
                    // img src="/wfm_favicon.ico" style="float: right; margin-left: 23px; padding-right: 13px;";
                }
            }
        }
    });
    pagecontent
}

pub fn uri_home(rq: Request) -> io::Result<()> {
    let pagecontent = html! {
    div id="screen" style="justify-content: center;" {
        div hx-ext="ws" ws-connect="ws://localhost:8069"
            div id="riven-table" class="row" {
            }
        }
    };
    rq.respond(tiny_http::Response::from_string(pagecontent.into_string()).with_header(tiny_http::Header {
        field: "Content-Type".parse().unwrap(),
        value: AsciiString::from_ascii("text/html; charset=utf8").unwrap(),
    }))
}

pub fn uri_edit_cancel(rq: Request) -> io::Result<()> {
    rq.respond(Response::empty(200))
}

pub fn uri_edit_open(rq: Request) -> io::Result<()> {
    let title = format!("Edit <RIVENNAME>");
    let pagecontent = html! {
        div id="edit_screen" style="display: block;" {
            div class="row_overlay" {
                div id="edit_screen_gui" {
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
                                min="1"
                                max="100000"
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
                        button class="cellbutton" hx-delete="/edit_cancel" style="background-color: #7bdaff;" hx-target="#edit_screen" hx-swap="outerHTML" {"Save"}
                        button class="cellbutton" hx-delete="/edit_cancel" hx-target="#edit_screen" hx-swap="outerHTML" {"Cancel"}
                        button class="cellbutton" style="float: right; margin-right: 13px" {"Blacklist"}
                    }
                }
            }
        }
    };
    rq.respond(tiny_http::Response::from_string(pagecontent.into_string()).with_header(tiny_http::Header {
        field: "Content-Type".parse().unwrap(),
        value: AsciiString::from_ascii("text/html; charset=utf8").unwrap(),
    }))
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

    rq.respond(tiny_http::Response::from_string(pagecontent.into_string())
        .with_header(tiny_http::Header {
            field: "Content-Type".parse().unwrap(),
            value: AsciiString::from_ascii("text/html; charset=utf8").unwrap(),
        })
        .with_status_code(StatusCode(403)))
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

    rq.respond(tiny_http::Response::from_string(pagecontent.into_string())
        .with_header(tiny_http::Header {
            field: "Content-Type".parse().unwrap(),
            value: AsciiString::from_ascii("text/html; charset=utf8").unwrap(),
        })
        .with_status_code(StatusCode(404)))
}
