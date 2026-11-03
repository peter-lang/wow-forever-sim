//! What the client's tables (1.60.1.70753) do not hold about the druid: the
//! numbers of the 2026-11-03 patch notes, a few values only the tooltips show
//! (foreverchanges.pro's reading of the launch realms), the rules and
//! assumptions of the levelling sim, and the playstyles and their talent
//! orders. The talent tree and the spell ranks come from the client.

use crate::talents::Tree;

// The 2026-11-03 notes (Kaivax, post 9 of the development notes; authoritative). 1.60.1.70753 carries none of
// these numbers. Shred, Claw, Ravage and Poised are also in foreverchanges.pro's tooltips of 2026-11-03; the rest
// are the notes' "about".
pub const CLAW_SHARE: f64 = 1.40;
pub const SHRED_SHARE: f64 = 2.00;
/// Rip, Rake, Ferocious Bite, Pounce: base points and attack power scaling "increased by about 25%"
pub const CAT_SCALE: f64 = 1.25;
/// Starfire: "base damage points increased by about 40%"
pub const STARFIRE_SCALE: f64 = 1.40;
const REND_AND_TEAR: [f64; 5] = [3.0, 6.0, 9.0, 12.0, 15.0];
/// Poised, learned at 32: at full energy out of combat the next ability costs 10 less
pub const POISED_LEVEL: i32 = 32;
pub const POISED_ENERGY: f64 = 10.0;

/// The notes' changes to the client's talents.
pub fn tree(t: &mut Tree) {
    t.set_curve("Rend and Tear", 0, &REND_AND_TEAR);
}

/// Ferocious Bite's first rank (the level the cat's Bite settings start at)
pub const BITE_LEVEL: i32 = 24;
pub const SHIFTING_POWER_COOLDOWN: f64 = 8.0;
pub const GRASP_COOLDOWN: f64 = 60.0;
/// **Assumption:** seconds out of a rooted mob's reach (a few steps), and as long to come back
pub const GRASP_STEP: f64 = 1.0;
/// Dash's ranks, (level, % speed), from their tooltips
pub const DASH: [(i32, f64); 2] = [(26, 50.0), (46, 60.0)];
pub const DASH_SECONDS: f64 = 15.0;
pub const DASH_COOLDOWN: f64 = 300.0;
pub const LACERATE_STACKS: u32 = 5;
/// "plus 10% weapon damage per existing application": a direct hit as it lands,
/// 50% at five (2026-10-31: per stack; the client's effect 1 is a bare 10)
pub const LACERATE_WEAPON: f64 = 0.10;

/// The spell each Hurricane rank's ticks are (the rank's own effects hold no damage; nothing in the
/// client links the two, their levels do)
pub const HURRICANE_TICKS: [(i64, i64); 3] = [(16914, 1278965), (17401, 1278968), (17402, 1278759)];

/// Pounce's bleed over 18 s (the client's, 6 ticks of its Pounce Bleed) and Ravage's flat damage by rank, their tooltips
pub fn pounce_bleed(level: i32) -> f64 {
    [(36, 90.0), (46, 120.0), (56, 150.0)].iter().filter(|r| r.0 <= level).last().map_or(0.0, |r| r.1)
}

pub fn ravage_flat(level: i32) -> f64 {
    [(32, 147.0), (42, 217.0), (50, 273.0), (58, 343.0)].iter().filter(|r| r.0 <= level).last().map_or(0.0, |r| r.1)
}

/// 435% since 2026-11-03 (350% before)
pub const RAVAGE_SHARE: f64 = 4.35;
pub const POUNCE_STUN: f64 = 2.0;
/// Frenzied Regeneration: 10 rage a second into 1% health a point, 10 s (tooltip)
pub const FRENZIED_RAGE: f64 = 10.0;
pub const FRENZIED_SECONDS: f64 = 10.0;
/// 2026-10-31: a Wrath or Starfire on a rooted mob breaks it about half the time;
/// Improved Entangling Roots' 75% more damage taken makes that rarer in proportion
pub const ROOT_BREAK_CHANCE: f64 = 0.5;

