//! The druid at any level from 1 to 60 (2026-10-26), built from the
//! client: the level-30 Python druid (`druid.sheet`, `druid_sim`,
//! `levelling`) carried to every level, High Order skyborne.
//!
//! - **Spells**: the highest rank trained by the level (`SkillLineAbility` +
//!   `SpellLevels`); a talent's spell from its talent, at the rank the
//!   level has (its first one before that).
//! - **The character**: MaNGOS Zero's night elf druid by level (the
//!   skyborne has no Classic row), the client's `basemp` and `hppersta`;
//!   Wind Blessed's haste and Read Leyline as Blizzard's notes of
//!   2026-10-26 have them (`assumptions::WIND_BLESSED_HASTE`,
//!   `READ_LEYLINE_*`; `NO_LEYLINE` set in the environment leaves it out);
//!   or, with `RACE=nightelf`, the night elf (2026-10-28): Quickness's
//!   dodge and run speed, Elune's Light (`NO_ELUNE` leaves it out). Mark of
//!   the Wild and Thorns at the level's rank.
//! - **Gear, two sets** (a druid levels as Balance in spell gear or
//!   as a feral in melee gear, and cannot swap): the leatherworking greens'
//!   stat shares of the budget, feral (strength, agility, stamina, attack
//!   power) or caster (intellect, spirit, stamina, spell power, healing,
//!   mp5), a full set at the item level of `GEAR_LAG` levels behind, the
//!   same item level for both; the weapons are the site's druid paths,
//!   every upgrade: the feral two-hander by DPS, for Balance the
//!   most spell power of a staff or a main hand and off-hand.
//! - **Mobs** of the druid's level (`ExpectedStat`, classic content).

use super::scenario::*;
use crate::assumptions as A;
use super::launch;
use crate::client::{Client, DRUID_BUILD};
use crate::items::{self, *};
use crate::talents::{Build, Tree};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};

const SKILL_LINES: [i64; 3] = [574, 134, 573];
const IMPROVED_WRATH: i64 = 16814;
/// talents whose spell has ranks in the book: the talent brings the level's rank
const TALENT_SPELLS: [&str; 7] = ["Insect Swarm", "Primal Bite", "Lacerate", "Feral Charge", "Moonkin Form", "Swiftmend", "Nature's Swiftness"];
pub const TAB_NAMES: [&str; 3] = ["Balance", "Feral", "Restoration"];
const BEAR_PASSIVE: i64 = 1178;
const DIRE_BEAR_PASSIVE: i64 = 9635;
const DIRE_BEAR_LEVEL: i32 = 40;
const CAT_PASSIVE: i64 = 3025;
const CAT_FORM: i64 = 768;
const BEAR_FORM: i64 = 5487;
const MOONKIN_AURA: i64 = 24907;
const LEADER_OF_THE_PACK: i64 = 24932;
const READ_LEYLINE: i64 = 1259705;
pub const QUICKNESS: i64 = 20582;
const ELUNES_LIGHT: i64 = 1259799;

/// The druid's race: High Order skyborne, or night elf (both on the night
/// elf's base attributes)
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Race {
    Skyborne,
    NightElf,
}
const CAT_LEAP: i64 = 414924;
const ECLIPSE: i64 = 408248;
const ECLIPSE_BUFF: i64 = 408255;
const NATURES_GRACE_BUFF: i64 = 16886;
const BLOOD_FRENZY_RAGE: i64 = 16959;
const FUROR_RAGE: i64 = 17057;
/// Natural Reaction's rage on a dodge: its tooltip's 5 (the effect points do not show it)
const NATURAL_REACTION_RAGE: f64 = 5.0;
const OMEN_LEVEL: i32 = 20;
/// food and drink by level (Classic's vendor tiers): the level, food, drink
const FOOD: [(i32, i64, i64); 6] = [(45, 1131, 1137), (35, 1129, 1135), (25, 1127, 1133), (15, 435, 432), (5, 434, 431), (1, 433, 430)];

// --- gear --------------------------------------------------------------------

