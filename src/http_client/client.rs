use core::fmt;
use std::{
    fmt::Display,
    io::ErrorKind,
    ops::Deref,
    str::FromStr,
    sync::Arc,
    time::{Duration, SystemTime},
};

use async_channel::{Receiver, RecvError, SendError as SError, Sender, TryRecvError};
use async_native_tls::{Certificate, TlsConnector, TlsStream};
use async_net::TcpStream;
use async_task::Task;
use futures_lite::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use once_cell::sync::OnceCell;
use serde_json::Value;
use smol_timeout::TimeoutExt;

use crate::{http_client::{qf_client::TEST_QF_STOPPED, wfm_client::TEST_WFM_STOPPED}, rate_limiter::RateLimiter, AppError, STOPPED};

use super::auth_state::AuthState;

#[derive(Debug, PartialEq)]
struct Header(Box<str>, Box<str>);
#[derive(Debug)]
pub struct Headers(Vec<Header>);

pub static TLS_CONNCECTOR: OnceCell<TlsConnector> = OnceCell::new();
static WFM_HANDLE: OnceCell<(ClientHandle, Receiver<Response>)> = OnceCell::new();
static QF_HANDLE: OnceCell<(ClientHandle, Receiver<Response>)> = OnceCell::new();

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
    pub status: StatusCode,
}

#[derive(Debug)]
pub struct StatusError {
    pub status: StatusCode,
}

#[derive(Debug)]
pub(crate) struct StatusCode {
    pub(crate) code: u16,
    pub(crate) text: Arc<str>,
}

impl Clone for StatusCode {
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
    uri: Option<Arc<str>>,
    headers: Headers,
    body: Option<Value>,
    timeout: Duration,
}

#[derive(Debug)]
struct Response {
    status: StatusCode,
    headers: Headers,
    body: Option<Arc<str>>,
}

impl Response {
    fn status(&self) -> StatusCode {
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
                Header("Connection".into(), "keep-alive".into()),
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
    SenderNone,
    UriNone,
    MethodNone,
    RequestTimeout(Duration),
    IoError(futures_lite::io::Error),
    MalformedResponse((Arc<str>, Arc<str>)),
    HttpNotSupported(Arc<str>),
    Recv(RecvError),
    TryRecv(TryRecvError),
    ChanSendError(SError<RequestBuilder>),
}

impl Display for SendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SendError::SenderNone => f.write_str("Sender part of channel not initialized"),
            SendError::UriNone => f.write_str("No uri provided"),
            SendError::MethodNone => f.write_str("No method provided"),
            SendError::RequestTimeout(to) => {
                f.write_str(format!("request timed out in {} seconds", to.as_secs()).as_str())
            }
            SendError::IoError(e) => f.write_str(format!("{e}").as_str()),
            SendError::MalformedResponse((from, raw_str)) => {
                f.write_str(format!("MalformedResponse {from}: {raw_str}").as_str())
            }
            SendError::HttpNotSupported(v) => {
                f.write_str(format!("Http addresses not supported: {v}").as_str())
            }
            SendError::Recv(e) => f.write_str(format!("RecvError: {}", e.to_string()).as_str()),
            SendError::TryRecv(e) => {
                f.write_str(format!("TryRecvError: {}", e.to_string()).as_str())
            }
            SendError::ChanSendError(e) => {
                f.write_str(format!("ChanSendError: {}", e.to_string()).as_str())
            }
        }
    }
}

impl std::error::Error for SendError {}

async fn collect_body_chunk(
    out: &mut String,
    size: usize,
    reader: &mut BufReader<&mut TlsStream<TcpStream>>,
) -> Result<(), SendError> {
    let mut buf = vec![0; size];
    reader
        .read_exact(&mut buf)
        .await
        .map_err(|e| SendError::IoError(e))?;
    let buf = String::from_utf8(buf).unwrap();
    let buf = buf.as_str();
    out.push_str(buf);
    Ok(())
}

