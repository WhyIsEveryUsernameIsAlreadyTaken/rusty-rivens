use core::fmt;
use std::{
    ops::{Deref, DerefMut},
    sync::Arc,
    time::{Duration, SystemTime},
};

use futures::lock::Mutex;
use http::{HeaderMap, Method, StatusCode};
use reqwest::Client;
use serde_json::{json, Value};
use url::Url;

use crate::{
    auth_state::AuthState, rate_limiter::RateLimiter, rivens::wfm_auctions::Auction, AppError,
};

#[derive(Clone, Debug)]
pub struct WFMClient {
    endpoint: String,
    limiter: Arc<Mutex<RateLimiter>>,
    pub auth: Arc<Mutex<AuthState>>,
}

#[derive(Debug)]
pub struct ApiResult {
    pub res: (Option<Value>, HeaderMap),
    status: StatusCode,
}

#[derive(Debug)]
pub struct StatusError {
    status: StatusCode,
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

impl WFMClient {
    pub fn new(auth: Arc<Mutex<AuthState>>) -> Self {
        WFMClient {
            endpoint: String::from("https://api.warframe.market/v1/"),
            limiter: Arc::new(Mutex::new(RateLimiter::new(1.0, Duration::new(1, 0)))),
            auth,
        }
    }

    pub async fn send_request(
        &self,
        method: &Method,
        url: &str,
        body: Option<Value>,
    ) -> Result<ApiResult, AppError> {
        let auth = self.auth.lock().await;
        let auth = auth.deref();
        let mut rate_limiter = self.limiter.lock().await;
        let rate_limiter = rate_limiter.deref_mut();

        rate_limiter.wait_for_token().await;

        let client = Client::new();
        let new_url = format!("{}{}", self.endpoint, url);
        let request = client
            .request(method.clone(), Url::parse(&new_url).unwrap())
            .header(
                "Authorization",
                format!("JWT {}", auth.access_token.clone().unwrap_or("".into())),
            )
            .header("User-Agent", "Rusty Rivens v0.1")
            .header("Language", "en")
            .timeout(Duration::from_secs(10));

        let request = match body.clone() {
            Some(content) => request.json(&content),
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
        let headers = response.headers().clone();
        let content = response.text().await.unwrap_or_default();

        if content == "".to_string() {
            return Ok(ApiResult {
                res: (None, headers),
                status,
            });
        }
        let response: Value = serde_json::from_str(content.as_str())
            .map_err(|e| AppError::new(e.to_string(), String::from("send_request")))?;

        Ok(ApiResult {
            res: (Some(response), headers),
            status,
        })
    }

    pub async fn login(&self, email: &str, password: &str) -> Result<StatusCode, AppError> {
        let url = "/auth/signin";
        let method = Method::POST;
        let body = json!({
        "email": email,
        "password": password,
        });
        let response = match self.send_request(&method, url, Some(body)).await {
            Ok(v) => v,
            Err(e) => return Err(AppError::new(e.to_string(), String::from("login: "))),
        };
        let (val, headers) = response.res;
        let token: Option<Arc<str>>;
        if let Some(cookie_header) = headers.get("set-cookie") {
            let cookies = cookie_header
                .to_str()
                .map_err(|e| AppError::new(e.to_string(), String::from("login: ")))?;
            token = Some(cookies[4..].split_once(';').unwrap_or(("", "")).0.into());
        } else {
            panic!("No access token returned!");
        };
        let mut user = AuthState::default();
        if response.status == StatusCode::OK {
            if let Some(v) = val {
                let data = v["payload"]["user"].clone();
                user = serde_json::from_value(data).map_err(|e| {
                    AppError::new(
                        e.to_string(),
                        String::from("login: from_value::<AuthState>"),
                    )
                })?;
                user.access_token = token;
                user.update().map_err(|e| e.prop("login: ".into()))?;
            }
        }
        self.auth.clone().try_lock().unwrap().deref_mut().set(user);
        Ok(response.status)
    }

    pub async fn get_all_rivens(&self) -> Result<Vec<Auction>, AppError> {
        let url = "/profile/auctions";
        let method = Method::GET;

        let (body_value, _) = match self.send_request(&method, url, None).await {
            Ok(v) => {
                println!("{} {}: {}", method, url, v.status);
                if v.status != StatusCode::OK {
                    return Err(AppError::new(
                        StatusError { status: v.status }.to_string(),
                        String::from("get_all_rivens: "),
                    ));
                }
                v.res
            }
            Err(e) => return Err(AppError::new(e.to_string(), String::from("get_all_rivens"))),
        };
        match body_value {
            Some(v) => {
                let data = v["payload"]["auctions"].clone();
                serde_json::from_value::<Vec<Auction>>(data).map_err(|e| {
                    AppError::new(
                        e.to_string(),
                        String::from("get_all_rivens: from_value::<Vec<Auction>>"),
                    )
                })
            }
            None => Err(AppError::new(
                String::from("No response body associated with response"),
                String::from("get_all_rivens"),
            )),
        }
    }
}
