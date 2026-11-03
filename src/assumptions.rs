//! The numbers the client does not give, as `forever_sim/assumptions.py`
//! states them (each with its source there). The golden test
//! (`tests/levels.rs`) holds this copy to the Python one.

pub const ARMOR_CAP: f64 = 0.75;
pub const ARMOR_K: (f64, f64) = (400.0, 85.0);
pub const ARMOR_PER_AGILITY: f64 = 2.0;
pub const BOSS_CRIT: f64 = 5.0;
pub const BOSS_MISS: f64 = 5.0;
pub const FIRST_20: f64 = 20.0;
pub const FIVE_SECOND_RULE: f64 = 5.0;
pub const GLANCE_BASE: f64 = 10.0;
pub const GLANCE_DAMAGE: f64 = 0.85;
pub const MANA_PER_INT: f64 = 15.0;
pub const MELEE_CRIT_MULTIPLIER: f64 = 2.0;
pub const MELEE_MISS: f64 = 5.0;
pub const MOB_DODGE: f64 = 5.0;
pub const PUSHBACK_HITS: u32 = 2;
pub const REGEN_TICK: f64 = 2.0;
pub const SPELL_CRIT_MULTIPLIER: f64 = 1.5;
pub const SPELL_GCD: f64 = 1.5;
pub const SPELL_HIT_CAP: f64 = 99.0;
pub const SPIRIT_REGEN: (f64, f64) = (5.0, 15.0);
/// `levelling.MOB_SWING`, `levelling.MOB_RUN_SPEED`
pub const MOB_SWING: f64 = 2.0;
pub const MOB_RUN_SPEED: f64 = 7.0;
/// the share of a level's experience from kills (50-60%)
pub const KILL_XP_SHARE: f64 = 0.55;

/// Experience from level L to L+1 (MaNGOS Zero), L = 1..59.
pub const XP_TO_LEVEL: [f64; 59] = [
    400., 900., 1400., 2100., 2800., 3600., 4500., 5400., 6500., 7600., 8800., 10100., 11400., 12900., 14400., 16000., 17700., 19400., 21300., 23200., 25200., 27300., 29400., 31700., 34000., 36400., 38900., 41400., 44300., 47400., 50800., 54500., 58600., 62800., 67100., 71600., 76100., 80800., 85700., 90700., 95800., 101000., 106300., 111800., 117500., 123200., 129100., 135100., 141200., 147500., 153900., 160400., 167100., 173900., 180800., 187900., 195000., 202300., 209800.,
];

pub fn xp_to_level(level: i32) -> f64 {
    XP_TO_LEVEL[level as usize - 1]
}

// --- the druid (`forever_sim/assumptions.py` for the level-30 ones) ----------

