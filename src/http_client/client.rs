use core::fmt;
use std::{
    borrow::{Borrow, BorrowMut}, convert::Infallible, fmt::Display, io::ErrorKind, ops::{Deref, DerefMut}, str::FromStr, sync::Arc, time::{Duration, SystemTime}
};

use serde_json::Value;
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    net::TcpStream,
    sync::{
        mpsc::{
            error::{RecvError, SendError as SError, TryRecvError},
            Receiver, Sender,
        },
        Mutex,
    },
    task::JoinHandle,
};
use tokio_rustls::{client::TlsStream, rustls::RootCertStore};

use crate::{AppError, STOPPED};

#[derive(Debug, PartialEq)]
pub struct Header(Arc<str>, Arc<str>);

impl FromStr for Header {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (key, val) = s.split_once(':').unwrap_or(("", ""));
        let val = val.trim();
        Ok(Header(key.into(), val.into()))
    }
}

#[derive(Debug)]
pub struct Headers(Vec<Header>);

impl Headers {
    pub fn get(&self, key: &str) -> Option<Arc<str>> {
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
pub struct StatusCode {
    pub code: u16,
    pub text: Arc<str>,
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

#[derive(Debug)]
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

#[derive(Debug, Clone)]
pub struct Request {
    method: Option<Method>,
    uri: Option<Arc<str>>,
    headers: Headers,
    body: Option<Value>,
    timeout: Duration,
}

impl Request {
    async fn send(&mut self, stream: &mut TlsStream<TcpStream>) -> Result<Response, SendError> {
        let req = self.build_request()?;
        let req = req.as_bytes();

        let mut out = String::new();
        stream
            .write_all(req)
            .await
            .map_err(|e| SendError::IoError(e))?;
        println!("{} bytes written for `{:?}`.", req.len(), self.uri);
        let mut reader = BufReader::new(stream);
        if let Ok(v) = tokio::time::timeout(self.timeout, reader.read_line(&mut out)).await {
            v.map_err(|e| SendError::IoError(e))?;
        } else {
            return Err(SendError::RequestTimeout(self.timeout));
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
                collect_payload_chunk(&mut out, size, &mut reader).await?;
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
            collect_payload_chunk(&mut out, size, &mut reader).await?;
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
        if self.uri.is_none() {
            return Err(SendError::UriNone);
        }
        let uri = self.uri.clone().unwrap();
        if self.method.is_none() {
            return Err(SendError::MethodNone);
        }
        let method = self.method.as_ref().unwrap();
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
        let body = match self.body.as_ref() {
            Some(v) => v.to_string(),
            None => "".to_string(),
        };
        let body = body.as_str();
        let content_length = body.as_bytes().len();

        if content_length != 0 {
            self.headers.0.push(Header(
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

#[derive(Debug, Clone)]
pub struct Response {
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

pub struct RequestBuilder {
    inner: Request,
}

#[derive(Debug)]
pub struct HeaderInsertError(Header);

impl Display for HeaderInsertError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(format!("Header `{}` is already populated", self.0 .0).as_str())
    }
}

impl std::error::Error for HeaderInsertError {}

#[derive(Debug)]
pub enum SendError {
    SenderNone,
    SenderClosed,
    UriNone,
    MethodNone,
    RequestTimeout(Duration),
    IoError(tokio::io::Error),
    MalformedResponse((Arc<str>, Arc<str>)),
    HttpNotSupported(Arc<str>),
    Recv,
    TryRecv(TryRecvError),
    ChanSendError(SError<Request>),
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
            SendError::Recv => f.write_str(format!("RecvError: ").as_str()),
            SendError::TryRecv(e) => {
                f.write_str(format!("TryRecvError: {}", e.to_string()).as_str())
            }
            SendError::ChanSendError(e) => {
                f.write_str(format!("ChanSendError: {}", e.to_string()).as_str())
            }
            SendError::SenderClosed => f.write_str("Sender part of channel closed prematurely"),
        }
    }
}

impl std::error::Error for SendError {}

async fn collect_payload_chunk(
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

impl Into<RequestBuilder> for Request {
    fn into(self) -> RequestBuilder {
        RequestBuilder { inner: self }
    }
}

impl RequestBuilder {
    pub fn new() -> Self {
        Self {
            inner: Default::default(),
        }
    }

    pub fn method(mut self, method: Method) -> Self {
        self.inner.method = Some(method);
        self
    }

    pub fn get_method(&self) -> Option<Method> {
        self.inner.method.clone()
    }

    pub fn uri(mut self, uri: &str) -> Self {
        self.inner.uri = Some(uri.into());
        self
    }

    pub fn headers(mut self, headers: Vec<Header>) -> Self {
        let mut headers: Vec<Header> = headers
            .into_iter()
            .filter(|header| !self.inner.headers.0.contains(header))
            .collect();
        self.inner.headers.0.append(&mut headers);
        self
    }

    pub fn header(mut self, header: Header) -> Self {
        if !self.inner.headers.0.contains(&header) {
            self.inner.headers.0.push(header);
        }
        self
    }

    pub fn body(mut self, body: Value) -> Self {
        self.inner.body = Some(body);
        self
    }

    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.inner.timeout = timeout;
        self
    }

    pub fn build(self) -> Request {
        self.inner
    }
}

#[derive(Debug)]
struct ClientHandleInner {
    host: Option<Arc<str>>,
    port: Option<u16>,
    timeout: Option<Duration>,
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

#[derive(Debug)]
pub struct ClientHandle {
    handle: Option<JoinHandle<Result<(), ConnectionError>>>,
    request_sender: Option<Sender<Request>>,
    response_receiver: Option<Receiver<Response>>,
    inner: ClientHandleInner,
}

#[derive(Debug)]
pub enum ConnectionError {
    ConnectionTimeout(Duration),
    IoError(tokio::io::Error),
    RustlsError(tokio_rustls::rustls::Error),
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
            Self::RustlsError(e) => f.write_str(format!("TlsError: {e}").as_str()),
            Self::HttpNotSupported(v) => {
                f.write_str(format!("Http addresses not supported: {v}").as_str())
            }
            Self::TlsConnectorNone => f.write_str("TLS connector not instantiated"),
            Self::HostNone => f.write_str("Host not instantiated"),
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
    pub fn new() -> Self {
        Self {
            handle: None,
            request_sender: None,
            response_receiver: None,
            inner: Default::default(),
        }
    }
    pub fn start_client(mut self, receiver: Receiver<Request>, sender: Sender<Response>) -> Self {
        self.handle = Some(tokio::task::spawn(handle(
            self.inner.clone(),
            receiver,
            sender,
        )));
        self
    }

    async fn send(
        &mut self,
        req: Request,
    ) -> Result<Response, SendError> {
        if self.request_sender.is_none() {
            return Err(SendError::SenderNone);
        };
        self.request_sender
            .as_ref()
            .unwrap()
            .send(req)
            .await
            .map_err(|e| SendError::ChanSendError(e))?;
        let mut receiver = self.response_receiver.take().expect("FATAL: Response Receiver dropped");
        let res = match receiver.recv().await {
            Some(v) => v,
            None => return Err(SendError::Recv),
        };
        self.response_receiver = Some(receiver);
        Ok(res)
    }

    pub fn send_channel(mut self, sender: Sender<Request>) -> Self {
        self.request_sender = Some(sender);
        self
    }

    pub fn receive_channel(mut self, receiver: Receiver<Response>) -> Self {
        self.response_receiver = Some(receiver);
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
    mut receiver: Receiver<Request>,
    sender: Sender<Response>,
) -> Result<(), ConnectionError> {
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
    let mut tstream = connect(&inner).await.unwrap();
    let sender = &sender;

    // nvm i dont like this anymore...
    // too much indentation...
    loop {
        if STOPPED.get().is_some() {
            drop(tstream);
            println!("Connection Closed for {addr}");
            return Ok(());
        }
        let mut request = if let Ok(req) = receiver
            .try_recv()
            .map_err(|e| ConnectionError::SendError(SendError::TryRecv(e)))
        {
            req
        } else {
            assert!(!sender.is_closed(), "FATAL Sender closed for response channel on {addr}");
            // println!("handle healty");
            if STOPPED.get().is_some() {
                drop(tstream);
                println!("Connection Closed for {addr}");
                return Ok(());
            }
            continue;
        };
        let resp = match request.send(&mut tstream).await {
            Ok(v) => v,
            Err(e) => match &e {
                SendError::IoError(ie) => match ie.kind() {
                    ErrorKind::WriteZero => {
                        println!("Connection Closed for {addr}");
                        println!("Reconnecting to {addr}");
                        tstream = connect(&inner).await.unwrap();
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
        sender.send(resp).await.expect("hello?");
        println!("sent response through channel");
    }
}

async fn connect(inner: &ClientHandleInner) -> Result<TlsStream<TcpStream>, ConnectionError> {
    let root_store = RootCertStore {
        roots: webpki_roots::TLS_SERVER_ROOTS.into(),
    };
    let config = tokio_rustls::rustls::ClientConfig::builder()
        .with_root_certificates(root_store)
        .with_no_client_auth();
    let host_str = inner.host.clone().unwrap();
    let host = host_str.deref().to_string().try_into().unwrap();
    let addr = format!("{}:{}", host_str, inner.port.unwrap());
    if let Ok(stream) =
        tokio::time::timeout(inner.timeout.unwrap(), tokio::net::TcpStream::connect(addr)).await
    {
        let stream = stream.map_err(|e| ConnectionError::IoError(e))?;
        let connector = tokio_rustls::TlsConnector::from(Arc::new(config));
        let resp = connector
            .connect(host, stream)
            .await
            .map_err(|e| ConnectionError::IoError(e));
        resp
    } else {
        return Err(ConnectionError::ConnectionTimeout(inner.timeout.unwrap()));
    }
}

pub type ArcClientHandle = Arc<Mutex<ClientHandle>>;

pub trait HttpClient {
    async fn sender_fn(
        &mut self,
        rq: RequestBuilder,
    ) -> Result<(ArcClientHandle, RequestBuilder), AppError>;
    async fn rate_limit(&self);
    async fn send_request(&mut self, rq: Request) -> Result<ApiResult, AppError> {
        let (client_handle, rq) = self
            .sender_fn(rq.into())
            .await
            .map_err(|e| e.prop("send_request".into()))?;
        let rq = rq.build();
        self.rate_limit().await;

        let start = SystemTime::now();

        let mut client_handle_mutex = client_handle.lock().await;
        let client_handle = client_handle_mutex.deref_mut();
        let method = match rq.method.clone() {
            Some(v) => v.to_string(),
            None => "NILMETHOD".to_string(),
        };
        let uri = match rq.uri.clone() {
            Some(v) => v,
            None => "NILURI".into(),
        };
        let response = client_handle
            .send(rq)
            .await
            .map_err(|e| AppError::new(e.to_string(), "send_request".into()))?;
        drop(client_handle_mutex);

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
            self.rate_limit().await;
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
        if status.code >= 400 {
            println!("Response body: {content}");
        }
        let response = response?;

        Ok(ApiResult {
            res: (Some(response), headers),
            status,
        })
    }
}
