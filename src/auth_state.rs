use std::env;
use std::{fs::File, path::PathBuf};
use std::io::{Read, Write};
use std::string::String;

use serde::{Deserialize, Serialize};

use crate::wfm_client::client::GenericError;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuthState {
    pub ingame_name: String,
    pub access_token: Option<String>,
    pub id: String,
}

impl Default for AuthState {
    fn default() -> Self {
        Self {
            id: "".to_string(),
            access_token: None,
            ingame_name: "".to_string(),
        }
    }
}

impl AuthState {
    pub fn setup() -> Result<Self, GenericError> {
        let path: PathBuf = env::var("PWD").map_err(|e| GenericError::new(e, "setup: env::var: ".to_string()))?.into();
        let path = path.join("auth.json");
        if !path.exists() {
            let mut file = File::create(path).map_err(|e| GenericError::new(e, "setup: create: ".to_string()))?;
            let default = AuthState::default();
            let json = serde_json::to_string_pretty(&default).map_err(|e| GenericError::new(e, "setup: to_string_pretty: ".to_string()))?;
            file.write_all(json.as_bytes()).map_err(|e| GenericError::new(e, "setup: as_bytes: ".to_string()))?;
            return Ok(default);
        }
        let mut file = File::open(path).map_err(|e| GenericError::new(e, "setup: open: ".to_string()))?;
        let mut content = String::new();
        file.read_to_string(&mut content).map_err(|e| GenericError::new(e, "setup: read_to_string: ".to_string()))?;
        let final_auth = serde_json::from_str(&content).map_err(|e| GenericError::new(e, "setup: from_str: ".to_string()))?;
        Ok(final_auth)
    }

    pub fn update(&self) -> Result<(), GenericError> {
        let path: PathBuf = env::var("PWD").map_err(|e| GenericError::new(e, "setup: env::var: ".to_string()))?.into();
        let path = path.join("auth.json");
        let mut file = File::create(path).map_err(|e| GenericError::new(e, "update: create: ".to_string()))?;
        let json = serde_json::to_string_pretty(self).map_err(|e| GenericError::new(e, "update: to_string_pretty: ".to_string()))?;
        file.write_all(json.as_bytes()).map_err(|e| GenericError::new(e, "update: write_all: ".to_string()))?;
        Ok(())
    }

    pub fn set(&mut self, new_auth: AuthState) {
        self.id = new_auth.id;
        self.ingame_name = new_auth.ingame_name;
        self.access_token = new_auth.access_token;
    }
}
