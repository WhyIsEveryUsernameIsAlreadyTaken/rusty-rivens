use std::{
    fs, io,
    net::{TcpListener, TcpStream},
    sync::Arc,
    thread,
    time::{Duration, SystemTime},
};

use crate::{
    rivens::inventory::{
        convert_raw_inventory::{Item, Units},
        database::{database::InventoryDB, inventory_sync::sync_db},
        riven_lookop::RivenDataLookup,
    },
    server::RIVEN_LOOKUP,
    STOPPED,
};
use maud::{html, PreEscaped};
use tokio::{
    sync::{broadcast, Mutex},
    task::JoinHandle,
};
use tungstenite::WebSocket;

#[derive(Debug, Clone)]
enum MessageType {
    CloseFrame,
    HTML(Vec<PreEscaped<String>>),
}

async fn handle(mut conn: WebSocket<TcpStream>, mut receiver: broadcast::Receiver<MessageType>) {
    loop {
        // if !conn.can_write() {
        //     println!("INFO: Closing connection");
        //     conn.close(None).unwrap();
        //     break;
        // }
        if let Ok(new_content) = receiver.try_recv() {
            println!("INFO: received content through channel");
            match new_content {
                MessageType::CloseFrame => break,
                MessageType::HTML(new_elements) => {
                    println!("{} new elements", new_elements.len());
                    new_elements.into_iter().for_each(|element| {
                        match conn.send(element.into_string().into()) {
                            Ok(_) => println!("INFO: Sent message to client"),
                            Err(e) => println!("ERROR: Could not send message to client: {e:?}"),
                        }
                        thread::sleep(Duration::from_millis(10));
                    });
                }
            }
        } else {
            continue;
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

#[tokio::main]
pub async fn start_websocket() {
    let server = TcpListener::bind("localhost:8069").expect("could not bind to port: ");
    let db = InventoryDB::open("inventory_db.sqlite3").expect("grrrr2");
    let db = Arc::new(Mutex::new(Some(db)));
    server
        .set_nonblocking(true)
        .expect("FATAL: Cannot set `TcpListener` as non-blocking");
    let mut current_connection: Option<JoinHandle<()>> = None;
    let mut rivens = Vec::new();
    let lookup = RIVEN_LOOKUP
        .get()
        .expect("FATAL: Could not access lookup data");
    let _ = sync_ui_rivens(&mut rivens, db.clone(), lookup).await;
    let mut last_modified = LastModified(SystemTime::UNIX_EPOCH, SystemTime::UNIX_EPOCH);
    let (sender, _) = broadcast::channel::<MessageType>(50);
    loop {
        if let Ok((stream, _addr)) = server.accept() {
            let wsoc_connection = tungstenite::accept(stream).expect("this should accept");
            if let Some(current_conn) = &current_connection {
                sender
                    .send(MessageType::CloseFrame)
                    .expect("FATAL: Could not send channel close frame");
                current_conn.abort();
                current_connection = None;
                println!("INFO: reconnecting");
            } else {
                current_connection = Some(tokio::task::spawn(handle(
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
        } else {
            if current_connection.is_some() {
                if last_modified.detect_file_change().unwrap_or(false) {
                    let (new_rivens, old_ids) =
                        sync_ui_rivens(&mut rivens, db.clone(), lookup).await;
                    let new_elements = sync_ui(new_rivens, old_ids).await;
                    if let Err(e) = sender.send(MessageType::HTML(new_elements)) {
                        println!("ERROR: Could not send message through channel: {e}")
                    };
                }
            }
            if STOPPED.get() == Some(&true) {
                println!("WebSocket Closed");
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

pub async fn sync_ui(
    mut new_rivens: Vec<Item>,
    delete_ids: Vec<Arc<str>>,
) -> Vec<PreEscaped<String>> {
    new_rivens.sort_by(|a, b| a.attributes.len().cmp(&b.attributes.len()));
    let mut pagecontent = new_rivens.iter().fold(Vec::with_capacity(new_rivens.len()), |mut acc, riven| {
        let title = format!("{} {}", riven.weapon_name, riven.name);
        let stats = riven.attributes.iter().fold(PreEscaped::default(), |acc, attr| {
            let stat = match attr.units {
                Units::Percent => {
                    if attr.positive {
                        format!("{}% {}", attr.value, attr.short_string)
                    } else {
                        format!("{}% {}", attr.value, attr.short_string)
                    }
                },
                Units::Multiply => format!("x{} {}", attr.value, attr.short_string),
                Units::Seconds => {
                    if attr.positive {
                        format!("{}s {}", attr.value, attr.short_string)
                    } else {
                        format!("{}s {}", attr.value, attr.short_string)
                    }
                },
                Units::Null => format!("{} {}", attr.value, attr.short_string),
            };
            html! {
                (acc)
                p style="text-align: center; margin: 10px;"{(stat)}
            }
        });
        let oid = riven.oid.clone();
        println!("{oid}");
        let id = format!("a{oid}");

        let edit_uri = format!("/edit_open/{oid}");

        acc.push(html! {
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
                            button class="cellbutton" hx-post=(edit_uri) hx-target="#screen" hx-swap="beforeend" {"Edit"}
                            // button class="cellbutton" hx-delete=(delete_uri) hx-target=(target) hx-swap="outerHTML swap:.08s" style="background-color: #ff4444;" {"Delete"}
                        }
                        // img src="/wfm_favicon.ico" style="float: right; margin-left: 23px; padding-right: 13px;";
                    }
                }
            }
        });
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
            div hx-delete=(uri) hx-swap="outerHTML settle:.08s" hx-target=(del_id_target) hx-trigger="load";
        }
    }
}
