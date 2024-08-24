// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use futures::lock::Mutex;
use http::StatusCode;
use rivens::inventory::{database::InventoryDB, riven_lookop::RivenDataLookup};
use serde::{ser::SerializeStruct, Deserialize, Serialize};
use std::{
    error::Error,
    fmt::{self, Display},
    ops::Deref,
    sync::Arc,
};
use tauri::{
    async_runtime::block_on,
    App,
    Manager,
};
use http_client::{auth_state::AuthState, qf_client::QFClient, wfm_client::WFMClient};
mod jwt;
mod rate_limiter;
mod riven_data_store;
mod rivens;
mod http_client;

#[derive(Debug, Deserialize)]
pub struct AppError {
    pub location: String,
    pub err: String,
}

impl Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(&format!("{} {:?}", self.location, self.err))
    }
}

impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct("AppError", 2)?;
        state.serialize_field("location", &self.location.deref())?;
        state.serialize_field("err", &self.err.deref())?;
        state.end()
    }
}

impl AppError {
    pub fn new(err: String, loc: String) -> Self {
        Self {
            location: loc,
            err: format!("{}", err).into(),
        }
    }
    pub fn prop(&self, new_loc: Arc<str>) -> Self {
        Self {
            location: format!("{}: {}", new_loc, self.location).into(),
            err: self.err.clone(),
        }
    }
}

impl Error for AppError {}

#[tauri::command]
async fn get_auth_state(
    wfm: tauri::State<'_, Arc<Mutex<WFMClient>>>,
) -> Result<bool, AppError> {
    let wfm = wfm.inner().clone();
    let wfm = wfm.lock().await;
    let wfm = wfm.deref();
    wfm.validate()
        .await
        .map_err(|e| e.prop("Tauri CMD: get_auth_state".into()))
}

#[derive(Serialize, Deserialize)]
struct WrappedStatus {
    #[serde(with = "http_serde::status_code")]
    status: StatusCode,
}
#[tauri::command]
async fn login(
    email: &str,
    password: &str,
    wfm: tauri::State<'_, Arc<Mutex<WFMClient>>>,
) -> Result<WrappedStatus, AppError> {
    let wfm = wfm.inner().clone();
    let wfm = wfm.lock().await;
    let wfm = wfm.deref();
    let status = wfm
        .login(email, password)
        .await
        .map_err(|e| e.prop("Tauri CMD: login".into()))?;
    println!("");
    Ok(WrappedStatus { status })
}
#[tauri::command]
fn reload_thing() -> bool {
    println!("test");
    true
}

async fn setup_app_state(app: &mut App) -> Result<(), AppError> {
    let auth_state = Arc::new(Mutex::new(AuthState::setup()?));
    app.manage(auth_state.clone());

    let wfm_client = Arc::new(Mutex::new(WFMClient::new(auth_state.clone())));
    app.manage(wfm_client.clone());

    let qf_client = Arc::new(Mutex::new(QFClient::new()));
    app.manage(qf_client.clone());

    let riven_lookup = Arc::new(Mutex::new(RivenDataLookup::setup(qf_client.clone())));
    app.manage(riven_lookup.clone());

    let database = InventoryDB::open().map_err(|e| AppError::new(e.to_string(), String::from("setup_app_state")));
    let database = Arc::new(Mutex::new(database));
    app.manage(database.clone());
    Ok(())
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(move |app| {
            block_on(setup_app_state(app)).unwrap();
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_auth_state,
            login,
            reload_thing,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
