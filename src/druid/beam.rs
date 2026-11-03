//! The druid's talent route (2026-10-26): `crate::route`'s beam with
//! what the druid gives it, one kit at a time (feral or Balance gear; no
//! respec between them, `cross` prices that once). A level costs its kill
//! experience over the XP an hour, plus the walk between areas at the
//! fastest the druid moves out of combat.
//!
//! **Walking** (about half a normal levelling's time): at Classic's
//! run speed a level's walk takes as long as the warlock's measured route
//! spends killing it (`assumptions::WALK_REFERENCE`), the same distance for
//! both kits; faster in Cat Form with Feral Swiftness (from 20, the talent),
//! Travel Form (30, +40%) or a mount (40, +60%), whichever is fastest.

use super::farm::{session, xp_per_hour, Tally};
use super::launch::{self, Style};
use super::levels::{Data, Kit, QUICKNESS, TAB_NAMES};
use super::play::{Grid, Play, LACERATE_SPAM};
use super::scenario::Scenario;
use crate::assumptions as A;
use crate::route::{self, run, xp_on, BeamRoute, Class, ClimbRow, Step, DESCENT_SEEDS, SECONDS};
use crate::talents::{Build, Tree};
use rayon::prelude::*;

const TRAVEL_FORM_PASSIVE: i64 = 5419;
const TRAVEL_FORM_LEVEL: i32 = 30;
const CAT_FORM_LEVEL: i32 = 20;
/// plays kept per level from the grid or a descent
const LIBRARY: usize = 6;
/// grid plays kept per form
const PER_FORM: usize = 2;
/// the least gain a descent step takes (common seeds: below it, chance)
const MIN_GAIN: f64 = 0.003;
/// energy a held Ferocious Bite waits for (0: at once)
const BITE_POOLS: [f64; 3] = [0.0, 60.0, 85.0];

fn bite_only(p: &Play) -> bool {
    p.form == "cat" && p.bite_cp > 0 && p.rip_cp == 0
}

impl Data {
    fn c_has_swipe(&self, level: i32) -> bool {
        self.rank("Swipe", level, false).is_some()
    }

    /// Run speed out of combat, a multiplier: the fastest form the level and build have.
    pub fn walk_speed(&self, level: i32, b: &Build) -> f64 {
        let mut v: f64 = 1.0;
        if level >= CAT_FORM_LEVEL {
            v = v.max(1.0 + self.tree.value(&self.c, "Feral Swiftness", 1, b.get("Feral Swiftness").copied().unwrap_or(0)) / 100.0);
        }
        if level >= TRAVEL_FORM_LEVEL {
            v = v.max(1.0 + self.c.points(TRAVEL_FORM_PASSIVE, 0, None) / 100.0);
        }
        if level >= A::MOUNT_LEVEL {
            v = v.max(1.0 + A::MOUNT_SPEED / 100.0);
        }
        // Dash in Cat Form on the road, whenever it is ready, until the mount makes it not worth
        // getting off for
        if let Some(&(_, dash)) = launch::DASH.iter().rev().find(|d| d.0 <= level && level < A::MOUNT_LEVEL) {
            let cat = 1.0 + self.tree.value(&self.c, "Feral Swiftness", 1, b.get("Feral Swiftness").copied().unwrap_or(0)) / 100.0;
            let fast = cat * (1.0 + dash / 100.0);
            if fast > v {
                v = (fast * launch::DASH_SECONDS + v * (launch::DASH_COOLDOWN - launch::DASH_SECONDS)) / launch::DASH_COOLDOWN;
            }
        }
        // Quickness stacks with the rest (Blizzard's notes, 2026-10-16), added as the Python does
        if self.race == super::levels::Race::NightElf {
            v += self.c.points(QUICKNESS, 1, None) / 100.0;
        }
        v
    }

    /// Hours of walking on a level at the build's speed.
    pub fn walk_hours(&self, level: i32, b: &Build) -> f64 {
        A::xp_to_level(level) * A::KILL_XP_SHARE / A::WALK_REFERENCE[level as usize - 1] / self.walk_speed(level, b)
    }

    pub fn kill_hours(level: i32, xp: f64) -> f64 {
        A::xp_to_level(level) * A::KILL_XP_SHARE / xp.max(1.0)
    }

    fn grid(&self, sc: &Scenario, b: &Build) -> Grid {
        Grid::new(sc, self.kit == Kit::Balance, b)
    }