/// A way of levelling (2026-10-31): how the druid fights changes little from level to
/// level; its talents make that easier.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Style {
    /// bear, several mobs at once
    BearAoe,
    /// cat, one mob at a time, leaving what its bleeds will kill
    Cat,
    /// one melee build, in whichever form is faster at the level: shifting costs no respec
    Feral,
    /// caster then Moonkin, Hurricane on several mobs from 40
    Hurricane,
    /// caster then Moonkin, one mob at a time
    MoonkinSingle,
}

impl Style {
    pub const ALL: [Style; 5] = [Style::BearAoe, Style::Cat, Style::Feral, Style::Hurricane, Style::MoonkinSingle];

    pub fn name(self) -> &'static str {
        match self {
            Style::BearAoe => "bear",
            Style::Cat => "cat",
            Style::Feral => "feral",
            Style::Hurricane => "hurricane",
            Style::MoonkinSingle => "single",
        }
    }

    pub fn from_name(s: &str) -> Style {
        *Style::ALL.iter().find(|x| x.name() == s).unwrap_or_else(|| panic!("a style, not {s}"))
    }

    /// The talents point by point from level 10, written by hand from the tooltips: what the
    /// fight leans on first, the row gates in mind.
    pub fn order(self) -> Vec<&'static str> {
        let list: &[(&str, u32)] = match self {
            Style::BearAoe => &[
                ("Ferocity", 5), ("Feral Instinct", 3), ("Feral Swiftness", 2), ("Savage Fury", 2), ("Sharpened Claws", 2), ("Feral Charge", 1), ("Predatory Strikes", 3), ("Blood Frenzy", 2),
                ("Leader of the Pack", 1), ("Natural Instinct", 2), ("Lacerate", 1), ("Thick Hide", 1), ("Natural Reaction", 5), ("Primal Bite", 1),
                ("Heart of the Wild", 5), ("Naturalist", 5), ("Genesis", 5), ("Nature's Majesty", 2), ("Nature's Reach", 2), ("Furor", 1),
            ],
            Style::Feral => &[
                ("Ferocity", 5), ("Feral Instinct", 3), ("Feral Swiftness", 2), ("Feral Charge", 1), ("Sharpened Claws", 2), ("Savage Fury", 2), ("Predatory Strikes", 3), ("Blood Frenzy", 2),
                ("Leader of the Pack", 1), ("Natural Instinct", 2), ("Lacerate", 1), ("Thick Hide", 1), ("Natural Reaction", 5), ("Primal Bite", 1),
                ("Heart of the Wild", 5), ("Naturalist", 5), ("Genesis", 5), ("Nature's Majesty", 2), ("Nature's Reach", 2), ("Furor", 1),
            ],
            Style::Cat => &[
                ("Ferocity", 5), ("Furor", 5), ("Feral Swiftness", 2), ("Savage Fury", 2), ("Sharpened Claws", 2), ("Feral Charge", 1), ("Predatory Strikes", 3), ("Blood Frenzy", 2),
                ("Leader of the Pack", 1), ("Natural Instinct", 2), ("Rend and Tear", 2),
                ("Genesis", 5), ("Nature's Majesty", 2), ("Nature's Reach", 2), ("Heart of the Wild", 5), ("Naturalist", 5), ("Natural Shapeshifter", 3), ("Rend and Tear", 2),
            ],
            Style::Hurricane => &[
                ("Improved Wrath", 5), ("Moonglow", 3), ("Improved Moonfire", 2), ("Nature's Reach", 2), ("Nature's Majesty", 2), ("Nature's Splendor", 1), ("Insect Swarm", 1), ("Vengeance", 5), ("Genesis", 4), ("Moonfury", 5), ("Moonkin Form", 1),
                ("Nature's Focus", 5), ("Naturalist", 5), ("Reflection", 3), ("Natural Shapeshifter", 3), ("Bloom", 1), ("Heart of the Wild", 3),
            ],
            Style::MoonkinSingle => &[
                ("Improved Wrath", 5), ("Moonglow", 3), ("Nature's Focus", 5), ("Reflection", 3), ("Improved Moonfire", 2), ("Nature's Reach", 2), ("Nature's Majesty", 2), ("Nature's Splendor", 1), ("Insect Swarm", 1), ("Vengeance", 5), ("Nature's Grace", 1), ("Eclipse", 3), ("Moonfury", 5), ("Moonkin Form", 1),
                ("Naturalist", 5), ("Improved Starfire", 5), ("Genesis", 2),
            ],
        };
        list.iter().flat_map(|(n, r)| std::iter::repeat_n(*n, *r as usize)).collect()
    }

    /// The order after a respec into this style, when the points before it no longer bind: the
    /// storm has no use for Insect Swarm, so Genesis takes its place on the way to Moonkin Form.
    pub fn after_respec(self) -> Vec<&'static str> {
        match self {
            Style::Hurricane => {
                let mut o = self.order();
                let at = o.iter().position(|n| *n == "Insect Swarm").expect("Insect Swarm");
                o.remove(at);
                let genesis = o.iter().rposition(|n| *n == "Genesis").expect("Genesis");
                o.insert(genesis, "Genesis");
                // not every pull can be three mobs: the fight one at a time gets these two
                // back, for what the last Heart of the Wild does in a storm (nothing to read)
                for _ in 0..2 {
                    o.remove(o.iter().rposition(|n| *n == "Heart of the Wild").expect("Heart of the Wild"));
                }
                let reflection = o.iter().rposition(|n| *n == "Reflection").expect("Reflection");
                o.insert(reflection + 1, "Nature's Grace");
                o.insert(reflection + 1, "Insect Swarm");
                // and Eclipse in place of Natural Shapeshifter: next to nothing either way in a storm
                for n in o.iter_mut().filter(|n| **n == "Natural Shapeshifter") {
                    *n = "Eclipse";
                }
                o
            }
            // one mob at a time after the respec: the 30 Balance points under Moonkin Form and the
            // form itself at 40 (it is the most a point is worth from 25 on), then Restoration's
            // points through Nature's Focus, and what is left by its worth a point at 40-59
            Style::MoonkinSingle => {
                let list: &[(&str, u32)] = &[
                    ("Improved Wrath", 5), ("Moonglow", 3), ("Improved Moonfire", 2), ("Nature's Majesty", 2), ("Nature's Reach", 2), ("Nature's Splendor", 1),
                    ("Insect Swarm", 1), ("Vengeance", 5), ("Nature's Grace", 1), ("Eclipse", 3), ("Moonfury", 5), ("Moonkin Form", 1),
                    ("Nature's Focus", 5), ("Naturalist", 5), ("Reflection", 3), ("Genesis", 5), ("Improved Starfire", 2),
                ];
                list.iter().flat_map(|(n, r)| std::iter::repeat_n(*n, *r as usize)).collect()
            }
            _ => self.order(),
        }
    }
}