/// classicwowforever.com's leatherworking recipes, all nine pages
/// (2026-10-26): the green armor of required level 1-30 with feral stats
/// and none of a caster's, and the other way round
/// (`tmp/druidlvl/lw_items.json`)
const FERAL_GREENS: [i64; 42] = [
    2303, 2307, 2310, 2315, 2316, 2317, 4237, 4239, 4243, 4246, 4249, 4252, 4254, 4456, 5780, 5781, 5782, 5957, 5961, 5963, 5964, 6466, 6467,
    7281, 7282, 7285, 7358, 7359, 7373, 7374, 276998, 276999, 277000, 277003, 277022, 277023, 277024, 277027, 277042, 277043, 277050, 277051,
];
const CASTER_GREENS: [i64; 28] = [
    2309, 2312, 3719, 4242, 4244, 4247, 4250, 4251, 4256, 4258, 5958, 5962, 6709, 7283, 20575, 252449, 277004, 277005, 277006, 277009, 277028,
    277029, 277030, 277033, 277044, 277045, 277052, 277053,
];
/// the slots a set fills: head, shoulders, chest, waist, legs, feet,
/// wrists, hands, back, neck and two rings (neck and rings take the cloak's
/// pattern, as for the warlock)
const SET_SLOTS: [i64; 12] = [1, 3, 5, 6, 7, 8, 9, 10, 16, 2, 11, 11];
const FERAL_STATS: [(&str, &[i64]); 4] = [("strength", &[STRENGTH]), ("agility", &[AGILITY]), ("stamina", &[STAMINA]), ("attack_power", &[ATTACK_POWER])];
const CASTER_STATS: [(&str, &[i64]); 6] = [
    ("intellect", &[INTELLECT]),
    ("spirit", &[SPIRIT]),
    ("stamina", &[STAMINA]),
    ("spell_power", &[SPELL_POWER, SPELL_DAMAGE]),
    ("healing", &[HEALING_POWER]),
    ("mp5", &[MP5]),
];
/// the set worn at level L is the one required at L - GEAR_LAG (assumption, as the warlock's)
pub const GEAR_LAG: i32 = 2;
/// The site's feral path (two-handers by DPS, High Order skyborne), every
/// upgrade (2026-10-26): the level worn from (the slot's or the
/// item's own, the later), the item.
const FERAL_PATH: [(i32, i64); 17] = [
    (1, 2132), (4, 2493), (7, 263312), (9, 257345), (11, 6214), (14, 279895), (16, 5187), (18, 7230), (23, 273817), (25, 1264), (29, 9449),
    (38, 250608), (39, 7723), (45, 9477), (50, 11608), (51, 12776), (58, 12796),
];
/// Path weapons the client's item table lacks (only `Item` has a row): the
/// site's Forever numbers, DPS, speed, stats (2026-10-26)
const SITE_WEAPONS: [(i64, f64, f64, &[(i64, f64)]); 2] = [
    (7723, 38.82, 3.4, &[(STRENGTH, 8.0), (STAMINA, 14.0), (INTELLECT, 16.0)]), // Mograine's Might
    (9477, 43.53, 3.4, &[]),                                                   // The Chief's Enforcer
];
/// Balance's: at each slot the most spell power of the caster staff path's
/// picks and alternatives, or of a main hand and an off-hand of the
/// one-hand path's (the site counts healing; Balance wants spell power,
/// the site's own note): the level, the site's spell power (the client
/// stores none for caster weapons), the items.
const BALANCE_PATH: [(i32, f64, &[i64]); 14] = [
    (1, 0.0, &[2480]),
    (10, 5.0, &[263937]),
    (11, 8.0, &[1928]),
    (13, 21.0, &[271095, 279898]),
    (16, 25.0, &[2169, 279898]),
    (20, 31.0, &[2567, 273137]),
    (24, 35.0, &[3414, 273137]),
    (26, 40.0, &[3414, 273027]),
    (30, 48.0, &[6691, 273027]),
    (32, 54.0, &[6691, 7685]),
    (34, 58.0, &[7714, 7685]),
    (35, 62.0, &[271804, 7685]),
    (54, 69.0, &[271804, 11904]),
    (58, 95.0, &[22688, 11904]),
];

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize, Hash)]
pub enum Kit {
    Feral,
    Balance,
}

impl Kit {
    pub fn name(self) -> &'static str {
        match self {
            Kit::Feral => "feral",
            Kit::Balance => "balance",
        }
    }
}

#[derive(Clone, Default, Debug, Serialize)]
pub struct Gear {
    pub stats: BTreeMap<i64, f64>,
    pub armor: f64,
    pub weapon_dps: f64,
    pub weapon_speed: f64,
    pub items: Vec<i64>,
}

/// Everything the levels share: the client, the talent tree, the spellbook
/// and each set stat's share of the budget.
pub struct Data {
    pub c: Client,
    pub race: Race,
    pub tree: Tree,
    book: HashMap<String, Vec<(i64, i64)>>,
    feral_shares: Vec<f64>,
    caster_shares: Vec<f64>,
    pub kit: Kit,
    /// `STYLE` in the environment: the way of levelling, whose plays are the only ones tried
    pub style: Option<launch::Style>,
}

struct At<'a> {
    d: &'a Data,
    level: i32,
    build: &'a Build,
}

impl Data {
    pub fn load(kit: Kit) -> Data {
        let c = Client::load(DRUID_BUILD);
        let mut tree = Tree::load(&c, IMPROVED_WRATH);
        launch::tree(&mut tree);
        let mut level: HashMap<i64, i64> = HashMap::new();
        for r in c.t("SpellLevels").iter() {
            level.insert(r.i("SpellID"), r.i("SpellLevel"));
        }
        let mut book: HashMap<String, Vec<(i64, i64)>> = HashMap::new();
        for r in c.t("SkillLineAbility").iter() {
            if SKILL_LINES.contains(&r.i("SkillLine")) && r.s("AcquireMethod") != "3" {
                let sid = r.i("Spell");
                if let Some(name) = c.spell_name(sid) {
                    book.entry(name.to_owned()).or_default().push((level.get(&sid).copied().unwrap_or(0), sid));
                }
            }
        }
        for v in book.values_mut() {
            v.sort();
            v.dedup();
        }
        let race = match std::env::var("RACE").as_deref() {
            Ok("nightelf") => Race::NightElf,
            Ok(r) if r != "skyborne" => panic!("RACE: skyborne or nightelf, not {r}"),
            _ => Race::Skyborne,
        };
        let mut d = Data { c, race, tree, book, feral_shares: Vec::new(), caster_shares: Vec::new(), kit, style: std::env::var("STYLE").ok().map(|n| launch::Style::from_name(&n)) };
        d.feral_shares = d.shares(&FERAL_GREENS, &FERAL_STATS);
        d.caster_shares = d.shares(&CASTER_GREENS, &CASTER_STATS);
        d
    }