    /// The best few distinct plays of the level's grid for a build, kept
    /// per form: an untalented bear is weak, but the bear's plays must be in
    /// the library for the builds that make it strong (Primal Bite, Berserk).
    fn gridded(&self, level: i32, b: &Build) -> Vec<Play> {
        let sc = Class::scenario(self, level, b);
        let g = self.grid(&sc, b);
        // the style's plays only, from the start: its best may not be the grid's best
        let all: Vec<Play> = g.indices().iter().filter_map(|ix| g.make(ix)).filter(|p| Class::keeps(self, level, p)).collect();
        let mut out = Vec::new();
        for form in ["caster", "cat", "bear"] {
            let mut plays: Vec<Play> = all.iter().filter(|p| p.form == form).cloned().collect();
            if plays.is_empty() {
                continue;
            }
            for (seeds, keep) in [(4, 24), (12, 8), (48, PER_FORM * 2)] {
                let xs = run(self, level, std::slice::from_ref(b), std::slice::from_ref(&plays), seeds, SECONDS).remove(0);
                let mut o: Vec<usize> = (0..plays.len()).collect();
                o.sort_by(|&a, &b| xs[b].total_cmp(&xs[a]));
                // plays that score the same behave the same: keep one
                let mut seen = Vec::new();
                plays = o.into_iter().filter(|&i| { let k = xs[i].round() as i64; let new = !seen.contains(&k); seen.push(k); new }).take(keep).map(|i| plays[i].clone()).collect();
            }
            plays.truncate(PER_FORM);
            out.extend(plays);
        }
        // the Bite-only line stays too: it pays with pooling and Predatory Discipline
        // (`taught`), which the grid does not try
        let bites: Vec<Play> = all.iter().filter(|p| bite_only(p)).cloned().collect();
        if !bites.is_empty() && !out.iter().any(bite_only) {
            let xs = run(self, level, std::slice::from_ref(b), std::slice::from_ref(&bites), 12, SECONDS).remove(0);
            let i = (0..xs.len()).max_by(|&a, &b| xs[a].total_cmp(&xs[b])).unwrap();
            out.push(bites[i].clone());
        }
        out
    }
}

impl Class for Data {
    type Play = Play;
    type Scenario = Scenario;
    type Tally = Tally;

    fn tree(&self) -> &Tree {
        &self.tree
    }

    fn tab_names(&self) -> [&'static str; 3] {
        TAB_NAMES
    }

    fn files(&self) -> (String, String) {
        let race = if self.race == super::levels::Race::NightElf { "_nightelf" } else { "" };
        let race = format!("{race}{}", self.style.map_or(String::new(), |x| format!("_{}", x.name())));
        (format!("out/library_{}{race}.json", self.kit.name()), format!("out/values_{}{race}.json", self.kit.name()))
    }

    fn scenario(&self, level: i32, b: &Build) -> Scenario {
        Data::scenario(self, level, b)
    }

    fn session(&self, sc: &Scenario, p: &Play, seed: u64, seconds: f64) -> Tally {
        session(sc, p, seed, seconds)
    }

    fn xp_per_hour(&self, ts: &[Tally], _seconds: f64) -> f64 {
        xp_per_hour(ts)
    }

    fn level_plays(&self, level: i32, b: &Build, below: Option<&[Play]>) -> Vec<Play> {
        match below {
            Some(ps) if !self.new_at(level) => {
                let sc = Class::scenario(self, level, b);
                let seeds: Vec<u64> = (1..=DESCENT_SEEDS as u64).collect();
                let mut found: Vec<(Play, f64)> = ps.iter().flat_map(|p| Class::descend(self, &sc, level, b, p, &seeds, SECONDS)).collect();
                found.sort_by(|a, b| b.1.total_cmp(&a.1));
                let mut out: Vec<Play> = Vec::new();
                let mut seen: Vec<i64> = Vec::new();
                for (p, x) in found {
                    if !seen.contains(&(x.round() as i64)) && !out.contains(&p) {
                        seen.push(x.round() as i64);
                        out.push(p);
                    }
                }
                // the best few of each form
                let mut kept: Vec<Play> = Vec::new();
                for form in ["caster", "cat", "bear"] {
                    kept.extend(out.iter().filter(|p| p.form == form).take(PER_FORM).cloned());
                }
                kept.truncate(LIBRARY.max(3 * PER_FORM));
                if !kept.iter().any(bite_only) {
                    kept.extend(out.iter().find(|p| bite_only(p)).cloned());
                }
                kept.retain(|p| Class::keeps(self, level, p));
                if kept.is_empty() { self.gridded(level, b) } else { kept }
            }
            _ => self.gridded(level, b),
        }
    }

    /// Coordinate descent on the grid from a play: each round tries every
    /// play one choice away and moves to the best while it gains.
    fn descend(&self, sc: &Scenario, _level: i32, b: &Build, start: &Play, seeds: &[u64], seconds: f64) -> Vec<(Play, f64)> {
        let g = self.grid(sc, b);
        // the grid knows no talent abilities: every play keeps the start's
        let make = |ix: &_| g.make(ix).map(|p| Play { moonkin: start.moonkin && p.form == "caster", swarm: start.swarm && p.form == "caster", leyline_cd: start.leyline_cd && p.form == "caster" && !start.moonkin, shifting: start.shifting && p.form == "cat", bite_pool: if p.form == "cat" && p.bite_cp > 0 { start.bite_pool } else { 0.0 }, frenzied: if p.form == "bear" { start.frenzied } else { 0.0 }, opener: if p.form == "cat" { start.opener } else { 0 }, roots: start.roots && p.form == "caster", hurricane: if p.form == "caster" { start.hurricane } else { 0 }, barkskin: start.barkskin && p.form == "caster", lacerate: if p.form == "bear" { start.lacerate } else { 0 }, ..p });
        let mut cur = g.index_of(start);
        let xp = |ps: &[Play]| -> Vec<f64> { ps.par_iter().map(|p| xp_on(self, sc, p, seeds, seconds)).collect() };
        let mut best = xp(&[make(&cur).unwrap_or_else(|| start.clone())])[0];
        let mut runner = None;
        for _ in 0..8 {
            let ns = g.neighbours(&cur);
            if ns.is_empty() {
                break;
            }
            let ps: Vec<Play> = ns.iter().map(|j| make(j).unwrap()).collect();
            let xs = xp(&ps);
            let i = (0..xs.len()).max_by(|&a, &b| xs[a].total_cmp(&xs[b])).unwrap();
            if xs[i] <= best * (1.0 + MIN_GAIN) {
                break;
            }
            runner = make(&cur).map(|p| (p, best));
            cur = ns[i];
            best = xs[i];
        }
        let mut out = vec![(make(&cur).unwrap_or_else(|| start.clone()), best)];
        out.extend(runner);
        out
    }