impl RequestBuilder {
    async fn send(&mut self, stream: &mut TlsStream<TcpStream>) -> Result<Response, SendError> {
        let req = self.build_request()?;

        let mut out = String::new();
        stream
            .write_all(req.as_bytes())
            .await
            .map_err(|e| SendError::IoError(e))?;
        let mut reader = BufReader::new(stream);
        if let Some(v) = reader.read_line(&mut out).timeout(self.inner.timeout).await {
            v.map_err(|e| SendError::IoError(e))?;
        } else {
            return Err(SendError::RequestTimeout(self.inner.timeout));
        }
        loop {
            let mut line = String::new();
            if reader
                .read_line(&mut line)
                .await
                .map_err(|e| SendError::IoError(e))?
                < 3
            {
                out.push_str(line.as_str());
                break;
            }
            out.push_str(line.as_str());
        }

        let mut content = out.split("\r\n");

        if content
            .find(|&v| v.contains("Transfer-Encoding: chunked"))
            .is_some()
        {
            loop {
                let mut line = String::new();
                reader
                    .read_line(&mut line)
                    .await
                    .map_err(|e| SendError::IoError(e))?;
                if line.trim().is_empty() {
                    continue;
                }
                let size = usize::from_str_radix(line.trim(), 16).unwrap();
                if size == 0 {
                    break;
                }
                collect_body_chunk(&mut out, size, &mut reader).await?;
            }
        } else if let Some(header) = out.split("\r\n").find(|&v| v.contains("Content-Length")) {
            let (_, val) = match header.split_once(": ") {
                Some(v) => v,
                None => {
                    return Err(SendError::MalformedResponse((
                        "from splitting header".into(),
                        header.into(),
                    )))
                }
            };

            let size = val.parse::<usize>().map_err(|_| {
                SendError::MalformedResponse((
                    format!("from parsing Content-Length: {}", header).into(),
                    out.clone().into(),
                ))
            })?;
            collect_body_chunk(&mut out, size, &mut reader).await?;
        } else {
            return Err(SendError::MalformedResponse((
                "No/Malformed Content-Length/Transfer-Encoding Header".into(),
                out.into(),
            )));
        };

        let out = out.as_str();

        build_response(out)
    }

    fn build_request(&mut self) -> Result<String, SendError> {
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
            self.inner.headers.0.push(Header(
                "Content-Length".into(),
                format!("{content_length}").into(),
            ));
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
            .headers
            .0
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
        // println!("{req}");
        Ok(req)
    }
}

fn build_response(out: &str) -> Result<Response, SendError> {
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
    let status = StatusCode {
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
    Ok(Response {
        status,
        headers,
        body,
    })
}

impl RequestBuilder {
    pub(crate) fn new() -> Self {
        Self {
            inner: Default::default(),
        }
    }

    pub(crate) fn method(mut self, method: Method) -> Self {
        self.inner.method = Some(method);
        self
    }

    pub(crate) fn get_method(&self) -> Option<Method> {
        self.inner.method.clone()
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

struct ClientHandleInner {
    host: Option<Arc<str>>,
    port: Option<u16>,
    timeout: Option<Duration>,
}

enum ClientType {
    QF,
    WFM,
}

impl Clone for ClientHandleInner {
    fn clone(&self) -> Self {
        Self {
            host: self.host.clone(),
            port: self.port.clone(),
            timeout: self.timeout.clone(),
        }
    }
}

struct ClientHandle {
    handle: Option<Task<Result<(), ConnectionError>>>,
    sender: Option<async_channel::Sender<RequestBuilder>>,
    inner: ClientHandleInner,
}

#[derive(Debug)]
enum ConnectionError {
    ConnectionTimeout(Duration),
    IoError(futures_lite::io::Error),
    TlsError(async_native_tls::Error),
    HttpNotSupported(Arc<str>),
    TlsConnectorNone,
    HostNone,
    PortNone,
    TimeoutNone,
    ChanSendError(SError<Response>),
    SendError(SendError),
}

impl Default for ClientHandleInner {
    fn default() -> Self {
        Self {
            host: None,
            port: None,
            timeout: None,
        }
    }
}

impl Display for ConnectionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IoError(e) => f.write_str(format!("{e}").as_str()),
            Self::ConnectionTimeout(to) => {
                f.write_str(format!("connection timed out in {} seconds", to.as_secs()).as_str())
            }
            Self::TlsError(e) => f.write_str(format!("TlsError: {e}").as_str()),
            Self::HttpNotSupported(v) => {
                f.write_str(format!("Http addresses not supported: {v}").as_str())
            }
            Self::TlsConnectorNone => f.write_str("TLS connector not instantiated"),
            Self::HostNone => f.write_str("None not instantiated"),
            Self::PortNone => f.write_str("Port not instantiated"),
            Self::TimeoutNone => f.write_str("Timeout not instantiated"),
            Self::ChanSendError(e) => {
                f.write_str(format!("ChanSendError: {}", e.to_string()).as_str())
            }
            Self::SendError(e) => f.write_str(e.to_string().as_str()),
        }
    }
}

impl std::error::Error for ConnectionError {}

impl ClientHandle {
    fn new() -> Self {
        Self {
            handle: None,
            sender: None,
            inner: Default::default(),
        }
    }
    fn start_client(
        mut self,
        receiver: Receiver<RequestBuilder>,
        sender: Sender<Response>,
        test_ctype: Option<ClientType>,
    ) -> Self {
        self.handle = Some(smolscale::spawn(handle(
            self.inner.clone(),
            receiver,
            sender,
            test_ctype,
        )));
        self
    }