    /// Every spell of the book by the level its first rank is learnt.
    pub fn book(&self) -> Vec<(i64, String, Vec<i64>)> {
        let mut out: Vec<(i64, String, Vec<i64>)> = self.book.iter().map(|(k, v)| (v[0].0, k.clone(), v.iter().map(|x| x.0).collect())).collect();
        out.sort();
        out
    }

    /// The highest rank of `name` learnt by `level`; a talent's first rank comes with it.
    pub fn rank(&self, name: &str, level: i32, talent: bool) -> Option<i64> {
        let ranks = self.book.get(name)?;
        ranks.iter().filter(|(lv, _)| *lv <= level as i64).map(|&(_, s)| s).last().or(if talent { Some(ranks[0].1) } else { None })
    }

    fn budget(&self, ilvl: i64, inv: i64) -> f64 {
        self.c.t("RandPropPoints").get(ilvl).expect("budget row").f(&format!("GoodF_{}", items::budget_slot(inv).expect("a slot")))
    }

    pub fn set_budget(&self, ilvl: i64) -> f64 {
        SET_SLOTS.iter().map(|&s| self.budget(ilvl, s)).sum()
    }

    /// Each set stat's points per point of a full set's budget, over the greens.
    fn shares(&self, ids: &[i64], stats: &[(&str, &[i64])]) -> Vec<f64> {
        let mut total = vec![0.0; stats.len()];
        let mut budget = 0.0;
        for &id in ids {
            let it = items::item(&self.c, id);
            let set = self.set_budget(it.item_level);
            let scale = set / self.budget(it.item_level, it.inventory_type);
            for (k, (_, ss)) in stats.iter().enumerate() {
                total[k] += ss.iter().map(|s| it.stats.get(s).copied().unwrap_or(0.0)).sum::<f64>() * scale;
            }
            budget += set;
        }
        total.iter().map(|t| t / budget).collect()
    }

    /// A green leather set's armor at a required level (item level + 5).
    pub fn set_armor(&self, level: i32) -> f64 {
        let ilvl = (level + 5).max(1) as i64;
        SET_SLOTS[..9].iter().map(|&inv| items::armor(&self.c, ilvl, 2, inv, 2)).sum()
    }

    /// The gear at `level`: the set `GEAR_LAG` behind, the weapons.
    pub fn gear(&self, level: i32) -> Gear {
        let at = level - GEAR_LAG;
        let budget = if at >= 5 { self.set_budget(at as i64 + 5) } else { 0.0 };
        let mut stats: BTreeMap<i64, f64> = BTreeMap::new();
        let (shares, which) = match self.kit {
            Kit::Feral => (&self.feral_shares, &FERAL_STATS[..]),
            Kit::Balance => (&self.caster_shares, &CASTER_STATS[..]),
        };
        for (k, (_, ss)) in which.iter().enumerate() {
            *stats.entry(ss[0]).or_insert(0.0) += shares[k] * budget;
        }
        let mut g = Gear { stats, armor: self.set_armor(at.max(1)), ..Gear::default() };
        let add = |g: &mut Gear, id: i64| {
            if let Some(&(_, dps, speed, stats)) = SITE_WEAPONS.iter().find(|w| w.0 == id) {
                for &(k, x) in stats {
                    *g.stats.entry(k).or_insert(0.0) += x;
                }
                g.items.push(id);
                return Some((dps * speed, dps * speed, speed));
            }
            let it = items::item(&self.c, id);
            for (k, x) in it.stats {
                *g.stats.entry(k).or_insert(0.0) += x;
            }
            g.items.push(id);
            it.weapon
        };
        match self.kit {
            Kit::Feral => {
                let &(_, id) = FERAL_PATH.iter().filter(|p| p.0 <= level).last().expect("a weapon");
                let w = add(&mut g, id).expect("a weapon's damage");
                g.weapon_dps = (w.0 + w.1) / 2.0 / w.2;
                g.weapon_speed = w.2;
            }
            Kit::Balance => {
                let &(_, sp, ids) = BALANCE_PATH.iter().filter(|p| p.0 <= level).last().expect("a weapon");
                let main = add(&mut g, ids[0]).expect("a weapon's damage");
                for &id in &ids[1..] {
                    add(&mut g, id);
                }
                *g.stats.entry(SPELL_POWER).or_insert(0.0) += sp;
                g.weapon_dps = (main.0 + main.1) / 2.0 / main.2;
                g.weapon_speed = main.2;
            }
        }
        g
    }

