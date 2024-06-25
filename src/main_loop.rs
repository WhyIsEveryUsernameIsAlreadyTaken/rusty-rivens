use std::time::{Duration, SystemTime};

use tokio::{sync::{mpsc, oneshot}, task::JoinHandle, time};
use crate::{riven_data_store::get_rivens, wfm_client::client::{GenericError, WFMClient}};

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
    GetAllRivens{
        resp: Responder,
        wfmc: WFMClient
    },
    Stop,
    _Test,
    _UpdateAllRivens,
}

pub struct ResponseCommand(pub &'static str);

type Responder = oneshot::Sender<ResponseCommand>;

pub fn spawn_event_loop() -> (LoopHandle, JoinHandle<()>) {
    let (send, recv) = mpsc::channel(16);
    let handle = LoopHandle {
        chan: send,
    };

    let join = tokio::spawn(async move {
        match event_loop(recv).await {
            Ok(()) => {},
            Err(err) => {eprintln!("{}", err)}
        }
    });

    (handle, join)
}


async fn event_loop(mut recv: mpsc::Receiver<Command>) -> Result<(), GenericError> {
    let mut _time = SystemTime::now();
    let mut handles = Vec::with_capacity(1024);

    while let Some(cmd) =recv.recv().await {
        match cmd {
            Command::Login{creds, resp, wfmc} => handles.push(tokio::spawn(
                async move {
                    match wfmc.login(creds.0, creds.1).await {
                        Ok(_) => {
                            let _ = resp.send(ResponseCommand("Logged in!"));
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

            Command::GetAllRivens{resp, wfmc} => handles.push(tokio::spawn(
                async move {
                    get_rivens(wfmc, resp).await?;
                    Ok(())
                }
            )),

            Command::_Test => handles.push(tokio::spawn(async {
                time::sleep(Duration::new(3, 0)).await;
                Ok(())
            })),

            Command::_UpdateAllRivens => {
                _time = SystemTime::now();
                handles.push(tokio::spawn(async {
                    todo!()
                }))
            }
        }

        if _time.elapsed().unwrap().as_secs() / 300 == 1 {
            println!("It's been 5 minutes since all rivens were updated. Please consider auto updating all rivens to stop them cuttahs.")
        }

        await_finished_handles(&mut handles).await.map_err(|e| e.prop("MainLoop: ".to_string()))?;
    }
    for v in handles {
        match v.await.map_err(|e| GenericError::new(e, "MainLoop: ".to_string()))? {
            Ok(_) => continue,
            Err(e) => return Err(e)
        }
    }
    println!("MainLoop: Exited");
    Ok(())
}

type HandleVec = Vec<JoinHandle<Result<(), GenericError>>>;

async fn await_finished_handles(hand_vec: &mut HandleVec) -> Result<(), GenericError> {
    let mut i: usize = 0;
    while i < hand_vec.len() {
        if hand_vec[i].is_finished() {
            match hand_vec.swap_remove(i).await.map_err(|e| GenericError::new(e, "await_finished_handles: ".to_string())) {
                Ok(_) => {
                    continue;
                },
                Err(e) => return Err(e)
            }
        }
        i += 1;
    }
    Ok(())
}
