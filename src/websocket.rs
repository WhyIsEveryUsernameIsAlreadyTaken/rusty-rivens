use std::{
    fs, io,
    net::{SocketAddr, TcpListener, TcpStream},
    sync::Arc,
    thread,
    time::{Duration, SystemTime},
};

use crate::{
    rivens::inventory::{
        convert_raw_inventory::{Attribute, Item, Units},
        database::{database::InventoryDB, inventory_sync::sync_db},
        riven_lookop::RivenDataLookup,
    },
    server::RIVEN_LOOKUP,
    StopSignal,
};
use maud::{html, PreEscaped};
use tokio::{
    select,
    sync::{
        broadcast::{self, Receiver},
        Mutex,
    },
    task::JoinHandle,
};
use tungstenite::WebSocket;

#[derive(Debug, Clone)]
enum MessageType {
    CloseFrame,
    HTML(Vec<PreEscaped<String>>),
}

fn send_elements(conn: &mut WebSocket<TcpStream>, new_elements: Vec<PreEscaped<String>>) {
    new_elements.into_iter().for_each(|element| {
        match conn.send(element.into_string().into()) {
            Ok(_) => println!("INFO: Sent message to client"),
            Err(e) => println!("ERROR: Could not send message to client: {e:?}"),
        }

        // just for looks, can be removed in the future
        thread::sleep(Duration::from_millis(10));
    });
}

async fn handle(mut conn: WebSocket<TcpStream>, mut receiver: Receiver<MessageType>) {
    loop {
        if let Ok(new_content) = receiver.recv().await {
            println!("INFO: received content through channel");
            match new_content {
                MessageType::CloseFrame => break,
                MessageType::HTML(new_elements) => {
                    println!("INFO: {} new elements", new_elements.len());
                    send_elements(&mut conn, new_elements);
                }
            }
        } else {
            println!("ERROR: Couldn't retrieve channel message");
        }
    }
}

struct LastModified(SystemTime, SystemTime);

impl LastModified {
    fn detect_file_change(&mut self) -> io::Result<bool> {
        let attrs = fs::metadata("lastData.dat")?;
        self.1 = attrs.modified().unwrap();
        if self.1 != self.0 {
            self.0 = self.1;
            return Ok(true);
        }
        Ok(false)
    }
}

async fn handle_connection(
    accept_result: Result<(TcpStream, SocketAddr), io::Error>,
    rivens: &mut Vec<Item>,
    current_connection: &mut Option<JoinHandle<()>>,
    last_modified: &mut LastModified,
    sender: &broadcast::Sender<MessageType>,
    db: Arc<Mutex<Option<InventoryDB>>>,
    lookup: &RivenDataLookup,
) {
    if let Ok((stream, _addr)) = accept_result {
        let wsoc_connection =
            tungstenite::accept(stream).expect("FATAL: Failed to handshake with client");

        // if there's an existing connection and the client is reconnecting
        if let Some(_current_conn) = &current_connection {
            sender
                .send(MessageType::CloseFrame)
                .expect("FATAL: Could not send channel close frame");
            *current_connection = None;
            println!("INFO: reconnecting");

            // if there isnt an existing connection and the client is connecting
            // for the first time
        } else {
            assert!(
                current_connection.is_none(),
                "FATAL: There should be no existing connection at this point"
            );
            *current_connection = Some(tokio::task::spawn(handle(
                wsoc_connection,
                sender.subscribe(),
            )));
            println!("INFO: Handshake complete");

            let new_elements = sync_ui(rivens.clone(), vec![]).await;
            if let Err(e) = sender.send(MessageType::HTML(new_elements)) {
                println!("ERROR: Could not send message through channel: {e}")
            } else {
                println!("INFO: Sent content through channel")
            };
        };

        // when not handling new connections
    } else {
        if current_connection.is_some() {
            if last_modified.detect_file_change().unwrap_or(false) {
                // get changes in database and inventory state
                let (new_rivens, old_ids) = sync_ui_rivens(rivens, db.clone(), lookup).await;
                let new_elements = sync_ui(new_rivens, old_ids).await;

                // send new elements to the connection handle
                if let Err(e) = sender.send(MessageType::HTML(new_elements)) {
                    println!("ERROR: Could not send message through channel: {e}")
                };
            }
        }
    }
}

