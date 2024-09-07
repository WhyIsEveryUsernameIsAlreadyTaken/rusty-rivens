use core::fmt;
use std::{
    fmt::Display,
    rc::Rc,
    str::FromStr,
    sync::Arc,
    time::{Duration, SystemTime},
};

use async_native_tls::TlsConnector;
use once_cell::sync::OnceCell;
use serde_json::Value;
use smol::io::{AsyncReadExt, AsyncWriteExt};
use smol_timeout::TimeoutExt;

use crate::{rate_limiter::RateLimiter, AppError};

use super::auth_state::AuthState;

#[derive(Debug, PartialEq)]
struct Header(Box<str>, Box<str>);
#[derive(Debug)]
struct Headers(Vec<Header>);

impl Headers {
    pub fn get(&self, key: &str) -> Option<Box<str>> {
        if let Some(header) = self.0.iter().find(|v| v.0 == key.into()) {
            Some(header.1.clone())
        } else {
            None
        }
    }
}

impl Clone for Header {
    fn clone(&self) -> Self {
        Self(self.0.clone(), self.1.clone())
    }
}

impl Clone for Headers {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

#[derive(Debug)]
pub struct ApiResult {
    pub res: (Option<Value>, Headers),
    pub status: Status,
}

#[derive(Debug)]
pub struct StatusError {
    pub status: Status,
}

#[derive(Debug)]
pub(crate) struct Status {
    pub(crate) code: u16,
    pub(crate) text: Arc<str>,
}

impl Clone for Status {
    fn clone(&self) -> Self {
        Self {
            code: self.code,
            text: self.text.clone(),
        }
    }
}

impl std::error::Error for StatusError {}

impl fmt::Display for StatusError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(&format!(
            "Request failed with status code: {:?}",
            self.status
        ))
    }
}

static TLS_CONNCECTOR: OnceCell<TlsConnector> = OnceCell::new();

pub enum Method {
    OPTIONS,
    GET,
    POST,
    PUT,
    DELETE,
    HEAD,
    TRACE,
    CONNECT,
    PATCH,
}

impl Display for Method {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OPTIONS => f.write_str("OPTIONS"),
            Self::GET => f.write_str("GET"),
            Self::POST => f.write_str("POST"),
            Self::PUT => f.write_str("PUT"),
            Self::DELETE => f.write_str("DELETE"),
            Self::HEAD => f.write_str("HEAD"),
            Self::TRACE => f.write_str("TRACE"),
            Self::CONNECT => f.write_str("CONNECT"),
            Self::PATCH => f.write_str("PATCH"),
        }
    }
}

impl Clone for Method {
    fn clone(&self) -> Self {
        match self {
            Self::OPTIONS => Self::OPTIONS,
            Self::GET => Self::GET,
            Self::POST => Self::POST,
            Self::PUT => Self::PUT,
            Self::DELETE => Self::DELETE,
            Self::HEAD => Self::HEAD,
            Self::TRACE => Self::TRACE,
            Self::CONNECT => Self::CONNECT,
            Self::PATCH => Self::PATCH,
        }
    }
}

struct Request {
    method: Option<Method>,
    uri: Option<Rc<str>>,
    headers: Headers,
    body: Option<Value>,
    timeout: Duration,
}

#[derive(Debug)]
struct Response {
    status: Status,
    headers: Headers,
    body: Option<Arc<str>>,
}

impl Response {
    fn status(&self) -> Status {
        self.status.clone()
    }
    fn body(&self) -> Option<Arc<str>> {
        self.body.clone()
    }
    fn headers(&self) -> Headers {
        self.headers.clone()
    }
}

impl Default for Request {
    fn default() -> Self {
        Self {
            method: None,
            uri: None,
            headers: Headers(vec![
                Header("User-Agent".into(), "Rusty Rivens v0.0.1".into()),
                Header("Connection".into(), "close".into()),
                Header("Accept".into(), "application/json".into()),
                Header("Content-Type".into(), "application/json".into()),
                Header("Accept-Language".into(), "en".into()),
            ]),
            body: None,
            timeout: Duration::from_secs(5),
        }
    }
}

struct RequestBuilder {
    inner: Request,
}

#[derive(Debug)]
struct HeaderInsertError(Header);

impl Display for HeaderInsertError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(format!("Header `{}` is already populated", self.0 .0).as_str())
    }
}

impl std::error::Error for HeaderInsertError {}

#[derive(Debug)]
enum SendError {
    UriNone,
    MethodNone,
    RequestTimeout(Duration),
    ConnectionTimeout(Duration),
    IoError(smol::io::Error),
    TlsError(async_native_tls::Error),
    MalformedResponse((Arc<str>, Arc<str>)),
    HttpNotSupported(Rc<str>),
    TlsConnectorNone,
}

