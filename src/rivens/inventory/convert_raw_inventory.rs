use std::{cmp::Ordering, error::Error, fmt::Display, ops::Deref, rc::Rc, sync::Arc};

use super::riven_lookop::RivenDataLookup;
use rusqlite::{types::FromSql, ToSql};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Upgrades {
    #[serde(alias = "UpgradeFingerprint")]
    pub upgrade_fingerprint: UpgradeFingerprint,
    #[serde(alias = "ItemType")]
    pub item_type: Arc<str>,
    #[serde(alias = "ItemId")]
    pub item_id: ItemID,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ItemID {
    #[serde(alias = "$oid")]
    pub oid: Arc<str>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UpgradeFingerprint {
    pub compat: Option<Arc<str>>,
    pub lim: i32,
    #[serde(alias = "lvlReq")]
    pub lvl_req: u8,
    #[serde(default)]
    pub lvl: u8,
    #[serde(default)]
    pub rerolls: i32,
    pub pol: Arc<str>,
    pub buffs: Vec<Buffs>,
    pub curses: Vec<Curses>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Buffs {
    #[serde(alias = "Tag")]
    pub tag: Arc<str>,
    #[serde(alias = "Value")]
    pub value: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Curses {
    #[serde(alias = "Tag")]
    pub tag: Arc<str>,
    #[serde(alias = "Value")]
    pub value: i32,
}


#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Item {
    pub mastery_level: u8,
    pub name: Arc<str>,
    pub weapon_name: Arc<str>,
    pub polarity: Arc<str>,
    pub attributes: Vec<Attribute>,
    pub weapon_url_name: Arc<str>,
    pub re_rolls: i32,
    pub mod_rank: u8,
    #[serde(default)]
    pub oid: Arc<str>,
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
            oid: "".into(),
            mod_rank: 0,
            weapon_name: "".into(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Attribute {
    pub value: f64,
    pub positive: bool,
    pub url_name: String,
    pub units: Units,
    pub short_string: String,
}

#[derive(Debug)]
struct RawAttributes<'a> {
    positive: bool,
    tag: &'a str,
    value: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, PartialOrd)]
struct AttributeInfo {
    positive: bool,
    value: i32,
    units: Units,
    wfm_url: Arc<str>,
    short_string: Arc<str>,
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

pub fn convert_inventory_data(
    lookup: &RivenDataLookup,
    upgrades: Vec<Upgrades>,
) -> Vec<Item> {
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
        upgrade.upgrade_fingerprint.buffs.iter().for_each(|buff| {
            raw_attributes.push(RawAttributes {
                positive: true,
                tag: &buff.tag,
                value: buff.value,
            });
        });
        upgrade.upgrade_fingerprint.curses.iter().for_each(|curse| {
            raw_attributes.push(RawAttributes {
                positive: false,
                tag: &curse.tag,
                value: curse.value,
            });
        });

        let (_weapon_url_name, weapon_name, weapon_type, disposition) = match lookup_weapon_data(
            lookup,
            upgrade.upgrade_fingerprint.compat.as_ref().unwrap().deref(),
        ) {
            Ok(v) => v,
            Err(e) => {
                println!("{}", e);
                return;
            }
        };
        let mut attribute_info = match lookup_riven_data(lookup, weapon_type.as_str(), raw_attributes) {
            Ok(v) => v,
            Err(e) => {
                println!("{}", e);
                return;
            }
        };
        attribute_info.sort();
        attribute_info.reverse();
        let name = parse_riven_name(&attribute_info, buff_count);
        let _attributes = calculate_attributes(
            attribute_info,
            good_multiplier,
            bad_multiplier,
            disposition,
            upgrade.upgrade_fingerprint.lvl,
        );
        let polarity = if upgrade.upgrade_fingerprint.pol == "AP_ATTACK".into() {
            "madurai"
        } else if upgrade.upgrade_fingerprint.pol == "AP_DEFENSE".into() {
            "vazarin"
        } else if upgrade.upgrade_fingerprint.pol == "AP_TACTIC".into() {
            "naramon"
        } else {
            panic!("Invalid polarity was given");
        };
        items.push(Item {
            mastery_level: upgrade.upgrade_fingerprint.lvl_req,
            name: name.into(),
            weapon_name: weapon_name.into(),
            polarity: polarity.into(),
            attributes: _attributes,
            weapon_url_name: _weapon_url_name.into(),
            re_rolls: upgrade.upgrade_fingerprint.rerolls,
            oid: upgrade.item_id.oid.clone(),
            mod_rank: upgrade.upgrade_fingerprint.lvl,
        })
    });
    items
}

fn parse_riven_name(attributes_info: &Vec<AttributeInfo>, num_buffs: usize) -> String {
    let name = if num_buffs == 2 {
        format!("{}{}", attributes_info[0].prefix, attributes_info[1].suffix)
    } else if num_buffs == 3 {
        format!(
            "{}-{}{}",
            attributes_info[0].prefix, attributes_info[1].prefix, attributes_info[2].suffix
        )
    } else {
        panic!("no buffs with the associated riven!")
    };
    let mut chars = name.chars();
    let first = chars.next().expect("there should be a character here");
    format!("{}{}", first.to_uppercase().collect::<String>(), chars.collect::<String>())
}

fn calculate_attributes(
    attribute_info: Vec<AttributeInfo>,
    good_multiplier: f64,
    bad_multiplier: f64,
    disposition: f64,
    lvl: u8,
) -> Vec<Attribute> {
    let mut attributes = Vec::with_capacity(2);
    attribute_info.iter().for_each(|attr| {
        let x = f64::min(
            f64::max(0.9 + attr.value as f64 / 53687091.0 / 100.0, 0.9),
            1.1,
        );
        let good_bad_multiplier = if attr.positive {
            good_multiplier
        } else {
            bad_multiplier
        };
        let y = 90.0 * attr.base_value * disposition * good_bad_multiplier;
        let value = x * y * 100.0 * (lvl + 1) as f64 / 9.0;
        let value = match attr.units {
            Units::Multiply => (value + 100.0).round() / 100.0,
            _ => (value * 10.0).round() / 10.0,
        };
        let (_, short_string) = attr.short_string.split_once('>').unwrap_or(("", attr.short_string.deref()));
        attributes.push(Attribute {
            value,
            positive: attr.positive,
            short_string: short_string.to_string(),
            units: attr.units.clone(),
            url_name: attr.wfm_url.to_string(),
        });
    });
    attributes
}

#[derive(Debug)]
enum WeaponLookupError {
    InvalidWeapon(Rc<str>),
    InvalidField(WeaponLookupField),
}

impl Display for WeaponLookupError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let err = format!(
            "WeaponLookupError: {}",
            match self {
                WeaponLookupError::InvalidWeapon(compat) =>
                    format!("Could not find weapon matching the conmpat: {}", compat),
                WeaponLookupError::InvalidField(field) => format!(
                    "InvalidField: {}",
                    match field {
                        WeaponLookupField::Weapons => "Weapons",
                        WeaponLookupField::UniqueName => "UniqueName",
                        WeaponLookupField::WeaponUrlName => "WeaponUrlName",
                        WeaponLookupField::UpgradeType => "UpgradeType",
                        WeaponLookupField::Disposition => "Disposition",
                    }
                ),
            }
        );

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
    UpgradeType,
}

fn lookup_weapon_data(
    lookup: &RivenDataLookup,
    compat: &str,
) -> Result<(String, String, String, f64), WeaponLookupError> {
    if lookup.weapons.is_none() {
        return Err(WeaponLookupError::InvalidField(WeaponLookupField::Weapons));
    }
    let weapons = lookup.weapons.as_ref().unwrap();
    if weapons
        .iter()
        .find(|&weap| weap.unique_name.is_none())
        .is_some()
    {
        return Err(WeaponLookupError::InvalidField(
            WeaponLookupField::UniqueName,
        ));
    }
    if let Some(weapon) = weapons
        .iter()
        .find(|&weap| weap.clone().unique_name.unwrap() == compat.into())
    {
        let url_name = match weapon.wfm_url_name.as_ref() {
            Some(v) => v.deref(),
            None => {
                return Err(WeaponLookupError::InvalidField(
                    WeaponLookupField::WeaponUrlName,
                ))
            }
        };
        let name = match weapon.name.as_ref() {
            Some(v) => v.deref(),
            None => {
                return Err(WeaponLookupError::InvalidField(
                    WeaponLookupField::WeaponUrlName,
                ))
            }
        };
        let disposition = match weapon.disposition {
            Some(v) => v,
            None => {
                return Err(WeaponLookupError::InvalidField(
                    WeaponLookupField::Disposition,
                ))
            }
        };
        let weapon_type = match weapon.upgrade_type.clone() {
            Some(v) => v,
            None => {
                return Err(WeaponLookupError::InvalidField(
                    WeaponLookupField::UpgradeType,
                ))
            }
        };
        return Ok((
            url_name.to_string(),
            name.to_string(),
            weapon_type.deref().into(),
            disposition,
        ));
    } else {
        return Err(WeaponLookupError::InvalidWeapon(compat.into()));
    }
}

#[derive(Debug)]
enum RivenLookupError<'a> {
    InvalidItemType(&'a str),
    InvalidAttribute(&'a str),
    UnitsLookupError(UnitsLookupError),
    InvalidField(RivenLookupField),
}

impl<'a> Display for RivenLookupError<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let err = format!(
            "RivenLookupError: {}",
            match self {
                RivenLookupError::InvalidItemType(itype) =>
                    format!("Could not find weapon type: {}", itype),
                RivenLookupError::InvalidAttribute(iattr) =>
                    format!("Could not find attribute type: {}", iattr),
                RivenLookupError::UnitsLookupError(uerror) =>
                    format!("UnitsLookupError: {}", uerror),
                RivenLookupError::InvalidField(field) => format!(
                    "InvalidField: {}",
                    match field {
                        RivenLookupField::RivenAttributes => "RivenAttributes",
                        RivenLookupField::UniqueName => "UniqueName",
                        RivenLookupField::ModifierTag => "ModifierTag",
                        RivenLookupField::WfmUrl => "WfmUrl",
                        RivenLookupField::ShortString => "ShortString",
                        RivenLookupField::PrefixTag => "PrefixTag",
                        RivenLookupField::SuffixTag => "SuffixTag",
                        RivenLookupField::BaseValue => "BaseValue",
                    }
                ),
            }
        );

        f.write_str(err.as_str())
    }
}

