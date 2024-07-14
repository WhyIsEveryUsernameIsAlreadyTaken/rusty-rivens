use std::env;
use std::ops::Deref;
use std::sync::Arc;
use std::{fs::File, path::PathBuf};
use std::io::{Read, Write};
use std::string::String;

use futures::lock::Mutex;
use http::Method;
use serde::{Deserialize,Serialize};
use serde_json::from_value;

use crate::jwt::jwt_is_valid;
use crate::wfm_client::client::WFMClient;
use crate::AppError;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuthState {
    pub ingame_name: Arc<str>,
    pub access_token: Option<Arc<str>>,
    pub id: String,
}

impl Default for AuthState {
    fn default() -> Self {
        Self {
            id: String::new(),
            access_token: None,
            ingame_name: "".into(),
        }
    }
}

impl AuthState {
    pub fn setup() -> Result<Self, AppError> {
        let path: PathBuf = env::var("PWD").map_err(|e| AppError::new(e.to_string().into(),"setup: env::var".into()))?.into();
        let path = path.join("auth.json");
        if !path.exists() {
            let mut file = File::create(path).map_err(|e| AppError::new(e.to_string().into(),"setup: create".into()))?;
            let default = AuthState::default();
            let json = serde_json::to_string_pretty(&default).map_err(|e| AppError::new(e.to_string().into(),"setup: to_string_pretty".into()))?;
            file.write_all(json.as_bytes()).map_err(|e| AppError::new(e.to_string().into(),"setup: as_bytes".into()))?;
            return Ok(default);
        };
        let mut file = File::open(path).map_err(|e| AppError::new(e.to_string().into(),"setup: open".into()))?;
        let mut content = String::new();
        file.read_to_string(&mut content).map_err(|e| AppError::new(e.to_string().into(),"setup: read_to_string".into()))?;
        let final_auth = serde_json::from_str(&content).map_err(|e| AppError::new(e.to_string().into(),"setup: from_str".into()))?;
        Ok(final_auth)
    }

    pub fn update(&self) -> Result<(), AppError> {
        let path: PathBuf = env::var("PWD").map_err(|e| AppError::new(e.to_string().into(),"setup: env::var".into()))?.into();
        let path = path.join("auth.json");
        let mut file = File::create(path).map_err(|e| AppError::new(e.to_string().into(),"update: create".into()))?;
        let json = serde_json::to_string_pretty(self).map_err(|e| AppError::new(e.to_string().into(),"update: to_string_pretty".into()))?;
        file.write_all(json.as_bytes()).map_err(|e| AppError::new(e.to_string().into(),"update: write_all".into()))?;
        Ok(())
    }

    pub fn set(&mut self, new_auth: AuthState) {
        self.id = new_auth.id;
        self.ingame_name = new_auth.ingame_name;
        self.access_token = new_auth.access_token;
    }
}

pub async fn validate(auth: Arc<Mutex<AuthState>>, wfm: Arc<Mutex<WFMClient>>) -> Result<bool, AppError> {
    let valid_jwt: bool;
    if let Some(token) = auth.lock().await.deref().clone().access_token {
        valid_jwt = jwt_is_valid(&token).map_err(|e| e.prop("validate".into()))?;
    } else {
        return Ok(false);
    }
    if !valid_jwt {
        return Ok(false);
    }
    let wfm = wfm.lock().await;
    let wfm = wfm.deref();
    let res = wfm.send_request(&Method::GET, "profile", None).await;
    let (body, headers) = match res {
        Ok(v) => v.res,
        Err(e) => return Err(e.prop("validate".into()))
    };
    let mut is_valid = false;
    if let Some(body) = body {
        let value = body["profile"].clone();
        let anonymous = from_value::<bool>(value["anonymous"].clone()).map_err(|e|
            AppError::new(e.to_string(), String::from("validate: from_value(anonymous)"))
        )?;
        let verification = from_value::<bool>(value["verification"].clone()).map_err(|e|
            AppError::new(e.to_string(), String::from("validate: from_value(verification)"))
        )?;
        if anonymous || !verification {
            is_valid = false;
        } else {
            is_valid = true;
        }
    }
    Ok(is_valid)
}
