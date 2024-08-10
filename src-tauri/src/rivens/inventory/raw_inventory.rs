// TODO: Need to replace panics with proper error handling and make test(s) for this.
// Won't be able to do the ladder until I get access to my desktop pc with the files
// needed to do the tests.
use std::{cmp::Ordering, error::Error, fmt::Display, ops::Deref, rc::Rc, sync::Arc};

use futures::lock::Mutex;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use super::riven_lookop::RivenDataLookup;

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
    lvl_req: u8,
    lvl: u8,
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
    re_rolls: i32,
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

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, PartialOrd)]
struct AttributeInfo {
    positive: bool,
    value: i32,
    wfm_url: Arc<str>,
    prefix: Arc<str>,
    suffix: Arc<str>,
    base_value: f64,
}

impl Eq for AttributeInfo {}

impl Ord for AttributeInfo {
    fn cmp(&self, other: &Self) -> Ordering {
        if self.value > other.value {
            Ordering::Greater
        } else if self.value < other.value {
            Ordering::Less
        } else {
            Ordering::Equal
        }
    }
}

async fn convert_inventory_data(lookup: Arc<Mutex<RivenDataLookup>>, upgrades: Vec<Upgrades>) -> Vec<Item> {
    let lookup = lookup.lock().await;
    let lookup = lookup.deref();
    let mut items: Vec<Item> = vec![];
    upgrades.iter().for_each(|upgrade| {
        let mut raw_attributes: Vec<RawAttributes> = vec![];
        let buff_count = upgrade.upgrade_fingerprint.buffs.len();
        let (good_multiplier, bad_multiplier) = if !upgrade.upgrade_fingerprint.curses.is_empty() {
            if buff_count == 2 {
                (1.2375, -0.495)
            } else if buff_count == 3 {
                (0.9375, -0.75)
            } else {
                panic!("no buffs with the associated riven!")
            }
        } else {
            if buff_count == 2 {
                (0.99, 0.0)
            } else if buff_count == 3 {
                (0.75, 0.0)
            } else {
                panic!("no buffs with the associated riven!")
            }
        };
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

        let (_weapon_url_name, weapon_type, disposition) = match lookup_weapon_data(lookup, upgrade.upgrade_fingerprint.compat.deref()) {
            Ok(v) => v,
            Err(e) => {
                println!("{}", e);
                return;
            }
        };
        let mut attribute_info = match lookup_riven_data(lookup, weapon_type, raw_attributes) {
            Ok(v) => v,
            Err(e) => {
                println!("{}", e);
                return;
            }
        };
        attribute_info.sort();
        attribute_info.reverse();
        let name = parse_riven_name(&attribute_info, buff_count);
        let _attributes = calculate_attributes(attribute_info, good_multiplier, bad_multiplier, disposition, upgrade.upgrade_fingerprint.lvl);
        let polarity = if upgrade.upgrade_fingerprint.pol == "AP_ATTACK".into() {
            String::from("madurai")
        } else if upgrade.upgrade_fingerprint.pol == "AP_DEFENSE".into() {
            String::from("vazarin")
        } else if upgrade.upgrade_fingerprint.pol == "AP_TACTIC".into() {
            String::from("naramon")
        } else {
            panic!("Invalid polarity was given");
        };
        items.push(Item {
            mastery_level: upgrade.upgrade_fingerprint.lvl_req,
            name,
            polarity,
            attributes: _attributes,
            weapon_url_name: _weapon_url_name.to_string(),
            re_rolls: upgrade.upgrade_fingerprint.rerolls,
            mod_rank: upgrade.upgrade_fingerprint.lvl,
        })
    });
    todo!()
}

fn parse_riven_name(attributes_info: &Vec<AttributeInfo>, num_buffs: usize, ) -> String {
    if num_buffs == 2 {
        format!("{}{}", attributes_info[0].prefix, attributes_info[1].suffix)
    } else if num_buffs == 3 {
        format!("{}-{}{}", attributes_info[0].prefix, attributes_info[1].prefix, attributes_info[2].suffix)
    } else {
        panic!("no buffs with the associated riven!")
    }
}

fn calculate_attributes(attribute_info: Vec<AttributeInfo>, good_multiplier: f64, bad_multiplier: f64, disposition: f64, lvl: u8) -> Vec<Attribute> {
    let mut attributes = Vec::with_capacity(2);
        attribute_info.iter().for_each(|attr| {
            let x = f64::min(f64::max(0.9 + attr.value as f64 / 53687091.0 / 100.0, 0.9), 1.1);
            let good_bad_multiplier = if attr.positive {
                good_multiplier
            } else {
                bad_multiplier
            };
            let y = 90.0 * attr.base_value * disposition * good_bad_multiplier;
            let value = x*y*100.0*(lvl + 1) as f64 / 9.0;
            attributes.push(Attribute {
                value,
                positive: attr.positive,
                url_name: attr.wfm_url.clone().to_string(),
            });
        });
    attributes
}

#[derive(Debug)]
enum WeaponLookupError {
    InvalidWeapon(Rc<str>),
    InvalidField(WeaponLookupField)
}

impl Display for WeaponLookupError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let err = format!("WeaponLookupError: {}", match self {
            WeaponLookupError::InvalidWeapon(compat) => format!("Could not find weapon matching the conmpat: {}", compat),
            WeaponLookupError::InvalidField(field) => format!("InvalidField: {}", match field {
                WeaponLookupField::Weapons => "Weapons",
                WeaponLookupField::UniqueName => "UniqueName",
                WeaponLookupField::WeaponUrlName => "WeaponUrlName",
                WeaponLookupField::WeaponType => "WeaponType",
                WeaponLookupField::Disposition => "Disposition",
            }),
        });

        f.write_str(err.as_str())
    }
}