    async fn send(
        &self,
        req: RequestBuilder,
        receiver: &Receiver<Response>,
    ) -> Result<Response, SendError> {
        if self.sender.is_none() {
            return Err(SendError::SenderNone);
        };
        self.sender
            .as_ref()
            .unwrap()
            .send(req)
            .await
            .map_err(|e| SendError::ChanSendError(e))?;
        let res = receiver.recv().await.map_err(|e| SendError::Recv(e));
        res
    }

    fn send_channel(mut self, sender: Sender<RequestBuilder>) -> Self {
        self.sender = Some(sender);
        self
    }

    pub fn addr(mut self, uri: &str) -> Result<Self, ConnectionError> {
        let https = uri.contains("https://");
        let host = if https {
            match uri.split_once("https://") {
                Some((_, v)) => v.into(),
                None => uri,
            }
        } else {
            return Err(ConnectionError::HttpNotSupported(uri.into()));
        };
        let host = match host.split_once("/") {
            Some((v, _)) => v.into(),
            None => host,
        };
        self.inner.host = Some(host.into());
        Ok(self)
    }

    pub fn port(mut self, port: u16) -> Self {
        self.inner.port = Some(port);
        self
    }

    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.inner.timeout = Some(timeout);
        self
    }
}

async fn handle(
    inner: ClientHandleInner,
    receiver: Receiver<RequestBuilder>,
    sender: Sender<Response>,
    test_ctype: Option<ClientType>,
) -> Result<(), ConnectionError> {
    let tls = async_native_tls::TlsConnector::new().add_root_certificate(
        Certificate::from_pem(include_bytes!("../../certificate.pem")).unwrap(),
    );

    let _ = TLS_CONNCECTOR.set(tls);
    if inner.host.is_none() {
        return Err(ConnectionError::HostNone).unwrap();
    }
    if inner.port.is_none() {
        return Err(ConnectionError::PortNone).unwrap();
    }
    let addr: Arc<str> = format!(
        "{}:{}",
        inner.host.clone().unwrap().deref(),
        inner.port.unwrap()
    )
    .into();
    if inner.timeout.is_none() {
        return Err(ConnectionError::TimeoutNone).unwrap();
    }

    println!("Connecting to {addr}");
    let mut tstream = connect(&inner, addr.clone()).await.unwrap();

    // i love this btw
    loop {
        if cfg!(test) {
            match test_ctype.as_ref().unwrap() {
                ClientType::QF => {
                    if TEST_QF_STOPPED.get().is_some() {
                        tstream
                            .close()
                            .await
                            .map_err(|e| ConnectionError::IoError(e))
                            .unwrap();
                        println!("Connection Closed for {addr}");
                        return Ok(());
                    }
                }
                ClientType::WFM => {
                    if TEST_WFM_STOPPED.get().is_some() {
                        tstream
                            .close()
                            .await
                            .map_err(|e| ConnectionError::IoError(e))
                            .unwrap();
                        println!("Connection Closed for {addr}");
                        return Ok(());
                    }
                }
            }
        }
        if STOPPED.get().is_some() {
            tstream
                .close()
                .await
                .map_err(|e| ConnectionError::IoError(e))
                .unwrap();
            println!("Connection Closed for {addr}");
            return Ok(());
        }
        let mut request = if let Ok(req) = receiver
            .try_recv()
            .map_err(|e| ConnectionError::SendError(SendError::TryRecv(e)))
        {
            req
        } else {
            continue;
        };
        let resp = match request.send(&mut tstream).await {
            Ok(v) => v,
            Err(e) => match &e {
                SendError::IoError(ie) => match ie.kind() {
                    ErrorKind::WriteZero => {
                        tstream
                            .close()
                            .await
                            .map_err(|e| ConnectionError::IoError(e))
                            .unwrap();
                        println!("Connection Closed for {addr}");
                        println!("Reconnecting to {addr}");
                        tstream = connect(&inner, addr.clone()).await.unwrap();
                        request
                            .send(&mut tstream)
                            .await
                            .map_err(|e| ConnectionError::SendError(e))
                    }
                    _ => panic!("{e}"),
                },
                _ => panic!("{e}"),
            }?,
        };
        sender
            .send(resp)
            .await
            .map_err(|e| ConnectionError::ChanSendError(e))
            .unwrap();
    }
}

