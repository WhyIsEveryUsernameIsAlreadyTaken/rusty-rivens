use std::sync::Arc;

use futures::lock::Mutex;

use crate::rivens::inventory::convert_raw_inventory::convert_inventory_data;

use super::{convert_raw_inventory::{Auction, Item, Upgrades}, raw_inventory::{decrypt_last_data, InventoryDecryptError}, riven_lookop::RivenDataLookup};

#[derive(Debug)]
enum DataBaseSyncError {
    DecryptError(InventoryDecryptError)
}

async fn sync_db(lookup: Arc<Mutex<RivenDataLookup>>) -> Result<Option<Vec<Auction>>, DataBaseSyncError> {
    // TODO: query rivens from database
    let db_auctions: Vec<Auction> = ;
    let db_items: Vec<Item> = vec![];

    let raw_upgrades = decrypt_last_data().map_err(|e| DataBaseSyncError::DecryptError(e))?;
    let raw_upgrades: Vec<Upgrades> = raw_upgrades.into_iter().filter(|upgr| {
        db_rivens.iter().find(|&au| au.oid != upgr.item_id.oid.to_string()).is_some()
    }).collect();

    if !raw_upgrades.is_empty() {
        let new_rivens = convert_inventory_data(lookup.clone(), raw_upgrades).await;
        // append new rivens to old rivens from db to be used by wfm_sync()
        let new_rivens = vec![Auction::default(); new_rivens.len()];

        // TODO: save new rivens to db

        // Display new rivens to the client

        Ok(Some(new_rivens))
    } else {
        Ok(None)
    }
}