/// A way to 60, one gear set throughout: its legs, each a talent order, how it fights, from a level.
pub struct Route {
    pub name: &'static str,
    pub legs: &'static [(Style, Style, i32)],
}

pub const ROUTES: [Route; 8] = [
    Route { name: "balance", legs: &[(Style::MoonkinSingle, Style::MoonkinSingle, 10), (Style::Hurricane, Style::Hurricane, 40)] },
    Route { name: "balance_plain", legs: &[(Style::Hurricane, Style::Hurricane, 10)] },
    Route { name: "feral", legs: &[(Style::Feral, Style::Feral, 10)] },
    Route { name: "bear", legs: &[(Style::BearAoe, Style::BearAoe, 10)] },
    Route { name: "cat", legs: &[(Style::Cat, Style::Cat, 10)] },
    // the cat's talents, fighting in whichever form is faster at the level
    Route { name: "cat_feral", legs: &[(Style::Cat, Style::Feral, 10)] },
    // Balance one mob at a time to 60, no Hurricane
    Route { name: "single", legs: &[(Style::MoonkinSingle, Style::MoonkinSingle, 10)] },
    // Balance's first leg, then a respec at 40 into Moonkin Form, still one mob at a time
    Route { name: "moonkin_single", legs: &[(Style::MoonkinSingle, Style::MoonkinSingle, 10), (Style::MoonkinSingle, Style::MoonkinSingle, 40)] },
];
