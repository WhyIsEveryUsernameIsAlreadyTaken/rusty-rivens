use core::fmt;
use std::{any::type_name, sync::{Arc, Mutex}, time::{Duration, SystemTime}};

use http::{HeaderMap, Method, StatusCode};
use reqwest::Client;
use serde::de::DeserializeOwned;
use serde_json::{json, Value};
use url::Url;

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
        f.write_str(&format!("{} {:?}", self.location, self.err))
    }
}

impl std::error::Error for GenericError {}

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

#[derive(Debug)]
pub struct StatusError(StatusCode);

impl std::error::Error for StatusError {}

impl fmt::Display for StatusError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(&format!("{:?}", self))
    }
}

impl WFMClient {
    pub fn new(auth: Arc<Mutex<AuthState>>) -> Self {
        WFMClient {
            endpoint: "https://api.warframe.market/v1/".to_string(),
            limiter: Arc::new(tokio::sync::Mutex::new(RateLimiter::new(
                1.0,
                Duration::new(1, 0),
            ))),
            auth,
        }
    }

    async fn send_request<T: DeserializeOwned>(
        &self,
        method: &Method,
        url: &str,
        payload_key: Option<&str>,
        body: Option<Value>
    ) -> Result<ApiResult<T>, GenericError> {
        let auth = match self.auth.lock() {
            Ok(v) => v.clone(),
            Err(err) => return Err(GenericError::new(err, "send_request: ".to_string()))
        };
        let mut rate_limiter = self.limiter.lock().await;

        rate_limiter.wait_for_token().await;

        let client = Client::new();
        let new_url = format!("{}{}", self.endpoint, url);
        let request = client
            .request(method.clone(), Url::parse(&new_url).unwrap())
            .header(
                "Authorization",
                format!("JWT {}", auth.access_token.unwrap_or("".to_string())),
            )
            .header(
                "User-Agent",
                "Rusty Rivens v0.1",
            )
            .header("Language", "en");

        let request = match body.clone() {
            Some(content) => request.json(&content),
            None => request,
        };

        let now = SystemTime::now();
        let response = request.send()
            .await
            .map_err(|e| GenericError::new(e, "send_request: ".to_string()))?;
        let elap = now.elapsed().unwrap();
        println!("Response Time: {}.{}", elap.as_secs(), elap.as_millis());
        let status = response.status();
        let headers = response.headers().clone();
        let content = response.text().await.unwrap_or_default();
        let response: Value = serde_json::from_str(content.as_str())
            .map_err(|e| GenericError::new(e, "send_request: ".to_string()))?;

        let data: Value;
        match payload_key {
            Some(key) => {
                data = response["payload"][key].clone();
                match serde_json::from_value::<T>(data) {
                    Ok(payload_final) => Ok(ApiResult{
                        res: (payload_final, headers),
                        status
                    }),
                    Err(e) => Err(GenericError::new(e, "send_request: ".to_string()))
                }
            }
            None => Err(GenericError::new(format!("Could not return {}", type_name::<T>()),
                "send_request: ".to_string()
            ))
        }
    }

    pub async fn login(&self, email: String, password: String) -> Result<AuthState, GenericError> {
        let url = "/auth/signin";
        let method = Method::POST;
        let body = json!({
        "email": email,
        "password": password,
        });

        let (mut user, headers): (AuthState, HeaderMap) = match self
            .send_request::<AuthState>(&method, url, Some("user"), Some(body)).await
        {
            Ok(v) => {
                println!("{} {}: {}", method, url, v.status);
                if v.status != StatusCode::OK {
                    return Err(GenericError::new(StatusError(v.status), "login: ".to_string()))
                }
                v.res
            },
            Err(e) => return Err(GenericError::new(e, "login: ".to_string()))
        };
        if let Some(cookie_header) = headers.get("set-cookie") {
            let cookies = cookie_header.to_str().map_err(|e| GenericError::new(e, "login: ".to_string()))?;
            let token: Option<String> = Some(cookies[4..].split_once(';').unwrap_or(("","")).0.to_string());
            user.access_token = token;
            user.update().map_err(|e| e.prop("login: ".to_string()))?;
        } else {
            panic!("No access token returned!");
        };
        Ok(user)
        // Err(GenericError::new("test error".to_string(), "login: ".to_string()))
    }
}
