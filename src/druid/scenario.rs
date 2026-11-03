//! Every number the druid's farm reads at one level and build
//! (`druid::levels` fills it from the client; the farm only reads it).

use serde::{Deserialize, Serialize};

/// A spell or ability at its rank, with the build's talents on it. Fields a
/// spell has no use for stay 0; what each means is said where it is read.
#[derive(Serialize, Deserialize, Clone, Default, Debug)]
pub struct Sp {
    pub known: bool,
    /// mana, energy or rage (rage in points, not tenths)
    pub cost: f64,
    pub cast: f64,
    pub cooldown: f64,
    pub duration: f64,
    /// the direct part: average, half its spread, spell power coefficient;
    /// for a melee ability its flat damage
    pub base: f64,
    pub spread: f64,
    pub coef: f64,
    /// a melee ability's share of normal damage
    pub pct: f64,
    /// the periodic part: a tick, its coefficient, its period
    pub tick: f64,
    pub tick_coef: f64,
    pub period: f64,
    /// what else it does (Rip's points a combo point, Bite's per combo
    /// point, Roar's attack power off, Faerie Fire's armor off, Enrage's
    /// instant rage, a charge's shortest reach)
    pub extra: f64,
    pub reach: f64,
}

/// The druid in one form: what a mob's swing and the druid's melee read.
#[derive(Serialize, Deserialize, Clone, Default, Debug)]
pub struct Form {
    pub known: bool,
    pub health: f64,
    pub armor: f64,
    pub attack_power: f64,
    /// percent
    pub crit: f64,
    pub dodge: f64,
    /// mana to shift into it
    pub shift: f64,
}

#[derive(Serialize, Deserialize, Clone, Default, Debug)]
pub struct MobD {
    pub level: i32,
    pub health: f64,
    pub armor: f64,
    pub dps: f64,
    pub xp: f64,
}

