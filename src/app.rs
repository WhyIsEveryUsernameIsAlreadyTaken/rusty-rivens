use std::{fmt::{self, Display}, future};

use leptos::*;
use leptos_router::{NavigateOptions, Redirect, Route, Router, Routes, State};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::from_str;
use tauri_sys::{tauri::invoke, Error};

use crate::pages::{home::Home, login::Login};

#[derive(Serialize, Deserialize)]
pub struct EmptyArgs;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppError {
    pub location: String,
    pub err: String,
}

impl Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(&format!("{}: {:?}", self.location, self.err))
    }
}

pub fn filter_err<T>(input: Result<T, tauri_sys::Error>) -> Result<T, AppError> {
    match input {
        Ok(v) => Ok(v),
        Err(e) => match e {
            Error::Command(err) => {
                logging::log!("{}", err);
                let err = &err.as_str()[15..err.len() - 2];
                let err: AppError = from_str(err).unwrap_or(AppError {
                    location: "filter_err".to_string(),
                    err: "unknown error".to_string()
                });
                logging::error!("{}", err.to_string());
                Err(err)
            },
            Error::Utf8(err) => {
                logging::error!("{}", err.to_str().unwrap());
                Err(AppError { location: String::new(), err: String::from(err.to_str().unwrap()) })
            },
            Error::Serde(err) => {
                logging::error!("{}", err.to_string());
                Err(AppError { location: String::new(), err: String::from(err) })
            },
            Error::OneshotCanceled(_) => {
                logging::error!("Oneshot Canceled");
                Err(AppError { location: String::new(), err: String::from("Oneshot Canceled") })
            },
        },
    }
}

#[component]
pub fn App() -> impl IntoView {
    view! {
    <Router>
        <main>
            <Routes>
                <Route path="/" view=|| view! {
                    <Redirect path="/auth" options=NavigateOptions {
                        resolve: true,
                        replace: true,
                        scroll: true,
                            state: State(None)
                    }/>
                }/>
                <Route path="/auth" view=Login/>
                <Route path="/home" view=Home/>
            </Routes>
        </main>
    </Router>
    }
}
