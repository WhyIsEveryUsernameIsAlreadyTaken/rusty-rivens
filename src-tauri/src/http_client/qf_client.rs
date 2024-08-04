use std::{
    ops::DerefMut,
    sync::Arc,
    time::Duration,
};

use futures::lock::Mutex;
use http::Method;
use serde::{Deserialize, Serialize};

use crate::{rate_limiter::RateLimiter, AppError};

use super::client::HttpClient;

#[derive(Clone, Debug)]
pub struct QFClient {
    endpoint: String,
    limiter: Arc<Mutex<RateLimiter>>,
    pub riven_data_lookup: Arc<Mutex<RivenDataLookup>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct RivenDataLookup {
    weapons: Vec<Weapon>,
    rivens_attributes: Vec<RivensAttribute>,
    available_attributes: Vec<AvailableAttribute>
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Weapon {
    wfm_url_name: Arc<str>,
    unique_name: Arc<str>,
    disposition: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct RivensAttribute {
    unique_name: Arc<str>,
    upgrades: Vec<Upgrade>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Upgrade {
    wfm_url: Arc<str>,
    modifier_tag: Arc<str>,
    prefix: Arc<str>,
    suffix: Arc<str>,
    value: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct AvailableAttribute {
    units: Arc<str>,
    url_name: Arc<str>,
}

impl Default for RivenDataLookup {
    fn default() -> Self {
        Self {
            weapons: vec![],
            rivens_attributes: vec![],
            available_attributes: vec![],
        }
    }
}

impl RivenDataLookup {
    fn set(&mut self, new_data: Self) {
        self.weapons = new_data.weapons;
        self.rivens_attributes = new_data.rivens_attributes;
        self.available_attributes = new_data.available_attributes;
    }
}

impl HttpClient for QFClient {}

impl QFClient {
    fn new() -> Self {
        Self {
            endpoint: String::from("https://api.quantframe.app/items/riven/raw"),
            limiter: Arc::new(Mutex::new(RateLimiter::new(1.0, Duration::new(1, 0)))),
            riven_data_lookup: Arc::new(Mutex::new(RivenDataLookup::default())),
        }
    }

    pub async fn setup() -> Result<(), AppError>{
        let temp = Self::new();
        let mut rate_limiter = temp.limiter.lock().await;
        let rate_limiter = rate_limiter.deref_mut();
        let (body_value, _) =match temp.send_request(
            Method::GET,
            temp.endpoint.as_str(),
            rate_limiter,
            None,
            None
        ).await {
            Ok(v) => v.res,
            Err(e) => return Err(
                e.prop("setup".into())
            )
        };
        let new_data: RivenDataLookup = match body_value {
            Some(v) => serde_json::from_value(v).map_err(|e| AppError::new(
                e.to_string(),
                String::from("QFClient::setup: from_value::<RivenDataLookup>")
            ))?,
            None => return Err(AppError::new(
                String::from("No response body associated with response"),
                String::from("QFClient::setup")
            ))
        };
        temp.riven_data_lookup.clone().try_lock().unwrap().deref_mut().set(new_data);
        Ok(())
    }
}
