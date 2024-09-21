use std::{ops::{Deref, DerefMut}, rc::Rc, sync::Arc};

use async_lock::Mutex;
use serde::{Deserialize, Serialize};

use crate::{http_client::{client::{HttpClient, Method}, qf_client::QFClient}, AppError};

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
    pub upgrade_type: Option<Arc<str>>
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
    pub fn setup() -> Result<Self, AppError>{
        let qf = QFClient::new();
        let res = smolscale::block_on(async move {
            qf.send_request(
                Method::GET,
                qf.endpoint.as_str(),
                &mut None,
                None,
                None
            ).await
        });
        let (body_value, _) = match res {
            Ok(v) => v.res,
            Err(e) => return Err(
                e.prop("setup".into())
            )
        };
        if body_value.is_none() {
            return Err(AppError::new(
                String::from("No response body associated with response"),
                String::from("QFClient::setup")
            ))
        }
        let new_data: Self = serde_json::from_value(body_value.unwrap()["items"].clone()).map_err(|e| AppError::new(
                    e.to_string(),
                    String::from("QFClient::setup: from_value::<RivenDataLookup>")
                ))?;
        Ok(new_data)
    }
}
