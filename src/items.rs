//! Items as the Forever client derives them (`forever_sim.items`): each stat a
//! share of the item level's budget (`RandPropPoints`), armor and wand damage
//! from per-item-level tables.

use crate::client::{Client, Row};
use std::collections::BTreeMap;

pub const AGILITY: i64 = 3;
pub const STRENGTH: i64 = 4;
pub const INTELLECT: i64 = 5;
pub const SPIRIT: i64 = 6;
pub const ATTACK_POWER: i64 = 38;
pub const STAMINA: i64 = 7;
pub const HEALING_POWER: i64 = 41;
pub const SPELL_DAMAGE: i64 = 42;
pub const MP5: i64 = 43;
pub const SPELL_POWER: i64 = 45;
pub const HP5: i64 = 46;
pub const FIRE_SPELL_DAMAGE: i64 = 85;
pub const FROST_SPELL_DAMAGE: i64 = 87;
pub const SHADOW_SPELL_DAMAGE: i64 = 88;

/// InventoryType -> the budget column (0 head/chest/legs/two-hand, 1
/// shoulders/waist/feet/hands, 2 neck/wrist/finger/back/off-hand, 3
/// one-hand, 4 ranged).
pub fn budget_slot(inv: i64) -> Option<usize> {
    Some(match inv {
        1 | 4 | 5 | 7 | 17 | 20 => 0,
        3 | 6 | 8 | 10 | 12 => 1,
        2 | 9 | 11 | 14 | 16 | 23 => 2,
        13 | 21 | 22 => 3,
        15 | 25 | 26 => 4,
        _ => return None,
    })
}

fn family(quality: i64) -> Option<&'static str> {
    Some(match quality {
        2 => "Good",
        3 => "Superior",
        4 | 5 => "Epic",
        _ => return None,
    })
}

fn round(x: f64) -> f64 {
    (x + 0.5).floor()
}

pub struct Item {
    pub id: i64,
    pub item_level: i64,
    pub quality: i64,
    pub inventory_type: i64,
    pub stats: BTreeMap<i64, f64>,
    /// a wand's (min, max, speed)
    pub wand: Option<(f64, f64, f64)>,
    /// any weapon's (min, max, speed), from its slot's damage table
    pub weapon: Option<(f64, f64, f64)>,
    pub class_id: i64,
    pub subclass_id: i64,
    pub required_level: i64,
}

fn stats(c: &Client, row: &Row) -> BTreeMap<i64, f64> {
    let mut out = BTreeMap::new();
    let (Some(slot), Some(fam)) = (budget_slot(row.i("InventoryType")), family(row.i("OverallQualityID"))) else { return out };
    let budget = c.t("RandPropPoints").get(row.i("ItemLevel")).expect("a budget row").f(&format!("{fam}F_{slot}"));
    for i in 0..10 {
        let (stat, share) = (row.i(&format!("StatModifier_bonusStat_{i}")), row.i(&format!("StatPercentEditor_{i}")));
        if stat >= 0 && share != 0 {
            out.insert(stat, round(budget * share as f64 / 10000.0));
        }
    }
    out
}

/// Armor of an armor item from its level, quality, slot and subclass (no shields).
pub fn armor(c: &Client, item_level: i64, quality: i64, inventory_type: i64, subclass: i64) -> f64 {
    let (inv, sub) = match inventory_type {
        16 => (16, 1), // every cloak is cloth
        20 => (5, subclass),
        x => (x, subclass),
    };
    let (material, column) = match sub {
        1 => ("Cloth", "Clothmodifier"),
        2 => ("Leather", "Leathermodifier"),
        3 => ("Mail", "Chainmodifier"),
        4 => ("Plate", "Platemodifier"),
        _ => return 0.0,
    };
    let Some(loc) = c.t("ArmorLocation").get(inv) else { return 0.0 };
    let total = c.t("ItemArmorTotal").get(item_level).expect("armor row").f(material);
    let q = c.t("ItemArmorQuality").get(item_level).expect("quality row").f(&format!("Qualitymod_{quality}"));
    round(total * q * loc.f(column))
}

const WAND: i64 = 19;

pub fn item(c: &Client, id: i64) -> Item {
    let row = c.t("ItemSparse").get(id).unwrap_or_else(|| panic!("no item {id}"));
    let base = c.t("Item").get(id).expect("an Item row");
    let wand = if base.i("ClassID") == 2 && base.i("SubclassID") == WAND && row.i("ItemDelay") != 0 {
        let dps = c.t("ItemDamageWand").get(row.i("ItemLevel")).expect("wand row").f(&format!("Quality_{}", row.i("OverallQualityID")));
        let speed = row.i("ItemDelay") as f64 / 1000.0;
        let avg = dps * speed;
        let var = row.f("DmgVariance");
        Some(((avg * (1.0 - var / 2.0)).floor(), round(avg * (1.0 + var / 2.0)), speed))
    } else {
        None
    };
    let table = match (base.i("SubclassID"), row.i("InventoryType")) {
        (WAND, _) => Some("ItemDamageWand"),
        (_, 13 | 21 | 22) => Some("ItemDamageOneHand"),
        (_, 17) => Some("ItemDamageTwoHand"),
        (_, 15 | 25 | 26) => Some("ItemDamageRanged"),
        _ => None,
    };
    let weapon = match (base.i("ClassID"), table, row.i("ItemDelay")) {
        (2, Some(t), delay) if delay != 0 => {
            let dps = c.t(t).get(row.i("ItemLevel")).expect("a damage row").f(&format!("Quality_{}", row.i("OverallQualityID")));
            let speed = delay as f64 / 1000.0;
            let avg = dps * speed;
            let var = row.f("DmgVariance");
            Some(((avg * (1.0 - var / 2.0)).floor(), round(avg * (1.0 + var / 2.0)), speed))
        }
        _ => None,
    };
    Item {
        weapon,
        class_id: base.i("ClassID"),
        subclass_id: base.i("SubclassID"),
        required_level: row.i("RequiredLevel"),
        id,
        item_level: row.i("ItemLevel"),
        quality: row.i("OverallQualityID"),
        inventory_type: row.i("InventoryType"),
        stats: stats(c, &row),
        wand,
    }
}