async fn connect(
    inner: &ClientHandleInner,
    addr: Arc<str>,
) -> Result<TlsStream<TcpStream>, ConnectionError> {
    if let Some(stream) = async_net::TcpStream::connect(addr.deref())
        .timeout(inner.timeout.unwrap())
        .await
    {
        let stream = stream.map_err(|e| ConnectionError::IoError(e))?;
        let tls = TLS_CONNCECTOR.get();
        if tls.is_none() {
            return Err(ConnectionError::TlsConnectorNone);
        };
        let resp = tls
            .unwrap()
            .connect(inner.host.clone().unwrap().deref(), stream)
            .await
            .map_err(|e| ConnectionError::TlsError(e));
        resp
    } else {
        return Err(ConnectionError::ConnectionTimeout(inner.timeout.unwrap()));
    }
}

pub trait HttpClient<'a> {
    async fn send_request(
        &self,
        method: Method,
        uri: &str,
        rate_limiter: &mut Option<RateLimiter>,
        auth: Option<AuthState>,
        body: Option<Value>,
    ) -> Result<ApiResult, AppError> {
        if let Some(rate_limiter) = rate_limiter {
            rate_limiter.wait_for_token().await;
        }
        let request = RequestBuilder::new().uri(uri).method(method.clone());
        // .header(
        //     "Authorization",
        //     format!("JWT {}", auth.access_token.clone().unwrap_or("".into())),
        // )
        // .timeout(Duration::from_secs(10));
        let request = match auth {
            Some(auth) => request
                .header(Header(
                    "Authorization".into(),
                    format!("JWT {}", &auth.access_token).into(),
                ))
                .map_err(|e| {
                    AppError::new(e.to_string(), "send_request: request.header".to_string())
                })?,
            None => request
                .header(Header("Authorization".into(), "JWT ".into()))
                .map_err(|e| {
                    AppError::new(e.to_string(), "send_request: request.header".to_string())
                })?,
        };
        let request = match body {
            Some(content) => request.body(content),
            None => request,
        };

        let (http_client, response_receiver) = if uri.contains("api.warframe.market") {
            WFM_HANDLE.get_or_try_init(|| -> Result<(ClientHandle, Receiver<_>), _> {
                let (sender, receiver) = async_channel::bounded::<RequestBuilder>(1);
                let (sender2, receiver2) = async_channel::bounded::<Response>(1);
                let ctype = if cfg!(test) {
                    Some(ClientType::WFM)
                } else {
                    None
                };
                Ok((
                    ClientHandle::new()
                        .port(443)
                        .addr(uri)
                        .map_err(|e| AppError::new(e.to_string(), "send_request".to_string()))?
                        .timeout(Duration::from_secs(5))
                        .send_channel(sender)
                        .start_client(receiver, sender2, ctype),
                    receiver2,
                ))
            })?
        } else if uri.contains("api.quantframe.app") {
            QF_HANDLE.get_or_try_init(|| -> Result<(ClientHandle, Receiver<_>), _> {
                let (sender, receiver) = async_channel::bounded::<RequestBuilder>(1);
                let (sender2, receiver2) = async_channel::bounded::<Response>(1);
                let ctype = if cfg!(test) {
                    Some(ClientType::WFM)
                } else {
                    None
                };
                Ok((
                    ClientHandle::new()
                        .port(443)
                        .addr(uri)
                        .map_err(|e| AppError::new(e.to_string(), "send_request".to_string()))?
                        .timeout(Duration::from_secs(5))
                        .send_channel(sender)
                        .start_client(receiver, sender2, ctype),
                    receiver2,
                ))
            })?
        } else {
            return Err(AppError::new(
                format!("unknown host with associated request: {uri}"),
                "send_request".to_string(),
            ));
        };

        let start = SystemTime::now();

        let response = http_client
            .send(request, response_receiver)
            .await
            .map_err(|e| AppError::new(e.to_string(), "send_request".into()))?;

        let status = response.status();
        println!(
            "{} {} {} {} in {:.2}s",
            method,
            uri,
            status.code,
            &status.text,
            SystemTime::now()
                .duration_since(start)
                .unwrap()
                .as_secs_f32()
        );
        if status.code == 429 {
            if let Some(rate_limiter) = rate_limiter {
                rate_limiter.add_delay(1.0);
            }
        }
        let headers = response.headers();
        let content = response.body().unwrap_or_default();

        if content == "".into() {
            return Ok(ApiResult {
                res: (None, headers),
                status,
            });
        }
        let response = serde_json::Value::from_str(&content).map_err(|e| {
            AppError::new(e.to_string(), String::from("send_request: Value::from_str"))
        });
        if response.is_err() {
            println!("Response body: {content}");
        }
        let response = response?;

        Ok(ApiResult {
            res: (Some(response), headers),
            status,
        })
    }
}
