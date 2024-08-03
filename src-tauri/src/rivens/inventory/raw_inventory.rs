use std::{fmt::Display, rc::Rc};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::OffsetDateTime;

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Upgrades {
    upgrade_fingerprint: UpgradeFingerprint,
    item_type: Rc<str>,
    item_id: ItemID,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct ItemID {
    oid: Rc<str>
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct UpgradeFingerprint {
    compat: Rc<str>,
    lim: i32,
    lvl_req: i32,
    lvl: i32,
    rerolls: i32,
    pol: Rc<str>,
    buffs: Vec<Buffs>,
    curses: Vec<Curses>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Buffs {
    tag: Rc<str>,
    value: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Curses {
    tag: Rc<str>,
    value: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Auction {
    starting_price: u32,
    pub item: Item,
    buyout_price: u32,
    owner: String,
    #[serde(with = "time::serde::rfc3339")]
    pub updated: OffsetDateTime,
    is_direct_sell: bool,
    id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Item {
    mastery_level: u8,
    pub name: String,
    polarity: String,
    attributes: Vec<Attribute>,
    pub weapon_url_name: String,
    re_rolls: u16,
    mod_rank: u8,
}

impl Default for Item {
    fn default() -> Self {
        Self {
            mastery_level: 0,
            name: "".into(),
            polarity: "".into(),
            attributes: Vec::new(),
            weapon_url_name: "".into(),
            re_rolls: 0,
            mod_rank: 0,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Attribute {
    value: f64,
    positive: bool,
    url_name: String,
}

struct RawAttributes {
    positive: bool,
    tag: Rc<str>,
    value: i32,
}

struct TempFixes {
    prefix: Rc<str>,
    suffix: Rc<str>,
}

fn convert_inventory_data(upgrades: Vec<Upgrades>) -> Vec<Item> {
    let items: Vec<Item>;
    upgrades.iter().for_each(|upgrade| {
        let mut raw_attributes: Vec<RawAttributes> = vec![];
        upgrade.upgrade_fingerprint.buffs.iter().for_each(|buff| raw_attributes.push(RawAttributes {
            positive: true,
            tag: buff.tag.clone(),
            value: buff.value,
        }));
        upgrade.upgrade_fingerprint.curses.iter().for_each(|curse| raw_attributes.push(RawAttributes {
            positive: false,
            tag: curse.tag.clone(),
            value: curse.value,
        }));

        let item = Item::default();
        raw_attributes.iter().for_each(|attr| {
        })
    });
    todo!()
}

enum WeaponLookupError {
    InvalidWeapon(Rc<str>),
    InvalidField(WeaponLookupField)
}

enum WeaponLookupField {
    Weapons,
    UniqueName,
    WeaponUrlName,
    Disposition,
}


fn lookup_weapon_data(compat: &str, attr: &RawAttributes) -> Result<(Rc<str>, f64), WeaponLookupError> {
    // need to figure out how to get the lookup data. most likely gonna use quantframe's api
    // but that might change. therefore everything here is subject to change because i really dont
    // know what the qf data looks like.
    let data: Value;
    let weapons = match data["weapons"].as_array() {
        Some(v) => v,
        None => return Err(WeaponLookupError::InvalidField(WeaponLookupField::Weapons))
    };
    if (weapons.iter().find(|&weap| weap["unique_name"].as_str() == None)).is_some() {
        return Err(WeaponLookupError::InvalidField(WeaponLookupField::UniqueName))
    }
    let weapon = match weapons.iter().find(|&weap| weap["unique_name"].as_str().unwrap() == compat) {
        Some(v) => v,
        None => return Err(WeaponLookupError::InvalidWeapon(compat.into())),
    };
    let url_name: Rc<str> = match weapon["weapon_url_name"].as_str() {
        Some(v) => v.into(),
        None => return Err(WeaponLookupError::InvalidField(WeaponLookupField::WeaponUrlName)),
    };
    let dispositon = match weapon["weapon_url_name"].as_f64() {
        Some(v) => v,
        None => return Err(WeaponLookupError::InvalidField(WeaponLookupField::WeaponUrlName)),
    };
    todo!();
    Ok((url_name, dispositon))
}

enum RivenLookupError {
    InvalidItemType(Rc<str>),
    InvalidAttribute(Rc<str>),
    InvalidField(RivenLookupField),
    NullField(RivenLookupField),
}

enum RivenLookupField {
    RivenAttributes,
    UniqueName,
    ModifierTag,
    Upgrades,
    WfmUrl,
    PrefixTag,
    SuffixTag,
    BaseValue,
}

enum Units {
    Percent,
    Multiply,
    Seconds,
    Null,
}

fn lookup_riven_data(item_type: &str, raw_attributes: Vec<RawAttributes>) -> Result<Vec<(Rc<str>, Rc<str>, Rc<str>, f64)>, RivenLookupError> {
    // this function is subject to change for the same reasons as lookup_weapon_data
    let data: Value;
    let riven_attributes = match data["rivens_attributes"].as_array() {
        Some(v) => v,
        None => return Err(RivenLookupError::InvalidField(RivenLookupField::RivenAttributes))
    };
    if (riven_attributes.iter().find(|&weap| weap["unique_name"].as_str() == None)).is_some() {
        return Err(RivenLookupError::InvalidField(RivenLookupField::UniqueName))
    }
    let upgrades = match riven_attributes.iter().find(|&weap| weap["unique_name"].as_str().unwrap() == item_type) {
        Some(v) => {
            if v["upgrades"].as_array().is_some() {
                return Err(RivenLookupError::InvalidField(RivenLookupField::Upgrades))
            }
            v["upgrades"].as_array().unwrap()
        }
        None => return Err(RivenLookupError::InvalidItemType(item_type.into()))
    };
    // TODO: grab base value, prefix, suffix, and url_name from rivens_attributes

    // TODO: grab units from availabe_attributes using url_name from rivens_attributes
    todo!();
}
