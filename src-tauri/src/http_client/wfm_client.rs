use std::{
    ops::{Deref, DerefMut},
    sync::Arc,
    time::Duration,
};

use futures::lock::Mutex;
use http::{Method, StatusCode};
use serde_json::{from_value, json};

use crate::{
    http_client::client::StatusError, jwt::jwt_is_valid, rate_limiter::RateLimiter, rivens::inventory::convert_raw_inventory::Auction, AppError
};

use super::{auth_state::AuthState, client::HttpClient};

#[derive(Clone, Debug)]
pub struct WFMClient {
    endpoint: String,
    limiter: Arc<Mutex<RateLimiter>>,
    pub auth: Arc<Mutex<AuthState>>,
}

impl HttpClient for WFMClient {}

impl WFMClient {
    pub fn new(auth: Arc<Mutex<AuthState>>) -> Self {
        WFMClient {
            endpoint: String::from("https://api.warframe.market/v1/"),
            limiter: Arc::new(Mutex::new(RateLimiter::new(1.0, Duration::new(1, 0)))),
            auth,
        }
    }

    pub async fn login(&self, email: &str, password: &str) -> Result<StatusCode, AppError> {
        let body = json!({
        "email": email,
        "password": password,
        });
        let auth = self.auth.lock().await;
        let auth = auth.deref();
        let mut rate_limiter = self.limiter.lock().await;
        let rate_limiter = rate_limiter.deref_mut();
        let response = match self.send_request(
            Method::POST,
            &format!("{}{}", self.endpoint, "/auth/signin"),
            rate_limiter,
            Some(auth),
            Some(body)
        ).await {
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

    pub async fn validate(
        &self,
    ) -> Result<bool, AppError> {
        let auth = self.auth.lock().await;
        let auth = auth.deref();
        let valid_jwt: bool;
        if let Some(token) = auth.clone().access_token {
            valid_jwt = jwt_is_valid(&token).map_err(|e| e.prop("validate".into()))?;
        } else {
            return Ok(false);
        }
        if !valid_jwt {
            return Ok(false);
        }
        let mut rate_limiter = self.limiter.lock().await;
        let rate_limiter = rate_limiter.deref_mut();
        let res = self.send_request(
            Method::GET,
            "profile",
            rate_limiter,
            Some(auth),
            None,
        ).await;
        let (body, _) = match res {
            Ok(v) => v.res,
            Err(e) => return Err(e.prop("validate".into())),
        };
        let mut is_valid = false;
        if let Some(body) = body {
            let value = body["profile"].clone();
            let anonymous = from_value::<bool>(value["anonymous"].clone()).map_err(|e| {
                AppError::new(
                    e.to_string(),
                    String::from("validate: from_value(anonymous)"),
                )
            })?;
            let verification = from_value::<bool>(value["verification"].clone()).map_err(|e| {
                AppError::new(
                    e.to_string(),
                    String::from("validate: from_value(verification)"),
                )
            })?;
            if anonymous || !verification {
                is_valid = false;
            } else {
                is_valid = true;
            }
        }
        Ok(is_valid)
    }

    pub async fn get_all_rivens(&self) -> Result<Vec<Auction>, AppError> {
        let url = "/profile/auctions";
        let method = Method::GET;

        let auth = self.auth.lock().await;
        let auth = auth.deref();
        let mut rate_limiter = self.limiter.lock().await;
        let rate_limiter = rate_limiter.deref_mut();
        let (body_value, _) = match self.send_request(
            method.clone(),
            &format!("{}{}", self.endpoint, url),
            rate_limiter,
            Some(auth),
            None,
        ).await {
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
