use tokio::{sync::{mpsc, oneshot::{self, Receiver}}, task::JoinHandle};
use crate::{auth_state::AuthState, wfm_client::client::{GenericError, WFMClient}};

#[derive(Debug)]
pub struct LoopHandle {
    pub chan: mpsc::Sender<Command>,
}

pub enum Command {
    Login{
        creds: (String, String),
        resp: Responder
    },
    Stop,
}

type Responder = oneshot::Sender<String>;

pub fn spawn_main_loop() -> (LoopHandle, JoinHandle<()>) {
    let (send, recv) = mpsc::channel(16);
    let handle = LoopHandle {
        chan: send,
    };

    let join = tokio::spawn(async move {
        match main_loop(recv).await {
            Ok(()) => {},
            Err(err) => {eprintln!("{}", err)}
        };
    });

    (handle, join)
}

async fn main_loop(mut recv: mpsc::Receiver<Command>) -> Result<(), GenericError> {
    let wfm_client = WFMClient::new();
    let mut _auth_state: AuthState;
    while let Some(cmd) = recv.recv().await {
        match cmd {
            Command::Login{creds, resp} => {
                _auth_state = match wfm_client.login(creds.0, creds.1).await {
                    Ok(v) => {
                        resp.send("Logged in!".to_string());
                        v
                    },
                    Err(err) => return Err(err.prop("MainLoop_Login: ".to_string()))
                }
            },
            Command::Stop => break
        }
    }
    println!("MainLoop: Exited");
    Ok(())
}
