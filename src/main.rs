use std::{error::Error, fmt::{self, Display}, sync::Arc, thread};

use dotenv::dotenv;
use once_cell::sync::OnceCell;
use serde::Deserialize;
use server::start_server;

mod pages;
mod resources;
mod server;
mod rate_limiter;
mod jwt;
mod file_consts;
mod rivens;
mod riven_data_store;
mod http_client;

static STOPPED: OnceCell<bool> = once_cell::sync::OnceCell::new();

#[derive(Debug, Deserialize)]
pub struct AppError {
    pub location: Arc<str>,
    pub err: Arc<str>,
}

impl Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(&format!("{}: {:?}", self.location, self.err))
    }
}

impl AppError {
    pub fn new(err: String, loc: String) -> Self {
        Self {
            location: loc.into(),
            err: err.into(),
        }
    }
    pub fn prop(&self, new_loc: Arc<str>) -> Self {
        let new_loc = new_loc.trim();
        Self {
            location: format!("{}::{}", new_loc, self.location).into(),
            err: self.err.clone(),
        }
    }
}

impl Error for AppError {}

fn main() {
    dotenv().unwrap(); // creds
    // START SERVER ON SEPERATE THREAD
    let server = thread::spawn(move || start_server().unwrap());

    web_view::builder()
        .title("Rusty Rivens v0.0.1")
        .content(web_view::Content::Url("http://127.0.0.1:8000/"))
        .size(1280, 720)
        .resizable(true)
        .debug(true)
        .user_data(())
        .invoke_handler(|_webview, _arg| Ok(()))
        .run().unwrap();

    STOPPED.set(true).unwrap();
    server.join().unwrap();
}
