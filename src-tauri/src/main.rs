// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use std::{error::Error, fmt::{self, Display}, ops::Deref, sync::Arc};
use auth_state::{validate, AuthState};
use futures::lock::Mutex;
use http::StatusCode;
use serde::{ser::SerializeStruct, Deserialize, Serialize};
use tauri::{async_runtime::{block_on, spawn_blocking}, App, Manager};
use wfm_client::client::WFMClient;
mod wfm_client;
mod rate_limiter;
mod auth_state;
mod jwt;

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
        S: serde::Serializer {
        let mut state = serializer.serialize_struct("AppError", 2)?;
        state.serialize_field("location", &self.location.deref())?;
        state.serialize_field("err", &self.err.deref())?;
        state.end()
    }
}

impl AppError {
    pub fn new(err: String, loc: String) -> Self {
        Self { location: loc, err: format!("{}", err).into() }
    }
    pub fn prop(&self, new_loc: Arc<str>) -> Self {
        Self { location: format!("{}: {}", new_loc, self.location).into(), err: self.err.clone() }
    }
}

impl Error for AppError {}



#[tauri::command]
async fn get_auth_state(auth: tauri::State<'_, Arc<Mutex<AuthState>>>, wfm: tauri::State<'_, Arc<Mutex<WFMClient>>>)
-> Result<bool, AppError> {
    validate(auth.inner().clone(), wfm.inner().clone()).await.map_err(|e|
        e.prop("Tauri CMD: get_auth_state".into())
    )
}

#[derive(Serialize, Deserialize)]
struct WrappedStatus {
    #[serde(with = "http_serde::status_code")]
    status: StatusCode
}
#[tauri::command]
async fn login(email: &str,
    password: &str,
    wfm: tauri::State<'_, Arc<Mutex<WFMClient>>>
) -> Result<WrappedStatus, AppError> {
    let wfm = wfm.inner().clone();
    let wfm = wfm.lock().await;
    let wfm = wfm.deref();
    let status = wfm.login(email, password).await.map_err(|e| e.prop("Tauri CMD: login".into()))?;
    println!("");
    Ok(WrappedStatus { status })
}



async fn setup_app_state(app: &mut App) -> Result<(), AppError> {
    let auth_state = Arc::new(Mutex::new(AuthState::setup()?));
    app.manage(auth_state.clone());

    let wfm_client = Arc::new(Mutex::new(WFMClient::new(auth_state.clone())));
    app.manage(wfm_client.clone());
    Ok(())
}

fn main() {
    tauri::Builder::default()
        .setup(move |app| {
            block_on(setup_app_state(app)).unwrap();
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_auth_state,
            login,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
