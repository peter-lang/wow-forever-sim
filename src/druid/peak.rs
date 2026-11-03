//! A playstyle's rotation, and a build's worth under it: the same few priorities at every level.

use super::farm::{session, xp_per_hour};
use super::launch::Style;
use super::levels::{Data, Kit};
use super::play::Play;
use crate::route::{Class, Step, FRESH, SECONDS};
use crate::talents::Build;
use std::collections::HashMap;
use std::sync::Mutex;

/// The style's priorities for this build at this level, one play or, for the bear, one for each
/// pull size that is safe to try. A spell not known yet is skipped by the fight itself; the
/// choices below are the ones that follow from what is known.
pub fn rotations(d: &Data, style: Style, level: i32, b: &Build) -> Vec<Play> {
    let has = |n: &str| b.get(n).copied().unwrap_or(0) > 0;
    let known = |n: &str| d.rank(n, level, false).is_some();
    let base = Play {
        balance: false,
        form: "cat".into(),
        mobs: 1,
        heal_below: 0.4,
        heal_to: 0.85,
        drag: true,
        rip_cp: 5,
        bite_cp: 3,
        faerie_fire: false,
        swipe_at: 2,
        roar: true,
        enrage: true,
        maul_spare: 80.0,
        emergency: 0.0,
        starfire_opener: true,
        moonfire: true,
        swarm: false,
        melee_below: 0.0,
        moonkin: false,
        leyline_cd: false,
        shifting: false,
        bite_pool: 0.0,
        lacerate: 0,
        frenzied: 0.0,
        opener: 0,
        roots_after: 0,
        grasp: false,
        starfire_filler: 0,
        roots: false,
        hurricane: 0,
        barkskin: false,
    };
    let caster = Play {
        balance: d.kit == Kit::Balance,
        form: "caster".into(),
        heal_below: 0.6,
        starfire_opener: false,
        melee_below: if level < 13 { 0.5 } else { 0.0 },
        moonkin: has("Moonkin Form"),
        swarm: has("Insect Swarm"),
        ..base.clone()
    };
    let bear = |mobs: usize| Play {
        form: "bear".into(),
        mobs,
        heal_below: 0.6,
        drag: false,
        maul_spare: if mobs > 1 { 80.0 } else { 30.0 },
        emergency: if level < 16 { 0.3 } else { 0.2 },
        // on a pull it only spends the next Swipe's rage
        lacerate: if has("Lacerate") && mobs == 1 { 5 } else { 0 },
        frenzied: if level >= 36 { 0.5 } else { 0.0 },
        ..base.clone()
    };
    let swipe = known("Swipe");
    let cat = Play {
            heal_below: 0.6,
            heal_to: 1.0,
            rip_cp: if level < 40 { 3 } else { 5 },
            shifting: has("Shifting Power"),
            bite_pool: if has("Predatory Discipline") { 60.0 } else { 0.0 },
            ..base.clone()
        };
    match style {
        Style::Feral => {
            let mut plays = if swipe { vec![bear(3)] } else { vec![bear(1), caster] };
            if known("Cat Form") {
                plays.push(cat);
            }
            plays
        }
        Style::BearAoe if known("Bear Form") && !swipe => vec![bear(1)],
        Style::BearAoe | Style::Cat if !known("Cat Form") && swipe => vec![bear(3)],
        Style::BearAoe if swipe => vec![bear(3)],
        Style::BearAoe => vec![caster],
        Style::Cat if !known("Cat Form") => vec![caster],
        Style::Cat => vec![cat],
        // the storm needs Moonkin Form's armour: without it the channel is one death after another
        Style::Hurricane if known("Hurricane") && has("Moonkin Form") => vec![Play { hurricane: 3, barkskin: true, heal_to: 1.0, ..caster }],
        Style::Hurricane | Style::MoonkinSingle => vec![Play { heal_below: 0.4, barkskin: true, ..caster }],
    }
}

/// The share of kills that must die alone even when pulling several: 5-10% (2026-10-31).
pub const ALONE: f64 = 0.075;

/// How this style fights a mob on its own.
fn one_target(d: &Data, style: Style, level: i32, b: &Build) -> Vec<Play> {
    let from = match style {
        Style::BearAoe | Style::Cat | Style::Feral => Style::Feral,
        Style::Hurricane | Style::MoonkinSingle => Style::MoonkinSingle,
    };
    let mut plays = rotations(d, from, level, b);
    plays.retain(|p| p.hurricane == 0 && p.mobs == 1);
    if from == Style::Feral && plays.is_empty() {
        plays = rotations(d, Style::Cat, level, b);
    }
    plays
}

pub struct Peak<'a> {
    pub d: &'a Data,
    pub style: Style,
    memo: Mutex<HashMap<(i32, Build), (f64, Play)>>,
}

const SEEDS: u64 = 48;

impl<'a> Peak<'a> {
    pub fn new(d: &'a Data, style: Style) -> Self {
        Peak { d, style, memo: Mutex::new(HashMap::new()) }
    }

    fn on(&self, level: i32, b: &Build, seeds: impl Iterator<Item = u64> + Clone) -> (f64, Play) {
        let sc = self.d.scenario(level, b);
        let run = |p: &Play| {
            let ts: Vec<_> = seeds.clone().map(|s| session(&sc, p, s, SECONDS)).collect();
            xp_per_hour(&ts)
        };
        let alone = one_target(self.d, self.style, level, b).iter().map(run).fold(0.0, f64::max);
        rotations(self.d, self.style, level, b)
            .into_iter()
            .map(|p| {
                let x = run(&p);
                let several = p.mobs > 1 || p.hurricane > 0;
                // XP an hour over the same XP from both: time adds up, not rate
                (if several { 1.0 / ((1.0 - ALONE) / x + ALONE / alone) } else { x }, p)
            })
            .max_by(|a, c| a.0.total_cmp(&c.0))
            .expect("a rotation")
    }

    /// A build's XP an hour at a level, with its rotation.
    pub fn xp(&self, level: i32, b: &Build) -> (f64, Play) {
        let key = (level, b.clone());
        if let Some(x) = self.memo.lock().unwrap().get(&key) {
            return x.clone();
        }
        let x = self.on(level, b, 1..=SEEDS);
        self.memo.lock().unwrap().insert(key, x.clone());
        x
    }

    /// Deaths in a day's play, on seeds the rotation was not chosen on.
    pub fn deaths(&self, level: i32, b: &Build, p: &Play) -> f64 {
        let sc = self.d.scenario(level, b);
        let n = FRESH.count() as f64;
        FRESH.map(|s| session(&sc, p, s, SECONDS).deaths as f64).sum::<f64>() / n * 86400.0 / SECONDS
    }

    pub fn step(&self, level: i32, b: &Build) -> Step<Play> {
        let (xp, play) = self.xp(level, b);
        let hours = if level < 60 { Class::hours(self.d, level, b, xp) } else { 0.0 };
        Step { level, build: b.clone(), xp_per_hour: xp, hours, play }
    }
}
