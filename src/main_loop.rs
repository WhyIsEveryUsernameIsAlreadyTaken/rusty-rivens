use std::{collections::LinkedList, time::Duration};

use tokio::{sync::{mpsc, oneshot}, task::JoinHandle, time};
use crate::{auth_state::AuthState, wfm_client::client::{GenericError, WFMClient}};

#[derive(Debug)]
pub struct LoopHandle {
    pub chan: mpsc::Sender<Command>,
}

pub enum Command {
    Login{
        creds: (String, String),
        resp: Responder,
        wfmc: WFMClient
    },
    Stop,
    Test,
}

pub enum ResponseCommand<'a> {
    LoggedIn{
        resp: &'a str,
        auth: AuthState
    }
}

type Responder = oneshot::Sender<ResponseCommand<'static>>;

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
    let mut handles = LinkedList::new();
    while let Some(cmd) = recv.recv().await {
        match cmd {
            Command::Login{creds, resp, wfmc} => handles.push_back(tokio::spawn(
                async move {
                    match wfmc.login(creds.0, creds.1).await {
                        Ok(v) => {
                            let _ = resp.send(ResponseCommand::LoggedIn{resp: "Logged in!", auth: v});
                        },
                        Err(err) => return Err(err.prop("MainLoop_Login: ".to_string()))
                    }
                    Ok(())
                }
            )),
            Command::Stop => {
                println!("STOPPED");
                break
            },
            Command::Test => handles.push_back(tokio::spawn(async {
                time::sleep(Duration::new(1, 0)).await;
                println!("TEST");
                Ok(())
            }))
        }
    }
    for v in handles.into_iter().filter(|v| v.is_finished()) {
        match v.await {
            Ok(v) => v?,
            Err(e) => return Err(GenericError::new(e, "MainLoop: ".to_string()))
        }
    }
    println!("MainLoop: Exited");
    Ok(())
}
