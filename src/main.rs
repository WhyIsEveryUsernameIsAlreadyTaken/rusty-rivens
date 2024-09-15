use std::{error::Error, fmt::{self, Display}, sync::Arc, thread};

use gtk::{
    prelude::{ApplicationExt, ApplicationExtManual, ContainerExt, WidgetExt}, Application, ApplicationWindow
};
use once_cell::sync::OnceCell;
use serde::Deserialize;
use server::start_server;
use webkit2gtk::{SettingsExt, WebContext, WebContextExt, WebView, WebViewExt};

mod pages;
mod resources;
mod server;
mod rate_limiter;
mod jwt;
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

// impl Serialize for AppError {
//     fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
//     where
//         S: serde::Serializer,
//     {
//         let mut state = serializer.serialize_struct("AppError", 2)?;
//         state.serialize_field("location", &self.location.deref())?;
//         state.serialize_field("err", &self.err.deref())?;
//         state.end()
//     }
// }

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
            location: format!("{}: {}", new_loc, self.location).into(),
            err: self.err.clone(),
        }
    }
}

impl Error for AppError {}

fn main() {
    // START SERVER ON SEPERATE THREAD
    let server = thread::spawn(move || start_server().unwrap());

    let app = Application::builder()
        .application_id("org.example.HelloWorld")
        .build();
    app.connect_activate(|app| {
        let win = ApplicationWindow::builder()
            .application(app)
            .default_width(1280)
            .default_height(720)
            .title("guhhhh")
            .build();

        let context = WebContext::default().unwrap();
        context.set_cache_model(webkit2gtk::CacheModel::WebBrowser);
        let webview = WebView::with_context(&context);
        // YOU CONNECT HERE RIGHT AFTER
        webview.load_uri("http://127.0.0.1:8000/");
        win.add(&webview);

        let settings = WebViewExt::settings(&webview).unwrap();

        settings.set_enable_developer_extras(true);
        settings.set_user_agent_with_application_details(Some("UwU"), Some("v0.0.1"));

        win.show_all();
    });

    app.run();
    STOPPED.set(true).unwrap();
    // server.join().unwrap();
}
