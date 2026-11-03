//! The game client's tables: wago.tools'
//! CSV of each DB2 table for the pinned build, with the build's hotfix rows
//! on top (latest push winning, a non-1 status deleting the row), and the
//! game tables (tab-separated). Everything is read from `data/<build>/`,
//! which `tools/fetch.py` fills.

use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// The druid's levelling (`druid::levels`): the last client before launch (it
/// carries the launch talent tree), with the hotfix rows `tools/fetch.py`
/// gathers, and `druid::launch` over it.
pub const DRUID_BUILD: &str = "1.60.1.70753";

pub fn build_dir(build: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("data").join(build)
}

/// One table: its rows in the table's order (a hotfix replaces a row in
/// place, a new one goes last, as a Python dict keeps them).
pub struct Table {
    col: HashMap<String, usize>,
    rows: Vec<Vec<String>>,
    order: Vec<usize>,
    by_id: HashMap<i64, usize>,
}

#[derive(Clone, Copy)]
pub struct Row<'a> {
    t: &'a Table,
    r: &'a [String],
}

impl<'a> Row<'a> {
    pub fn s(&self, c: &str) -> &'a str {
        let i = *self.t.col.get(c).unwrap_or_else(|| panic!("no column {c}"));
        &self.r[i]
    }

    /// A number; an empty cell reads 0 (`int(x or 0)`).
    pub fn f(&self, c: &str) -> f64 {
        let v = self.s(c);
        if v.is_empty() {
            return 0.0;
        }
        v.parse().unwrap_or_else(|_| panic!("{c}={v} is not a number"))
    }

    pub fn i(&self, c: &str) -> i64 {
        let v = self.s(c);
        if v.is_empty() {
            return 0;
        }
        v.parse::<i64>().unwrap_or_else(|_| self.f(c) as i64)
    }
}

#[derive(Deserialize)]
struct Hotfix {
    push_id: i64,
    record_id: i64,
    status: i64,
    data: Vec<serde_json::Value>,
}

fn flatten(v: &serde_json::Value, out: &mut Vec<String>) {
    match v {
        serde_json::Value::Array(a) => a.iter().for_each(|x| flatten(x, out)),
        serde_json::Value::String(s) => out.push(s.clone()),
        serde_json::Value::Null => out.push(String::new()),
        x => out.push(x.to_string()),
    }
}

impl Table {
    fn read(path: &Path, delimiter: u8, id: Option<&str>) -> Table {
        let mut rd = csv::ReaderBuilder::new()
            .delimiter(delimiter)
            .flexible(true)
            .from_path(path)
            .unwrap_or_else(|e| panic!("{}: {e}; run `forever-sim fetch`", path.display()));
        let header: Vec<String> = rd.headers().expect("a header").iter().map(str::to_owned).collect();
        let col: HashMap<String, usize> = header.iter().enumerate().map(|(i, h)| (h.clone(), i)).collect();
        let mut rows = Vec::new();
        for rec in rd.records() {
            let rec = rec.expect("a CSV row");
            rows.push(rec.iter().map(str::to_owned).collect::<Vec<_>>());
        }
        let mut t = Table { col, rows, order: Vec::new(), by_id: HashMap::new() };
        for i in 0..t.rows.len() {
            let key = match id {
                Some(c) => Row { t: &t, r: &t.rows[i] }.i(c),
                None => i as i64,
            };
            // a repeated ID keeps its first place and the last row (a dict)
            if let Some(&j) = t.by_id.get(&key) {
                t.rows[j] = t.rows[i].clone();
            } else {
                t.by_id.insert(key, i);
                t.order.push(i);
            }
        }
        t
    }

    fn hotfix(&mut self, path: &Path) {
        if !path.exists() {
            return;
        }
        let mut fixes: Vec<Hotfix> = serde_json::from_str(&std::fs::read_to_string(path).expect("a hotfix file")).expect("hotfix JSON");
        fixes.sort_by_key(|h| h.push_id);
        let ncol = self.col.len();
        for h in fixes {
            if h.status != 1 {
                if let Some(i) = self.by_id.remove(&h.record_id) {
                    self.order.retain(|&j| j != i);
                }
                continue;
            }
            let mut vals = Vec::with_capacity(ncol);
            h.data.iter().for_each(|v| flatten(v, &mut vals));
            assert_eq!(vals.len(), ncol, "{}: hotfix {} has {} values", path.display(), h.record_id, vals.len());
            match self.by_id.get(&h.record_id) {
                Some(&i) => self.rows[i] = vals,
                None => {
                    self.rows.push(vals);
                    let i = self.rows.len() - 1;
                    self.by_id.insert(h.record_id, i);
                    self.order.push(i);
                }
            }
        }
    }

    pub fn get(&self, id: i64) -> Option<Row<'_>> {
        self.by_id.get(&id).map(|&i| Row { t: self, r: &self.rows[i] })
    }

    pub fn iter(&self) -> impl Iterator<Item = Row<'_>> {
        self.order.iter().map(move |&i| Row { t: self, r: &self.rows[i] })
    }
}

