use std::{env, fs::{self, File}, io::Write, path::PathBuf};

use crate::{rivens::wfm_auctions::Auction, AppError};

// pub async fn get_rivens(wfmc: WFMClient, resp: Sender<ResponseCommand>) -> Result<(), AppError> {
//     let auctions = match wfmc.get_all_rivens().await {
//         Ok(v) => {
//             let _ = resp.send(ResponseCommand("Got Rivens!"));
//             v
//         }
//         Err(err) => return Err(err.prop("MainLoop_GetAllRivens: ".into()))
//     };
//     for auction in auctions {
//         write_rivens_store(auction).map_err(|err| err.prop("get_all_rivens: ".into()))?;
//     }
//     Ok(())
// }

fn write_rivens_store(auction: Auction) -> Result<(), AppError> {
    let path: PathBuf = env::var("PWD").map_err(|e| AppError::new(e.to_string().into(), "write_rivens_store: env::var: ".into()))?.into();

    let path = path.join("rivens");
    if path.exists() {
        fs::remove_dir_all(path.clone()).map_err(|e| AppError::new(e.to_string().into(), "write_rivens_store: remove_dir_all: ".into()))?;
    }
    fs::create_dir(path.clone()).map_err(|e| AppError::new(e.to_string().into(), "write_rivens_store: create_dir: ".into()))?;

    let path = path.join(auction.clone().item.weapon_url_name);
    if !path.exists() {
        fs::create_dir(path.clone()).map_err(|e| AppError::new(e.to_string().into(), "write_rivens_store: create_dir: ".into()))?;
    }

    let path = path.join(format!("{}.json", auction.item.name));
    let mut file = File::create(path).map_err(|e| AppError::new(e.to_string().into(), "write_rivens_store: create: ".into()))?;
    let json = serde_json::to_string_pretty::<Auction>(&auction).map_err(|e| AppError::new(e.to_string().into(), "write_rivens_store: to_string_pretty: ".into()))?;
    file.write_all(json.as_bytes()).map_err(|e| AppError::new(e.to_string().into(), "write_rivens_store: write_all: ".into()))?;
    Ok(())
}
