use std::sync::Arc;

use rusqlite::{params, Connection, Transaction};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::rivens::inventory::convert_raw_inventory::{Attribute, Item, Units};

pub struct InventoryDB {
    connection: Connection,
}

// TODO: need to refactor this to not have everything be public, and refactor
// other places to have better controlled access to this struct
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Auction {
    pub starting_price: Option<u32>,
    pub buyout_price: Option<u32>,
    pub owner: Option<String>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub updated: Option<OffsetDateTime>,
    pub is_direct_sell: bool,
    pub id: Option<String>,
    pub oid: String,
}

impl Default for Auction {
    fn default() -> Self {
        Self {
            starting_price: None,
            buyout_price: None,
            owner: None,
            updated: None,
            is_direct_sell: true,
            id: None,
            oid: String::new(),
        }
    }
}

static SQL_TABLE_ITEMS: &str = "CREATE TABLE IF NOT EXISTS items ( item_id text primary key, mastery_level integer, name text, weapon_name text, polarity text, weapon_url_name text, re_rolls integer, mod_rank integer)";
static SQL_TABLE_ATTRIBUTES: &str = "CREATE TABLE IF NOT EXISTS attributes ( item_id text, value float, positive bit, units text, url_name text, short_string text)";
static SQL_TABLE_AUCTIONS: &str = "CREATE TABLE IF NOT EXISTS auctions ( item_id text primary key, wfm_id text, starting_price integer, buyout_price integer, owner text, updated datetime, is_direct_sell bit)";

static SQL_ATTRIBUTE_INSERT: &str = "INSERT INTO attributes ( item_id, value, positive, units, url_name, short_string) values (?1, ?2, ?3, ?4, ?5, ?6)";
static SQL_AUCTION_INSERT: &str = "INSERT INTO auctions ( oid, wfm_id, starting_price, buyout_price, owner, updated, is_direct_sell) values (?1, ?2, ?3, ?4, ?5, ?6, ?7)";
static SQL_ITEM_INSERT: &str = "INSERT INTO items ( item_id, mastery_level, name, weapon_name, polarity, weapon_url_name, re_rolls, mod_rank) values (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)";

static SQL_SELECT_ITEMS: &str = "SELECT * FROM items";
static SQL_SELECT_ATTRIBUTES: &str = "SELECT * FROM attributes WHERE item_id = ?1";
static SQL_SELECT_AUCTIONS: &str = "SELECT * FROM auctions";

static SQL_DELETE_ITEMS: &str = "DELETE FROM items WHERE item_id = ?1";
static SQL_DELETE_ATTRIBUTES: &str = "DELETE FROM attributes WHERE item_id = ?1";
static SQL_DELETE_AUCTIONS: &str = "DELETE FROM auctions WHERE oid = ?1";

impl InventoryDB {
    pub fn open(custom_path: &str) -> Result<Self, rusqlite::Error> {
        let mut connection = Connection::open(custom_path)?;
        let tx = connection.transaction()?;
        tx.execute(SQL_TABLE_ITEMS, ())?;
        tx.execute(SQL_TABLE_AUCTIONS, ())?;
        tx.execute(SQL_TABLE_ATTRIBUTES, ())?;
        tx.commit()?;
        Ok(Self { connection })
    }

    pub fn close(self) -> Result<(), (Connection, rusqlite::Error)> {
        println!("INFO: Database connection closed");
        self.connection.close()
    }

    fn insert_auctions(
        &mut self,
        auctions: Vec<Auction>,
        oid: &str,
    ) -> Result<(), rusqlite::Error> {
        let tx = self.connection.transaction()?;
        let mut auc_insert = tx.prepare(SQL_AUCTION_INSERT)?;

        auctions
            .iter()
            .try_for_each(|auc| -> Result<(), rusqlite::Error> {
                let res = auc_insert.execute(params![
                    &oid,
                    &auc.id,
                    &auc.starting_price,
                    &auc.buyout_price,
                    &auc.owner,
                    &auc.updated,
                    &auc.is_direct_sell
                ]);
                if res.is_err() {
                    Err(res.unwrap_err())
                } else {
                    Ok(())
                }
            })?;

        drop(auc_insert);
        tx.commit()
    }
}