/// MaNGOS Zero's night elf druid by level (player_levelstats race 4, class
/// 11): strength, agility, stamina, intellect, spirit. The High Order
/// skyborne has no Classic row and takes these (`BASE_ATTRIBUTES`).
pub const DRUID_LEVELS: [[f64; 5]; 60] = [
    [18., 25., 19., 22., 22.], [19., 25., 20., 23., 23.], [19., 26., 20., 24., 24.], [20., 26., 21., 25., 25.], [20., 27., 21., 26., 26.],
    [21., 27., 22., 27., 27.], [21., 28., 23., 28., 28.], [22., 28., 23., 28., 29.], [22., 29., 24., 29., 30.], [23., 29., 24., 30., 31.],
    [23., 30., 25., 31., 33.], [24., 30., 26., 32., 34.], [24., 31., 26., 33., 35.], [25., 31., 27., 34., 36.], [26., 32., 28., 36., 37.],
    [26., 32., 28., 37., 38.], [27., 33., 29., 38., 40.], [27., 34., 30., 39., 41.], [28., 34., 30., 40., 42.], [29., 35., 31., 41., 43.],
    [29., 35., 32., 42., 45.], [30., 36., 33., 43., 46.], [31., 37., 33., 44., 47.], [31., 37., 34., 46., 49.], [32., 38., 35., 47., 50.],
    [33., 38., 36., 48., 51.], [33., 39., 36., 49., 53.], [34., 40., 37., 50., 54.], [35., 40., 38., 52., 56.], [35., 41., 39., 53., 57.],
    [36., 42., 40., 54., 58.], [37., 42., 41., 56., 60.], [38., 43., 41., 57., 61.], [38., 44., 42., 58., 63.], [39., 44., 43., 60., 64.],
    [40., 45., 44., 61., 66.], [41., 46., 45., 62., 68.], [42., 46., 46., 64., 69.], [42., 47., 47., 65., 71.], [43., 48., 48., 67., 72.],
    [44., 49., 49., 68., 74.], [45., 49., 50., 70., 76.], [46., 50., 51., 71., 77.], [47., 51., 51., 73., 79.], [47., 52., 52., 74., 81.],
    [48., 53., 53., 76., 83.], [49., 53., 55., 77., 84.], [50., 54., 56., 79., 86.], [51., 55., 57., 81., 88.], [52., 56., 58., 82., 90.],
    [53., 57., 59., 84., 92.], [54., 58., 60., 86., 94.], [55., 59., 61., 87., 96.], [56., 59., 62., 89., 98.], [57., 60., 63., 91., 100.],
    [58., 61., 64., 93., 102.], [59., 62., 65., 94., 104.], [60., 63., 67., 96., 106.], [61., 64., 68., 98., 108.], [62., 65., 69., 100., 110.],
];
/// MaNGOS Zero's druid base health by level (player_classlevelstats, class 11)
pub const DRUID_HEALTH: [f64; 60] = [
    33., 41., 58., 65., 82., 89., 96., 113., 120., 137., 144., 151., 168., 175., 182., 199., 206., 214., 233., 243., 254., 266., 289., 303.,
    318., 334., 361., 379., 398., 418., 439., 461., 494., 518., 543., 569., 596., 624., 653., 683., 714., 746., 779., 823., 858., 894., 921.,
    959., 998., 1038., 1079., 1121., 1164., 1208., 1253., 1299., 1346., 1384., 1433., 1483.,
];
pub const DRUID_SPELL_CRIT_FROM_INT: (f64, f64, f64) = (3.33, 12.41, 0.79);
/// agility per 1% melee crit and dodge: 4.6 at level 1, 20 at 60, linear between
pub const AGI_PER_PERCENT: (f64, f64) = (4.6, 20.0);
pub const AP_PER_STRENGTH: f64 = 2.0;
pub const AP_BASE: f64 = -20.0;
/// health regeneration out of combat a 2 s tick: 0.09 x spirit + 6.5 (vmangos)
pub const DRUID_HP_REGEN: (f64, f64) = (0.09, 6.5);
pub const RAGE_CONVERSION: (f64, f64, f64) = (0.0091107836, 3.225598133, 4.2652911);
pub const RAGE_DEALT: f64 = 7.5;
pub const RAGE_TAKEN: f64 = 2.5;
pub const RAGE_CAP: f64 = 100.0;
/// rage lost a second out of combat (vmangos; not read in Forever)
pub const RAGE_DECAY: f64 = 1.0;
pub const CRIT_RAGE: f64 = 2.0;
pub const RIP_AP_AT_5: f64 = 0.24;
pub const SWIPE_AP: f64 = 0.03;
pub const THORNS_SPELL_POWER: f64 = 0.06;
/// Ferocious Bite's attack power share a combo point: the client stores none
/// (assumption, TBC's 3%; `README` *Talent builds*)
pub const BITE_AP_PER_CP: f64 = 0.03;
pub const OMEN_PPM: f64 = 3.5;
pub const OMEN_ICD: f64 = 10.0;
pub const MOB_PARRY: f64 = 5.0;
pub const MIN_GCD: f64 = 1.0;
/// The druid's death (2026-10-31): two minutes back to the corpse and healed to full
pub const DRUID_DEATH_SECONDS: f64 = 120.0;
pub const RUN_SPEED: f64 = 7.0;
/// The racials of Blizzard's notes, 2026-10-26, not yet in the client
/// (1.60.1.70532 still has Wind Blessed 3% and Energized +100% for 30 s, or
/// 15 min at a leyline): Wind Blessed 2% haste; Read Leyline +50% health and
/// mana regeneration for 15 s (100% while Attuned, an hour after casting it
/// at a leyline; 2026-10-28: levelling takes 50%, leylines are not
/// sought out). 2026-10-28: drinking and eating are lifted for
/// certain, and it is taken as the last factor on all regeneration (spirit
/// in the share that flows, mp5, drinking, eating); 1.12's rule for its
/// aura 110 would lift spirit only (README *Racials*).
pub const WIND_BLESSED_HASTE: f64 = 2.0;
pub const READ_LEYLINE_REGEN: f64 = 50.0;
pub const READ_LEYLINE_SECONDS: f64 = 15.0;
/// riding from 40, +60% (Classic; 2026-10-26)
pub const MOUNT_LEVEL: i32 = 40;
pub const MOUNT_SPEED: f64 = 60.0;
/// Walking between areas, towns and quests (2026-10-26: about half
/// the time of a normal levelling): at Classic's run speed a level's walk
/// takes as long as the warlock's measured route (README *The warlock from
/// 1 to 50*, the respec route) spends killing that level. These are that
/// route's XP an hour by level, 1-49 (`tmp/levelling/nb50_both.json`).
/// 50-59 go on at the 2% a level it grew over 40-49 (assumption: that route stops at 49).
pub const WALK_REFERENCE: [f64; 59] = [
    17754., 15180., 12970., 15275., 27417., 23500., 23153., 23821., 22095., 23022., 21792., 21306., 20863., 21936., 23510., 24698., 24245.,
    32996., 31582., 30293., 29013., 28843., 26920., 31391., 33249., 32696., 31065., 34595., 34073., 33930., 35183., 36661., 36575., 36962.,
    35035., 34988., 34711., 35622., 35140., 44365., 44458., 45794., 45933., 46066., 46778., 48790., 49630., 51656., 52925.,
    53984., 55063., 56164., 57288., 58433., 59602., 60794., 62010., 63250., 64515.,
];
