use std::{env, fs::{read_to_string, File}, io::Write, path::PathBuf, sync::Arc};

use serde::{Deserialize, Serialize};
use serde_json::{from_str, from_value, to_string_pretty, to_value, Value};
use time::OffsetDateTime;
use tokio::sync::Mutex;

use crate::{block_in_place, http_client::{auth_state::AuthState, client::{HttpClient, Method, RequestBuilder}, qf_client::QFClient}, AppError};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RivenDataLookup {
    pub weapons: Option<Vec<Weapon>>,
    pub rivens_attributes: Option<Vec<RivensAttribute>>,
    pub available_attributes: Option<Vec<AvailableAttribute>>
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct RivenDataLookupMeta {
    unix_ts: i64,
    data: RivenDataLookup,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Weapon {
    pub wfm_url_name: Option<Arc<str>>,
    pub unique_name: Option<Arc<str>>,
    pub name: Option<Arc<str>>,
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
    pub short_string: Option<Arc<str>>,
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

static MONTH_IN_SECONDS: i64 = 2592000;

fn from_file(path: PathBuf) -> Option<RivenDataLookup> {
    let s = read_to_string(path).expect("could not read from file");
    let mut v = from_str::<Value>(s.as_str()).expect("malformed json");
    let ts = v["unix_ts"].as_i64();
    if ts.is_none() {
        println!(
        "WARNING: No timestamp associated with riven lookup data\nGetting data from external server..."
        );
        return None;
    }
    use OffsetDateTime as ODT;
    let now = ODT::now_utc().unix_timestamp();
    let elapsed = now - ts.expect("could not parse timestamp");
    if elapsed >= MONTH_IN_SECONDS {
        println!(
        "WARNING: Riven lookup data is too old\nGetting data from external server..."
        );
        return None;
    }
    let res = from_value::<RivenDataLookup>(v["data"].take());
    if res.is_err() {
        println!(
        "WARNING: Could not parse riven lookup data\nGetting data from external server..."
        );
        return None;
    }
    Some(res.unwrap())
}

impl RivenDataLookup {
    pub async fn setup() -> Result<Self, AppError> {
        let path: PathBuf = env::var("PWD")
            .map_err(|e| AppError::new(e.to_string().into(), "setup: env::var".into()))?
            .into();
        let path = path.join("rivenLookupData.json");
        let riven_data: Option<Self> = if path.exists() {
            from_file(path.clone())
        } else {
            println!(
            "WARNING: `rivenLookupData.json` does not exist\nGetting data from external server..."
            );
            None
        };
        let riven_data = if let Some(data) = riven_data {
            data
        } else {
            let auth = AuthState::setup().unwrap();
            let auth = Arc::new(Mutex::new(auth));
            let mut qf = QFClient::new(auth.clone());
            let res = {
                let req = RequestBuilder::new()
                    .method(Method::GET)
                    .uri(format!("{}/items/riven/raw", qf.endpoint).as_str())
                    .build();
                qf.send_request(req).await
            };
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
            let data = serde_json::from_value::<Self>(body_value.unwrap()).map_err(|e| AppError::new(
                e.to_string(),
                String::from("QFClient::setup: from_value::<RivenDataLookup>")
            ))?;
            if let Ok(mut f) = File::create(path) {
                use OffsetDateTime as ODT;
                let now = ODT::now_utc().unix_timestamp();
                let json = to_string_pretty(&RivenDataLookupMeta { unix_ts: now, data: data.clone() });
                if json.is_err() {
                    println!("ERR: Could not write lookup data to file (serialize failed)");
                };
                if f.write_all(json.unwrap().as_bytes()).is_err() {
                    println!("ERR: Could not write lookup data to file (write failed)");
                };
            } else {
                println!("ERR: Could not write lookup data to file (file create failed)")
            };
            data
        };
        Ok(riven_data)
    }
}
