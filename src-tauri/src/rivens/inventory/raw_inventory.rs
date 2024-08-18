use std::{env::{self, VarError}, error::Error, fmt::Display, fs::File, io::{self, Read}, num::ParseIntError, ops::Deref, path::PathBuf, rc::Rc, string::FromUtf8Error};

use aes::cipher::{block_padding::{NoPadding, UnpadError}, BlockDecryptMut, KeyIvInit};
use serde_json::{from_value, Value};

use crate::rivens::inventory::convert_raw_inventory::Upgrades;

type DecryptThingy = cbc::Decryptor<aes::Aes128>;

#[derive(Debug)]
pub enum InventoryDecryptError {
    DecryptorError(UnpadError),
    ParseError(ParseErrorType),
    IoError(io::Error),
    EnvVarError(VarError),
    DeserializeError(serde_json::Error),
    OtherError(Rc<str>),
}

impl Error for InventoryDecryptError {}
impl Display for InventoryDecryptError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let err = match self {
            InventoryDecryptError::DecryptorError(e) => format!("DecryptorError: {}", e),
            InventoryDecryptError::ParseError(e) => format!("ParseError: {}", e),
            InventoryDecryptError::IoError(e) => format!("IoErrort: {}", e),
            InventoryDecryptError::EnvVarError(e) => format!("EnvVarErrort: {}", e),
            InventoryDecryptError::DeserializeError(e) => format!("DeserializeErrort: {}", e),
            InventoryDecryptError::OtherError(e) => String::from(e.deref()),
        };
        f.write_str(err.as_str())
    }
}

#[derive(Debug)]
pub enum ParseErrorType {
    ParseInt(ParseIntError),
    ParseUtf8(FromUtf8Error),
}

impl Display for ParseErrorType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let etype = match self {
            ParseErrorType::ParseInt(e) => format!("ParseInt: {}", e),
            ParseErrorType::ParseUtf8(e) => format!("ParseUtf8: {}", e),
        };
        f.write_str(etype.as_str())
    }
}

pub fn decrypt_last_data() -> Result<Vec<Upgrades>, InventoryDecryptError> {
    let path:PathBuf = PathBuf::from("../lastData.dat");
    let mut file = File::open(path).map_err(|e| InventoryDecryptError::IoError(e))?;
    let mut ciphertext: Vec<u8> = vec![];
    file.read_to_end(&mut ciphertext).map_err(|e| InventoryDecryptError::IoError(e))?;

    let key_var = env::var("KEY").map_err(|e| InventoryDecryptError::EnvVarError(e))?;
    let mut key: Vec<u8> = Vec::with_capacity(16);
    key_var.split(",").try_for_each(|num| -> Result<(), ParseIntError> {
        let num: u8 = num.parse()?;
        key.push(num);
        Ok(())
    }).map_err(|e| InventoryDecryptError::ParseError(ParseErrorType::ParseInt(e)))?;
    let iv_var = env::var("IV").map_err(|e| InventoryDecryptError::EnvVarError(e))?;
    let mut iv: Vec<u8> = Vec::with_capacity(16);
    iv_var.split(",").try_for_each(|num| -> Result<(), ParseIntError> {
        let num: u8 = num.parse()?;
        iv.push(num);
        Ok(())
    }).map_err(|e| InventoryDecryptError::ParseError(ParseErrorType::ParseInt(e)))?;

    let res = DecryptThingy::new(key[..].into(), iv[..].into())
        .decrypt_padded_vec_mut::<NoPadding>(&ciphertext)
        .map_err(|e| InventoryDecryptError::DecryptorError(e))?;

    let res = String::from_utf8(res).map_err(|e| InventoryDecryptError::ParseError(ParseErrorType::ParseUtf8(e)))?;
    let res = res.replace("\"{", "{");
    let res = res.replace("}\"", "}");
    let res: String = res.split(r"\").collect();
    let res = res.trim_end();
    let res = serde_json::from_str::<Value>(res).map_err(|e| InventoryDecryptError::DeserializeError(e))?;
    let upgrades_raw = match res["Upgrades"].as_array() {
        Some(v) => v,
        None => return Err(InventoryDecryptError::OtherError("No array associated with the field: \"Upgrades\"".into())),
    };
    let mut upgrades: Vec<Upgrades> = vec![];
    upgrades_raw.iter()
        .filter(|&upgrade| !upgrade["UpgradeFingerprint"]["compat"].is_null())
        .try_for_each(|upgrade| -> Result<(), serde_json::Error> {
            upgrades.push(from_value(upgrade.clone())?);
            Ok(())
        }).map_err(|e| InventoryDecryptError::DeserializeError(e))?;
    Ok(upgrades)
}

#[cfg(test)]
mod tests {
    use dotenv::dotenv;

    use super::decrypt_last_data;

    #[test]
    fn test_deserialize() {
        dotenv().unwrap();
        let upgrades = decrypt_last_data().unwrap();
    }
}
