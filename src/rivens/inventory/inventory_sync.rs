use std::{ops::DerefMut, sync::Arc};

use futures::lock::Mutex;

use super::{
    convert_raw_inventory::{convert_inventory_data, Item, Upgrades},
    database::{Auction, InventoryDB},
    raw_inventory::{decrypt_last_data, InventoryDecryptError},
    riven_lookop::RivenDataLookup,
};

#[derive(Debug)]
enum DataBaseSyncError {
    DecryptError(InventoryDecryptError),
    DatabaseError(rusqlite::Error),
}

async fn sync_db(
    lookup: Arc<Mutex<RivenDataLookup>>,
    db: Arc<Mutex<InventoryDB>>,
) -> Result<Option<Vec<Item>>, DataBaseSyncError> {
    let inventory_items = decrypt_last_data().map_err(|e| DataBaseSyncError::DecryptError(e))?;

    let mut db = db.lock().await;
    let db = db.deref_mut();
    let mut db_items: Vec<Item> = db.select_items().unwrap();

    db_items.iter().next();

    let num_old_items = db_items
        .iter()
        .filter(|item| {
            inventory_items
                .iter()
                .find(|&inv| inv.item_id.oid != item.oid)
                .is_some()
        })
        .count();
    let old_items: Option<Vec<&Item>> = if num_old_items != 0 {
        let acc: Vec<&Item> = db_items.iter().fold(vec![], |mut acc, item| {
            if inventory_items.iter().find(|&inv| inv.item_id.oid != item.oid).is_some() {
                acc.push(item);
            }
            acc
        });
        Some(acc)
    } else {
        None
    };

    let db_auctions = db_items.iter().try_fold(
        vec![],
        |mut acc, item| -> Result<Vec<Auction>, DataBaseSyncError> {
            acc.push(
                db.select_auction(item.oid.clone())
                    .map_err(|e| DataBaseSyncError::DatabaseError(e))?,
            );
            Ok(acc)
        },
    );

    let new_raw_upgrades: Vec<Upgrades> = inventory_items
        .into_iter()
        .filter(|upgr| {
            db_items
                .iter()
                .find(|&item| item.oid != upgr.item_id.oid)
                .is_some()
        })
        .collect();

    if !new_raw_upgrades.is_empty() {
        let mut new_rivens = convert_inventory_data(lookup.clone(), new_raw_upgrades).await;
        // append new rivens to old rivens from db to be used by wfm_sync()

        db.insert_items(&new_rivens)
            .map_err(|e| DataBaseSyncError::DatabaseError(e))?;

        db_items.append(&mut new_rivens);

        // Display new rivens to the client

        Ok(Some(new_rivens))
    } else {
        Ok(None)
    }
}