#[derive(Serialize, Deserialize, Clone, Default, Debug)]
pub struct Scenario {
    pub level: i32,
    pub mob: MobD,
    pub caster: Form,
    pub cat: Form,
    pub bear: Form,
    /// Moonkin Form (talent): its form, +crit to spells, and Omen's chance x this
    pub moonkin: Form,
    pub moonkin_crit: f64,
    pub moonkin_omen: f64,
    pub mana: f64,
    pub base_mana: f64,
    /// spirit regeneration a second out of the five-second rule; Reflection's share inside it
    pub regen: f64,
    pub reflection: f64,
    pub mp5: f64,
    /// health a second out of combat
    pub hp_regen: f64,
    pub spell_power: f64,
    pub healing: f64,
    pub spell_crit: f64,
    /// spell hit against a mob of the level, percent; melee hit off the miss chance
    pub spell_hit: f64,
    pub melee_hit: f64,
    /// multiplier (1.02: Wind Blessed)
    pub haste: f64,
    /// percent, always on, added to any other (Quickness)
    pub run_speed: f64,
    /// Elune's Light, pressed at a pull when ready (off the global cooldown):
    /// crit with spells and attacks (0: not a night elf), seconds, cooldown
    pub elune_crit: f64,
    pub elune_seconds: f64,
    pub elune_cooldown: f64,
    /// damage spells (Vengeance), heals, melee (Natural Instinct)
    pub spell_crit_mult: f64,
    pub heal_crit_mult: f64,
    pub melee_crit_mult: f64,
    /// all damage (Naturalist), periodic damage and healing (Genesis),
    /// Arcane and Nature spells (Moonfury)
    pub damage_mult: f64,
    pub periodic_mult: f64,
    pub balance_mult: f64,
    pub weapon_dps: f64,
    pub weapon_speed: f64,
    /// percent: Cat Form's run speed (Feral Swiftness)
    pub cat_speed: f64,
    /// Read Leyline cast as the druid sits to rest: its regeneration bonus
    /// (0.5; 0 untaken), seconds, cooldown and cast
    pub leyline_bonus: f64,
    pub leyline_seconds: f64,
    pub leyline_cooldown: f64,
    pub leyline_cast: f64,
    // caster
    pub wrath: Sp,
    pub starfire: Sp,
    pub moonfire: Sp,
    pub insect_swarm: Sp,
    /// Insect Swarm's cut to the mob's hit, percent
    pub swarm_miss: f64,
    /// Nature's Grace's seconds (0 untaken); Eclipse: Starfire's cut, charges a Wrath, the cap, seconds
    pub grace: f64,
    pub eclipse_cut: f64,
    pub eclipse_per: f64,
    pub eclipse_max: f64,
    pub eclipse_secs: f64,
    /// Moonfire's extra crit (Improved Moonfire)
    pub moonfire_crit: f64,
    /// Nature's Focus: the chance a hit does not push a cast back
    pub focus: f64,
    // healing
    pub rejuvenation: Sp,
    pub regrowth: Sp,
    pub healing_touch: Sp,
    pub swiftmend: Sp,
    pub swiftness: Sp,
    /// Bloom: health when a HoT runs out (0 untaken)
    pub bloom: f64,
    pub regrowth_crit: f64,
    pub thorns: Sp,
    // cat
    pub claw: Sp,
    pub rake: Sp,
    pub rip: Sp,
    pub bite: Sp,
    /// Bite's damage a point of extra energy; Predatory Discipline keeps that energy
    pub bite_energy: f64,
    pub discipline: bool,
    /// energy off the first ability from full energy out of combat (Poised)
    pub poised: f64,
    /// its energy in `extra`
    pub shifting: Sp,
    pub faerie_fire: Sp,
    pub cat_leap: Sp,
    // bear
    pub maul: Sp,
    pub swipe: Sp,
    pub roar: Sp,
    pub enrage: Sp,
    pub primal_bite: Sp,
    /// its share of weapon damage per application in `extra`
    pub lacerate: Sp,
    pub bear_charge: Sp,
    /// Savage Fury, Feral Instinct (Swipe), Rend and Tear: multipliers
    pub savage: f64,
    pub swipe_mult: f64,
    pub rend: f64,
    /// Blood Frenzy: a crit's chance of rage (bear) and of a second combo point (cat); the rage
    pub frenzy_rage: f64,
    pub frenzy_cp: f64,
    pub frenzy_amount: f64,
    /// Natural Reaction: a dodge's chance of rage, and the rage
    pub reaction: f64,
    pub reaction_rage: f64,
    /// Furor: rage on the shift into Bear Form; the cat's energy kept
    pub furor_rage: f64,
    pub furor_energy: f64,
    /// the bear's armor while Enraged: base armor (agility's) cut by this share
    pub enrage_armor: f64,
    pub base_armor: f64,
    /// Innervate (40): its regeneration bonus in `extra` (4: +400%), seconds in `duration`
    pub innervate: Sp,
    /// Entangling Roots; the damage that breaks it, a share of the mob's health
    pub roots: Sp,
    pub root_break: f64,
    /// Hurricane: a tick to each mob in `tick`, the swing slowed by `extra`
    pub hurricane: Sp,
    /// Barkskin: physical damage taken cut by `extra`, no pushback
    pub barkskin: Sp,
    /// Frenzied Regeneration: rage a second in `base`, health a point of rage in `extra`
    pub frenzied: Sp,
    /// Prowl: run speed in `extra`; Pounce and Ravage from it
    pub prowl: Sp,
    pub pounce: Sp,
    pub ravage: Sp,
    pub shred: Sp,
    pub grasp: Sp,
    pub omen: bool,
    pub omen_ppm: f64,
    pub omen_icd: f64,
    /// food and drink a second, eating or drinking
    pub food: f64,
    pub drink: f64,
    pub rage_c: f64,
}
