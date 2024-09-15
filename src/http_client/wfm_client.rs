use std::{
    ops::{Deref, DerefMut}, sync::Arc, thread::JoinHandle, time::Duration
};

use async_lock::Mutex;

use crate::{jwt::jwt_is_valid, rate_limiter::RateLimiter, AppError};

use super::{
    auth_state::AuthState,
    client::{HttpClient, Method, StatusCode},
};

#[derive(Debug)]
pub struct WFMClient {
    endpoint: String,
    limiter: Arc<Mutex<RateLimiter>>,
    auth: AuthState,
    http_client: Option<JoinHandle<()>>
}

impl<'a> HttpClient<'a> for WFMClient {}

impl WFMClient {
    pub fn new(auth: AuthState) -> Self {
        WFMClient {
            endpoint: String::from("https://api.warframe.market/v1"),
            limiter: Arc::new(Mutex::new(RateLimiter::new(1.0, Duration::new(1, 0)))),
            auth,
            http_client: None,
        }
    }

    pub async fn login(&mut self, email: &str, password: &str) -> Result<StatusCode, AppError> {
        let body = serde_json::json!({"email": email, "password": password});
        let mut rate_limiter = self.limiter.lock().await;
        let rate_limiter = rate_limiter.deref_mut();
        let response = match self
            .send_request(
                Method::POST,
                &format!("{}{}", self.endpoint, "/auth/signin"),
                rate_limiter,
                None,
                Some(body),
            )
            .await
        {
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

    pub async fn validate(&self) -> Result<bool, AppError> {
        let valid_jwt: bool;
        let valid_jwt = if !self.auth.access_token.is_empty() {
            jwt_is_valid(&self.auth.access_token).map_err(|e| e.prop("validate".into()))?
        } else {
            println!("WARNING: No JWT Found");
            return Ok(false);
        };
        if !valid_jwt {
            return Ok(false);
        }
        let mut rate_limiter = self.limiter.lock().await;
        let rate_limiter = rate_limiter.deref_mut();
        let res = self
            .send_request(Method::GET, format!("{}/profile", self.endpoint).as_str(), rate_limiter, Some(self.auth.clone()), None)
            .await;
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

#[cfg(test)]
mod tests {
    use std::{ops::{Deref, DerefMut}, time::Duration};

    use crate::{http_client::{auth_state::AuthState, client::{HttpClient, Method}, wfm_client::WFMClient}, rate_limiter::{self, RateLimiter}, STOPPED};

    #[test]
    fn test_wfmclient() {
        let client = WFMClient::new(AuthState::setup().unwrap());
        let req = smolscale::block_on( async move {
            let mut limiter = client.limiter.lock().await;
            client.send_request(
                Method::GET,
                format!("{}/profile/toopsi", client.endpoint).as_str(),
                limiter.deref_mut(),
                None,
                None
            ).await
        });
    }
}