impl<'a> Error for RivenLookupError<'a> {}

#[derive(Debug)]
enum RivenLookupField {
    RivenAttributes,
    UniqueName,
    ModifierTag,
    WfmUrl,
    ShortString,
    PrefixTag,
    SuffixTag,
    BaseValue,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, PartialOrd)]
pub enum Units {
    Percent,
    Multiply,
    Seconds,
    Null,
}

#[derive(Debug)]
pub struct InvalidUnitType(Arc<str>);

impl TryFrom<String> for Units {
    type Error = InvalidUnitType;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_str() {
            "Percent" => Ok(Units::Percent),
            "Multiply" => Ok(Units::Multiply),
            "Seconds" => Ok(Units::Seconds),
            "Null" => Ok(Units::Null),
            _ => Err(InvalidUnitType(value.into()))
        }
    }
}

impl ToSql for Units {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        match self {
            Units::Percent => "Percent".to_sql(),
            Units::Multiply => "Multiply".to_sql(),
            Units::Seconds => "Seconds".to_sql(),
            Units::Null => "Null".to_sql(),
        }
    }
}

#[derive(Debug)]
enum UnitsLookupError {
    InvalidField(UnitsLookupField),
    InvalidAttribute(Rc<str>),
    InvalidUnits(Arc<str>),
}

