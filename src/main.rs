use std::{error::Error, fmt::{self, Display}, sync::Arc, thread};


use once_cell::sync::OnceCell;
use serde::Deserialize;
use server::start_server;
use tao::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};
use tokio::runtime::{Handle, Runtime};
use wry::WebViewBuilder;

mod pages;
mod resources;
mod server;
mod rate_limiter;
mod jwt;
mod file_consts;
mod rivens;
mod riven_data_store;
mod api_operations;
mod websocket;
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

#[macro_export]
macro_rules! block_in_place {
    ($x:expr) => {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on($x)
        })
    };
}

#[tokio::main]
async fn main() -> wry::Result<()> {
    tokio::task::spawn(start_server());
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new().build(&event_loop).unwrap();

    #[cfg(any(
        target_os = "windows",
        target_os = "macos",
        target_os = "ios",
        target_os = "android"
    ))]
    let builder = WebViewBuilder::new(&window);

    #[cfg(not(any(
        target_os = "windows",
        target_os = "macos",
        target_os = "ios",
        target_os = "android"
    )))]
    let builder = {
        use tao::platform::unix::WindowExtUnix;
        use wry::WebViewBuilderExtUnix;
        let vbox = window.default_vbox().unwrap();
        WebViewBuilder::new_gtk(vbox)
    };

    let _webview = builder
        .with_url("http://127.0.0.1:8000")
        .with_drag_drop_handler(|e| {
            match e {
                wry::DragDropEvent::Enter { paths, position } => {
                    println!("DragEnter: {position:?} {paths:?} ")
                }
                wry::DragDropEvent::Over { position } => println!("DragOver: {position:?} "),
                wry::DragDropEvent::Drop { paths, position } => {
                    println!("DragDrop: {position:?} {paths:?} ")
                }
                wry::DragDropEvent::Leave => println!("DragLeave"),
                _ => {}
            }

            true
        })
        .build()?;

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        if let Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            ..
        } = event
        {
            STOPPED.set(true).unwrap();
            *control_flow = ControlFlow::Exit
        }
    });
}
