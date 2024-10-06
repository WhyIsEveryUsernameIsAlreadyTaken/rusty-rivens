use std::io::{self, Cursor};

use ascii::AsciiString;
use maud::html;
use tiny_http::{Request, Response};


pub fn uri_login(rq: Request) -> io::Result<()> {
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
    rq.respond(tiny_http::Response::from_string(pagecontent.into_string()).with_header(tiny_http::Header {
        field: "Content-Type".parse().unwrap(),
        value: AsciiString::from_ascii("text/html; charset=utf8").unwrap(),
    }))
}