/// The tables the druid's numbers come from (`tools/fetch.py` downloads the same list).
const TABLES: [&str; 31] = [
    "SpellName",
    "SpellRadius",
    "SpellLevels",
    "SkillLineAbility",
    "SpellEffect",
    "SpellMisc",
    "SpellDuration",
    "SpellCooldowns",
    "SpellRange",
    "SpellCastTimes",
    "SpellPower",
    "SpellClassOptions",
    "TraitNodeEntry",
    "TraitDefinition",
    "CurvePoint",
    "TraitDefinitionEffectPoints",
    "TraitNode",
    "TraitNodeXTraitNodeEntry",
    "TraitEdge",
    "ItemSparse",
    "Item",
    "RandPropPoints",
    "ItemArmorTotal",
    "ItemArmorQuality",
    "ArmorLocation",
    "ItemDamageWand",
    "ExpectedStat",
    "ItemDamageOneHand",
    "ItemDamageTwoHand",
    "ItemDamageRanged",
    "SpellAuraOptions",
];
const GAMETABLES: [&str; 2] = ["basemp", "hppersta"];
/// Per-spell tables: their rows grouped by SpellID, in the table's order.
const BY_SPELL: [&str; 7] = ["SpellEffect", "SpellMisc", "SpellLevels", "SpellPower", "SpellCooldowns", "SpellClassOptions", "SpellAuraOptions"];

pub struct Client {
    tables: HashMap<String, Table>,
    by_spell: HashMap<String, HashMap<i64, Vec<usize>>>,
}

impl Client {
    pub fn load(build: &str) -> Client {
        use rayon::prelude::*;
        let dir = build_dir(build);
        let mut tables: HashMap<String, Table> = TABLES
            .par_iter()
            .map(|name| {
                let mut t = Table::read(&dir.join(format!("{name}.csv")), b',', Some("ID"));
                t.hotfix(&dir.join(format!("hotfix_{name}.json")));
                (name.to_string(), t)
            })
            .collect();
        for g in GAMETABLES {
            tables.insert(format!("gt_{g}"), Table::read(&dir.join(format!("gt_{g}.txt")), b'\t', None));
        }
        let by_spell = BY_SPELL
            .iter()
            .map(|name| {
                let t = &tables[*name];
                let mut m: HashMap<i64, Vec<usize>> = HashMap::new();
                for &i in &t.order {
                    m.entry(Row { t, r: &t.rows[i] }.i("SpellID")).or_default().push(i);
                }
                (name.to_string(), m)
            })
            .collect();
        Client { tables, by_spell }
    }

    pub fn t(&self, name: &str) -> &Table {
        self.tables.get(name).unwrap_or_else(|| panic!("table {name} not loaded"))
    }

    /// A spell's rows in a per-spell table.
    pub fn rows(&self, table: &str, sid: i64) -> Vec<Row<'_>> {
        let t = self.t(table);
        self.by_spell[table].get(&sid).map(|v| v.iter().map(|&i| Row { t, r: &t.rows[i] }).collect()).unwrap_or_default()
    }

    // --- spells (`forever_sim.spells`) ---

    pub fn effect(&self, sid: i64, index: i64) -> Row<'_> {
        self.rows("SpellEffect", sid)
            .into_iter()
            .find(|e| e.i("EffectIndex") == index)
            .unwrap_or_else(|| panic!("spell {sid} has no effect {index}"))
    }

    /// An effect's base points, grown by level up to the spell's MaxLevel.
    pub fn points(&self, sid: i64, index: i64, level: Option<i32>) -> f64 {
        let e = self.effect(sid, index);
        let mut v = e.f("EffectBasePointsF");
        let per = e.f("EffectRealPointsPerLevel");
        let lv = self.rows("SpellLevels", sid);
        if let (true, Some(level), Some(l)) = (per != 0.0, level, lv.first()) {
            let top = if l.i("MaxLevel") != 0 { l.i("MaxLevel") } else { level as i64 };
            v += per * (0.max((level as i64).min(top) - l.i("BaseLevel"))) as f64;
        }
        v
    }

    fn misc(&self, sid: i64, c: &str) -> i64 {
        self.rows("SpellMisc", sid)[0].i(c)
    }

    pub fn duration(&self, sid: i64) -> f64 {
        match self.t("SpellDuration").get(self.misc(sid, "DurationIndex")) {
            Some(r) if r.i("Duration") > 0 => r.i("Duration") as f64 / 1000.0,
            _ => 0.0,
        }
    }

    pub fn cooldown(&self, sid: i64) -> f64 {
        match self.rows("SpellCooldowns", sid).first() {
            Some(r) => r.i("RecoveryTime").max(r.i("CategoryRecoveryTime")) as f64 / 1000.0,
            None => 0.0,
        }
    }

    pub fn reach(&self, sid: i64) -> f64 {
        self.t("SpellRange").get(self.misc(sid, "RangeIndex")).map(|r| r.f("RangeMax_0")).unwrap_or(0.0)
    }

    pub fn cast_time(&self, sid: i64) -> f64 {
        self.t("SpellCastTimes").get(self.misc(sid, "CastingTimeIndex")).map(|r| r.i("Base") as f64 / 1000.0).unwrap_or(0.0)
    }

    pub fn cost(&self, sid: i64) -> f64 {
        self.rows("SpellPower", sid).first().map(|r| r.f("ManaCost")).unwrap_or(0.0)
    }

    pub fn spell_name(&self, sid: i64) -> Option<&str> {
        self.t("SpellName").get(sid).map(|r| r.s("Name_lang"))
    }

    /// A game table's value in `column` at `level`.
    pub fn gametable(&self, name: &str, level: i32, column: &str) -> f64 {
        self.t(&format!("gt_{name}"))
            .iter()
            .find(|r| r.i("Level") == level as i64)
            .unwrap_or_else(|| panic!("gt_{name} has no level {level}"))
            .f(column)
    }
}
