use core::fmt;
use std::borrow::Borrow;
use std::env;
use std::error;
use std::error::Error;
use dotenv::dotenv;
use main_loop::spawn_main_loop;
use main_loop::Command;
use tokio::sync::*;

mod wfm_client;
mod rate_limiter;
mod auth_state;
mod main_loop;

impl fmt::Display for RequestError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let description = match self {
            RequestError::Parse(e) => format!("{}", e),
            RequestError::Reqw(e) => format!("{}", e),
            RequestError::EnvVar(e) => format!("{}",e),
        };
        f.write_str(&description)
    }
}

impl error::Error for RequestError {}

#[derive(Debug)]
#[allow(dead_code)]
enum RequestError {
    Parse(url::ParseError),
    Reqw(reqwest::Error),
    EnvVar(env::VarError),
}

#[allow(dead_code)]
impl<'a> RequestError {
    fn from_parse(err: url::ParseError) -> RequestError {
        RequestError::Parse(err)
    }

    fn from_reqw(err: reqwest::Error) -> RequestError {
        RequestError::Reqw(err)
    }
    fn from_envvar(err: env::VarError) -> RequestError {
        RequestError::EnvVar(err)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenv().ok();
    let (handle, join) = spawn_main_loop();
    let email = env::var("USERNAME").unwrap();
    let password = env::var("PASSWORD").unwrap();
    let (resp_tx, resp_rx) = oneshot::channel::<String>();
    let cmd = Command::Login { creds: (email, password), resp: resp_tx };
    handle.chan.send(cmd);
    println!("{}", resp_rx.await.unwrap());

    let cmd = Command::Stop;
    handle.chan.send(cmd);

    join.await.unwrap();
    Ok(())
}

async fn run_client(tx: watch::Sender<&str>, mut rx: watch::Receiver<&str>) -> Result<(), RequestError> {
    // let mut headers: HeaderMap = HeaderMap::new();
    // headers.insert("cookie", "JWT=L".parse().unwrap());
    // headers.insert("Content-Type", "application/json; utf-8".parse().unwrap());
    // headers.insert("Accept", "application/json".parse().unwrap());
    // headers.insert("Authorization", "JWT".parse().unwrap());
    // headers.insert("platform", "pc".parse().unwrap());
    // headers.insert("language", "en".parse().unwrap());
    // let client = Client::builder()
    //     .cookie_store(true)
    //     .default_headers(headers).build().map_err(RequestError::from_reqw)?;
    // let mut logged_in = false;
    // let mut url: Url = Url::parse("https://api.warframe.market/v1").map_err(RequestError::from_parse)?;
    // let mut last_status: StatusCode = StatusCode::OK;
    println!("STARTED");
    loop {
        if *rx.borrow_and_update() == "STOP" {
            println!("STOPPED");
            break;
        };
        if *rx.borrow() == "LOGIN" {
            println!("something");
            tx.send("RECEIVED").unwrap();
        };
        if *rx.borrow_and_update() == "STOP" {
        }
    };
    Ok(())
}

/*
async fn login(client: &Client, url: &mut Url, status: &mut StatusCode, email: &str, password: &str ) -> Result<(), RequestError> {
    *url = Url::parse("https://api.warframe.market/v1/auth/signin").map_err(RequestError::from_parse)?;
    let resp = client.request(Method::POST, url.clone())
        .body(format!("{{ \"email\":\"{}\", \"password\":\"{}\" }}", email, password))
        .send().await.map_err(RequestError::from_reqw)?;
    *status = resp.status();
    println!("{}\n{}", resp.status(), resp.text().await.map_err(RequestError::from_reqw)?);
    Ok(())
}
*/
