use hyper::{header::{CACHE_CONTROL, CONTENT_TYPE, ETAG}, Response, StatusCode};
use wry::http::Error;

use crate::{file_consts::{HTMX, LOGO, STYLES, WFMLOGO}, server::{full, BoxBody}};

pub fn uri_styles() -> Result<Response<BoxBody>, Error> {
    let hash = md5::compute(STYLES);

    let ct = "text/css; charset=utf8".parse::<hyper::header::HeaderValue>()
        .unwrap();
    let cc = "public, max-age=31536000, immutable".parse::<hyper::header::HeaderValue>()
        .unwrap();
    let et = format!("{:x}", hash).parse::<hyper::header::HeaderValue>()
        .unwrap();
    let res = Response::builder()
        .header(CONTENT_TYPE, ct)
        .header(CACHE_CONTROL, cc)
        .header(ETAG, et)
        .body(full(STYLES));

    Ok(match res {
        Ok(v) => v,
        Err(_) => {
            let mut res = Response::new(full(""));
            *res.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
            res
        },
    })
}

pub fn uri_htmx() -> Result<Response<BoxBody>, Error> {
    let hash = md5::compute(HTMX);

    let ct = "text/javascript; charset=utf8".parse::<hyper::header::HeaderValue>()
        .unwrap();
    let cc = "public, max-age=31536000, immutable".parse::<hyper::header::HeaderValue>()
        .unwrap();
    let et = format!("{:x}", hash).parse::<hyper::header::HeaderValue>()
        .unwrap();
    let res = Response::builder()
        .header(CONTENT_TYPE, ct)
        .header(CACHE_CONTROL, cc)
        .header(ETAG, et)
        .body(full(HTMX));

    Ok(match res {
        Ok(v) => v,
        Err(_) => {
            let mut res = Response::new(full(""));
            *res.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
            res
        },
    })
}

pub fn uri_logo() -> Result<Response<BoxBody>, Error> {
    let hash = md5::compute(LOGO);

    let ct = "image/svg+xml; charset=utf8".parse::<hyper::header::HeaderValue>()
        .unwrap();
    let cc = "public, max-age=31536000, immutable".parse::<hyper::header::HeaderValue>()
        .unwrap();
    let et = format!("{:x}", hash).parse::<hyper::header::HeaderValue>()
        .unwrap();
    let res = Response::builder()
        .header(CONTENT_TYPE, ct)
        .header(CACHE_CONTROL, cc)
        .header(ETAG, et)
        .body(full(LOGO));

    Ok(match res {
        Ok(v) => v,
        Err(_) => {
            let mut res = Response::new(full(""));
            *res.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
            res
        },
    })
}

pub fn uri_wfmlogo() -> Result<Response<BoxBody>, Error> {
    let hash = md5::compute(WFMLOGO);

    let ct = "image/vnd.microsoft.icon".parse::<hyper::header::HeaderValue>()
        .unwrap();
    let cc = "public, max-age=31536000, immutable".parse::<hyper::header::HeaderValue>()
        .unwrap();
    let et = format!("{:x}", hash).parse::<hyper::header::HeaderValue>()
        .unwrap();
    let res = Response::builder()
        .header(CONTENT_TYPE, ct)
        .header(CACHE_CONTROL, cc)
        .header(ETAG, et)
        .body(full(WFMLOGO));

    Ok(match res {
        Ok(v) => v,
        Err(_) => {
            let mut res = Response::new(full(""));
            *res.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
            res
        },
    })
}
