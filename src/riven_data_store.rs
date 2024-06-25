use std::{env, fs::{self, File}, io::Write, path::PathBuf};

use tokio::sync::oneshot::Sender;

use crate::{main_loop::ResponseCommand, rivens::wfm_auctions::Auction, wfm_client::client::{GenericError, WFMClient}};

pub async fn get_rivens(wfmc: WFMClient, resp: Sender<ResponseCommand>) -> Result<(), GenericError> {
    let auctions = match wfmc.get_all_rivens().await {
        Ok(v) => {
            let _ = resp.send(ResponseCommand("Got Rivens!"));
            v
        }
        Err(err) => return Err(err.prop("MainLoop_GetAllRivens: ".to_string()))
    };
    for auction in auctions {
        write_rivens_store(auction).map_err(|err| err.prop("get_all_rivens: ".to_string()))?;
    }
    Ok(())
}

fn write_rivens_store(auction: Auction) -> Result<(), GenericError> {
    let path: PathBuf = env::var("PWD").map_err(|e| GenericError::new(e, "write_rivens_store: env::var: ".to_string()))?.into();

    let path = path.join("rivens");
    if path.exists() {
        fs::remove_dir_all(path.clone()).map_err(|e| GenericError::new(e, "write_rivens_store: remove_dir_all: ".to_string()))?;
    }
    fs::create_dir(path.clone()).map_err(|err| GenericError::new(err, "write_rivens_store: create_dir: ".to_string()))?;

    let path = path.join(auction.clone().item.weapon_url_name);
    if !path.exists() {
        fs::create_dir(path.clone()).map_err(|err| GenericError::new(err, "write_rivens_store: create_dir: ".to_string()))?;
    }

    let path = path.join(format!("{}.json", auction.item.name));
    let mut file = File::create(path).map_err(|e| GenericError::new(e, "write_rivens_store: create: ".to_string()))?;
    let json = serde_json::to_string_pretty::<Auction>(&auction).map_err(|e| GenericError::new(e, "write_rivens_store: to_string_pretty: ".to_string()))?;
    file.write_all(json.as_bytes()).map_err(|e| GenericError::new(e, "write_rivens_store: write_all: ".to_string()))?;
    Ok(())
}
