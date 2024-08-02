use std::rc::Rc;

use serde::{Deserialize, Serialize};
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
    value: f32,
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

fn get_weapon_data_lookup() {
}