#[derive(Debug)]
enum UnitsLookupField {
    AvailableAttributes,
    Units,
    UrlName,
}

impl Display for UnitsLookupError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let err = match self {
            UnitsLookupError::InvalidField(v) => format!("Invalid field: {}", v),
            UnitsLookupError::InvalidAttribute(v) => format!("Invalid Attribute: {}", v),
            UnitsLookupError::InvalidUnits(v) => format!("Invalid Units: {}", v),
        };
        f.write_str(err.as_str())
    }
}

impl Display for UnitsLookupField {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let v = match self {
            UnitsLookupField::AvailableAttributes => "AvailableAttributes",
            UnitsLookupField::Units => "Units",
            UnitsLookupField::UrlName => "UrlName",
        };
        f.write_str(v)
    }
}

fn lookup_units<'a>(lookup: &'a RivenDataLookup, wfm_url: &'a str) -> Result<Units, UnitsLookupError> {
    if lookup.available_attributes.is_none() {
        return Err(UnitsLookupError::InvalidField(
            UnitsLookupField::AvailableAttributes,
        ));
    }
    let available_attributes = lookup.available_attributes.as_ref().unwrap();
    if available_attributes
        .iter()
        .find(|attr| attr.url_name.is_none())
        .is_some()
    {
        return Err(UnitsLookupError::InvalidField(UnitsLookupField::UrlName));
    }

    let attr = available_attributes
        .iter()
        .find(|&attr| attr.url_name.clone() == Some(wfm_url.into()));
    if attr.is_none() {
        return Err(UnitsLookupError::InvalidAttribute(wfm_url.into()));
    }
    let attr = attr.unwrap();
    match attr.units.clone() {
        Some(unit) => match &unit[..] {
            "percent" => Ok(Units::Percent),
            "multiply" => Ok(Units::Multiply),
            "seconds" => Ok(Units::Seconds),
            _ => Err(UnitsLookupError::InvalidUnits(unit)),
        },
        None => Ok(Units::Null),
    }
}