    /// Whether `level` trains a spell rank or brings a form or a weapon that
    /// `level - 1` did not have: the choices a play makes may change there.
    pub fn new_at(&self, level: i32) -> bool {
        const NAMES: [&str; 29] = [
            "Wrath", "Moonfire", "Starfire", "Insect Swarm", "Rejuvenation", "Regrowth", "Healing Touch", "Thorns", "Mark of the Wild", "Claw",
            "Rake", "Rip", "Ferocious Bite", "Maul", "Swipe", "Demoralizing Roar", "Enrage", "Primal Bite", "Lacerate", "Faerie Fire", "Bear Form", "Cat Form",
            "Dire Bear Form", "Innervate", "Hurricane", "Entangling Roots", "Pounce", "Ravage", "Prowl",
        ];
        let weapon = |l: i32| match self.kit {
            Kit::Feral => FERAL_PATH.iter().filter(|p| p.0 <= l).count(),
            Kit::Balance => BALANCE_PATH.iter().filter(|p| p.0 <= l).count(),
        };
        level == 1 || level == OMEN_LEVEL || NAMES.iter().any(|n| self.rank(n, level, false) != self.rank(n, level - 1, false)) || weapon(level) != weapon(level - 1)
    }

    pub fn scenario(&self, level: i32, build: &Build) -> Scenario {
        self.scenario_with(level, build, &self.gear(level))
    }

    pub fn scenario_with(&self, level: i32, build: &Build, gear: &Gear) -> Scenario {
        At { d: self, level, build }.export(gear)
    }
}

