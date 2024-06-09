use core::fmt;
use std::{error::Error, fmt::Display, sync::{Arc, Mutex}, time::Duration};

use http::{HeaderMap, Method, StatusCode};
use serde::de::DeserializeOwned;
use serde_json::{json, Value};

use crate::{auth_state::AuthState, rate_limiter::RateLimiter};

#[derive(Clone, Debug)]
pub struct WFMClient {
    endpoint: String,
    limiter: Arc<tokio::sync::Mutex<RateLimiter>>,
    auth: Arc<Mutex<AuthState>>,
}

#[derive(Debug)]
pub struct GenericError {
    location: String,
    err: String,
}

impl fmt::Display for GenericError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(&format!("{} {}", self.location, self.err))
    }
}

impl GenericError {
    pub fn new<T: std::fmt::Display>(err: T, loc: String) -> Self {
        GenericError { location: loc, err: format!("{}", err) }
    }
    pub fn prop(&self, new_loc: String) -> GenericError {
        GenericError { location: format!("{}{}", new_loc, self.location), err: self.err.clone() }
    }
}

#[derive(Debug)]
pub struct ApiResult<T> {
    res: (T, HeaderMap),
    status: StatusCode
}

impl WFMClient {
    pub fn new() -> Self {
        WFMClient {
            endpoint: "https://api.warframe.market/v1/".to_string(),
            limiter: Arc::new(tokio::sync::Mutex::new(RateLimiter::new(
                1.0,
                Duration::new(1, 0),
            ))),
        }
    }

    async fn send_request<T: DeserializeOwned>(
        &self,
        method: Method,
        url: &str,
        payload_key: Option<&str>,
        body: Option<Value>
    ) -> Result<ApiResult<T>, Box<dyn Error>> {
        let auth = self.auth.lock()?.clone();
        let mut rate_limiter = self.limiter.lock().await;

        rate_limiter.wait_for_token().await;
        todo!()
    }

    pub async fn login(&self, email: String, password: String) -> Result<AuthState, GenericError> {
        let body = json!({
        "email": email,
        "password": password,
        });

        let (mut user, headers): (AuthState, HeaderMap) = match self
            .send_request::<AuthState>(Method::POST, "/auth/signin", Some("user"), Some(body)).await
        {
            Ok(v) => v.res,
            Err(e) => return Err(GenericError::new(e, "login: ".to_string()))
        };
    }
}
