use std::env;
use std::io::{Read, Write};
use std::string::String;
use std::sync::Arc;
use std::{fs::File, path::PathBuf};

use serde::{Deserialize, Serialize};

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
        let path: PathBuf = env::var("PWD")
            .map_err(|e| AppError::new(e.to_string().into(), "setup: env::var".into()))?
            .into();
        let path = path.join("auth.json");
        if !path.exists() {
            let mut file = File::create(path)
                .map_err(|e| AppError::new(e.to_string().into(), "setup: create".into()))?;
            let default = AuthState::default();
            let json = serde_json::to_string_pretty(&default).map_err(|e| {
                AppError::new(e.to_string().into(), "setup: to_string_pretty".into())
            })?;
            file.write_all(json.as_bytes())
                .map_err(|e| AppError::new(e.to_string().into(), "setup: as_bytes".into()))?;
            return Ok(default);
        };
        let mut file = File::open(path)
            .map_err(|e| AppError::new(e.to_string().into(), "setup: open".into()))?;
        let mut content = String::new();
        file.read_to_string(&mut content)
            .map_err(|e| AppError::new(e.to_string().into(), "setup: read_to_string".into()))?;
        let final_auth = serde_json::from_str(&content)
            .map_err(|e| AppError::new(e.to_string().into(), "setup: from_str".into()))?;
        Ok(final_auth)
    }

    pub fn update(&self) -> Result<(), AppError> {
        let path: PathBuf = env::var("PWD")
            .map_err(|e| AppError::new(e.to_string().into(), "setup: env::var".into()))?
            .into();
        let path = path.join("auth.json");
        let mut file = File::create(path)
            .map_err(|e| AppError::new(e.to_string().into(), "update: create".into()))?;
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| AppError::new(e.to_string().into(), "update: to_string_pretty".into()))?;
        file.write_all(json.as_bytes())
            .map_err(|e| AppError::new(e.to_string().into(), "update: write_all".into()))?;
        Ok(())
    }

    pub fn set(&mut self, new_auth: AuthState) {
        self.id = new_auth.id;
        self.ingame_name = new_auth.ingame_name;
        self.access_token = new_auth.access_token;
    }
}
