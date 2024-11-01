use std::{ops::DerefMut, sync::Arc};

use tokio::sync::Mutex;

use super::{
    convert_raw_inventory::{convert_inventory_data, Item, Upgrades},
    database::InventoryDB,
    raw_inventory::{decrypt_last_data, InventoryDecryptError},
    riven_lookop::RivenDataLookup,
};

#[derive(Debug)]
pub enum DataBaseSyncError {
    DecryptError(InventoryDecryptError),
    DatabaseError(rusqlite::Error),
}

pub async fn sync_db(
    db: Arc<Mutex<InventoryDB>>,
    lookup: &RivenDataLookup,
    inventory_items_test: Option<Vec<Upgrades>>,
) -> Result<(Vec<Item>, Vec<Arc<str>>), DataBaseSyncError> {

    let mut db = db.lock().await;
    let db = db.deref_mut();
    let db_items: Vec<Item> = db.select_items().unwrap();
    let inventory_items = if let Some(invitest) = inventory_items_test {
        invitest
    } else {
        decrypt_last_data(None).map_err(|e| DataBaseSyncError::DecryptError(e))?
    };

    let mut same_items = get_same_items(&db_items, &inventory_items);

    let old_items: Vec<Item> = get_old_items(&db_items, &inventory_items);

    // DELETE OLD ITEMS + ANY AUCTIONS FOR OLD ITEMS IN DB

    let delete_ids: Vec<Arc<str>> = old_items.into_iter().map(|item| item.oid).collect();

    db.delete_items_auctions(delete_ids.clone()).map_err(|e| DataBaseSyncError::DatabaseError(e))?;

    let new_items = get_new_items(&db_items, inventory_items);

    // ADD NEW ITEMS TO DB

    let mut new_items = convert_inventory_data(lookup, new_items);
    db.insert_items(&new_items).unwrap();
    // PUSH CHANGES UP TO UI
    same_items.append(&mut new_items);
    Ok((same_items, delete_ids))
}

fn get_same_items(db_items: &Vec<Item>, inventory_items: &Vec<Upgrades>) -> Vec<Item> {
    db_items.iter()
        .filter_map(|item| {
            if inventory_items.iter()
                .find(|&upgrade| item.oid == upgrade.item_id.oid ).is_some() {
                Some(item.clone())
            } else {
                None
            }
        }).collect()
}

fn get_old_items(db_items: &Vec<Item>, inventory_items: &Vec<Upgrades>) -> Vec<Item> {
    db_items.iter()
        .filter_map(|item| {
            if inventory_items.iter()
                .find(|&upgrade| upgrade.item_id.oid == item.oid).is_none() {
                Some(item.clone())
            } else {
                None
            }
        }).collect()
}

fn get_new_items(db_items: &Vec<Item>, inventory_items: Vec<Upgrades>) -> Vec<Upgrades> {
    inventory_items.into_iter()
        .filter(|upgrade|
            db_items.iter()
                .find(|&item| item.oid == upgrade.item_id.oid).is_none()
        ).collect()
}

#[cfg(test)]
mod tests {

    use std::sync::Arc;

    use dotenv::dotenv;
    use tokio::sync::Mutex;

    use crate::{http_client::{auth_state::AuthState, qf_client::QFClient}, rivens::inventory::{convert_raw_inventory::{convert_inventory_data, Item}, database::{self, InventoryDB}, inventory_sync::{get_new_items, get_old_items, get_same_items}, raw_inventory::decrypt_last_data, riven_lookop::RivenDataLookup}};

    fn update_db(db: &mut InventoryDB, new: Option<Vec<Item>>, old: Option<Vec<Item>>) -> Vec<Item> {
        if let Some(new) = new {
            db.insert_items(&new).unwrap();
        };
        if let Some(old) = old {
            let old = old.into_iter().map(|item| item.oid).collect();
            db.delete_items_auctions(old).unwrap();
        };
        db.select_items().unwrap()
    }

    #[tokio::test]
    async fn test_sync_db() {
        dotenv().unwrap();
        let contrl_items = decrypt_last_data(Some("lastDataControl.dat")).unwrap();
        let added_items = decrypt_last_data(Some("lastDataAdded.dat")).unwrap();
        let subtracted_items = decrypt_last_data(Some("lastDataSubtracted.dat")).unwrap();
        let auth = AuthState::setup().expect("hehe");
        let auth = Arc::new(Mutex::new(auth));
        let qf = QFClient::new(auth);
        let qf = Arc::new(Mutex::new(qf));
        let lookup = RivenDataLookup::setup(qf).await.unwrap();
        let mut db = database::InventoryDB::open("test_db.sqlite3").unwrap();

        let mut db_items = update_db(&mut db, None, None);
        let same = {
            let inventory_items = contrl_items;
            let same = get_same_items(&db_items, &inventory_items).len();
            let new = get_new_items(&db_items, inventory_items);
            let new = convert_inventory_data(&lookup, new);
            db_items = update_db(&mut db, Some(new), None);
            same
        };
        assert_eq!(same, 0, "same itms: {same}");

        let (same, old, new) = {
            let inventory_items = added_items;
            let same = get_same_items(&db_items, &inventory_items).len();
            let old = get_old_items(&db_items, &inventory_items);
            let new = get_new_items(&db_items, inventory_items);
            let new = convert_inventory_data(&lookup, new);
            db_items = update_db(&mut db, Some(new.clone()), Some(old.clone()));
            (same, old.len(), new.len())
        };
        assert_ne!(same, 0, "same itms: {same}");
        assert_eq!(old, 0, "{old}");
        assert_eq!(new, 1, "{new} added");

        let (same, old, new) = {
            let inventory_items = subtracted_items;
            let same = get_same_items(&db_items, &inventory_items).len();
            let old = get_old_items(&db_items, &inventory_items);
            let new = get_new_items(&db_items, inventory_items);
            let new = convert_inventory_data(&lookup, new);
            update_db(&mut db, Some(new.clone()), Some(old.clone()));
            (same, old.len(), new.len())
        };
        drop(db);

        println!("same: {same}");
        assert_eq!(old, 2, "{old}");
        assert_eq!(new, 0, "{new}");

        std::fs::remove_file("test_db.sqlite3").unwrap();
    }
}
