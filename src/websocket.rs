use std::{net::{TcpListener, TcpStream}, sync::Arc, thread, time::Duration};

use maud::{html, PreEscaped};
use tokio::{sync::Mutex, task::JoinHandle};
use tungstenite::WebSocket;
use crate::{rivens::inventory::{convert_raw_inventory::{Item, Units}, database::InventoryDB, inventory_sync::sync_db, riven_lookop::RivenDataLookup}, server::RIVEN_LOOKUP, STOPPED};


pub async fn sync_ui(
    current_ui_rivens: &Vec<Item>,
    db: Arc<Mutex<InventoryDB>>,
    lookup: &RivenDataLookup,
) -> (Vec<Item>, Vec<Arc<str>>) {
    let (current_db_items, old_ids) = sync_db(db, lookup, None).await.unwrap();
    let new_items: Vec<Item> = current_db_items.into_iter()
        .filter(|upgrade|
            current_ui_rivens.iter()
                .find(|&item| item.oid == upgrade.oid).is_none()
    ).collect();
    (new_items, old_ids)
}

pub async fn init_rivens() -> Box<[PreEscaped<String>]> {
    let _ = std::fs::remove_file("inventory_db.sqlite3");
    let db = InventoryDB::open("inventory_db.sqlite3").expect("grrrr2");
    let db = Arc::new(Mutex::new(db));
    let lookup = RIVEN_LOOKUP.get().expect("FATAL: Could not access lookup data");
    let (mut new_rivens, _) = sync_ui(&Vec::new(), db, lookup).await;
    assert!(!new_rivens.is_empty());
    println!("new items: {}", new_rivens.len());
    new_rivens.sort_by(|a, b| a.attributes.len().cmp(&b.attributes.len()));
    let pagecontent = new_rivens.iter().fold(Vec::new(), |mut acc, riven| {
        let title = format!("{} {}", riven.weapon_name, riven.name);
        let stats = riven.attributes.iter().fold(PreEscaped::default(), |acc, attr|{
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

        // let target = format!("#{id}");

        // let height = format!("height: calc(126px + (2.2em * {}));", riven.attributes.len());
        acc.push(html! {
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
        });
        acc
    });
    pagecontent.into()
}

struct WsocHandle {
    handle: JoinHandle<()>,
    is_closed: bool,
}

impl WsocHandle {
    fn new(conn: WebSocket<TcpStream>) -> Self {
        // let handle = tokio::task::spawn(self);
        todo!()
    }
    fn close(&mut self) {
        self.is_closed = true;
    }
    // async fn handle(&mut self, mut conn: WebSocket<TcpStream>) {
    //     let rivens = init_rivens();
    //     let msg = html! {
    //         div id="riven-table" class="row" hx-swap-oob="beforeend" {
    //             (rivens)
    //         }
    //     };
    //     conn.send(msg.into_string().into()).unwrap();
    //     loop {
    //         if self.is_closed {
    //             let _ = conn.close(None);
    //             break;
    //         }
    //     }
    // }
}

#[tokio::main]
pub async fn start_websocket() {
    let server = TcpListener::bind("localhost:8069").expect("could not bind to port: ");
    server.set_nonblocking(true).expect("FATAL: Cannot set `TcpListener` as non-blocking");
    loop {
        if let Ok((stream, _addr)) = server.accept() {
            let mut wsoc_connection = tungstenite::accept(stream).expect("this should accept");
            println!("handshake complete");
            let rivens = init_rivens().await;
            rivens.iter().for_each(|riven| {
                assert!(!riven.clone().into_string().is_empty());
                /* 6532fb0a9b3e2564890cc667
                * 65d6e4d8833ce06fd505ba12
                * 6636d57a575ceadbef0880a6
                * 6637edfec7286187bd0974cf
                */
                let msg = html! {
                    div id="riven-table" class="row" hx-swap-oob="beforeend" {
                        (riven)
                    }
                };
                wsoc_connection.send(msg.into_string().into()).unwrap();
                thread::sleep(Duration::from_millis(10));
            });
            thread::sleep(Duration::from_secs(1));
            wsoc_connection.send(delete_riven("6532fb0a9b3e2564890cc667").into_string().into()).unwrap();
            thread::sleep(Duration::from_secs(1));
            wsoc_connection.send(delete_riven("65d6e4d8833ce06fd505ba12").into_string().into()).unwrap();
            thread::sleep(Duration::from_secs(1));
            wsoc_connection.send(delete_riven("6636d57a575ceadbef0880a6").into_string().into()).unwrap();
            thread::sleep(Duration::from_secs(1));
            wsoc_connection.send(delete_riven("6637edfec7286187bd0974cf").into_string().into()).unwrap();
            thread::sleep(Duration::from_secs(1));
            wsoc_connection.close(None).expect("should be closed");
            // let mut conn = WsocHandle::new(wsoc_connection);
        } else {
            if STOPPED.get() == Some(&true) {
                println!("WebSocket Closed");
                break;
            }
        };
        //if let Some(mut v) = current_conn.take() { // close the last connection
        //    v.close();
        //};
    }
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