#[tokio::main]
pub async fn start_websocket(mut stop_signal: Receiver<StopSignal>) {
    let server = TcpListener::bind("localhost:8069").expect("FATAL: could not bind to port: ");

    let db = InventoryDB::open("inventory_db.sqlite3").expect("grrrr2");
    let db = Arc::new(Mutex::new(Some(db)));

    let lookup = RIVEN_LOOKUP
        .get()
        .expect("FATAL: Could not access lookup data");

    let mut current_connection: Option<JoinHandle<()>> = None;
    let mut rivens = Vec::new();
    let mut last_modified = LastModified(SystemTime::UNIX_EPOCH, SystemTime::UNIX_EPOCH);

    // dont need the returned rivens or old id's as they're automatically
    // appended to `rivens` on startup and we wont have anything.
    let _ = sync_ui_rivens(&mut rivens, db.clone(), lookup).await;

    // we're using a channel here to be able to communicate with connection
    // handles being used throughout the server's lifetime.
    // although only one connection will be handled at any given time, using a
    // broadcast channel is just for convenience to be able to quickly get a new
    // receiver just by subscribing to the sender end for each reconnection.
    let (sender, _) = broadcast::channel::<MessageType>(50);

    loop {
        select! {
            accept_result = async { server.accept() } => {
                handle_connection(
                    accept_result,
                    &mut rivens,
                    &mut current_connection,
                    &mut last_modified,
                    &sender,
                    db.clone(),
                    lookup
                ).await
            }
                _ = stop_signal.recv() => {
                    sender
                        .send(MessageType::CloseFrame)
                        .expect("FATAL: Could not send channel close frame");

                    if let Some(conn) = current_connection {
                        conn.await
                            .expect("FATAL: could not shut down client connection");
                    }

                    println!("INFO: shutting down client connection");
                    println!("INFO: WebSocket Closed");
                    break;
                }
        };
    }

    let mut db_mutex = db.lock_owned().await;
    let db = db_mutex.take();
    db.unwrap()
        .close()
        .expect("FATAL: Error while closing database connection");
}

pub async fn sync_ui_rivens(
    current_ui_rivens: &mut Vec<Item>,
    db: Arc<Mutex<Option<InventoryDB>>>,
    lookup: &RivenDataLookup,
) -> (Vec<Item>, Vec<Arc<str>>) {
    let (current_db_items, old_ids) = sync_db(db, lookup, None).await.unwrap();

    let mut new_items: Vec<Item> = current_db_items
        .into_iter()
        .filter(|upgrade| {
            current_ui_rivens
                .iter()
                .find(|&item| item.oid == upgrade.oid)
                .is_none()
        })
        .collect();

    if !old_ids.is_empty() {
        old_ids.iter().for_each(|oid| {
            *current_ui_rivens = current_ui_rivens
                .iter()
                .filter_map(|item| {
                    if item.oid != oid.clone() {
                        Some(item.to_owned())
                    } else {
                        None
                    }
                })
                .collect()
        });
    }
    current_ui_rivens.append(&mut new_items);
    (new_items, old_ids)
}

fn construct_stats(attributes: &[Attribute]) -> PreEscaped<String> {
    attributes.iter().fold(PreEscaped::default(), |acc, attr| {
        let stat = match attr.units {
            Units::Percent => {
                if attr.positive {
                    format!("{}% {}", attr.value, attr.short_string)
                } else {
                    format!("{}% {}", attr.value, attr.short_string)
                }
            }
            Units::Multiply => format!("x{} {}", attr.value, attr.short_string),
            Units::Seconds => {
                if attr.positive {
                    format!("{}s {}", attr.value, attr.short_string)
                } else {
                    format!("{}s {}", attr.value, attr.short_string)
                }
            }
            Units::Null => format!("{} {}", attr.value, attr.short_string),
        };
        html! {
        (acc)
        p style="text-align: center; margin: 10px;"{(stat)}
        }
    })
}

fn construct_riven_element(
    id: &str,
    title: &str,
    edit_uri: &str,
    stats: PreEscaped<String>,
) -> PreEscaped<String> {
    html! {
        div id="riven-table" class="row" hx-swap-oob="beforeend" {
            div class="cell" id=(id) {
                div class="celltitle" {
                    (title)
                }
                hr style="width: 100%";
                div style="flex-grow: 1"{
                    (stats)
                }
                div class="cellfooterdiv" {
                    div style="float: left;" {
                        button
                            class="cellbutton"
                            hx-post=(edit_uri)
                            hx-target="#screen"
                            hx-swap="beforeend" {"Edit"}
                    }
                    // img src="/wfm_favicon.ico" style="float: right; margin-left: 23px; padding-right: 13px;";
                }
            }
        }
    }
}

pub async fn sync_ui(
    mut new_rivens: Vec<Item>,
    delete_ids: Vec<Arc<str>>,
) -> Vec<PreEscaped<String>> {
    new_rivens.sort_by(|a, b| a.attributes.len().cmp(&b.attributes.len()));
    let mut pagecontent =
        new_rivens
            .iter()
            .fold(Vec::with_capacity(new_rivens.len()), |mut acc, riven| {
                let title = format!("{} {}", riven.weapon_name, riven.name);

                let stats = construct_stats(&riven.attributes);

                let oid = riven.oid.clone();
                let id = format!("a{oid}");

                let edit_uri = format!("/edit_open/{oid}");

                acc.push(construct_riven_element(
                    id.as_str(),
                    title.as_str(),
                    edit_uri.as_str(),
                    stats,
                ));
                acc
            });
    if !delete_ids.is_empty() {
        pagecontent.reserve(delete_ids.len());
        delete_ids.iter().for_each(|id| {
            pagecontent.push(delete_riven(id));
        });
    }
    pagecontent
}

fn delete_riven(id: &str) -> PreEscaped<String> {
    let del_id = format!("a{id}");
    let del_id_target = format!("#{del_id}");
    let target = format!("outerHTML:#{del_id}");
    let uri = format!("/api/delete_riven/{id}");
    html! {
        div id=(del_id) hx-swap-oob=(target) {
            div
                hx-delete=(uri)
                hx-swap="outerHTML settle:.08s"
                hx-target=(del_id_target)
                hx-trigger="load";
        }
    }
}
