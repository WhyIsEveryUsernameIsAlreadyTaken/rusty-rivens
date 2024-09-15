use std::{fs::File, io::{Cursor, Read}, path::Path};

use ascii::AsciiString;
use tiny_http::Response;

pub fn uri_styles() -> Response<Cursor<Vec<u8>>> {
    let mut file = File::open(&Path::new("styles.css")).unwrap();
    let mut text = String::new();
    file.read_to_string(&mut text).unwrap();
    let hash = md5::compute(text.as_str());

    tiny_http::Response::from_string(text)
        .with_header(tiny_http::Header {
            field: "Content-Type".parse().unwrap(),
            value: AsciiString::from_ascii("text/css; charset=utf8")
                .unwrap(),
        })
        .with_header(tiny_http::Header {
            field: "Cache-Control".parse().unwrap(),
            value: AsciiString::from_ascii("public, max-age=31536000, immutable")
                .unwrap(),
        })
        .with_header(tiny_http::Header {
            field: "ETag".parse().unwrap(),
            value: AsciiString::from_ascii(format!("{:x}", hash))
                .unwrap(),
        })
}

pub fn uri_htmx() -> Response<Cursor<Vec<u8>>> {
    let mut file = File::open(&Path::new("htmx.min.js")).unwrap();
    let mut text = String::new();
    file.read_to_string(&mut text).unwrap();
    let hash = md5::compute(text.as_str());

    tiny_http::Response::from_string(text)
        .with_header(tiny_http::Header {
            field: "Content-Type".parse().unwrap(),
            value: AsciiString::from_ascii("text/javascript; charset=utf8")
                .unwrap(),
        })
        .with_header(tiny_http::Header {
            field: "Cache-Control".parse().unwrap(),
            value: AsciiString::from_ascii("public, max-age=31536000, immutable")
                .unwrap(),
        })
        .with_header(tiny_http::Header {
            field: "ETag".parse().unwrap(),
            value: AsciiString::from_ascii(format!("{:x}", hash))
                .unwrap(),
        })
}

pub fn uri_logo() -> Response<Cursor<Vec<u8>>> {
    let file = File::open(&Path::new("./public/logo.svg")).unwrap();
    let data = file.bytes().fold(vec![], |mut acc, byte| {
        acc.push(byte.unwrap());
        acc
    });
    let hash = md5::compute(data.clone());

    tiny_http::Response::from_data(data)
        .with_header(tiny_http::Header {
            field: "Content-Type".parse().unwrap(),
            value: AsciiString::from_ascii("image/svg+xml")
                .unwrap(),
        })
        .with_header(tiny_http::Header {
            field: "Cache-Control".parse().unwrap(),
            value: AsciiString::from_ascii("public, max-age=31536000, immutable")
                .unwrap(),
        })
        .with_header(tiny_http::Header {
            field: "ETag".parse().unwrap(),
            value: AsciiString::from_ascii(format!("{:x}", hash))
                .unwrap(),
        })
}