impl At<'_> {
    fn c(&self) -> &Client {
        &self.d.c
    }

    /// The talent's effect `index` at the build's rank (0 untaken).
    fn t(&self, name: &str, index: i64) -> f64 {
        self.d.tree.value(&self.d.c, name, index, self.build.get(name).copied().unwrap_or(0))
    }

    fn has(&self, name: &str) -> bool {
        self.build.get(name).copied().unwrap_or(0) > 0
    }

    fn sid(&self, name: &str) -> Option<i64> {
        let talent = TALENT_SPELLS.contains(&name);
        if talent && !self.has(name) {
            return None;
        }
        self.d.rank(name, self.level, talent)
    }

    /// An effect's points, grown with the level (to the rank's MaxLevel).
    fn pts(&self, sid: i64, index: i64) -> f64 {
        self.c().points(sid, index, Some(self.level))
    }

    fn pct_cost(&self, sid: i64, base_mana: f64) -> f64 {
        self.c().rows("SpellPower", sid).first().map_or(0.0, |p| p.f("PowerCostPct") / 100.0 * base_mana)
    }

    fn range_min(&self, sid: i64) -> f64 {
        let idx = self.c().rows("SpellMisc", sid)[0].i("RangeIndex");
        self.c().t("SpellRange").get(idx).map_or(0.0, |r| r.f("RangeMin_0"))
    }

    /// A spell's rank at the level: cost, cast, cooldown, duration, and its
    /// direct (`direct` effect) and periodic (`tick` effect) parts.
    fn spell(&self, name: &str, direct: Option<i64>, tick: Option<i64>) -> Sp {
        let Some(sid) = self.sid(name) else { return Sp::default() };
        let c = self.c();
        let mut s = Sp {
            known: true,
            cost: c.cost(sid),
            cast: c.cast_time(sid),
            cooldown: c.cooldown(sid),
            duration: c.duration(sid),
            reach: c.reach(sid),
            ..Sp::default()
        };
        if let Some(i) = direct {
            let e = c.effect(sid, i);
            s.base = self.pts(sid, i);
            s.spread = e.f("Variance") * s.base / 2.0;
            s.coef = e.f("EffectBonusCoefficient");
        }
        if let Some(i) = tick {
            let e = c.effect(sid, i);
            s.tick = self.pts(sid, i);
            s.tick_coef = e.f("EffectBonusCoefficient");
            s.period = e.i("EffectAuraPeriod") as f64 / 1000.0;
        }
        s
    }

    /// A melee ability: flat damage (effect 0) and share of normal damage (effect 1).
    fn melee(&self, name: &str) -> Sp {
        let Some(sid) = self.sid(name) else { return Sp::default() };
        let c = self.c();
        let has_pct = c.rows("SpellEffect", sid).iter().any(|e| e.i("EffectIndex") == 1 && e.s("Effect") == "31");
        Sp {
            known: true,
            cost: c.cost(sid),
            cooldown: c.cooldown(sid),
            duration: c.duration(sid),
            base: self.pts(sid, 0),
            pct: if has_pct { self.pts(sid, 1) / 100.0 } else { 0.0 },
            reach: c.reach(sid),
            ..Sp::default()
        }
    }

    fn export(&self, g: &Gear) -> Scenario {
        let c = self.c();
        let level = self.level;
        let lvf = level as f64;
        let st = |k: i64| g.stats.get(&k).copied().unwrap_or(0.0);
        let base = A::DRUID_LEVELS[level as usize - 1];
        let motw = self.d.rank("Mark of the Wild", level, false);
        let motw_attr = motw.map_or(0.0, |m| if c.rows("SpellEffect", m).iter().any(|e| e.i("EffectIndex") == 1) { c.points(m, 1, None) } else { 0.0 });
        let motw_armor = motw.map_or(0.0, |m| c.points(m, 0, None));
        let attr = |k: usize, stat: i64| base[k] + motw_attr + st(stat);
        let strength = attr(0, STRENGTH);
        let agility = attr(1, AGILITY);
        let stamina = attr(2, STAMINA);
        let intellect = attr(3, INTELLECT) * (1.0 + self.t("Heart of the Wild", 0) / 100.0);
        let spirit = attr(4, SPIRIT) * (1.0 + self.t("Living Spirit", 0) / 100.0);
        let hppersta = c.gametable("hppersta", level, "Health");
        let base_mana = c.gametable("basemp", level, "Druid");
        let health_of = |sta: f64| A::DRUID_HEALTH[level as usize - 1] + sta.min(A::FIRST_20) + (sta - A::FIRST_20).max(0.0) * hppersta;
        let mana = base_mana + intellect.min(A::FIRST_20) + (intellect - A::FIRST_20).max(0.0) * A::MANA_PER_INT;
        let (lo, hi) = A::AGI_PER_PERCENT;
        let agi_rate = lo * (60.0 - lvf) / 59.0 + hi * (lvf - 1.0) / 59.0;
        let majesty = self.t("Nature's Majesty", 0);
        let (a, b, cc) = A::DRUID_SPELL_CRIT_FROM_INT;
        let spell_crit = a + intellect / (b + cc * lvf) + majesty;
        let base_crit = agility / agi_rate + majesty;
        let ap0 = A::AP_PER_STRENGTH * strength + A::AP_BASE + st(ATTACK_POWER);
        let armor0 = g.armor + agility * A::ARMOR_PER_AGILITY + motw_armor;
        let shape = 1.0 + self.t("Natural Shapeshifter", 0) / 100.0;
        let feral_crit = base_crit + self.t("Sharpened Claws", 0) + if self.has("Leader of the Pack") { c.points(LEADER_OF_THE_PACK, 0, None) } else { 0.0 };
        let elf = self.d.race == Race::NightElf;
        let quick = if elf { c.points(QUICKNESS, 0, None) } else { 0.0 };
        let feral_dodge = agility / agi_rate + quick + self.t("Feral Swiftness", 0) + self.t("Natural Reaction", 0);
        let predatory = self.t("Predatory Strikes", 0) / 100.0 * lvf;
        let thick = self.t("Thick Hide", 1) * lvf;
        let caster = Form { known: true, health: health_of(stamina), armor: armor0, attack_power: ap0, crit: base_crit, dodge: agility / agi_rate + quick, shift: 0.0 };
        let cat = if self.d.rank("Cat Form", level, false).is_some() {
            let s = strength * (1.0 + self.t("Heart of the Wild", 2) / 100.0);
            Form {
                known: true,
                health: health_of(stamina),
                armor: g.armor + thick + agility * A::ARMOR_PER_AGILITY + motw_armor,
                attack_power: A::AP_PER_STRENGTH * s + A::AP_BASE + st(ATTACK_POWER) + self.pts(CAT_PASSIVE, 0) + agility + predatory,
                crit: feral_crit,
                dodge: feral_dodge,
                shift: self.pct_cost(CAT_FORM, base_mana) * shape,
            }
        } else {
            Form::default()
        };
        let bear = if self.d.rank("Bear Form", level, false).is_some() {
            let passive = if level >= DIRE_BEAR_LEVEL { DIRE_BEAR_PASSIVE } else { BEAR_PASSIVE };
            let sta = stamina * (1.0 + self.t("Heart of the Wild", 1) / 100.0);
            Form {
                known: true,
                health: health_of(sta) + self.pts(passive, 2),
                armor: (g.armor + thick) * (1.0 + c.points(passive, 0, None) / 100.0) + agility * A::ARMOR_PER_AGILITY + motw_armor,
                attack_power: ap0 + self.pts(passive, 3) + predatory,
                crit: feral_crit,
                dodge: feral_dodge,
                shift: self.pct_cost(BEAR_FORM, base_mana) * shape,
            }
        } else {
            Form::default()
        };
        let moonkin = match self.sid("Moonkin Form") {
            Some(m) => Form {
                known: true,
                health: health_of(stamina),
                armor: g.armor * (1.0 + 3.6) + thick + agility * A::ARMOR_PER_AGILITY + motw_armor,
                attack_power: ap0,
                crit: base_crit,
                dodge: agility / agi_rate + quick,
                shift: self.pct_cost(m, base_mana) * shape,
            },
            None => Form::default(),
        };
        let power = st(SPELL_POWER) + st(SPELL_DAMAGE);
        let cost_cut = 1.0 + self.t("Moonglow", 0) / 100.0;
        let mut wrath = self.spell("Wrath", Some(0), None);
        wrath.cast = (wrath.cast + self.t("Improved Wrath", 0) / 1000.0).max(0.0);
        wrath.cost *= cost_cut + self.t("Improved Wrath", 1) / 100.0;
        let mut starfire = self.spell("Starfire", Some(0), None);
        starfire.cast = (starfire.cast + self.t("Improved Starfire", 0) / 1000.0).max(0.0);
        starfire.cost *= cost_cut;
        starfire.base *= launch::STARFIRE_SCALE;
        starfire.spread *= launch::STARFIRE_SCALE;
        let mut moonfire = self.spell("Moonfire", Some(1), Some(0));
        moonfire.cost *= cost_cut;
        moonfire.duration += self.t("Nature's Splendor", 0) / 1000.0;
        let mf_bonus = 1.0 + self.t("Improved Moonfire", 1) / 100.0;
        moonfire.base *= mf_bonus;
        moonfire.spread *= mf_bonus;
        moonfire.extra = mf_bonus; // the ticks' multiplier
        let mut insect_swarm = self.spell("Insect Swarm", None, Some(0));
        insect_swarm.cost *= cost_cut;
        insect_swarm.duration += self.t("Nature's Splendor", 2) / 1000.0;
        let swarm_miss = self.sid("Insect Swarm").map_or(0.0, |s| -c.points(s, 1, None));
        let gift = 1.0 + self.t("Gift of Nature", 0) / 100.0;
        let mut rejuvenation = self.spell("Rejuvenation", None, Some(0));
        rejuvenation.duration += self.t("Nature's Splendor", 0) / 1000.0;
        rejuvenation.extra = gift * (1.0 + self.t("Improved Rejuvenation", 0) / 100.0);
        let mut regrowth = self.spell("Regrowth", Some(0), Some(1));
        regrowth.duration += self.t("Nature's Splendor", 1) / 1000.0;
        regrowth.extra = gift;
        let mut healing_touch = self.spell("Healing Touch", Some(0), None);
        healing_touch.cast = (healing_touch.cast + self.t("Naturalist", 0) / 1000.0).max(0.0);
        healing_touch.extra = gift;
        let mut swiftmend = self.spell("Swiftmend", None, None);
        if swiftmend.known {
            swiftmend.cost = self.pct_cost(self.sid("Swiftmend").unwrap(), base_mana);
        }
        let swiftness = self.spell("Nature's Swiftness", None, None);
        let mut thorns = self.spell("Thorns", None, None);
        if let Some(s) = self.sid("Thorns") {
            thorns.base = c.points(s, 0, None);
        }
        let ferocity_energy = self.t("Ferocity", 1);
        let ferocity_rage = self.t("Ferocity", 0) / 10.0;
        let mut claw = self.melee("Claw");
        claw.cost += ferocity_energy;
        if claw.known {
            claw.pct = launch::CLAW_SHARE;
        }
        let mut rake = self.melee("Rake");
        if let Some(s) = self.sid("Rake") {
            rake.tick = self.pts(s, 1);
            rake.pct = 0.0;
            rake.period = c.effect(s, 1).i("EffectAuraPeriod") as f64 / 1000.0;
            if rake.period == 0.0 {
                rake.period = 3.0;
            }
        }
        rake.cost += ferocity_energy;
        rake.base *= launch::CAT_SCALE;
        rake.tick *= launch::CAT_SCALE;
        let mut rip = self.melee("Rip");
        if let Some(s) = self.sid("Rip") {
            let e = c.effect(s, 0);
            rip.base = e.f("EffectBasePointsF");
            rip.extra = e.f("EffectPointsPerResource");
            rip.period = e.i("EffectAuraPeriod") as f64 / 1000.0;
            rip.pct = 0.0;
        }
        rip.base *= launch::CAT_SCALE;
        rip.extra *= launch::CAT_SCALE;
        let mut bite = self.melee("Ferocious Bite");
        if let Some(s) = self.sid("Ferocious Bite") {
            bite.pct = 0.0;
            let e = c.effect(s, 0);
            bite.base = e.f("EffectBasePointsF");
            bite.extra = e.f("EffectPointsPerResource");
            bite.spread = e.f("Variance") * bite.base / 2.0;
            bite.base *= launch::CAT_SCALE;
            bite.extra *= launch::CAT_SCALE;
            bite.spread *= launch::CAT_SCALE;
        }
        let mut faerie_fire = self.spell("Faerie Fire", None, None);
        if let Some(s) = self.sid("Faerie Fire") {
            faerie_fire.extra = -c.points(s, 0, None);
        }
        let mut cat_leap = Sp::default();
        let mut bear_charge = Sp::default();
        if let Some(s) = self.sid("Feral Charge") {
            bear_charge = Sp { known: true, cooldown: c.cooldown(s), reach: c.reach(s), extra: self.range_min(s), cost: c.cost(s) / 10.0, ..Sp::default() };
            cat_leap = Sp { known: true, cooldown: c.cooldown(CAT_LEAP), reach: c.reach(CAT_LEAP), extra: self.range_min(CAT_LEAP), ..Sp::default() };
        }
        let mut maul = self.melee("Maul");
        maul.cost = maul.cost / 10.0 + ferocity_rage;
        let mut swipe = self.melee("Swipe");
        swipe.cost = swipe.cost / 10.0 + ferocity_rage;
        let mut roar = self.melee("Demoralizing Roar");
        roar.cost /= 10.0;
        roar.extra = -roar.base;
        roar.base = 0.0;
        let mut enrage = self.melee("Enrage");
        if let Some(s) = self.sid("Enrage") {
            enrage.base = c.points(s, 0, None); // rage over its duration
            enrage.extra = c.points(s, 1, None) / 10.0; // at once
        }
        let mut primal_bite = self.melee("Primal Bite");
        primal_bite.cost = primal_bite.cost / 10.0 + ferocity_rage;
        let mut lacerate = self.melee("Lacerate");
        if let Some(s) = self.sid("Lacerate") {
            lacerate.cost = lacerate.cost / 10.0 + self.t("Shredding Attacks", 1) / 10.0;
            lacerate.base = 0.0;
            lacerate.pct = 0.0;
            lacerate.tick = self.pts(s, 0);
            lacerate.period = 3.0;
            lacerate.duration = 15.0;
            lacerate.extra = launch::LACERATE_WEAPON;
        }
        let mut innervate = self.spell("Innervate", None, None);
        if let Some(s) = self.sid("Innervate") {
            innervate.cost = self.pct_cost(s, base_mana);
            innervate.extra = c.points(s, 1, None) / 100.0;
        }
        // instant, on the way to a pull; a melee hit on the caster roots the attacker
        let mut grasp = Sp::default();
        if let Some(sid) = self.sid("Nature's Grasp") {
            grasp = Sp { known: true, cost: c.cost(sid), cooldown: launch::GRASP_COOLDOWN, ..Sp::default() };
        }
        let mut roots = self.spell("Entangling Roots", None, None);
        roots.cost *= cost_cut;
        let root_break = launch::ROOT_BREAK_CHANCE / (1.0 + self.t("Improved Entangling Roots", 0) / 100.0);
        let mut hurricane = self.spell("Hurricane", None, None);
        if let Some(s) = self.sid("Hurricane") {
            let tick = launch::HURRICANE_TICKS.iter().find(|r| r.0 == s).expect("Hurricane's tick spell").1;
            hurricane.tick = self.pts(tick, 0);
            hurricane.tick_coef = c.effect(s, 0).f("EffectBonusCoefficient");
            hurricane.period = 1.0;
            hurricane.duration = 10.0;
            hurricane.extra = 0.2;
            hurricane.cost *= cost_cut;
        }
        // its tooltip: 20% less physical damage, no spellcasting delays, 15 s; 1 min
        let barkskin = if self.d.rank("Barkskin", level, false).is_some() { Sp { known: true, cooldown: 60.0, duration: 15.0, extra: 0.2, ..Sp::default() } } else { Sp::default() };
        let mut frenzied = Sp::default();
        if level >= 36 {
            frenzied = Sp { known: true, cooldown: 180.0, base: launch::FRENZIED_RAGE, extra: 0.01, duration: launch::FRENZIED_SECONDS, ..Sp::default() };
        }
        let mut prowl = Sp::default();
        if let Some(s) = self.sid("Prowl") {
            prowl = Sp { known: true, cooldown: 10.0, extra: 1.0 + c.points(s, 1, None) / 100.0, ..Sp::default() };
        }
        let mut pounce = Sp::default();
        if self.sid("Pounce").is_some() {
            pounce = Sp { known: true, cost: 50.0, tick: launch::pounce_bleed(level) * launch::CAT_SCALE / 6.0, period: 3.0, duration: 18.0, ..Sp::default() };
        }
        // from Prowl only: alone with a mob the cat is in front of it
        let mut shred = self.melee("Shred");
        shred.cost += self.t("Shredding Attacks", 0);
        if shred.known {
            shred.pct = launch::SHRED_SHARE;
        }
        let mut ravage = Sp::default();
        if self.sid("Ravage").is_some() {
            ravage = Sp { known: true, cost: 60.0, base: launch::ravage_flat(level), pct: launch::RAVAGE_SHARE, ..Sp::default() };
        }
        let mut shifting = Sp::default();
        if self.has("Shifting Power") {
            let s = self.d.tree.get("Shifting Power").spell_id;
            shifting = Sp { known: true, cost: self.pct_cost(s, base_mana) * shape, cooldown: launch::SHIFTING_POWER_COOLDOWN, extra: c.points(s, 0, None), ..Sp::default() };
        }
        let eclipse_buff_max = c.rows("SpellAuraOptions", ECLIPSE_BUFF).first().map_or(0.0, |r| r.f("CumulativeAura"));
        let mob = c.t("ExpectedStat").iter().find(|r| r.i("Lvl") == level as i64 && r.s("ExpansionID") == "-2").expect("a mob row");
        let food = FOOD.iter().find(|f| f.0 <= level).expect("food");
        let naturalist = 1.0 + self.t("Naturalist", 1) / 100.0;
        let vengeance = self.t("Vengeance", 0) / 100.0;
        Scenario {
            level,
            mob: MobD { level, health: mob.f("CreatureHealth"), armor: mob.f("CreatureArmor"), dps: mob.f("CreatureAutoAttackDps"), xp: 45.0 + 5.0 * lvf },
            caster,
            cat,
            bear,
            moonkin_crit: if self.has("Moonkin Form") { c.points(MOONKIN_AURA, 0, None) } else { 0.0 },
            moonkin_omen: if self.has("Moonkin Form") { 2.0 } else { 1.0 },
            moonkin,
            mana,
            base_mana,
            regen: (spirit / A::SPIRIT_REGEN.0 + A::SPIRIT_REGEN.1) / A::REGEN_TICK,
            reflection: self.t("Reflection", 0) / 100.0,
            mp5: st(MP5),
            hp_regen: (A::DRUID_HP_REGEN.0 * spirit + A::DRUID_HP_REGEN.1) / A::REGEN_TICK,
            spell_power: power,
            healing: power + st(HEALING_POWER) + self.t("Natural Instinct", 1) / 100.0 * intellect,
            spell_crit,
            spell_hit: A::SPELL_HIT_CAP.min(96.0 + self.t("Nature's Reach", 2)),
            melee_hit: self.t("Nature's Reach", 1),
            haste: if elf { 1.0 } else { 1.0 + A::WIND_BLESSED_HASTE / 100.0 },
            run_speed: if elf { c.points(QUICKNESS, 1, None) } else { 0.0 },
            elune_crit: if elf && std::env::var("NO_ELUNE").is_err() { c.points(ELUNES_LIGHT, 0, None) } else { 0.0 },
            elune_seconds: c.duration(ELUNES_LIGHT),
            elune_cooldown: c.cooldown(ELUNES_LIGHT),
            spell_crit_mult: 1.0 + (A::SPELL_CRIT_MULTIPLIER - 1.0) * (1.0 + vengeance),
            heal_crit_mult: A::SPELL_CRIT_MULTIPLIER,
            melee_crit_mult: 1.0 + (A::MELEE_CRIT_MULTIPLIER - 1.0) * (1.0 + self.t("Natural Instinct", 0) / 100.0),
            damage_mult: naturalist,
            periodic_mult: 1.0 + self.t("Genesis", 0) / 100.0,
            balance_mult: 1.0 + self.t("Moonfury", 0) / 100.0,
            weapon_dps: g.weapon_dps,
            weapon_speed: g.weapon_speed,
            cat_speed: self.t("Feral Swiftness", 1),
            leyline_bonus: if elf || std::env::var("NO_LEYLINE").is_ok() { 0.0 } else { A::READ_LEYLINE_REGEN / 100.0 },
            leyline_seconds: A::READ_LEYLINE_SECONDS,
            leyline_cooldown: c.cooldown(READ_LEYLINE),
            leyline_cast: c.cast_time(READ_LEYLINE),
            wrath,
            starfire,
            moonfire,
            insect_swarm,
            swarm_miss,
            grace: if self.has("Nature's Grace") { c.duration(NATURES_GRACE_BUFF) } else { 0.0 },
            eclipse_cut: -self.t("Eclipse", 1) / 1000.0,
            eclipse_per: if self.has("Eclipse") { c.points(ECLIPSE, 2, None) } else { 0.0 },
            eclipse_max: eclipse_buff_max,
            eclipse_secs: c.duration(ECLIPSE_BUFF),
            moonfire_crit: self.t("Improved Moonfire", 0),
            focus: self.t("Nature's Focus", 0) / 100.0,
            rejuvenation,
            regrowth,
            healing_touch,
            swiftmend,
            swiftness,
            bloom: if self.has("Bloom") { spirit } else { 0.0 },
            regrowth_crit: self.t("Improved Regrowth", 0),
            thorns,
            claw,
            rake,
            rip,
            bite,
            // a point of extra energy's damage: the rank's effect 1, in hundredths (its tooltip's "0.6 a point" at 60)
            bite_energy: self.sid("Ferocious Bite").map_or(0.0, |s| self.pts(s, 1) / 100.0),
            discipline: self.has("Predatory Discipline"),
            poised: if level >= launch::POISED_LEVEL { launch::POISED_ENERGY } else { 0.0 },
            shifting,
            faerie_fire,
            cat_leap,
            maul,
            swipe,
            roar,
            enrage,
            primal_bite,
            lacerate,
            bear_charge,
            savage: 1.0 + self.t("Savage Fury", 0) / 100.0,
            swipe_mult: 1.0 + self.t("Feral Instinct", 1) / 100.0,
            rend: 1.0 + self.t("Rend and Tear", 0) / 100.0,
            frenzy_rage: self.t("Blood Frenzy", 0) / 100.0,
            frenzy_cp: self.t("Blood Frenzy", 1) / 100.0,
            frenzy_amount: c.points(BLOOD_FRENZY_RAGE, 0, None) / 10.0,
            reaction: self.t("Natural Reaction", 1) / 100.0,
            reaction_rage: NATURAL_REACTION_RAGE,
            furor_rage: self.t("Furor", 0) / 100.0 * c.points(FUROR_RAGE, 0, None) / 10.0,
            furor_energy: self.t("Furor", 1) / 100.0,
            // its tooltip: 27% of base armor in Bear Form, 16% in Dire Bear Form
            enrage_armor: if level >= DIRE_BEAR_LEVEL { 0.16 } else { 0.27 },
            base_armor: agility * A::ARMOR_PER_AGILITY,
            innervate,
            roots,
            root_break,
            hurricane,
            barkskin,
            frenzied,
            prowl,
            pounce,
            ravage,
            shred,
            grasp,
            omen: level >= OMEN_LEVEL,
            omen_ppm: A::OMEN_PPM,
            omen_icd: A::OMEN_ICD,
            food: c.points(food.1, 0, None) / 5.0,
            drink: c.points(food.2, 0, None) / 5.0,
            rage_c: A::RAGE_CONVERSION.0 * lvf * lvf + A::RAGE_CONVERSION.1 * lvf + A::RAGE_CONVERSION.2,
        }
    }
}
