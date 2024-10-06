use std::io::{self, Cursor};
use ascii::AsciiString;
use tiny_http::{Request, Response};

use crate::file_consts::{HTMX, LOGO, WFMLOGO, STYLES};

pub fn uri_styles(rq: Request) -> io::Result<()> {
    let hash = md5::compute(STYLES);

    rq.respond(tiny_http::Response::from_string(STYLES)
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
        }))
}

pub fn uri_htmx(rq: Request) -> io::Result<()> {
    let hash = md5::compute(HTMX);

    rq.respond(tiny_http::Response::from_string(HTMX)
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
        }))
}

pub fn uri_logo(rq: Request) -> io::Result<()> {
    let hash = md5::compute(LOGO);

    rq.respond(tiny_http::Response::from_data(LOGO)
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
        }))
}

pub fn uri_wfmlogo(rq: Request) -> io::Result<()> {
    let hash = md5::compute(WFMLOGO);

    rq.respond(tiny_http::Response::from_data(WFMLOGO)
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
        }))
}