    /// Insect Swarm kept up when talented; Moonkin Form both ways when talented.
    fn taught(&self, p: &Play, b: &Build, level: i32) -> Vec<Play> {
        let has = |n: &str| b.get(n).copied().unwrap_or(0) > 0;
        let known = |n: &str| self.rank(n, level, false).is_some();
        if p.form == "cat" {
            let q = Play { shifting: false, bite_pool: 0.0, opener: 0, ..p.clone() };
            let pools: &[f64] = if q.bite_cp > 0 && level >= launch::BITE_LEVEL { &BITE_POOLS } else { &[0.0] };
            let shifts: &[bool] = if has("Shifting Power") { &[false, true] } else { &[false] };
            let mut openers = vec![0u8];
            if known("Pounce") {
                openers.push(1);
            }
            if known("Ravage") {
                openers.push(2);
            }
            let mut out = Vec::new();
            for &bite_pool in pools {
                for &shifting in shifts {
                    for &opener in &openers {
                        out.push(Play { bite_pool, shifting, opener, ..q.clone() });
                    }
                }
            }
            return out;
        }
        if p.form == "bear" {
            let q = Play { lacerate: 0, frenzied: 0.0, ..p.clone() };
            let lacs: &[u32] = if has("Lacerate") { &[0, 1, 5, LACERATE_SPAM] } else { &[0] };
            let frs: &[f64] = if level >= 36 { &[0.0, 0.5] } else { &[0.0] };
            let mut out = Vec::new();
            for &lacerate in lacs {
                for &frenzied in frs {
                    out.push(Play { lacerate, frenzied, ..q.clone() });
                }
            }
            return out;
        }
        let q = Play { swarm: has("Insect Swarm"), moonkin: false, leyline_cd: false, roots: false, hurricane: 0, barkskin: false, ..p.clone() };
        // the Hurricane playstyle fights with it once it has it
        let storm_only = self.style == Some(Style::Hurricane) && known("Hurricane");
        let mut out = if storm_only { Vec::new() } else { vec![q.clone()] };
        if !storm_only {
            if self.race == super::levels::Race::Skyborne {
                out.push(Play { leyline_cd: true, ..q.clone() });
            }
            if known("Entangling Roots") {
                out.push(Play { roots: true, ..q.clone() });
            }
            if has("Moonkin Form") {
                out.push(Play { moonkin: true, ..q.clone() });
                if known("Entangling Roots") {
                    out.push(Play { moonkin: true, roots: true, ..q.clone() });
                }
            }
        }
        if known("Hurricane") && self.kit == Kit::Balance && self.style != Some(Style::MoonkinSingle) {
            for n in [2, 3] {
                let barks: &[bool] = if known("Barkskin") { &[false, true] } else { &[false] };
                for &barkskin in barks {
                    out.push(Play { hurricane: n, barkskin, ..q.clone() });
                    if has("Moonkin Form") {
                        out.push(Play { hurricane: n, barkskin, moonkin: true, ..q.clone() });
                    }
                }
            }
        }
        out
    }

    fn same(&self, a: &Play, b: &Play) -> bool {
        a == b
    }

    /// The feral kit's sections: any play, the cat's, the bear's (
    /// 2026-10-26: the bear's line, Primal Bite to Berserk, must live on
    /// beside the cat's until its talents pay); Balance plays one form.
    fn sections(&self) -> Vec<u8> {
        if self.style.is_some() {
            return vec![0];
        }
        match self.kit {
            Kit::Feral => vec![1, 2],
            Kit::Balance => vec![0],
        }
    }

    fn keeps(&self, level: i32, p: &Play) -> bool {
        // a mob at a time until there is Swipe to fight several with
        if !self.c_has_swipe(level) && p.mobs > 1 {
            return false;
        }
        match self.style {
            Some(Style::BearAoe) => p.form == "bear" && (p.mobs >= 2 || !self.c_has_swipe(level)) || (level < 10 && p.form == "caster"),
            Some(Style::Cat) => p.form == "cat" || level < CAT_FORM_LEVEL,
            _ => self.kit == Kit::Balance || p.form != "caster" || level < CAT_FORM_LEVEL,
        }
    }

    fn sim_version(&self) -> u64 {
        sim_version()
    }

    /// 2026-10-31: the whole run in minutes, no respec (README *The launch druid*).
    /// (The feral kit's plays and tree splits made 577 routes at 45 and five
    /// minutes not enough: it keeps its seeded orders and the beam instead.)
    fn niche(&self) -> usize {
        match self.kit {
            Kit::Feral => 0,
            Kit::Balance => 1,
        }
    }

    fn rounds(&self) -> usize {
        2
    }