fn lookup_riven_data<'a>(
    lookup: &'a RivenDataLookup,
    weapon_type: &'a str,
    rattrs: Vec<RawAttributes<'a>>,
) -> Result<Vec<AttributeInfo>, RivenLookupError<'a>> {
    if lookup.rivens_attributes.is_none() {
        return Err(RivenLookupError::InvalidField(
            RivenLookupField::RivenAttributes,
        ));
    }
    let riven_attributes = lookup.rivens_attributes.as_ref().unwrap();
    if riven_attributes
        .iter()
        .find(|&attr| attr.unique_name.is_none())
        .is_some()
    {
        return Err(RivenLookupError::InvalidField(RivenLookupField::UniqueName));
    }
    let attrs = match riven_attributes
        .iter()
        .find(|&attr| attr.unique_name.clone() == Some(weapon_type.into()))
    {
        Some(v) => v,
        None => return Err(RivenLookupError::InvalidItemType(weapon_type.into())),
    };
    if attrs.upgrades.is_none() {
        return Err(RivenLookupError::InvalidField(
            RivenLookupField::RivenAttributes,
        ));
    }
    let upgrades = attrs.upgrades.as_ref().unwrap();
    if upgrades
        .iter()
        .find(|&upgr| upgr.modifier_tag.is_none())
        .is_some()
    {
        return Err(RivenLookupError::InvalidField(
            RivenLookupField::ModifierTag,
        ));
    }
    let mut attr_info: Vec<AttributeInfo> = Vec::with_capacity(2);
    rattrs
        .iter()
        .try_for_each(|rattr: &RawAttributes| -> Result<(), RivenLookupError> {
            let upgrade = match upgrades
                .iter()
                .find(|&upgr| upgr.modifier_tag == Some(rattr.tag.into()))
            {
                Some(v) => v,
                None => return Err(RivenLookupError::InvalidAttribute(rattr.tag)),
            };
            let wfm_url = match upgrade.wfm_url.clone() {
                Some(v) => v,
                None => return Err(RivenLookupError::InvalidField(RivenLookupField::WfmUrl)),
            };
            let short_string = match upgrade.short_string.clone() {
                Some(v) => v,
                None => return Err(RivenLookupError::InvalidField(RivenLookupField::ShortString)),
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
            let units = lookup_units(lookup, &wfm_url)
                .map_err(|e| RivenLookupError::UnitsLookupError(e))?;
            attr_info.push(AttributeInfo {
                positive: rattr.positive,
                value: rattr.value,
                wfm_url: wfm_url.clone(),
                prefix: prefix.clone(),
                suffix: suffix.clone(),
                base_value,
                units,
                short_string,
            });
            Ok(())
        })?;
    Ok(attr_info)
}

#[cfg(test)]
mod tests {
    use std::{fs::File, io::Write, sync::Arc};

    use dotenv::dotenv;
    use serde_json::to_value;
    use tokio::sync::Mutex;

    use crate::{http_client::{auth_state, qf_client::QFClient}, rivens::inventory::{raw_inventory::decrypt_last_data, riven_lookop::RivenDataLookup}};

    use super::convert_inventory_data;

    #[tokio::test]
    async fn test_convert_inventory_data() {
        dotenv().unwrap();
        let auth = auth_state::AuthState::setup().expect("hehe");
        let auth = Arc::new(Mutex::new(auth));
        let qf = QFClient::new(auth);
        let qf = Arc::new(Mutex::new(qf));
        let lookup = RivenDataLookup::setup(qf).await.unwrap();
        let raw_upgrades = decrypt_last_data(None).unwrap();
        let items = convert_inventory_data(&lookup, raw_upgrades);
        let out = to_value(items).unwrap();
        let mut file = File::create("rivenData.json").unwrap();
        file.write_all(out.to_string().as_bytes()).unwrap();
        // println!("{:#?}", items);
    }
}
