use std::io::Cursor;
use ascii::AsciiString;
use tiny_http::Response;

use crate::file_consts::{HTMX, LOGO, WFMLOGO, STYLES};

pub fn uri_styles() -> Response<Cursor<Vec<u8>>> {
    let hash = md5::compute(STYLES);

    tiny_http::Response::from_string(STYLES)
        .with_header(tiny_http::Header {
            field: "Content-Type".parse().unwrap(),
            value: AsciiString::from_ascii("text/css; charset=utf8")
                .unwrap(),
        })
        // .with_header(tiny_http::Header {
        //     field: "Cache-Control".parse().unwrap(),
        //     value: AsciiString::from_ascii("public, max-age=31536000, immutable")
        //         .unwrap(),
        // })
        .with_header(tiny_http::Header {
            field: "ETag".parse().unwrap(),
            value: AsciiString::from_ascii(format!("{:x}", hash))
                .unwrap(),
        })
}

pub fn uri_htmx() -> Response<Cursor<Vec<u8>>> {
    let hash = md5::compute(HTMX);

    tiny_http::Response::from_string(HTMX)
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
    let hash = md5::compute(LOGO);

    tiny_http::Response::from_data(LOGO)
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

pub fn uri_wfmlogo() -> Response<Cursor<Vec<u8>>> {
    let hash = md5::compute(WFMLOGO);

    tiny_http::Response::from_data(WFMLOGO)
        .with_header(tiny_http::Header {
            field: "Content-Type".parse().unwrap(),
            value: AsciiString::from_ascii("image/ico")
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