fn insert_attributes(
    tx: &Transaction,
    attributes: &Vec<Attribute>,
    oid: &str,
) -> Result<(), rusqlite::Error> {
    let mut attr_insert = tx.prepare(SQL_ATTRIBUTE_INSERT)?;

    println!("{attributes:#?}");
    attributes
        .iter()
        .try_for_each(|attr| -> Result<(), rusqlite::Error> {
            let res = attr_insert.execute(params![
                oid,
                attr.value,
                attr.positive,
                attr.units,
                attr.url_name,
                attr.short_string,
            ]);
            if res.is_err() {
                Err(res.unwrap_err())
            } else {
                Ok(())
            }
        })
}

impl InventoryDB {
    pub(super) fn insert_items(&mut self, items: &Vec<Item>) -> Result<(), rusqlite::Error> {
        println!("inserting items");
        let tx = self.connection.transaction()?;
        let mut item_insert = tx.prepare(SQL_ITEM_INSERT)?;

        items
            .iter()
            .try_for_each(|item| -> Result<(), rusqlite::Error> {
                let res = item_insert.execute(params![
                    item.oid,
                    item.mastery_level,
                    item.name,
                    item.weapon_name,
                    item.polarity,
                    item.weapon_url_name,
                    item.re_rolls,
                    item.mod_rank
                ]);
                insert_attributes(&tx, &item.attributes, &item.oid)?;
                if res.is_err() {
                    Err(res.unwrap_err())
                } else {
                    Ok(())
                }
            })?;

        drop(item_insert);
        tx.commit()
    }

    pub(super) fn delete_items_auctions(
        &mut self,
        items: Vec<Arc<str>>,
    ) -> Result<(), rusqlite::Error> {
        items
            .into_iter()
            .try_for_each(|oid| -> Result<(), rusqlite::Error> {
                let mut items_delete = self.connection.prepare(SQL_DELETE_ITEMS)?;
                let mut attrs_delete = self.connection.prepare(SQL_DELETE_ATTRIBUTES)?;
                let mut aucs_delete = self.connection.prepare(SQL_DELETE_AUCTIONS)?;

                items_delete.execute(&[&oid])?;
                attrs_delete.execute(&[&oid])?;
                aucs_delete.execute(&[&oid])?;
                Ok(())
            })
    }

    pub(super) fn select_items(&self) -> Result<Vec<Item>, rusqlite::Error> {
        let mut items_select = self.connection.prepare(SQL_SELECT_ITEMS)?;
        let items = items_select
            .query_map([], |row| {
                Ok(Item {
                    mastery_level: row.get("mastery_level")?,
                    name: row.get("name")?,
                    weapon_name: row.get("weapon_name")?,
                    polarity: row.get("polarity")?,
                    attributes: vec![],
                    weapon_url_name: row.get("weapon_url_name")?,
                    re_rolls: row.get("re_rolls")?,
                    mod_rank: row.get("mod_rank")?,
                    oid: row.get("item_id")?,
                })
            })?
            .try_fold(
                vec![],
                |mut acc, item| -> Result<Vec<Item>, rusqlite::Error> {
                    let mut item = item?;
                    let attributes = self.select_attributes(item.oid.clone())?;
                    item.attributes = attributes;
                    acc.push(item);
                    Ok(acc)
                },
            )?;
        Ok(items)
    }