impl Display for SendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SendError::UriNone => f.write_str("No uri provided"),
            SendError::MethodNone => f.write_str("No method provided"),
            SendError::RequestTimeout(to) => {
                f.write_str(format!("request timed out in {} seconds", to.as_secs()).as_str())
            }
            SendError::IoError(e) => f.write_str(format!("{e}").as_str()),
            SendError::MalformedResponse((from, raw_str)) => {
                f.write_str(format!("MalformedResponse {from}: {raw_str}").as_str())
            }
            SendError::ConnectionTimeout(to) => {
                f.write_str(format!("connection timed out in {} seconds", to.as_secs()).as_str())
            }
            SendError::TlsError(e) => f.write_str(format!("TlsError: {e}").as_str()),
            SendError::HttpNotSupported(v) => {
                f.write_str(format!("Http addresses not supported: {v}").as_str())
            }
            SendError::TlsConnectorNone => f.write_str("TLS connector not instantiated"),
        }
    }
}

impl std::error::Error for SendError {}

impl RequestBuilder {
    pub(crate) async fn send(mut self) -> Result<Response, SendError> {
        let (req, host) = self.build_request()?;
        let addr = format!("{}:443", host);

        let start = SystemTime::now();
        if let Some(stream) = smol::net::TcpStream::connect(addr)
            .timeout(self.inner.timeout).await
        {
            println!("Connection time: {}s", SystemTime::now().duration_since(start).unwrap().as_secs_f32());
            let stream = stream.map_err(|e| SendError::IoError(e))?;
            let tls = TLS_CONNCECTOR.get();
            if tls.is_none() {
                return Err(SendError::TlsConnectorNone);
            };
            let start = SystemTime::now();
            let mut stream = tls.unwrap()
                .connect(host, stream)
                .await
                .map_err(|e| SendError::TlsError(e))?;
            println!("Tls Connection time: {}s", SystemTime::now().duration_since(start).unwrap().as_secs_f32());
            let mut out = String::new();
            stream
                .write_all(req.as_bytes())
                .await
                .map_err(|e| SendError::IoError(e))?;
            println!("request sent");
            let start = SystemTime::now();
            if stream
                .read_to_string(&mut out)
                .timeout(self.inner.timeout)
                .await
                .is_none()
            {
                return Err(SendError::RequestTimeout(self.inner.timeout));
            };
            let end = SystemTime::now().duration_since(start);
            let out = out.as_str();

            let start = SystemTime::now();
            let (head, body) = match out.split_once("\r\n\r\n") {
                Some(v) => v,
                None => {
                    return Err(SendError::MalformedResponse((
                        "from splitting head and body".into(),
                        out.into(),
                    )))
                }
            };
            let mut lines = head.split("\r\n");
            let status_raw = match lines.next() {
                Some(v) => v,
                None => {
                    return Err(SendError::MalformedResponse((
                        "from polling status line".into(),
                        out.into(),
                    )))
                }
            };
            let (_, status_raw) = match status_raw.split_once(" ") {
                Some(v) => v,
                None => {
                    return Err(SendError::MalformedResponse((
                        "from splitting status line (version removal)".into(),
                        out.into(),
                    )))
                }
            };
            let (code, text) = match status_raw.split_once(" ") {
                Some(v) => v,
                None => {
                    return Err(SendError::MalformedResponse((
                        "from splitting status line (status pieces)".into(),
                        out.into(),
                    )))
                }
            };
            let code = code.parse().map_err(|e| {
                SendError::MalformedResponse((
                    format!("from parsing status code: {}", e).into(),
                    out.into(),
                ))
            })?;
            let status = Status {
                code,
                text: text.into(),
            };

            let headers: Headers = lines.try_fold(
                Headers(vec![]),
                |mut acc, raw_header| -> Result<Headers, SendError> {
                    let (key, val) = match raw_header.split_once(": ") {
                        Some(v) => v,
                        None => {
                            return Err(SendError::MalformedResponse((
                                "from splitting header".into(),
                                raw_header.into(),
                            )))
                        }
                    };
                    acc.0.push(Header(key.into(), val.into()));
                    Ok(acc)
                },
            )?;
            let body: Option<Arc<str>> = if body != "" { Some(body.into()) } else { None };
            println!("Response time: {}s", end.unwrap().as_secs_f32());
            println!("Processing time: {}s", SystemTime::now().duration_since(start).unwrap().as_secs_f32());
            Ok(Response {
                status,
                headers,
                body,
            })
        } else {
            Err(SendError::ConnectionTimeout(self.inner.timeout))
        }
    }

