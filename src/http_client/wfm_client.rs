use std::{
    ops::{Deref, DerefMut},
    sync::Arc,
    thread::JoinHandle,
    time::Duration,
};

use once_cell::sync::OnceCell;
use tokio::sync::{
    mpsc::{channel, Receiver},
    Mutex,
};

use crate::{
    http_client::client::{ClientHandle, Header, Request, Response},
    jwt::jwt_is_valid,
    rate_limiter::RateLimiter,
    AppError,
};

use super::{
    auth_state::AuthState,
    client::{HttpClient, Method, RequestBuilder, StatusCode},
};

#[derive(Debug)]
pub struct WFMClient {
    endpoint: String,
    limiter: Option<Arc<Mutex<RateLimiter>>>,
    auth: AuthState,
    client_handle: Option<Arc<Mutex<ClientHandle>>>,
    response_receiver: Option<Arc<Mutex<Receiver<Response>>>>,
}

impl<'a> HttpClient<'a> for WFMClient {
    async fn send_fn(
        &mut self,
        mut req: super::client::RequestBuilder,
    ) -> Result<
        (
            Arc<Mutex<ClientHandle>>,
            Arc<Mutex<Receiver<Response>>>,
            RequestBuilder,
        ),
        AppError,
    > {
        if let Some(rate_limiter) = self.limiter.clone() {
            let mut rate_limiter = rate_limiter.lock().await;
            let rate_limiter = rate_limiter.deref_mut();
            rate_limiter.wait_for_token().await;
        };
        let header = match Header::try_from_str(
            format!("Authorization: JWT {}", self.auth.access_token.deref()).as_str(),
        ) {
            Ok(v) => v,
            Err(e) => {
                println!("{}", e.to_string());
                return Err(AppError::new(
                    e.to_string(),
                    "WFMClient::send_fn".to_string(),
                ));
            }
        };
        req = match req.header(header) {
            Ok(v) => v,
            Err(e) => {
                println!("Error while inseerting Authorization Header: {e}");
                return Err(AppError::new(
                    e.to_string(),
                    "WFMClient::send_fn".to_string(),
                ));
            }
        };
        let (client_handle, response_receiver) =
            if let Some(client_handle) = self.client_handle.clone() {
                (
                    client_handle,
                    self.response_receiver
                        .clone()
                        .expect("reciever should be availabe after creating the client handle"),
                )
            } else {
                let (request_sender, request_receiver) = channel::<Request>(1);
                let (response_sender, response_receiver) = channel::<Response>(1);
                let response_receiver_arc = Arc::new(Mutex::new(response_receiver));
                self.response_receiver = Some(response_receiver_arc.clone());
                let client_handle = ClientHandle::create(
                    self.endpoint.as_str(),
                    request_sender,
                    request_receiver,
                    response_sender,
                )
                .map_err(|e| e.prop("WFMClient::send_fn".into()))?;
                let client_handle = Arc::new(Mutex::new(client_handle));
                (client_handle, response_receiver_arc)
            };
        Ok((client_handle, response_receiver, req))
    }

    async fn rate_limit(&self) {
        if let Some(rate_limiter) = self.limiter.clone() {
            let mut limiter = rate_limiter.lock().await;
            let limiter = limiter.deref_mut();
            limiter.add_delay(1.0);
        }
    }
}

impl WFMClient {
    pub fn new(auth: AuthState) -> Self {
        WFMClient {
            endpoint: String::from("https://api.warframe.market/v1"),
            limiter: Some(Arc::new(Mutex::new(RateLimiter::new(
                1.0,
                Duration::new(1, 0),
            )))),
            auth,
            client_handle: None,
            response_receiver: None,
        }
    }

    pub async fn login(&mut self, email: &str, password: &str) -> Result<StatusCode, AppError> {
        let body = serde_json::json!({"email": email, "password": password});
        let req = RequestBuilder::new()
            .method(Method::POST)
            .uri(format!("{}{}", self.endpoint, "/auth/signin").as_str())
            .body(body)
            .build();
        let response = match self.send_request(req).await {
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
            if let Some(_) = val {
                // let data = v["payload"]["user"].clone();
                // let data = data.to_string();
                // println!("{data}");
                // user = serde_json::from_str(data.as_str()).map_err(|e| {
                //     AppError::new(
                //         e.to_string(),
                //         String::from("login: from_str"),
                //     )
                // })?;
                user.access_token = token;
                user.update().map_err(|e| e.prop("login: ".into()))?;
            }
        }
        self.auth.set(user);
        Ok(response.status)
    }

    pub async fn validate(&mut self) -> Result<bool, AppError> {
        let valid_jwt = if !self.auth.access_token.is_empty() {
            jwt_is_valid(&self.auth.access_token).map_err(|e| e.prop("validate".into()))?
        } else {
            println!("WARNING: No JWT Found");
            return Ok(false);
        };
        if !valid_jwt {
            return Ok(false);
        }
        let req = RequestBuilder::new()
            .method(Method::GET)
            .uri(format!("{}/profile", self.endpoint).as_str())
            .build();
        let res = self.send_request(req).await;
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

    // pub async fn get_all_rivens(&self) -> Result<Vec<Auction>, AppError> {
    //     let url = "/profile/auctions";
    //     let method = Method::GET;

    //     let auth = self.auth.lock().unwrap();
    //     let auth = auth.deref();
    //     let mut rate_limiter = self.limiter.lock().unwrap();
    //     let rate_limiter = rate_limiter.deref_mut();
    //     let (body_value, _) = match self
    //         .send_request(
    //             method.clone(),
    //             &format!("{}{}", self.endpoint, url),
    //             rate_limiter,
    //             Some(auth),
    //             None,
    //         )
    //         .await
    //     {
    //         Ok(v) => {
    //             println!("{} {}: {}", method, url, v.status.code);
    //             if v.status.code < 300 {
    //                 return Err(AppError::new(
    //                     StatusError { status: v.status }.to_string(),
    //                     String::from("get_all_rivens: "),
    //                 ));
    //             }
    //             v.res
    //         }
    //         Err(e) => return Err(AppError::new(e.to_string(), String::from("get_all_rivens"))),
    //     };
    //     match body_value {
    //         Some(v) => {
    //             let data = v["payload"]["auctions"].clone();
    //             let data = data.to_string();
    //             serde_json::from_str::<Vec<Auction>>(data.as_str()).map_err(|e| {
    //                 AppError::new(
    //                     e.to_string(),
    //                     String::from("get_all_rivens: from_str::<Vec<Auction>>"),
    //                 )
    //             })
    //         }
    //         None => Err(AppError::new(
    //             String::from("No response body associated with response"),
    //             String::from("get_all_rivens"),
    //         )),
    //     }
    // }
}

pub static TEST_WFM_STOPPED: OnceCell<bool> = once_cell::sync::OnceCell::new();

#[cfg(test)]
mod tests {

    use crate::http_client::{
        auth_state::AuthState,
        client::{HttpClient, Method, RequestBuilder},
        wfm_client::WFMClient,
    };

    use super::TEST_WFM_STOPPED;

    #[tokio::test]
    async fn test_wfmclient() {
        let mut client = WFMClient::new(AuthState::setup().unwrap());
        let _resp = {
            let req = RequestBuilder::new()
                .method(Method::GET)
                .uri(format!("{}/profile/toopsi", client.endpoint).as_str())
                .build();
            client.send_request(req).await
        };
        TEST_WFM_STOPPED.set(true).unwrap();
    }
}