    fn select_attributes(&self, oid: Arc<str>) -> Result<Vec<Attribute>, rusqlite::Error> {
        let mut attributes_select = self.connection.prepare(SQL_SELECT_ATTRIBUTES)?;
        let attributes = attributes_select
            .query_map(&[&oid], |row| {
                let units: String = row.get("units")?;
                let units = Units::try_from(units).expect("Units must be parsed correctly");
                Ok(Attribute {
                    value: row.get("value")?,
                    positive: row.get("positive")?,
                    url_name: row.get("url_name")?,
                    units,
                    short_string: row.get("short_string")?,
                })
            })?
            .try_fold(
                vec![],
                |mut acc, attr| -> Result<Vec<Attribute>, rusqlite::Error> {
                    acc.push(attr?);
                    Ok(acc)
                },
            )?;
        // println!("{attributes:#?}");
        Ok(attributes)
    }

    pub(super) fn select_auction(&self, oid: Arc<str>) -> Result<Auction, rusqlite::Error> {
        let mut auctions_select = self.connection.prepare(SQL_SELECT_AUCTIONS)?;
        let auc = auctions_select.query_row(&[&oid], |row| {
            Ok(Auction {
                starting_price: row.get("starting_price")?,
                buyout_price: row.get("buyout_price")?,
                owner: row.get("owner")?,
                updated: row.get("updated")?,
                is_direct_sell: row.get("is_direct_sell")?,
                id: row.get("id")?,
                oid: row.get("item_id")?,
            })
        })?;
        Ok(auc)
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs::OpenOptions,
        io::Write,
        ops::{Add, Sub},
        sync::Arc,
    };

    use dotenv::dotenv;
    use rand::random;
    use time::Duration as LibDuration;
    use tokio::sync::Mutex;

    use crate::{
        http_client::{auth_state::AuthState, qf_client::QFClient},
        rivens::inventory::{
            convert_raw_inventory::convert_inventory_data, raw_inventory::decrypt_last_data,
            riven_lookop::RivenDataLookup,
        },
    };

    use super::Auction;

    async fn _test_insert_data() {
        dotenv().unwrap();
        let auth = AuthState::setup().expect("hehe");
        let auth = Arc::new(Mutex::new(auth));
        let qf = QFClient::new(auth);
        let qf = Arc::new(Mutex::new(qf));
        let lookup = RivenDataLookup::setup(qf).await.unwrap();
        let raw_upgrades = decrypt_last_data(None).unwrap();
        let items = convert_inventory_data(&lookup, raw_upgrades);
        let mut auctions = Vec::with_capacity(items.len());
        auctions.fill(Auction::default());
    }

    unsafe fn _any_as_u8_slice<T: Sized>(p: &T) -> &[u8] {
        ::core::slice::from_raw_parts((p as *const T) as *const u8, ::core::mem::size_of::<T>())
    }

    fn _write_dump() {
        let mut file = OpenOptions::new()
            .write(true)
            .append(true)
            .create(true)
            .open("data.dump")
            .unwrap();

        let mut total_time = LibDuration::new(0, 0);
        (0..94).for_each(|_| {
            let start = time::OffsetDateTime::now_utc();
            let mut buf: [u8; 256] = [0; 256];
            buf.fill_with(|| random::<u8>());
            file.write(&buf).unwrap();
            let fin = time::OffsetDateTime::now_utc().sub(start);
            total_time = total_time.add(fin);
            println!("file write: {}s", fin.as_seconds_f32());
        });
        println!(
            "Total file write took {} seconds",
            total_time.as_seconds_f32()
        );
    }

    async fn _test_write_dump() {
        // dotenv().unwrap();
        // let qf = QFClient::new();
        // let mut limiter = RateLimiter::new(1.0, Duration::from_secs(1));
        // let (body, _) = qf.send_request(Method::GET, qf.endpoint.as_str(), &mut limiter, None, None).await.unwrap().res;
        // let lookup = Arc::new(Mutex::new(from_value::<RivenDataLookup>(body.unwrap()).unwrap()));
        // let raw_upgrades = decrypt_last_data().unwrap();
        // let items = convert_inventory_data(lookup, raw_upgrades).await;
        // let auctions = items.into_iter().fold(vec![], |mut acc, item| {
        //     acc.push(Auction {
        //         item,
        //         ..Default::default()
        //     });
        //     acc
        // });
        _write_dump();
    }
}
