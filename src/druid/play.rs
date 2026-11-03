//! A druid's levelling play: its form, how many mobs at once, when it heals,
//! and each form's choices (the plays from the level-30 sims, README
//! *Levelling*, as the defaults), the grid of them a level allows, and the
//! descent over it.

use super::scenario::Scenario;
use crate::talents::Build;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct Play {
    /// in the Balance kit (spell gear), else the feral kit (melee gear)
    #[serde(default)]
    pub balance: bool,
    /// "caster" (Balance; before the forms, any druid), "cat", "bear"
    pub form: String,
    /// mobs a pull (the bear's triangle; the others one at a time)
    pub mobs: usize,
    /// between pulls: below this share of its health the druid heals to `heal_to`
    pub heal_below: f64,
    pub heal_to: f64,
    /// cat and caster: a mob its bleeds or DoTs will kill is left to them,
    /// and follows, hitting, while the next is pulled (the play)
    pub drag: bool,
    /// cat: Rip at this many combo points or more (0: never)
    pub rip_cp: u32,
    /// cat: Ferocious Bite at this many combo points or more while Rip
    /// ticks or is not used (0: never)
    pub bite_cp: u32,
    /// cat and bear: Faerie Fire on each mob
    pub faerie_fire: bool,
    /// bear: Swipe while this many mobs stand; Demoralizing Roar on two or
    /// more; Enrage at a pull; Maul with this much rage to spare
    pub swipe_at: usize,
    pub roar: bool,
    pub enrage: bool,
    pub maul_spare: f64,
    /// bear: mid-pull, below this share of health, out of form to heal (0: never)
    #[serde(default)]
    pub emergency: f64,
    /// caster: Starfire (when known) or Wrath to open, Moonfire and Insect
    /// Swarm kept up, melee instead of casting below this share of mana,
    /// in Moonkin Form (talent)
    pub starfire_opener: bool,
    pub moonfire: bool,
    pub swarm: bool,
    pub melee_below: f64,
    pub moonkin: bool,
    /// caster, out of Moonkin Form (High Order skyborne): Read Leyline at a
    /// pull when ready, not only at rest (2026-10-28: Reflection
    /// lets it work in combat)
    #[serde(default)]
    pub leyline_cd: bool,
    /// cat: Shifting Power when nothing is affordable and its energy fits (talent)
    #[serde(default)]
    pub shifting: bool,
    /// cat: with Bite's combo points ready, hold them until this much energy
    /// (Bite turns the extra into damage; Predatory Discipline keeps it; 0: at once)
    #[serde(default)]
    pub bite_pool: f64,
    /// bear: Lacerate on the target up to this many applications (talent, 0:
    /// never); `LACERATE_SPAM`: kept at five and cast with spare rage, 50% weapon damage each
    #[serde(default)]
    pub lacerate: u32,
    /// bear: Frenzied Regeneration below this share of health (0: never)
    #[serde(default)]
    pub frenzied: f64,
    /// cat: from Prowl (no chain pulls): 0 no, 1 Pounce, 2 Ravage, 3 Shred
    #[serde(default)]
    pub opener: u8,
    /// caster: Entangling Roots after the first spell, while the mob still runs in: 0 not, 1 once, 2 again each time it breaks
    #[serde(default)]
    pub roots_after: u8,
    /// caster: Nature's Grasp walking to a pull when ready; rooted, the mob is stepped away from
    #[serde(default)]
    pub grasp: bool,
    /// caster: Starfire in place of Wrath as the filler: 0 never, 1 always, 2 with Eclipse's charges
    #[serde(default)]
    pub starfire_filler: u8,
    /// caster: Entangling Roots on the mob running in
    #[serde(default)]
    pub roots: bool,
    /// caster: this many mobs under Hurricane (0: one at a time)
    #[serde(default)]
    pub hurricane: usize,
    /// caster under Hurricane: Barkskin as a channel starts, when ready
    #[serde(default)]
    pub barkskin: bool,
}

pub const LACERATE_SPAM: u32 = 6;

impl Play {
    pub fn key(&self) -> String {
        serde_json::to_string(self).unwrap()
    }

    pub fn cat_default() -> Play {
        Play::base("cat")
    }

    fn base(form: &str) -> Play {
        Play {
            balance: false,
            form: form.into(),
            mobs: 1,
            heal_below: 0.6,
            heal_to: 1.0,
            drag: true,
            rip_cp: 3,
            bite_cp: 0,
            faerie_fire: false,
            swipe_at: 3,
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
        }
    }