    /// One respec for either kit (2026-10-31: Balance into Moonkin Form; feral
    /// once the bear's talents are dead weight).
    fn respec(&self) -> bool {
        true
    }

    /// Balance when Moonkin Form can be had (31 points), feral once the cat has taken over.
    fn respec_from(&self) -> i32 {
        match self.kit {
            Kit::Feral => CAT_FORM_LEVEL,
            Kit::Balance => 40,
        }
    }

    fn admits(&self, k: u8, p: &Play) -> bool {
        match k {
            1 => p.form == "cat",
            2 => p.form == "bear",
            _ => true,
        }
    }

    fn hours(&self, level: i32, b: &Build, xp: f64) -> f64 {
        Data::kill_hours(level, xp) + self.walk_hours(level, b)
    }
}

/// The druid's sim's sources: stored values hold while these do.
pub fn sim_version() -> u64 {
    route::version_of(&[
        include_str!("farm.rs"),
        include_str!("levels.rs"),
        include_str!("launch.rs"),
        include_str!("scenario.rs"),
        include_str!("../assumptions.rs"),
        include_str!("../items.rs"),
        include_str!("../talents.rs"),
        include_str!("../client.rs"),
    ])
}

/// A route's levels: XP an hour, hours, the points new at the level, the play.
pub fn show_steps(d: &Data, steps: &[Step<Play>]) {
    let mut prev = Build::new();
    for s in steps {
        let new: Vec<String> = s.build.iter().filter(|(k, v)| prev.get(*k) != Some(v)).map(|(k, v)| format!("{k} {v}")).collect();
        let gone: Vec<String> = prev.keys().filter(|k| !s.build.contains_key(*k)).map(|k| format!("-{k}")).collect();
        let sc = d.scenario(s.level, &s.build);
        println!("  {:2}: {:6.0} XP/h {:5.2} h  {:34}  {}", s.level, s.xp_per_hour, s.hours, [new, gone].concat().join(", "), s.play.words(&sc));
        prev = s.build.clone();
    }
}

fn kit(s: &str) -> Kit {
    match s {
        "feral" => Kit::Feral,
        "balance" => Kit::Balance,
        _ => panic!("a kit: feral or balance"),
    }
}

fn tabs(s: &str) -> Vec<usize> {
    s.split(',').map(|n| TAB_NAMES.iter().position(|t| *t == n).unwrap_or_else(|| panic!("a tree: {TAB_NAMES:?}"))).collect()
}