    fn build_request(&mut self) -> Result<(String, String), SendError> {
        if self.inner.uri.is_none() {
            return Err(SendError::UriNone);
        }
        let uri = self.inner.uri.clone().unwrap();
        if self.inner.method.is_none() {
            return Err(SendError::MethodNone);
        }
        let method = self.inner.method.as_ref().unwrap();
        let https = uri.contains("https://");
        let host = if https {
            match uri.clone().split_once("https://") {
                Some((_, v)) => v.into(),
                None => uri.clone(),
            }
        } else {
            return Err(SendError::HttpNotSupported(uri));
        };

        let host = match host.split_once("/") {
            Some((v, _)) => v.into(),
            None => host,
        };
        let body = match self.inner.body.as_ref() {
            Some(v) => v.to_string(),
            None => "".to_string(),
        };
        let body = body.as_str();
        let content_length = body.as_bytes().len();

        if content_length != 0 {
            self.inner
                .headers.0
                .push(Header("Content-Length".into(), format!("{content_length}").into()));
        }

        let method = match method {
            Method::OPTIONS => "OPTIONS",
            Method::GET => "GET",
            Method::POST => "POST",
            Method::PUT => "PUT",
            Method::DELETE => "DELETE",
            Method::HEAD => "HEAD",
            Method::TRACE => "TRACE",
            Method::CONNECT => "CONNECT",
            Method::PATCH => "PATCH",
        };

        let headers = self
            .inner
            .headers.0
            .iter()
            .fold(String::new(), |mut acc, header| {
                acc.push_str(format!("\r\n{}: {}", &header.0, &header.1).as_str());
                acc
            });
        let headers = headers.as_str();
        let req = format!(
            "{} {} HTTP/1.1\r\nHost: {}{}\r\n\r\n{}",
            method, &uri, host, headers, body
        );
        println!("{req}");
        Ok((req, host.to_string()))
    }
    pub(crate) fn new() -> Self {
        Self {
            inner: Default::default(),
        }
    }

    pub(crate) fn method(mut self, method: Method) -> Self {
        self.inner.method = Some(method);
        self
    }

    pub(crate) fn uri(mut self, uri: &str) -> Self {
        self.inner.uri = Some(uri.into());
        self
    }

    pub(crate) fn headers(mut self, headers: Vec<Header>) -> Self {
        let mut headers: Vec<Header> = headers
            .into_iter()
            .filter(|header| !self.inner.headers.0.contains(header))
            .collect();
        self.inner.headers.0.append(&mut headers);
        self
    }

    pub(crate) fn header(mut self, header: Header) -> Result<Self, HeaderInsertError> {
        if self.inner.headers.0.contains(&header) {
            return Err(HeaderInsertError(header));
        }
        self.inner.headers.0.push(header);
        Ok(self)
    }

    pub(crate) fn body(mut self, body: Value) -> Self {
        self.inner.body = Some(body);
        self
    }

    pub(crate) fn timeout(mut self, timeout: Duration) -> Self {
        self.inner.timeout = timeout;
        self
    }
}

pub trait HttpClient<'a> {
    async fn send_request(
        &self,
        method: Method,
        uri: &str,
        rate_limiter: &mut RateLimiter,
        auth: Option<&AuthState>,
        body: Option<Value>,
    ) -> Result<ApiResult, AppError> {
        rate_limiter.wait_for_token().await;
        let request = RequestBuilder::new().uri(uri).method(method);
        // .header(
        //     "Authorization",
        //     format!("JWT {}", auth.access_token.clone().unwrap_or("".into())),
        // )
        // .timeout(Duration::from_secs(10));
        let request = match auth {
            Some(auth) => request
                .header(Header(
                    "Authorization".into(),
                    format!("JWT {}", auth.access_token.clone().unwrap_or("".into())).into(),
                ))
                .map_err(|e| {
                    AppError::new(e.to_string(), "send_request: request.header".to_string())
                })?,
            None => request,
        };
        let request = match body {
            Some(content) => request.body(content),
            None => request,
        };

        let now = SystemTime::now();
        println!("request sent");
        let response = request
            .send()
            .await
            .map_err(|e| AppError::new(e.to_string(), "send_request".into()))?;
        println!("response received");
        let elap = now.elapsed().unwrap();
        println!("HTTP Response Time: {}secs", elap.as_secs_f32());
        let status = response.status();
        let headers = response.headers();
        let content = response.body().unwrap_or_default();

        if content == "".into() {
            return Ok(ApiResult {
                res: (None, headers),
                status,
            });
        }
        let response = serde_json::Value::from_str(&content).map_err(|e| {
            AppError::new(e.to_string(), String::from("Value::from_str: send_request"))
        })?;

        Ok(ApiResult {
            res: (Some(response), headers),
            status,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::time::SystemTime;

    use async_native_tls::Certificate;
    use serde::Serialize;
    use serde_json::to_value;

    use crate::http_client::client::TLS_CONNCECTOR;

    use super::{Method, RequestBuilder};
    #[derive(Serialize)]
    struct Foo {
        bar: String,
    }

    #[test]
    fn test_sendcustom() {
        let start = SystemTime::now();
        let tls = async_native_tls::TlsConnector::new().add_root_certificate(
            Certificate::from_pem(include_bytes!("certificate.pem")).unwrap(),
        );
        TLS_CONNCECTOR.set(tls).unwrap();
        let body = Foo {
            bar: "baz".to_string(),
        };
        let body = to_value(body).unwrap();
        let req = RequestBuilder::new()
            .method(Method::GET)
            .uri("https://api.warframe.market/v1/profile/toopsi");
        let resp = smol::block_on(async { req.send().await }).unwrap();
        //println!("{:#?}", resp)
    }
}
