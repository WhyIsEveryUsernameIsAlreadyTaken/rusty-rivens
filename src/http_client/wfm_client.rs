use std::{
    ops::{Deref, DerefMut}, sync::Arc, time::Duration
};

use tokio::sync::{mpsc::Receiver, Mutex};

use crate::{jwt::jwt_is_valid, rate_limiter::RateLimiter, AppError};

use super::{
    auth_state::AuthState,
    client::{ClientHandle, HttpClient, Method, Request, RequestBuilder, Response, StatusCode},
};

type ArcClientHandle = Arc<Mutex<ClientHandle>>;

#[derive(Debug)]
pub struct WFMClient {
    endpoint: String,
    limiter: Arc<Mutex<RateLimiter>>,
    auth: Arc<Mutex<AuthState>>,
    client_handle: Option<ArcClientHandle>
}

impl HttpClient for WFMClient {
    async fn sender_fn(&mut self, rq: RequestBuilder) -> Result<(ArcClientHandle, Receiver<Response>, RequestBuilder), AppError> {
        let mut limiter_mutex = self.limiter.lock().await;
        let limiter = limiter_mutex.deref_mut();
        limiter.wait_for_token().await;
        drop(limiter_mutex);
        let auth_mutex = self.auth.lock().await; // WHY DEADLOCK ?????????????????????????????????????
        let auth = auth_mutex.deref();
        let rq = rq
            .header(format!("Authorization: JWT {}", auth.wfm_access_token).parse().expect("infallible"));
        drop(auth_mutex);
        let (request_sender, request_receiver) = tokio::sync::mpsc::channel::<Request>(1);
        let (respones_sender, response_receiver) = tokio::sync::mpsc::channel::<Response>(1);
        let client_handle = ClientHandle::new()
            .port(443)
            .addr("https://api.warframe.market/")
            .map_err(|e| AppError::new(e.to_string(), "send_request".to_string()))?
            .timeout(Duration::from_secs(5))
            .send_channel(request_sender)
            .start_client(request_receiver, respones_sender);
        let client_handle = Arc::new(Mutex::new(client_handle));
        self.client_handle = Some(client_handle.clone());
        Ok((client_handle, response_receiver, rq))
    }

    async fn rate_limit(&self) {
        let mut limiter = self.limiter.lock().await;
        let limiter = limiter.deref_mut();
        limiter.add_delay(1.0);
    }
}

impl WFMClient {
    pub fn new(auth: Arc<Mutex<AuthState>>) -> Self {
        WFMClient {
            endpoint: String::from("https://api.warframe.market/v1"),
            limiter: Arc::new(Mutex::new(RateLimiter::new(1.0, Duration::new(1, 0)))),
            auth,
            client_handle: None,
        }
    }

    pub async fn login(&mut self, email: &str, password: &str) -> Result<(StatusCode, Arc<str>, Arc<str>, Arc<str>), AppError> {
        let body = serde_json::json!({"email": email, "password": password});
        let req = RequestBuilder::new()
            .method(Method::POST)
            .uri(&format!("{}{}", self.endpoint, "/auth/signin"))
            .body(body);
        let response = match self
            .send_request(req.build()).await {
            Ok(v) => v,
            Err(e) => return Err(AppError::new(e.to_string(), String::from("login: "))),
        };
        let (val, headers) = response.res;
        let token: Arc<str>;
        if let Some(cookie_header) = headers.get("Set-Cookie") {
            let cookies = &cookie_header;
            token = cookies[4..].split_once(';').unwrap_or(("", "")).0.into();
        } else {
            panic!("No access token returned!");
        };
        let mut user = AuthState::default();
        if response.status.code < 300 {
            if let Some(v) = val {
                let data = v["payload"]["user"].clone();
                let data = data.to_string();
                user = serde_json::from_str(data.as_str()).map_err(|e| {
                    AppError::new(
                        e.to_string(),
                        String::from("login: from_str"),
                    )
                })?;
                user.wfm_access_token = token;
                user.update().map_err(|e| e.prop("login: ".into()))?;
            }
        }
        let mut auth = self.auth.lock().await;
        let auth = auth.deref_mut();
        auth.set(user);
        Ok((response.status, auth.id.clone(), auth.check_code.clone(), auth.ingame_name.clone()))
    }

    pub async fn validate(&mut self) -> Result<bool, AppError> {
        let auth_mutex = self.auth.lock().await;
        let auth = auth_mutex.deref();
        let valid_jwt = if !auth.wfm_access_token.is_empty() {
            println!("jwt found, validating");
            jwt_is_valid(auth.wfm_access_token.deref()).map_err(|e| e.prop("validate".into()))?
        } else {
            println!("WARNING: No JWT Found");
            return Ok(false);
        };
        if !valid_jwt {
            println!("jwt not valid");
            return Ok(false);
        }
        drop(auth_mutex);
        let req = RequestBuilder::new()
            .method(Method::GET)
            .uri(format!("{}/profile", self.endpoint).as_str())
            .build();
        let res = self
            .send_request(req).await;
        let (body, _) = match res {
            Ok(v) => v.res,
            Err(e) => return Err(e.prop("validate".into())),
        };
        let mut is_valid = false;
        if let Some(body) = body {
            let value = body["profile"].clone();
            let anonymous = match value["anonymous"].as_bool() {
                Some(v) => v,
                None => {
                    return Err(AppError::new(
                        String::from("failed to deserialize json value: anonymous"),
                        String::from("validate: .as_bool()"),
                    ))
                }
            };
            let verification = match value["verification"].as_bool() {
                Some(v) => v,
                None => {
                    return Err(AppError::new(
                        String::from("failed to deserialize json value: verification"),
                        String::from("validate: .as_bool()"),
                    ))
                }
            };
            if anonymous || !verification {
                is_valid = false;
            } else {
                is_valid = true;
            }
        }
        Ok(is_valid)
    }
}

#[cfg(test)]
mod tests {

    use std::sync::Arc;

    use tokio::sync::Mutex;

    use crate::{block_in_place, http_client::{auth_state::AuthState, client::{HttpClient, Method, RequestBuilder}, wfm_client::WFMClient}};

    #[test]
    fn test_wfmclient() {
        let auth = AuthState::setup().unwrap();
        let auth = Arc::new(Mutex::new(auth));
        let mut client = WFMClient::new(auth);
        let _req = block_in_place!(async move {
            let req = RequestBuilder::new()
                .method(Method::GET)
                .uri(format!("{}/profile/toopsi", client.endpoint).as_str())
                .build();
            client.send_request(req).await
        });
    }
}
