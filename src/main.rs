use std::{error::Error, fmt::{self, Display}, ops::Deref, rc::Rc, sync::Arc, thread};

use dotenv::dotenv;

use once_cell::sync::OnceCell;
use serde::Deserialize;
use server::start_server;
use tao::{
    event::{Event, StartCause, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
  };
use wry::WebViewBuilder;

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

fn main() -> wry::Result<()> {
    dotenv().unwrap(); // creds
    // START SERVER ON SEPERATE THREAD
    // let server = thread::spawn(move || start_server().unwrap());
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("Rusty Rivens v0.0.1")
        .build(&event_loop).unwrap();

    let _webview = WebViewBuilder::new(&window)
        .with_url("https://tauri.app")
        .with_user_agent("Rusty Rivens v0.0.1")
        .with_devtools(true)
        .with_visible(true)
        .with_transparent(false)
        .with_focused(true)
        .build()?;

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        println!("I hate this framework");
        match event {
            Event::NewEvents(StartCause::Init) => println!("I hate this framework"),
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                STOPPED.set(true).unwrap();
                *control_flow = ControlFlow::Exit
            },
            _ => ()
        }
    });
}