pub fn cli(args: &[String]) {
    match args.first().map(String::as_str) {
        Some("book") => {
            for (lv, name, ranks) in Data::load(Kit::Feral).book() {
                println!("{lv:3} {name} {ranks:?}");
            }
        }
        Some("gear") => {
            // forever druid-level gear <kit>
            let d = Data::load(kit(&args[1]));
            for lv in (1..=60).filter(|l| l % 5 == 0 || *l < 3 || [29, 31].contains(l)) {
                let g = d.gear(lv);
                let st = |k: i64| g.stats.get(&k).copied().unwrap_or(0.0);
                println!(
                    "{lv:2}: str {:5.1} agi {:5.1} sta {:5.1} ap {:5.1} int {:5.1} spi {:5.1} sp {:5.1} heal {:5.1} mp5 {:4.1} armor {:5.0} weapon {:5.1} dps {:.1} s {:?}",
                    st(4), st(3), st(7), st(38), st(5), st(6), st(45) + st(42), st(41), st(43), g.armor, g.weapon_dps, g.weapon_speed, g.items
                );
            }
        }
        Some("scenario") => {
            // forever druid-level scenario <kit> <level> ['<build JSON>']
            let d = Data::load(kit(&args[1]));
            let level: i32 = args[2].parse().expect("a level");
            let b: Build = args.get(3).map(|j| serde_json::from_str(j).expect("a build")).unwrap_or_default();
            println!("{}", serde_json::to_string_pretty(&d.scenario(level, &b)).unwrap());
        }
        Some("grid") => {
            // forever druid-level grid <kit> <level> ['<build JSON>']: the grid's best plays
            let d = Data::load(kit(&args[1]));
            let level: i32 = args[2].parse().expect("a level");
            let b: Build = args.get(3).map(|j| serde_json::from_str(j).expect("a build")).unwrap_or_default();
            let t = std::time::Instant::now();
            let sc = d.scenario(level, &b);
            let g = Grid::new(&sc, d.kit == Kit::Balance, &b);
            let n = g.indices().len();
            let plays = d.gridded(level, &b);
            let fresh: Vec<u64> = route::FRESH.collect();
            println!("level {level} ({}), {n} plays, {:.1} s", d.kit.name(), t.elapsed().as_secs_f64());
            for p in &plays {
                let ts: Vec<Tally> = fresh.par_iter().map(|&s| session(&sc, p, s, SECONDS)).collect();
                let deaths: u32 = ts.iter().map(|t| t.deaths).sum();
                let secs: f64 = ts.iter().map(|t| t.seconds).sum();
                let share = |f: &dyn Fn(&Tally) -> f64| ts.iter().map(f).sum::<f64>() / secs * 100.0;
                println!(
                    "  {:6.0} XP/h  deaths {deaths:2}  heal {:4.1}%  rest {:4.1}%  walk {:4.1}%  {}",
                    xp_per_hour(&ts), share(&|t| t.healing), share(&|t| t.resting), share(&|t| t.walking), p.words(&sc)
                );
            }
        }
        Some("beam") => {
            // forever druid-level beam <kit> <trees> <width> <out.json> [top level, 60] [seeds.json: talent orders from 10]
            let d = Data::load(kit(&args[1]));
            let width: usize = args[3].parse().expect("a width");
            let top: i32 = args.get(5).map(|x| x.parse().unwrap()).unwrap_or(60);
            let seeds: Vec<Vec<String>> = args.get(6).map_or(Vec::new(), |f| serde_json::from_str(&std::fs::read_to_string(f).expect("a seeds file")).expect("a list of talent orders"));
            let r: BeamRoute<Play> = route::beam(&d, &tabs(&args[2]), width, top, &seeds);
            std::fs::write(&args[4], serde_json::to_string_pretty(&r).unwrap()).expect("the output file");
            let early: f64 = r.early.iter().map(|s| s.hours).sum();
            println!("{} {} beam {}: levels 1-9 {:.2} h", d.kit.name(), r.trees.join("+"), r.width, early);
            show_steps(&d, &r.early);
            println!("best: {:.2} h from 10 ({:.2} h from 1){}", r.hours, early + r.hours, r.respec_at.map_or(String::new(), |a| format!(", one respec at {a}")));
            show_steps(&d, &r.path);
            if r.respec_at.is_some() {
                println!("without a respec: {:.2} h from 10 ({:.2} h from 1)", r.plain_hours, early + r.plain_hours);
                show_steps(&d, &r.plain);
            }
            println!("hours from 10 by the respec's level (none: {:.2}):", r.plain_hours);
            for c in r.by_respec.chunks(10) {
                println!("  {}", c.iter().map(|(l, h)| format!("{l}: {h:.2}")).collect::<Vec<_>>().join("  "));
            }
        }
        Some("climb") => {
            // forever druid-level climb <kit> <beam.json> <out.json> [plain|path]
            let d = Data::load(kit(&args[1]));
            let r: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&args[2]).expect("a beam file")).expect("a beam");
            let which = args.get(4).map_or("plain", |s| s.as_str());
            // `bear` or `cat`: that form's best route, valued by its plays
            let section = match which { "cat" => Some(1u8), "bear" => Some(2u8), _ => None };
            let steps: Vec<Step<Play>> = match section {
                Some(k) => serde_json::from_value(r["by_section"].as_array().unwrap().iter().find(|e| e[0] == k).expect("a section")[2].clone()).expect("a route"),
                None => serde_json::from_value(r[which].clone()).expect("a route"),
            };
            let early: Vec<Step<Play>> = serde_json::from_value(r["early"].clone()).expect("levels 1-9");
            let rows: Vec<ClimbRow<Play>> = route::climb(&d, &steps, &early, section);
            std::fs::write(&args[3], serde_json::to_string_pretty(&rows).unwrap()).expect("the output file");
            for c in &rows {
                println!(
                    "{:2}: {:6.0} XP/h  level {:+5.1}%  point {:+5.1}%  {:28}  play on fresh seeds {:6.0}, a descent {:+4.1}%",
                    c.level, c.xp, (c.before_build / c.before_level - 1.0) * 100.0, (c.xp / c.before_build - 1.0) * 100.0, c.new.join(", "), c.fresh.0, (c.fresh.1 / c.fresh.0 - 1.0) * 100.0
                );
            }
        }
        Some("tree") => {
            // forever druid-level tree: the launch tree as JSON, for the page
            let d = Data::load(Kit::Feral);
            let ts: Vec<serde_json::Value> = d.tree.talents.iter().map(|t| serde_json::json!({"name": t.name, "spell": t.spell_id, "max": t.max_rank, "tab": t.tab(), "row": t.row(), "col": t.col()})).collect();
            let mut arrows: Vec<(String, String)> = d.tree.parents.iter().flat_map(|(&b, ps)| ps.iter().map(move |&a| (a, b))).map(|(a, b)| (d.tree.talents[a].name.clone(), d.tree.talents[b].name.clone())).collect();
            arrows.sort();
            println!("{}", serde_json::json!({"talents": ts, "arrows": arrows}));
        }
        Some("audit") => {
            // forever druid-level audit <kit> <beam.json> <out.json>: each untaken talent added to the
            // route's build at 40, 50 and 60, its rows and arrows aside (what a one-point talent would give)
            let d = Data::load(kit(&args[1]));
            let r: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&args[2]).expect("a beam file")).expect("a beam");
            let form = args.get(4).map(String::as_str);
            let steps: Vec<Step<Play>> = match form {
                Some("bear") => serde_json::from_value(r["by_section"].as_array().unwrap().iter().find(|e| e[0] == 2).expect("a section")[2].clone()).expect("a route"),
                _ => serde_json::from_value(r["path"].clone()).expect("a route"),
            };
            let (lib_file, _) = Class::files(&d);
            let lib: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(lib_file).expect("a library")).expect("a library");
            let mut out = Vec::new();
            for s in steps.iter().filter(|s| [40, 50, 60].contains(&s.level)) {
                let plays: Vec<Play> = lib["lib"].as_array().unwrap().iter().find(|e| e[0] == s.level).map(|e| serde_json::from_value::<Vec<Play>>(e[1].clone()).unwrap()).unwrap().into_iter().filter(|p| form.is_none_or(|f| p.form == f)).collect();
                let (base, _) = route::value(&d, s.level, &s.build, &plays);
                for t in d.tree.talents.iter().filter(|t| t.max_rank == 1 && !s.build.contains_key(&t.name)) {
                    let mut b = s.build.clone();
                    b.insert(t.name.clone(), 1);
                    let (x, p) = route::value(&d, s.level, &b, &plays);
                    println!("{} +{:28} {:+5.1}%  {}", s.level, t.name, (x / base - 1.0) * 100.0, p.words(&d.scenario(s.level, &b)));
                    out.push(serde_json::json!({"level": s.level, "talent": t.name, "gain": x / base - 1.0, "form": p.form}));
                }
            }
            std::fs::write(&args[3], serde_json::to_string(&out).unwrap()).expect("the output file");
        }
        Some("value") => {
            // forever druid-level value <kit> <level> '<build JSON>'...: each build by the level's library plays
            let d = Data::load(kit(&args[1]));
            let level: i32 = args[2].parse().expect("a level");
            let (lib_file, _) = Class::files(&d);
            let lib: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(lib_file).expect("a library")).expect("a library");
            let plays: Vec<Play> = lib["lib"].as_array().unwrap().iter().find(|e| e[0] == level).map(|e| serde_json::from_value(e[1].clone()).unwrap()).unwrap();
            for j in &args[3..] {
                let b: Build = serde_json::from_str(j).expect("a build");
                let (x, p) = route::value(&d, level, &b, &plays);
                let fresh: Vec<u64> = route::FRESH.collect();
                println!("{x:6.0} on 24 seeds, {:6.0} on fresh seeds  legal {}  {}", xp_on(&d, &d.scenario(level, &b), &p, &fresh, SECONDS), d.tree.legal(&b), p.words(&d.scenario(level, &b)));
            }
        }
        Some("bark") => {
            // forever druid-level bark <beam.json>: Balance's route from 44, 3 mobs under Hurricane without and with Barkskin, fresh seeds
            let d = Data::load(Kit::Balance);
            let r: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&args[1]).expect("a beam file")).expect("a beam");
            let steps: Vec<Step<Play>> = serde_json::from_value(r["path"].clone()).expect("a route");
            let (lib_file, _) = Class::files(&d);
            let lib: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(lib_file).expect("a library")).expect("a library");
            let fresh: Vec<u64> = route::FRESH.collect();
            let fine: Vec<u64> = (1..=route::FINE_SEEDS as u64).collect();
            for s in steps.iter().filter(|s| s.level >= 44 && s.level < 60) {
                let plays: Vec<Play> = lib["lib"].as_array().unwrap().iter().find(|e| e[0] == s.level).map(|e| serde_json::from_value(e[1].clone()).unwrap()).unwrap();
                let sc = d.scenario(s.level, &s.build);
                let vars: Vec<Play> = plays.iter().flat_map(|p| Class::taught(&d, p, &s.build, s.level)).filter(|p| p.moonkin && p.hurricane == 3).collect();
                let mut line = format!("{}", s.level);
                for bark in [false, true] {
                    let best = vars.iter().filter(|p| p.barkskin == bark).map(|p| (xp_on(&d, &sc, p, &fine, SECONDS), p)).max_by(|a, b| a.0.total_cmp(&b.0));
                    if let Some((_, p)) = best {
                        let ts: Vec<Tally> = fresh.par_iter().map(|&x| session(&sc, p, x, SECONDS)).collect();
                        line += &format!(" {:.0} {}", xp_per_hour(&ts), ts.iter().map(|t| t.deaths).sum::<u32>());
                    }
                }
                println!("{line}");
            }
        }
        Some("price") => {
            // forever druid-level price <kit> <beam.json> <climb.json>: a route's talents and plays held, 96 seeds
            // a level, in this process's race (`RACE`, `NO_LEYLINE`, `NO_ELUNE`): level, XP an hour, hours, deaths
            let d = Data::load(kit(&args[1]));
            let r: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&args[2]).expect("a beam")).expect("a beam");
            let rows: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&args[3]).expect("a climb")).expect("a climb");
            let seeds: Vec<u64> = (2001..=2096).collect();
            let mut steps: Vec<(i32, Build, Play)> = Vec::new();
            for e in r["early"].as_array().unwrap() {
                steps.push((e["level"].as_i64().unwrap() as i32, Build::new(), serde_json::from_value(e["play"].clone()).unwrap()));
            }
            for x in rows.as_array().unwrap() {
                steps.push((x["level"].as_i64().unwrap() as i32, serde_json::from_value(x["build"].clone()).unwrap(), serde_json::from_value(x["play"].clone()).unwrap()));
            }
            for (level, b, p) in steps.iter().filter(|s| s.0 < 60) {
                let sc = d.scenario(*level, b);
                let ts: Vec<Tally> = seeds.par_iter().map(|&x| session(&sc, p, x, SECONDS)).collect();
                let xp = xp_per_hour(&ts);
                println!("{level} {xp:.0} {:.4} {}", Class::hours(&d, *level, b, xp), ts.iter().map(|t| t.deaths).sum::<u32>());
            }
        }
        Some("hand") => {
            // forever druid-level hand <route> [out.json] (with STYLE the first leg's): the route's levels
            let r = launch::ROUTES.iter().find(|r| r.name == args[1]).unwrap_or_else(|| panic!("a route, not {}", args[1]));
            let d = Data::load(if matches!(r.legs[0].1, Style::BearAoe | Style::Cat | Style::Feral) { Kit::Feral } else { Kit::Balance });
            let w = super::style::walk(&d, r);
            for (s, x) in w.path.iter().zip(&w.rows) {
                println!("{}: {:.0} XP/h, {:.2} h, {:.1} deaths a day, {}", s.level, s.xp_per_hour, s.hours, x.deaths_a_day, s.play.form);
            }
            println!("{}: {:.2} h, respec {:?}", r.name, w.early_hours + w.hours, w.respec_at);
            if let Some(f) = args.get(2) {
                std::fs::write(f, serde_json::to_string(&w).unwrap()).expect("the output file");
            }
        }
        Some("understand") => {
            // forever druid-level understand <style>: the hand order, where its time goes and what each talent adds
            let st = Style::from_name(&args[1]);
            let d = Data::load(if matches!(st, Style::BearAoe | Style::Cat | Style::Feral) { Kit::Feral } else { Kit::Balance });
            let s = super::peak::Peak::new(&d, st);
            let order = if st == Style::Hurricane { st.after_respec() } else { st.order() };
            let seeds: Vec<u64> = route::FRESH.collect();
            for level in (10..=60).filter(|l| l % 5 == 0 || *l == 59) {
                let b: Build = order[..(level - 9) as usize].iter().fold(Build::new(), |mut b, n| { *b.entry(n.to_string()).or_insert(0) += 1; b });
                let x = s.step(level, &b);
                let sc = d.scenario(level, &b);
                let ts: Vec<Tally> = seeds.iter().map(|&k| session(&sc, &x.play, k, route::SECONDS)).collect();
                let sum = |f: &dyn Fn(&Tally) -> f64| ts.iter().map(f).sum::<f64>();
                let (all, kills) = (sum(&|t| t.seconds), sum(&|t| t.kills as f64));
                let (heal, rest, walk) = (sum(&|t| t.healing), sum(&|t| t.resting), sum(&|t| t.walking));
                let dead = sum(&|t| t.deaths as f64) * 120.0;
                println!(
                    "L{level} {} mobs {} {:.0} XP/h {:.2} h/level | {:.1} s/kill {:.0} XP/kill | fight {:.0}% walk {:.0}% heal {:.0}% rest {:.0}% dead {:.0}%",
                    x.play.form, x.play.mobs, x.xp_per_hour, x.hours, all / kills, sum(&|t| t.xp) / kills,
                    (all - heal - rest - walk - dead) / all * 100.0, walk / all * 100.0, heal / all * 100.0, rest / all * 100.0, dead / all * 100.0,
                );
                let base = x.xp_per_hour;
                let mut worth: Vec<(f64, String, u32)> = d.tree.talents.par_iter().map(|t| {
                    let have = b.get(&t.name).copied().unwrap_or(0);
                    let mut o = b.clone();
                    if have > 0 { o.remove(&t.name); } else { o.insert(t.name.clone(), t.max_rank); }
                    let v = s.xp(level, &o).0;
                    let per = if have > 0 { (base / v).ln() / have as f64 } else { (v / base).ln() / t.max_rank as f64 };
                    (per * 100.0, t.name.clone(), have)
                }).collect();
                worth.sort_by(|a, c| c.0.total_cmp(&a.0));
                let row: Vec<String> = worth.iter().filter(|w| w.0.abs() >= 0.05).map(|w| format!("{}{} {:+.1}", w.1, if w.2 > 0 { format!("[{}]", w.2) } else { String::new() }, w.0)).collect();
                println!("   per point: {}", row.join(", "));
            }
        }
        Some("deaths") => {
            // forever druid-level deaths <kit> <climb.json>: each level's play on 48 fresh seeds, deaths counted
            let d = Data::load(kit(&args[1]));
            let rows: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&args[2]).expect("a climb")).expect("a climb");
            let fresh: Vec<u64> = route::FRESH.collect();
            for r in rows.as_array().unwrap() {
                let level = r["level"].as_i64().unwrap() as i32;
                let b: Build = serde_json::from_value(r["build"].clone()).unwrap();
                let p: Play = serde_json::from_value(r["play"].clone()).unwrap();
                let sc = d.scenario(level, &b);
                let ts: Vec<Tally> = fresh.par_iter().map(|&x| session(&sc, &p, x, SECONDS)).collect();
                println!("{level} {} {} {:.0} {:.0} {}", ts.iter().map(|t| t.deaths).sum::<u32>(), ts.iter().filter(|t| t.deaths > 0).count(), r["xp"].as_f64().unwrap(), xp_per_hour(&ts), p.hurricane);
            }
        }
        Some("walk") => {
            // forever druid-level walk: the walk's hours by level, base speed and the fastest form
            let d = Data::load(Kit::Feral);
            let fs = Build::from([("Feral Swiftness".to_string(), 2u32)]);
            let mut total = (0.0, 0.0, 0.0);
            for lv in 1..60 {
                let base = d.walk_hours(lv, &Build::new()) * d.walk_speed(lv, &Build::new());
                let (a, b) = (d.walk_hours(lv, &Build::new()), d.walk_hours(lv, &fs));
                total = (total.0 + base, total.1 + a, total.2 + b);
                println!("{lv:2}: {:.2} h at a walk, {:.2} h without Feral Swiftness, {:.2} h with it", base, a, b);
            }
            println!("1-59: {:.1} h at a walk, {:.1} h without Feral Swiftness, {:.1} h with it", total.0, total.1, total.2);
        }
        Some("adopt") => {
            // forever druid-level adopt <values.json>: stored values written by an earlier
            // binary of the same sim sources (a change to the search only) are kept
            let path = &args[1];
            let mut v: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(path).expect("a values file")).expect("values");
            v["sim"] = serde_json::json!(sim_version());
            std::fs::write(path, serde_json::to_string(&v).unwrap()).expect("the values file");
            println!("{path}: now sim {}", sim_version());
        }
        Some("path") => {
            // forever druid-level path <kit> '<JSON list: a talent a point, from 10>' [out.json]: the order valued
            let d = Data::load(kit(&args[1]));
            let order: Vec<String> = serde_json::from_str(&args[2]).expect("a list of talents");
            let steps = route::path(&d, &order, 60);
            println!("{:.2} h from 10", steps.iter().map(|s| s.hours).sum::<f64>());
            show_steps(&d, &steps);
            if let Some(out) = args.get(3) {
                std::fs::write(out, serde_json::to_string_pretty(&steps).unwrap()).expect("the output file");
            }
        }
        Some("plays") => {
            // forever druid-level plays <style> <level> '<build JSON>': every rotation the style tries, XP an hour each (48 seeds)
            let style = launch::Style::from_name(&args[1]);
            let d = Data::load(if matches!(style, Style::BearAoe | Style::Cat | Style::Feral) { Kit::Feral } else { Kit::Balance });
            let level: i32 = args[2].parse().expect("a level");
            let b: Build = serde_json::from_str(&args[3]).expect("a build");
            let sc = d.scenario(level, &b);
            for p in super::peak::rotations(&d, style, level, &b) {
                let ts: Vec<Tally> = (1..=48u64).into_par_iter().map(|s| session(&sc, &p, s, SECONDS)).collect();
                println!("  {:6.0} XP/h  {}", xp_per_hour(&ts), p.words(&sc));
            }
        }
        Some("forms") => {
            // forever druid-level forms <kit> <level> '<build JSON>': the cat and the bear
            // (1-4 mobs) at their best grid play each, on fresh seeds
            let d = Data::load(kit(&args[1]));
            let level: i32 = args[2].parse().expect("a level");
            let b: Build = serde_json::from_str(&args[3]).expect("a build");
            let sc = d.scenario(level, &b);
            let g = Grid::new(&sc, false, &b);
            let fresh: Vec<u64> = route::FRESH.collect();
            let all: Vec<Play> = g.indices().iter().filter_map(|ix| g.make(ix)).collect();
            for (form, mobs, em) in [("cat", 1, 0.0), ("bear", 1, 0.0), ("bear", 2, 0.0), ("bear", 3, 0.0), ("bear", 3, 0.3), ("bear", 3, 0.4), ("bear", 4, 0.0), ("bear", 4, 0.4)] {
                let ps: Vec<Play> = all.iter().filter(|p| p.form == form && p.mobs == mobs && p.emergency == em).cloned().collect();
                let best = ps.par_iter().map(|p| (xp_on(&d, &sc, p, &fresh[..12], SECONDS), p)).max_by(|a, b| a.0.total_cmp(&b.0));
                let Some((_, p)) = best else { continue };
                let ts: Vec<Tally> = fresh.par_iter().map(|&s| session(&sc, p, s, SECONDS)).collect();
                let secs: f64 = ts.iter().map(|t| t.seconds).sum();
                let share = |f: &dyn Fn(&Tally) -> f64| ts.iter().map(f).sum::<f64>() / secs * 100.0;
                let alive: Vec<Tally> = ts.iter().filter(|t| t.deaths == 0).cloned().collect();
                println!("  {form} {mobs}: {:6.0} XP/h (alive sessions {:6.0}), deaths {:2}, heal {:4.1}%, rest {:4.1}%, taken {:5.1}/s  {}", xp_per_hour(&ts), xp_per_hour(&alive),
                    ts.iter().map(|t| t.deaths).sum::<u32>(), share(&|t| t.healing), share(&|t| t.resting), ts.iter().map(|t| t.taken).sum::<f64>() / secs, p.words(&sc));
            }
            println!("  bear health {:.0} armor {:.0} ap {:.0} dodge {:.1} | cat ap {:.0} | mob {:.0} hp {:.1} dps", sc.bear.health, sc.bear.armor, sc.bear.attack_power, sc.bear.dodge, sc.cat.attack_power, sc.mob.health, sc.mob.dps);
        }
        _ => eprintln!("usage: forever druid-level book | gear <kit> | scenario <kit> <level> [build] | grid <kit> <level> [build] | beam <kit> <trees> <width> <out> [top] | climb <kit> <beam> <out> [plain|path] | walk"),
    }
}

/// XP an hour of a play on a scenario, 24 seeds of the route's sessions.
pub fn xp_of(sc: &Scenario, p: &Play) -> f64 {
    let ts: Vec<Tally> = (1..=route::FINE_SEEDS as u64).into_par_iter().map(|s| session(sc, p, s, SECONDS)).collect();
    xp_per_hour(&ts)
}
