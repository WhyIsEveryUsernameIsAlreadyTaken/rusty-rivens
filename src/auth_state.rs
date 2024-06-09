use std::{fs::File, path::PathBuf};
use std::io::{Read, Write, Error};
use std::string::String;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuthState {
    pub ingame_name: String,
    pub access_token: Option<String>,
    pub id: String,
    pub status: Option<String>,
}

impl Default for AuthState {
    fn default() -> Self {
        Self {
            id: "".to_string(),
            access_token: None,
            ingame_name: "".to_string(),
            status: Some("invisible".to_string()),
        }
    }
}

impl AuthState {
    pub fn setup() -> Result<Self, Error>  {
        let path: PathBuf = env!("PATH").into();
        if !path.exists() {
            let mut file = File::create(path)?;
            let default = AuthState::default();
            let json = serde_json::to_string_pretty(&default)?;
            file.write_all(json.as_bytes())?;
            return Ok(default);
        }
        let mut file = File::open(path)?;
        let mut content = String::new();
        file.read_to_string(&mut content)?;
        let final_auth = serde_json::from_str(&content)?;
        Ok(final_auth)
    }
}
