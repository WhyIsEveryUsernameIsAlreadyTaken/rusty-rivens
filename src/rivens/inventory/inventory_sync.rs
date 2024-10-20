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
) -> Result<(usize, usize, usize), DataBaseSyncError> {

    let mut db = db.lock().await;
    let db = db.deref_mut();
    let db_items: Vec<Item> = db.select_items().unwrap();
    let inventory_items = if let Some(invitest) = inventory_items_test {
        invitest
    } else {
        decrypt_last_data(None).map_err(|e| DataBaseSyncError::DecryptError(e))?
    };

    let same_items = db_items.iter()
        .filter(|&item|
            inventory_items.iter()
                .find(|&upgrade| item.oid == upgrade.item_id.oid ).is_some()
        ).count();

    let old_items: Vec<Item> = db_items.iter()
        .filter_map(|item| {
            if inventory_items.iter()
                .find(|&upgrade| upgrade.item_id.oid == item.oid).is_none() {
                Some(item.clone())
            } else {
                None
            }
        }).collect();

    // DELETE OLD ITEMS + ANY AUCTIONS FOR OLD ITEMS IN DB
    let old_len = old_items.len();
    db.delete_items_auctions(old_items).map_err(|e| DataBaseSyncError::DatabaseError(e))?;

    let new_items = inventory_items.into_iter()
        .filter(|upgrade|
            db_items.iter()
                .find(|&item| item.oid == upgrade.item_id.oid).is_none()
    ).collect();

    // ADD NEW ITEMS TO DB

    let new_items = convert_inventory_data(lookup, new_items).await;
    db.insert_items(&new_items).unwrap();
    // PUSH CHANGES UP TO UI

    Ok((new_items.len(), old_len, same_items))
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use dotenv::dotenv;
    use serde_json::{from_str, Value};
    use tokio::sync::Mutex;

    use crate::rivens::inventory::{database, raw_inventory::decrypt_last_data, riven_lookop::RivenDataLookup};

    use super::sync_db;

    fn test_sync_setup_lookup() -> RivenDataLookup {
        let data = include_str!("mock_riven_lookup.json");
        let val: Value = from_str(data).unwrap();
        serde_json::from_value(val["items"].clone()).unwrap()
    }

    #[tokio::test]
    async fn test_sync_db() {
        dotenv().unwrap();
        let contrl_items = Some(decrypt_last_data(Some("lastDataControl.dat")).unwrap());
        let added_items = Some(decrypt_last_data(Some("lastDataAdded.dat")).unwrap());
        let subtracted_items = Some(decrypt_last_data(Some("lastDataSubtracted.dat")).unwrap());
        let lookup = test_sync_setup_lookup();
        let db = database::InventoryDB::open("test_db.sqlite3").unwrap();
        let db = Arc::new(Mutex::new(db));

        let (init, _, same) = {
            let db = db.clone();
            let lookup = lookup.clone();
            sync_db(db, &lookup, contrl_items).await.unwrap()
        };
        assert_eq!(same, 0, "same itms: {same}");

        let (added, removed, same) = {
            let db = db.clone();
            let lookup = lookup.clone();
            sync_db(db, &lookup, added_items).await.unwrap()
        };
        assert!(same != 0, "same itms: {same}");
        assert_eq!(added, 1, "{added} added");
        assert_eq!(removed, 0, "{removed}");

        let (added, removed, kept) = {
            let db = db.clone();
            let lookup = lookup.clone();
            sync_db(db, &lookup, subtracted_items).await.unwrap()
        };

        println!("kept: {kept}");
        assert_eq!(added, 0, "{added}");
        assert_eq!(removed, 2, "{removed}");
        drop(db);

        std::fs::remove_file("test_db.sqlite3").unwrap();
    }
}