    /// The play in words, the page's (Hungarian).
    pub fn words(&self, sc: &Scenario) -> String {
        let heal = format!("gyógyítás {:.0}% alatt {:.0}%-ig", self.heal_below * 100.0, self.heal_to * 100.0);
        match self.form.as_str() {
            "cat" => {
                let mut v = vec!["cat".to_string()];
                if self.opener == 1 && sc.pounce.known {
                    v.push("Prowl, Pounce".into());
                }
                if self.opener == 2 && sc.ravage.known {
                    v.push("Prowl, Ravage".into());
                }
                if self.faerie_fire && sc.faerie_fire.known {
                    v.push("Faerie Fire".into());
                }
                if sc.rake.known {
                    v.push("Rake".into());
                }
                if self.rip_cp > 0 && sc.rip.known {
                    v.push(format!("Rip {}+ CP", self.rip_cp));
                }
                if self.bite_cp > 0 && sc.bite.known {
                    v.push(if self.bite_pool > 0.0 { format!("Ferocious Bite {}+ CP, {:.0} energiáig gyűjtve", self.bite_cp, self.bite_pool) } else { format!("Ferocious Bite {}+ CP", self.bite_cp) });
                }
                v.push("Claw".into());
                if self.shifting && sc.shifting.known {
                    v.push("Shifting Power".into());
                }
                if self.drag {
                    v.push("a vérző mobot otthagyja".into());
                }
                v.push(heal);
                v.join(", ")
            }
            "bear" => {
                let mut v = vec![format!("bear, {} mob", self.mobs)];
                if self.enrage && sc.enrage.known {
                    v.push("Enrage".into());
                }
                if self.roar && sc.roar.known && self.mobs > 1 {
                    v.push("Demoralizing Roar".into());
                }
                if self.lacerate > 0 && sc.lacerate.known {
                    v.push(if self.lacerate == LACERATE_SPAM { "Lacerate 5x, utána rage-ből folyamatosan".to_string() } else { format!("Lacerate {}x", self.lacerate) });
                }
                if self.frenzied > 0.0 && sc.frenzied.known {
                    v.push(format!("Frenzied Regeneration {:.0}% HP alatt", self.frenzied * 100.0));
                }
                if sc.primal_bite.known {
                    v.push("Primal Bite".into());
                }
                if sc.swipe.known && self.swipe_at <= self.mobs {
                    v.push(format!("Swipe {}+ mobon", self.swipe_at));
                }
                v.push(format!("Maul {:.0} rage felett", self.maul_spare));
                if self.emergency > 0.0 {
                    v.push(format!("harc közben gyógyít {:.0}% HP alatt", self.emergency * 100.0));
                }
                if self.faerie_fire && sc.faerie_fire.known {
                    v.push("Faerie Fire".into());
                }
                v.push(heal);
                v.join(", ")
            }
            _ => {
                let mut v = vec![if self.moonkin { "moonkin".to_string() } else { "caster".to_string() }];
                if self.hurricane > 0 && sc.hurricane.known {
                    v.push(format!("{} mob, Hurricane{}", self.hurricane, if self.barkskin && sc.barkskin.known { ", Barkskin" } else { "" }));
                }
                if self.roots && sc.roots.known {
                    v.push("Entangling Roots a befutó mobra".into());
                }
                if self.leyline_cd && sc.leyline_bonus > 0.0 {
                    v.push("Read Leyline a pull elején, ha kész".into());
                }
                v.push(if self.starfire_opener && sc.starfire.known { "Starfire nyitás".into() } else { "Wrath nyitás".into() });
                if self.moonfire && sc.moonfire.known {
                    v.push("Moonfire".into());
                }
                if self.swarm && sc.insect_swarm.known {
                    v.push("Insect Swarm".into());
                }
                v.push("Wrath".into());
                if self.melee_below > 0.0 {
                    v.push(format!("közelharc {:.0}% mana alatt", self.melee_below * 100.0));
                }
                if self.drag {
                    v.push("a DoT-olt mobot otthagyja".into());
                }
                v.push(heal);
                v.join(", ")
            }
        }
    }
}

