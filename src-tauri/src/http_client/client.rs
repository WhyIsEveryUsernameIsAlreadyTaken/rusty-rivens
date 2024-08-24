use core::fmt;
use std::{str::FromStr, time::{Duration, SystemTime}};

use http::{HeaderMap, Method, StatusCode};
use reqwest::Client;
use serde_json::Value;
use url::Url;

use crate::{rate_limiter::RateLimiter, AppError};

use super::auth_state::AuthState;

#[derive(Debug)]
pub struct ApiResult {
    pub res: (Option<Value>, HeaderMap),
    pub status: StatusCode,
}

#[derive(Debug)]
pub struct StatusError {
    pub status: StatusCode,
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

pub trait HttpClient<'a> {
    async fn send_request(
        &self,
        method: Method,
        url: &str,
        rate_limiter: &mut RateLimiter,
        auth: Option<&AuthState>,
        body: Option<Value>,
    ) -> Result<ApiResult, AppError> {
        rate_limiter.wait_for_token().await;
        let client = Client::new();
        // let new_url = format!("{}{}", self.endpoint, url);
        let request = client
            .request(method, Url::parse(url).unwrap())
            // .header(
            //     "Authorization",
            //     format!("JWT {}", auth.access_token.clone().unwrap_or("".into())),
            // )
            .header("User-Agent", "Rusty Rivens v0.1")
            .header("Language", "en")
            .timeout(Duration::from_secs(10));
        let request = match auth {
            Some(auth) => request.header(
                "Authorization",
                format!("JWT {}", auth.access_token.clone().unwrap_or("".into())),
            ),
            None => request,
        };
        let request = match body {
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
        let response = serde_json::Value::from_str(content.as_str()).map_err(|e|
            AppError::new(e.to_string(), String::from("Value::from_str: send_request"))
        )?;

        Ok(ApiResult {
            res: (Some(response), headers),
            status,
        })
    }
}
