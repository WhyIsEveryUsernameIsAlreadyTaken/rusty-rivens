use http_body_util::Full;
use hyper::{body::Bytes, header::CONTENT_TYPE, Response, StatusCode};
use maud::html;


pub fn uri_login() -> Response<Full<Bytes>> {
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
    let cc = "text/html; charset=utf8".parse::<hyper::header::HeaderValue>().unwrap();

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