impl Error for WeaponLookupError {}

#[derive(Debug)]
enum WeaponLookupField {
    Weapons,
    UniqueName,
    WeaponUrlName,
    Disposition,
    WeaponType
}

fn lookup_weapon_data(
    lookup: &RivenDataLookup,
    compat: &str
) -> Result<(Rc<str>, Rc<str>, f64), WeaponLookupError> {
    if lookup.weapons.is_none() {
        return Err(WeaponLookupError::InvalidField(WeaponLookupField::Weapons))
    }
    let weapons = lookup.weapons.as_ref().unwrap();
    if weapons.iter().find(|&weap| weap.unique_name.is_none()).is_some() {
        return Err(WeaponLookupError::InvalidField(WeaponLookupField::UniqueName))
    }
    if let Some(weapon) = weapons.iter().find(|&weap| weap.clone().unique_name.unwrap() == compat.into()) {
        let url_name = match weapon.wfm_url_name.as_ref() {
            Some(v) => v.deref(),
            None => return Err(WeaponLookupError::InvalidField(WeaponLookupField::WeaponUrlName)),
        };
        let disposition = match weapon.disposition {
            Some(v) => v,
            None => return Err(WeaponLookupError::InvalidField(WeaponLookupField::Disposition)),
        };
        let weapon_type = match weapon.weapon_type.clone() {
            Some(v) => v,
            None => return Err(WeaponLookupError::InvalidField(WeaponLookupField::WeaponType)),
        };
        return Ok((url_name.into(), weapon_type.deref().into(), disposition))
    } else {
        return Err(WeaponLookupError::InvalidWeapon(compat.into()))
    }
}

#[derive(Debug)]
enum RivenLookupError {
    InvalidItemType(Rc<str>),
    InvalidAttribute(Rc<str>),
    InvalidField(RivenLookupField),
}

impl Display for RivenLookupError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let err = format!("WeaponLookupError: {}", match self {
            RivenLookupError::InvalidItemType(itype) => format!("Could not find weapon type: {}", itype),
            RivenLookupError::InvalidAttribute(iattr) => format!("Could not find attribute type: {}", iattr),
            RivenLookupError::InvalidField(field) => format!("InvalidField: {}", match field {
                RivenLookupField::RivenAttributes => "RivenAttributes",
                RivenLookupField::UniqueName => "UniqueName",
                RivenLookupField::ModifierTag => "ModifierTag",
                RivenLookupField::Upgrades => "Upgrades",
                RivenLookupField::WfmUrl => "WfmUrl",
                RivenLookupField::PrefixTag => "PrefixTag",
                RivenLookupField::SuffixTag => "SuffixTag",
                RivenLookupField::BaseValue => "BaseValue",
            }),
        });

        f.write_str(err.as_str())
    }
}

impl Error for RivenLookupError {}

#[derive(Debug)]
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

fn lookup_riven_data(
    lookup: &RivenDataLookup,
    weapon_type: Rc<str>,
    rattrs: Vec<RawAttributes>
) -> Result<Vec<AttributeInfo>, RivenLookupError> {
    if lookup.rivens_attributes.is_none() {
        return Err(RivenLookupError::InvalidField(RivenLookupField::RivenAttributes))
    }
    let riven_attributes = lookup.rivens_attributes.as_ref().unwrap();
    if riven_attributes.iter().find(|&attr| attr.unique_name.is_none()).is_some() {
        return Err(RivenLookupError::InvalidField(RivenLookupField::UniqueName))
    }
    let attrs = match riven_attributes.iter().find(|&attr| attr.unique_name.clone().unwrap() == weapon_type.deref().into()) {
        Some(v) => v,
        None => return Err(RivenLookupError::InvalidItemType(weapon_type.into())),
    };
    if attrs.upgrades.is_none() {
        return Err(RivenLookupError::InvalidField(RivenLookupField::RivenAttributes))
    }
    let upgrades = attrs.upgrades.as_ref().unwrap();
    if upgrades.iter().find(|&upgr| upgr.modifier_tag.is_none()).is_some() {
        return Err(RivenLookupError::InvalidField(RivenLookupField::ModifierTag))
    }
    let mut attr_info: Vec<AttributeInfo> = Vec::with_capacity(2);
    rattrs.iter().try_for_each(|rattr: &RawAttributes| -> Result<(), RivenLookupError> {
        let upgrade = match upgrades.iter().find(|&upgr| upgr.modifier_tag.clone().unwrap() == rattr.tag.deref().into()) {
            Some(v) => v,
            None => return Err(RivenLookupError::InvalidAttribute(rattr.tag.clone())),
        };
        let wfm_url = match upgrade.wfm_url.clone() {
            Some(v) => v,
            None => return Err(RivenLookupError::InvalidField(RivenLookupField::WfmUrl)),
        };
        let prefix = match upgrade.prefix.clone() {
            Some(v) => v,
            None => return Err(RivenLookupError::InvalidField(RivenLookupField::PrefixTag)),
        };
        let suffix = match upgrade.suffix.clone() {
            Some(v) => v,
            None => return Err(RivenLookupError::InvalidField(RivenLookupField::SuffixTag)),
        };
        let base_value = match upgrade.value.clone() {
            Some(v) => v,
            None => return Err(RivenLookupError::InvalidField(RivenLookupField::BaseValue)),
        };
        attr_info.push(AttributeInfo {
            positive: rattr.positive,
            value: rattr.value,
            wfm_url: wfm_url.clone(),
            prefix: prefix.clone(),
            suffix: suffix.clone(),
            base_value,
        });
        Ok(())
    })?;
    Ok(attr_info)
}
