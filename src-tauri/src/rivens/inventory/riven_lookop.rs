use std::{ops::{Deref, DerefMut}, sync::Arc};

use futures::lock::Mutex;
use http::Method;
use serde::{Deserialize, Serialize};

use crate::{http_client::{client::HttpClient, qf_client::QFClient}, AppError};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RivenDataLookup {
    pub weapons: Option<Vec<Weapon>>,
    pub rivens_attributes: Option<Vec<RivensAttribute>>,
    pub available_attributes: Option<Vec<AvailableAttribute>>
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Weapon {
    pub wfm_url_name: Option<Arc<str>>,
    pub unique_name: Option<Arc<str>>,
    pub disposition: Option<f64>,
    pub weapon_type: Option<Arc<str>>
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RivensAttribute {
    pub unique_name: Option<Arc<str>>,
    pub upgrades: Option<Vec<Upgrade>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Upgrade {
    pub wfm_url: Option<Arc<str>>,
    pub modifier_tag: Option<Arc<str>>,
    pub prefix: Option<Arc<str>>,
    pub suffix: Option<Arc<str>>,
    pub value: Option<f64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AvailableAttribute {
    pub units: Option<Arc<str>>,
    pub url_name: Option<Arc<str>>,
}

impl Default for RivenDataLookup {
    fn default() -> Self {
        Self {
            weapons: None,
            rivens_attributes: None,
            available_attributes: None,
        }
    }
}

impl RivenDataLookup {
    pub async fn setup(qf: Arc<Mutex<QFClient>>) -> Result<Self, AppError>{
        let qf = qf.lock().await;
        let qf = qf.deref();
        let mut rate_limiter = qf.limiter.lock().await;
        let rate_limiter = rate_limiter.deref_mut();
        let (body_value, _) =match qf.send_request(
            Method::GET,
            qf.endpoint.as_str(),
            rate_limiter,
            None,
            None
        ).await {
            Ok(v) => v.res,
            Err(e) => return Err(
                e.prop("setup".into())
            )
        };
        let new_data: Self = match body_value {
            Some(v) => serde_json::from_value(v).map_err(|e| AppError::new(
                e.to_string(),
                String::from("QFClient::setup: from_value::<RivenDataLookup>")
            ))?,
            None => return Err(AppError::new(
                String::from("No response body associated with response"),
                String::from("QFClient::setup")
            ))
        };
        Ok(new_data)
    }
}