/// The choices of a level's grid, each a dimension; a play is one index per dimension.
pub type Ix = [usize; 13];
const HEAL_BELOW: [f64; 3] = [0.4, 0.6, 0.75];
const HEAL_TO: [f64; 2] = [0.85, 1.0];
const RIP_CP: [u32; 3] = [0, 3, 5];
const BITE_CP: [u32; 3] = [0, 3, 5];
const SWIPE_AT: [usize; 2] = [2, 3];
const MAUL: [f64; 2] = [30.0, 80.0];
const MELEE: [f64; 3] = [0.0, 0.2, 0.5];
const EMERGENCY: [f64; 3] = [0.0, 0.3, 0.4];
/// dimensions the full grid holds at their default: the descent tries them
const DESCENT_ONLY: [usize; 2] = [1, 8];

pub struct Grid {
    forms: Vec<&'static str>,
    bear_mobs: usize,
    starfire: bool,
    moonfire: bool,
    bite: bool,
    ff: bool,
    swipe: bool,
    roar: bool,
    enrage: bool,
    moonkin: bool,
    swarm: bool,
}

impl Grid {
    /// The forms a level and kit allow: Balance stays a caster (spell gear); a
    /// feral is a caster before Bear Form, then bear, then cat or bear.
    pub fn new(sc: &Scenario, balance: bool, build: &Build) -> Grid {
        // a feral with Cat Form never wins as a caster: the grid drops it
        let mut forms = if balance || !sc.cat.known { vec!["caster"] } else { vec![] };
        if !balance {
            if sc.bear.known {
                forms.push("bear");
            }
            if sc.cat.known {
                forms.push("cat");
            }
        }
        let talent = |n: &str| build.get(n).copied().unwrap_or(0) > 0;
        Grid {
            forms,
            bear_mobs: 4,
            starfire: sc.starfire.known,
            moonfire: sc.moonfire.known,
            bite: sc.bite.known,
            ff: sc.faerie_fire.known,
            swipe: sc.swipe.known,
            roar: sc.roar.known,
            enrage: sc.enrage.known,
            moonkin: talent("Moonkin Form"),
            swarm: talent("Insect Swarm"),
        }
    }

    /// [form, heal below, heal to, drag, cat rip, cat bite or bear emergency, faerie fire,
    /// bear mobs, bear swipe at, roar, maul, caster opener + moonfire, caster melee]
    pub fn sizes(&self) -> Ix {
        let b = |x: bool| if x { 2 } else { 1 };
        [
            self.forms.len(),
            HEAL_BELOW.len(),
            HEAL_TO.len(),
            2,
            RIP_CP.len(),
            if self.bite || self.forms.contains(&"bear") { BITE_CP.len() } else { 1 },
            b(self.ff),
            self.bear_mobs,
            if self.swipe { SWIPE_AT.len() } else { 1 },
            b(self.roar),
            MAUL.len(),
            1 + usize::from(self.starfire) + usize::from(self.moonfire) * 2,
            MELEE.len(),
        ]
    }

    pub fn make(&self, ix: &Ix) -> Option<Play> {
        let form = self.forms[ix[0]];
        let mut p = Play::base(form);
        p.heal_below = HEAL_BELOW[ix[1]];
        p.heal_to = HEAL_TO[ix[2]];
        if p.heal_to <= p.heal_below {
            return None;
        }
        p.drag = ix[3] == 0;
        p.moonkin = self.moonkin && form == "caster";
        p.swarm = self.swarm && form == "caster";
        p.enrage = self.enrage;
        // a choice another form makes stays at its first value
        let off = |dims: &[usize]| dims.iter().any(|&d| ix[d] != 0);
        match form {
            "cat" => {
                if off(&[7, 8, 9, 10, 11, 12]) {
                    return None;
                }
                p.rip_cp = RIP_CP[ix[4]];
                if !self.bite && ix[5] > 0 {
                    return None;
                }
                p.bite_cp = if self.bite { BITE_CP[ix[5]] } else { 0 };
                p.faerie_fire = ix[6] == 1;
            }
            "bear" => {
                if off(&[3, 4, 11, 12]) {
                    return None;
                }
                p.emergency = EMERGENCY[ix[5]];
                p.drag = false;
                p.mobs = ix[7] + 1;
                p.swipe_at = if self.swipe { SWIPE_AT[ix[8]] } else { 3 };
                if p.swipe_at > p.mobs && ix[8] > 0 {
                    return None;
                }
                p.roar = self.roar && ix[9] == 0;
                p.maul_spare = MAUL[ix[10]];
                p.faerie_fire = ix[6] == 1;
            }
            _ => {
                if off(&[4, 5, 6, 7, 8, 9, 10]) {
                    return None;
                }
                // the opener and Moonfire: Wrath, Starfire, Wrath + Moonfire, Starfire + Moonfire
                let k = ix[11];
                let (sf, mf) = match (self.starfire, k) {
                    (true, 0) => (true, false),
                    (true, 1) => (false, false),
                    (true, 2) => (true, true),
                    (true, _) => (false, true),
                    (false, 0) => (false, false),
                    (false, _) => (false, true),
                };
                p.starfire_opener = sf;
                p.moonfire = mf;
                p.melee_below = MELEE[ix[12]];
            }
        }
        Some(p)
    }

