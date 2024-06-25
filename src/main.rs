use std::env;
use std::error::Error;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::SystemTime;
use auth_state::AuthState;
use dotenv::dotenv;
use main_loop::spawn_event_loop;
use main_loop::Command;
use main_loop::ResponseCommand;
use tokio::sync::*;
use wfm_client::client::WFMClient;

mod wfm_client;
mod rivens;
mod rate_limiter;
mod auth_state;
mod riven_data_store;
mod main_loop;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let _now = SystemTime::now();
    dotenv().ok();
    let (handle, join) = spawn_event_loop();
    let email = env::var("USERNAME").unwrap();
    let password = env::var("PASSWORD").unwrap();
    let (resp_tx, resp_rx) = oneshot::channel::<ResponseCommand>();
    let mut _auth_state = Arc::new(Mutex::new(AuthState::setup()?));
    let wfm_client = WFMClient::new(_auth_state);
    let cmd = Command::Login { creds: (email, password), resp: resp_tx, wfmc: wfm_client.clone()};
    handle.chan.send(cmd).await?;
    match resp_rx.await {
        Ok(v) => match v {
            ResponseCommand(resp) => {
                println!("{}", resp);
            }
        },
        Err(e) => println!("{}", e.to_string())
    };
    let (resp_tx, resp_rx) = oneshot::channel::<ResponseCommand>();

    let cmd = Command::GetAllRivens { resp: resp_tx, wfmc: wfm_client.clone() };
    handle.chan.send(cmd).await?;
    match resp_rx.await {
        Ok(v) => match v {
            ResponseCommand(resp) => {
                println!("{}", resp);
            }
        },
        Err(e) => println!("{}", e.to_string())
    }

    // let mut futures = Vec::new();
    // for _ in 0..100000 {
    //     futures.push(handle.chan.send(Command::_Test));
    // }

    // for f in futures {
    //     f.await?
    // }

    let cmd = Command::Stop;
    handle.chan.send(cmd).await?;

    join.await.unwrap();
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
