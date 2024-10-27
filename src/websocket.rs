use std::{future::Future, net::{TcpListener, TcpStream}, sync::Arc};

use maud::{html, PreEscaped};
use serde_json::from_str;
use tokio::{sync::Mutex, task::JoinHandle};
use tungstenite::WebSocket;
use crate::{block_in_place, rivens::inventory::{convert_raw_inventory::{self, convert_inventory_data, Item, Units}, database::InventoryDB, inventory_sync::sync_db, raw_inventory::{self, decrypt_last_data}, riven_lookop::RivenDataLookup}};


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

pub async fn init_rivens() -> PreEscaped<String> {
    std::fs::remove_file("inventory_db.sqlite3").unwrap();
    let db = InventoryDB::open("inventory_db.sqlite3").expect("grrrr2");
    let db = Arc::new(Mutex::new(db));
    let lookup = RivenDataLookup::setup().await.expect("grrrr3");
    let (mut new_rivens, _) = sync_ui(&Vec::new(), db, &lookup).await;
    assert!(!new_rivens.is_empty());
    println!("new items: {}", new_rivens.len());
    new_rivens.sort_by(|a, b| a.attributes.len().cmp(&b.attributes.len()));
    let pagecontent = new_rivens.iter().fold(PreEscaped::default(),|acc, riven| {
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
        let id = format!("a{oid}");
        let uri = format!("/api/delete_riven/{oid}");
        let target = format!("#{id}");

        // let height = format!("height: calc(126px + (2.2em * {}));", riven.attributes.len());
        html! {
            (acc)
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
                        button class="cellbutton" hx-post="/edit" hx-target="#screen" hx-swap="beforeend" {"Edit"}
                        button class="cellbutton" hx-delete=(uri) hx-target=(target) hx-swap="outerHTML swap:.08s" style="background-color: #ff4444;" {"Delete"}
                    }
                    // img src="/wfm_favicon.ico" style="float: right; margin-left: 23px; padding-right: 13px;";
                }
            }
        }
    });
    pagecontent
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
    let lookup = Arc::new(
        RivenDataLookup::setup().await.expect(
        "FATAL: Could not retrieve riven lookup data"
        )
    );
    let server = TcpListener::bind("localhost:8069").expect("could not bind to port: ");
    let mut current_handle: Option<JoinHandle<()>> = None;
    loop {
        let (stream, _addr) = server.accept().expect("could not accept connection");
        //if let Some(mut v) = current_conn.take() { // close the last connection
        //    v.close();
        //};
        let mut wsoc_connection = tungstenite::accept(stream).expect("this should accept");
        println!("handshake complete");
        let rivens = init_rivens().await;
        assert!(!rivens.clone().into_string().is_empty());
        let msg = html! {
            div id="riven-table" class="row" hx-swap-oob="beforeend" {
                (rivens)
            }
        };
        wsoc_connection.send(msg.into_string().into()).unwrap();
        wsoc_connection.close(None).expect("should be closed");
        // let mut conn = WsocHandle::new(wsoc_connection);
    }
}