    pub fn indices(&self) -> Vec<Ix> {
        let sz = self.sizes();
        let mut out = vec![[0usize; 13]];
        // the defaults the descent moves from: heal below 60%
        out[0][1] = 1;
        for d in 0..13 {
            if DESCENT_ONLY.contains(&d) {
                continue;
            }
            out = out.into_iter().flat_map(|ix| (0..sz[d]).map(move |v| { let mut j = ix; j[d] = v; j })).collect();
        }
        // the bear's pull size is a choice of the full grid too
        let bears: Vec<Ix> = out
            .iter()
            .filter(|ix| self.forms[ix[0]] == "bear")
            .flat_map(|ix| (1..sz[7]).flat_map(move |m| (0..sz[8]).map(move |s| { let mut j = *ix; j[7] = m; j[8] = s; j })))
            .collect();
        out.extend(bears);
        out.retain(|ix| self.make(ix).is_some());
        out
    }

    /// The nearest index of a play.
    pub fn index_of(&self, p: &Play) -> Ix {
        let near = |xs: &[f64], x: f64| (0..xs.len()).min_by(|&a, &b| (xs[a] - x).abs().total_cmp(&(xs[b] - x).abs())).unwrap_or(0);
        let sz = self.sizes();
        let mut ix = [0usize; 13];
        ix[0] = self.forms.iter().position(|f| *f == p.form).unwrap_or(0);
        ix[1] = near(&HEAL_BELOW, p.heal_below);
        ix[2] = near(&HEAL_TO, p.heal_to);
        match p.form.as_str() {
            "cat" => {
                ix[3] = usize::from(!p.drag);
                ix[4] = RIP_CP.iter().position(|&x| x == p.rip_cp).unwrap_or(1);
                ix[5] = BITE_CP.iter().position(|&x| x == p.bite_cp).unwrap_or(0);
                ix[6] = usize::from(p.faerie_fire);
            }
            "bear" => {
                ix[6] = usize::from(p.faerie_fire);
                ix[7] = p.mobs.saturating_sub(1);
                ix[8] = SWIPE_AT.iter().position(|&x| x == p.swipe_at).unwrap_or(1);
                ix[9] = usize::from(!p.roar);
                ix[10] = near(&MAUL, p.maul_spare);
                ix[5] = near(&EMERGENCY, p.emergency);
            }
            _ => {
                ix[3] = usize::from(!p.drag);
                ix[11] = match (self.starfire, p.starfire_opener, p.moonfire) {
                    (true, true, false) => 0,
                    (true, false, false) => 1,
                    (true, true, true) => 2,
                    (true, false, true) => 3,
                    (false, _, false) => 0,
                    (false, _, true) => 1,
                };
                ix[12] = near(&MELEE, p.melee_below);
            }
        }
        for d in 0..13 {
            ix[d] = ix[d].min(sz[d] - 1);
        }
        ix
    }

    /// Every valid index one choice away (the form's own choices; a new
    /// form at its defaults).
    pub fn neighbours(&self, ix: &Ix) -> Vec<Ix> {
        let sz = self.sizes();
        let mut out = Vec::new();
        for d in 0..13 {
            for v in 0..sz[d] {
                if v != ix[d] {
                    let mut j = *ix;
                    j[d] = v;
                    if d == 0 {
                        // another form: its own choices from their first values
                        j = [v, ix[1], ix[2], 0, 1, 0, 0, if self.forms[v] == "bear" { 2 } else { 0 }, 1.min(sz[8] - 1), 0, 1, 0, 0];
                        if self.make(&j).is_none() {
                            j[7] = 0;
                            j[8] = 0;
                        }
                    }
                    if self.make(&j).is_some() && !out.contains(&j) {
                        out.push(j);
                    }
                }
            }
        }
        out
    }
}
