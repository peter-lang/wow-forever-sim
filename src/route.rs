//! The fastest talent route from level 10 to a top level, any class (the
//! warlock's first, README *The levelling route method*). A class supplies
//! its scenario at a level and build, a session of its sim, its plays and
//! their refinement (`Class`); the search is the same for every class.
//!
//! - **Talent states** (`Talents`): a point a level from 10; a talent begun
//!   is filled before the next, unless the point opened a row; rows
//!   open at 5 points in their own tab, an arrowed parent must be full.
//! - **The beam** keeps the `width` fastest routes so far, and besides them
//!   the `NICHE` fastest of every tree split (points in each tree
//!   2026-10-26) and section (the druid's form, valued by that form's plays):
//!   a route that pays only later is not crowded out by its neighbours of
//!   the moment. Routes whose builds the sim cannot tell apart
//!   (inert talents) share a place. The earlier rounds' best routes keep
//!   theirs (elitism: without it a round's winner fell out mid-route the next
//!   round, 2026-10-26).
//! - **Two-stage values.** Every child is screened (each play, 2 seeds);
//!   only those that could take a place are measured exactly (the best two
//!   plays, 24 seeds).
//! - **One respec, afterwards.** A respec at R is the best plain route to
//!   R-1 followed by the best plain continuation from any measured build
//!   of R: a backward pass over the measured builds gives it for every R.
//! - Then the plays are refined on the route's builds (a descent from each
//!   one's play) and the search runs again, until the route holds. Values
//!   are kept by scenario across rounds and runs (only new plays are run),
//!   the plays across runs.

use crate::talents::{Build, Tree, ROW_GATE};
use rayon::prelude::*;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::time::Instant;

pub const FIRST_TALENT: i32 = 10;
pub const SECONDS: f64 = 1800.0;
pub const FINE_SEEDS: usize = 24;
/// a descent's seeds: steps of 0.3% must stand out of their noise
pub const DESCENT_SEEDS: usize = 48;
/// rounds of re-searching after the plays are refined (the plays still gained in round 5 at 50, 2026-10-26)
const ROUNDS: usize = 8;
/// routes kept per tree split, besides the `width` fastest
pub const NICHE: usize = 3;
/// children measured exactly: this many times the places they compete for
const SCREEN: usize = 2;
/// a screen's seeds (the first of the exact ones)
pub const SCREEN_SEEDS: usize = 2;
/// plays per build measured exactly: the screen's best
const KEEP: usize = 2;
/// fresh seeds for checking a play (the search never saw them)
pub const FRESH: std::ops::RangeInclusive<u64> = 1001..=1048;

/// A class's levelling: what the route search needs of it.
pub trait Class: Sync {
    type Play: Clone + Serialize + DeserializeOwned + Send + Sync;
    type Scenario: Serialize + Send + Sync;
    type Tally: Send;
    fn tree(&self) -> &Tree;
    fn tab_names(&self) -> [&'static str; 3];
    /// where the plays and the values are kept between runs
    fn files(&self) -> (String, String);
    fn scenario(&self, level: i32, b: &Build) -> Self::Scenario;
    fn session(&self, sc: &Self::Scenario, p: &Self::Play, seed: u64, seconds: f64) -> Self::Tally;
    /// XP an hour of a play's sessions (a death earns nothing)
    fn xp_per_hour(&self, ts: &[Self::Tally], seconds: f64) -> f64;
    /// A level's best few plays for a build: the grid where the level brings
    /// something new, else a descent from the level below's.
    fn level_plays(&self, level: i32, b: &Build, below: Option<&[Self::Play]>) -> Vec<Self::Play>;
    /// The plays a descent settles on from `start`, best first, with their XP an hour.
    fn descend(&self, sc: &Self::Scenario, level: i32, b: &Build, start: &Self::Play, seeds: &[u64], seconds: f64) -> Vec<(Self::Play, f64)>;
    /// A library play taught the build's talent abilities (variants).
    fn taught(&self, p: &Self::Play, b: &Build, level: i32) -> Vec<Self::Play>;
    fn same(&self, a: &Self::Play, b: &Self::Play) -> bool;
    /// The beam's sections (2026-10-26: as the warlock kept its
    /// trees, the druid keeps its forms): 0 is any play; a route in section
    /// k is valued by its section's plays only (any, while the level has
    /// none of them), so a form's line lives on until its talents pay.
    fn sections(&self) -> Vec<u8> {
        vec![0]
    }
    /// Whether section `k` plays `p`.
    fn admits(&self, _k: u8, _p: &Self::Play) -> bool {
        true
    }
    /// A section's group (the druid's kit): a route stays in its group but
    /// at its one respec, and its hours are its group's best plays'.
    fn group(&self, _k: u8) -> u8 {
        0
    }
    fn in_group(&self, _g: u8, _p: &Self::Play) -> bool {
        true
    }
    /// Whether the library keeps a play at a level (the druid's feral kit
    /// drops its caster plays once it has Cat Form: they never win).
    fn keeps(&self, _level: i32, _p: &Self::Play) -> bool {
        true
    }
    /// The sim's version: stored values are valid while it holds. By
    /// default this binary (any rebuild drops them); a class may name its
    /// sim's sources instead, so a change to the search keeps them.
    fn sim_version(&self) -> u64 {
        std::env::current_exe().ok().and_then(|p| std::fs::read(p).ok()).map_or(0, |b| hash_of(&b[..]))
    }
    /// Routes kept per tree split besides the `width` fastest, rounds of
    /// re-searching, and whether a respec is looked for: what a class trades
    /// for a search of minutes.
    fn niche(&self) -> usize {
        NICHE
    }
    fn rounds(&self) -> usize {
        ROUNDS
    }
    fn respec(&self) -> bool {
        true
    }
    /// Whether a respec may land on this build (the druid: Balance respecs for Moonkin Form).
    fn respec_into(&self, _b: &Build) -> bool {
        true
    }
    /// The earliest level a respec is looked for.
    fn respec_from(&self) -> i32 {
        FIRST_TALENT + 1
    }
    /// Hours a level takes: its kill experience over the XP an hour, and
    /// whatever else the level costs the build (walking).
    fn hours(&self, level: i32, b: &Build, xp: f64) -> f64;
}

// --- talent states -----------------------------------------------------------

/// bits a talent's rank takes in a state (ranks go to 5)
const BITS: u32 = 3;
/// ranks a word holds
const PER_WORD: usize = 21;
pub const NONE: u8 = u8::MAX;

/// A talent state: every talent's rank (3 bits each, 21 to a word) and the
/// talent being filled (`NONE` when the last point finished one).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub struct St {
    pub r: [u64; 3],
    pub cur: u8,
}

impl St {
    pub const START: St = St { r: [0; 3], cur: NONE };
}

/// The talents a route may take, by index.
pub struct Talents {
    pub names: Vec<String>,
    pub max: Vec<u8>,
    pub tab: Vec<usize>,
    /// how many begun talents may wait at once (0: none, the fill rule)
    pub paused_max: usize,
}

impl Talents {
    pub fn new(tree: &Tree, tabs: &[usize]) -> Talents {
        let mut names: Vec<String> = tree.talents.iter().filter(|t| tabs.contains(&t.tab())).map(|t| t.name.clone()).collect();
        names.sort();
        assert!(names.len() <= 3 * PER_WORD && names.iter().all(|n| tree.get(n).max_rank <= 7));
        let max = names.iter().map(|n| tree.get(n).max_rank as u8).collect();
        let tab = names.iter().map(|n| tree.get(n).tab()).collect();
        Talents { names, max, tab, paused_max: usize::MAX }
    }

    pub fn rank(&self, s: St, i: u8) -> u8 {
        let i = i as usize;
        (s.r[i / PER_WORD] >> (BITS as usize * (i % PER_WORD)) & 0x7) as u8
    }

    fn bump(&self, s: St, i: u8) -> St {
        let mut r = s.r;
        let i = i as usize;
        r[i / PER_WORD] += 1u64 << (BITS as usize * (i % PER_WORD));
        let n = St { r, cur: NONE };
        let k = self.rank(n, i as u8);
        St { r, cur: if k == self.max[i] { NONE } else { i as u8 } }
    }

    pub fn ranks(&self, s: St) -> Vec<(u8, u8)> {
        (0..self.names.len() as u8).map(|i| (i, self.rank(s, i))).filter(|&(_, r)| r > 0).collect()
    }

    /// Points in each tab.
    pub fn per_tab(&self, s: St) -> [u8; 3] {
        let mut out = [0u8; 3];
        for (i, r) in self.ranks(s) {
            out[self.tab[i as usize]] += r;
        }
        out
    }

    pub fn build(&self, s: St) -> Build {
        self.ranks(s).into_iter().map(|(i, r)| (self.names[i as usize].clone(), r as u32)).collect()
    }

    /// A talent order's states, a point at a time (the talent being filled kept,
    /// as the beam's own successors keep it).
    pub fn walk(&self, order: &[String]) -> Vec<St> {
        let mut s = St::START;
        order
            .iter()
            .map(|n| {
                let i = self.names.iter().position(|x| x == n).unwrap_or_else(|| panic!("{n} is not in the route's trees")) as u8;
                s = self.bump(s, i);
                s
            })
            .collect()
    }

    /// The state of a build (no talent being filled).
    pub fn state(&self, b: &Build) -> St {
        let mut s = St::START;
        for (n, &r) in b {
            let i = self.names.iter().position(|x| x == n).unwrap_or_else(|| panic!("{n} is not in the route's trees")) as u8;
            for _ in 0..r {
                s = self.bump(s, i);
            }
        }
        St { r: s.r, cur: NONE }
    }

    /// The next point. A talent begun is filled to its max before the next
    ///, unless the point just spent opened a row of its tab (5, 10,
    /// ... points there): then any talent may follow, the begun one left
    /// for later (2026-10-25: a new row may outbid it).
    pub fn successors(&self, tree: &Tree, s: St) -> Vec<St> {
        let mut out = Vec::new();
        let free = match s.cur {
            NONE => true,
            c => {
                out.push(self.bump(s, c));
                let tab = self.tab[c as usize];
                let in_tab: u32 = self.ranks(s).iter().filter(|(i, _)| self.tab[*i as usize] == tab).map(|r| r.1 as u32).sum();
                in_tab % ROW_GATE == 0
            }
        };
        if free {
            let base = self.build(s);
            for i in 0..self.names.len() as u8 {
                if Some(i) == (s.cur != NONE).then_some(s.cur) || self.rank(s, i) == self.max[i as usize] {
                    continue;
                }
                let mut b = base.clone();
                *b.entry(self.names[i as usize].clone()).or_insert(0) += 1;
                let n = self.bump(s, i);
                let paused = (0..self.names.len() as u8).filter(|&j| j != n.cur && self.rank(n, j) > 0 && self.rank(n, j) < self.max[j as usize]).count();
                if paused <= self.paused_max && tree.legal(&b) {
                    out.push(n);
                }
            }
        }
        out
    }

    /// Every state by points to `top` points (index p: p points), each with its successors.
    pub fn states(&self, tree: &Tree, top: usize) -> (Vec<Vec<St>>, HashMap<St, Vec<St>>) {
        let mut by = vec![vec![St::START]];
        let mut next = HashMap::new();
        let mut seen: HashSet<St> = HashSet::from([St::START]);
        for p in 0..top {
            let succ: Vec<(St, Vec<St>)> = by[p].par_iter().map(|&s| (s, self.successors(tree, s))).collect();
            let mut layer = Vec::new();
            for (s, n) in succ {
                for &x in &n {
                    if seen.insert(x) {
                        layer.push(x);
                    }
                }
                next.insert(s, n);
            }
            by.push(layer);
        }
        (by, next)
    }
}

/// A scenario's fingerprint: the same for builds the sim cannot tell apart.
pub fn fingerprint<S: Serialize>(sc: &S) -> u64 {
    // through a Value: its maps are sorted, a scenario's HashMaps are not
    hash_of(&serde_json::to_value(sc).unwrap().to_string())
}

pub fn hash_of<T: Hash + ?Sized>(x: &T) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    x.hash(&mut h);
    h.finish()
}

fn play_hash<P: Serialize>(p: &P) -> u64 {
    hash_of(&serde_json::to_string(p).unwrap())
}

/// A version from sources: a hash of their text.
pub fn version_of(sources: &[&str]) -> u64 {
    hash_of(&sources.concat())
}

// --- values ------------------------------------------------------------------

/// Each (build, play) on the first `nseeds` seeds: XP an hour.
pub fn run<C: Class>(c: &C, level: i32, builds: &[Build], variants: &[Vec<C::Play>], nseeds: usize, seconds: f64) -> Vec<Vec<f64>> {
    let scs: Vec<C::Scenario> = builds.par_iter().map(|b| c.scenario(level, b)).collect();
    let sd: Vec<u64> = (1..=nseeds as u64).collect();
    let jobs: Vec<(usize, usize, usize)> =
        (0..builds.len()).flat_map(|i| (0..variants[i].len()).flat_map(move |j| (0..nseeds).map(move |s| (i, j, s)))).collect();
    let tallies: Vec<C::Tally> = jobs.par_iter().map(|&(i, j, s)| c.session(&scs[i], &variants[i][j], sd[s], seconds)).collect();
    let mut it = tallies.into_iter();
    (0..builds.len())
        .map(|i| (0..variants[i].len()).map(|_| c.xp_per_hour(&it.by_ref().take(nseeds).collect::<Vec<_>>(), seconds)).collect())
        .collect()
}

/// XP an hour of a play on seeds (common to every play and build).
pub fn xp_on<C: Class>(c: &C, sc: &C::Scenario, p: &C::Play, seeds: &[u64], seconds: f64) -> f64 {
    let ts: Vec<C::Tally> = seeds.par_iter().map(|&s| c.session(sc, p, s, seconds)).collect();
    c.xp_per_hour(&ts, seconds)
}

/// A build's best play of `plays` (each taught), screened then measured: XP an hour.
pub fn value<C: Class>(c: &C, level: i32, b: &Build, plays: &[C::Play]) -> (f64, C::Play) {
    let mut vars: Vec<C::Play> = plays.iter().flat_map(|p| c.taught(p, b, level)).collect();
    if vars.len() > KEEP {
        let xs = run(c, level, std::slice::from_ref(b), std::slice::from_ref(&vars), SCREEN_SEEDS, SECONDS).remove(0);
        let mut o: Vec<usize> = (0..vars.len()).collect();
        o.sort_by(|&a, &b| xs[b].total_cmp(&xs[a]));
        vars = o.into_iter().take(KEEP).map(|j| vars[j].clone()).collect();
    }
    let xs = run(c, level, std::slice::from_ref(b), std::slice::from_ref(&vars), FINE_SEEDS, SECONDS).remove(0);
    let j = (0..xs.len()).max_by(|&a, &b| xs[a].total_cmp(&xs[b])).unwrap_or(0);
    (xs[j], vars[j].clone())
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(bound(serialize = "P: Serialize", deserialize = "P: DeserializeOwned"))]
pub struct Step<P> {
    pub level: i32,
    pub build: Build,
    pub xp_per_hour: f64,
    pub hours: f64,
    pub play: P,
}

#[derive(Clone)]
struct Node {
    s: St,
    /// the section, and the hours by its plays (ranking) and by the best play (the route's)
    k: u8,
    hours: f64,
    true_h: f64,
    path: Vec<St>,
}

#[derive(Serialize)]
#[serde(bound = "P: Serialize")]
pub struct BeamRoute<P> {
    pub trees: Vec<String>,
    pub width: usize,
    pub niche: usize,
    pub early: Vec<Step<P>>,
    pub path: Vec<Step<P>>,
    pub hours: f64,
    pub respec_at: Option<i32>,
    pub plain: Vec<Step<P>>,
    pub plain_hours: f64,
    /// hours from 10 with the respec at each level
    pub by_respec: Vec<(i32, f64)>,
    /// each level's best builds by their own XP an hour
    pub best_builds: Vec<(i32, Build, f64)>,
    /// the fastest plain route to each level (hours from 10, the builds), for
    /// a respec into another gear (`cross`)
    pub best_to: Vec<(i32, f64, Vec<Build>)>,
    /// from each level, the fastest plain way on to the top from any build
    /// measured there: (level, hours from that level on, the builds), for a
    /// respec into this gear (`cross`)
    pub best_on: Vec<(i32, f64, Vec<Build>)>,
    pub log: Vec<String>,
    /// each section's best route, by its own plays
    pub by_section: Vec<(u8, f64, Vec<Step<P>>)>,
}

/// Every level's plays: from the empty build, then refined on routes' builds.
/// Kept in a file for every later search (any trees, any top up to its own).
#[derive(Serialize, Deserialize)]
#[serde(bound(serialize = "P: Serialize", deserialize = "P: DeserializeOwned"))]
struct Library<P> {
    top: i32,
    early: Vec<Step<P>>,
    lib: Vec<(i32, Vec<P>)>,
}

/// A scenario's values: each taught variant of each library play screened,
/// the best `KEEP` measured exactly.
#[derive(Serialize, Deserialize, Clone, Default)]
struct Val {
    /// library plays screened
    screened: Vec<u64>,
    /// (library play, index in `taught`, XP an hour on the screen, exactly)
    vars: Vec<(u64, u8, f64, Option<f64>)>,
}

impl Val {
    /// the variants of a section (all when it has none)
    fn within(&self, ok: &dyn Fn(u64) -> bool) -> Vec<usize> {
        let v: Vec<usize> = (0..self.vars.len()).filter(|&i| ok(self.vars[i].0)).collect();
        if v.is_empty() { (0..self.vars.len()).collect() } else { v }
    }
    fn screen_in(&self, ok: &dyn Fn(u64) -> bool) -> f64 {
        self.within(ok).into_iter().map(|i| self.vars[i].2).fold(0.0, f64::max)
    }
    fn top_in(&self, ok: &dyn Fn(u64) -> bool) -> Vec<usize> {
        let mut o = self.within(ok);
        o.sort_by(|&a, &b| self.vars[b].2.total_cmp(&self.vars[a].2));
        o.truncate(KEEP);
        o
    }
    fn exact_in(&self, ok: &dyn Fn(u64) -> bool) -> Option<(f64, usize)> {
        if self.vars.is_empty() || self.top_in(ok).iter().any(|&i| self.vars[i].3.is_none()) {
            return None;
        }
        self.within(ok).into_iter().filter_map(|i| self.vars[i].3.map(|x| (x, i))).max_by(|a, b| a.0.total_cmp(&b.0))
    }
    fn top(&self) -> Vec<usize> {
        let mut o: Vec<usize> = (0..self.vars.len()).collect();
        o.sort_by(|&a, &b| self.vars[b].2.total_cmp(&self.vars[a].2));
        o.truncate(KEEP);
        o
    }
    /// the exact value and its variant, once the screen's best are measured
    fn exact(&self) -> Option<(f64, usize)> {
        if self.vars.is_empty() || self.top().iter().any(|&i| self.vars[i].3.is_none()) {
            return None;
        }
        (0..self.vars.len()).filter_map(|i| self.vars[i].3.map(|x| (x, i))).max_by(|a, b| a.0.total_cmp(&b.0))
    }
}

/// Values by (level, scenario), valid for one build of the sim.
#[derive(Serialize, Deserialize, Default)]
struct Store {
    sim: u64,
    vals: Vec<(i32, u64, Val)>,
}

type Vals = HashMap<(i32, u64), Val>;

fn load_vals<P: Serialize>(path: &str, lib: &HashMap<i32, Vec<P>>, sim: u64) -> Vals {
    let mut vals: Vals = std::fs::read_to_string(path)
        .ok()
        .and_then(|t| serde_json::from_str::<Store>(&t).ok())
        .filter(|s| s.sim == sim)
        .map(|s| s.vals.into_iter().map(|(l, f, v)| ((l, f), v)).collect())
        .unwrap_or_default();
    // drop stored values of plays the library no longer has
    let known: HashMap<i32, HashSet<u64>> = lib.iter().map(|(l, ps)| (*l, ps.iter().map(play_hash).collect())).collect();
    vals.retain(|(l, _), v| {
        let Some(k) = known.get(l) else { return false };
        v.screened.retain(|h| k.contains(h));
        v.vars.retain(|x| k.contains(&x.0));
        !v.vars.is_empty()
    });
    vals
}

fn save_vals(path: &str, vals: &Vals, sim: u64) {
    let store = Store { sim, vals: vals.iter().map(|((l, f), v)| (*l, *f, v.clone())).collect() };
    std::fs::write(path, serde_json::to_string(&store).unwrap()).expect("the values file");
}

/// Screen the scenarios on the library plays they have not met yet.
fn screen<C: Class>(c: &C, level: i32, kids: &[(u64, Build)], lib: &[(u64, C::Play)], vals: &mut Vals) -> usize {
    let mut todo: Vec<(u64, Build, Vec<(u64, u8, C::Play)>)> = Vec::new();
    for (fp, b) in kids {
        let v = vals.entry((level, *fp)).or_default();
        let vars: Vec<(u64, u8, C::Play)> = lib
            .iter()
            .filter(|(h, _)| !v.screened.contains(h))
            .flat_map(|(h, p)| c.taught(p, b, level).into_iter().enumerate().map(move |(k, q)| (*h, k as u8, q)))
            .collect();
        if !vars.is_empty() {
            todo.push((*fp, b.clone(), vars));
        }
    }
    if todo.is_empty() {
        return 0;
    }
    let builds: Vec<Build> = todo.iter().map(|t| t.1.clone()).collect();
    let variants: Vec<Vec<C::Play>> = todo.iter().map(|t| t.2.iter().map(|v| v.2.clone()).collect()).collect();
    for (t, xs) in todo.iter().zip(run(c, level, &builds, &variants, SCREEN_SEEDS, SECONDS)) {
        let v = vals.get_mut(&(level, t.0)).unwrap();
        for (var, x) in t.2.iter().zip(xs) {
            v.vars.push((var.0, var.1, x, None));
        }
        v.screened.extend(lib.iter().map(|l| l.0).filter(|h| !v.screened.contains(h)).collect::<Vec<_>>());
    }
    todo.len()
}

/// Measure exactly the screen's best plays of the scenarios (of each section), where not yet done.
fn measure<C: Class>(c: &C, level: i32, kids: &[(u64, Build)], lib: &HashMap<u64, C::Play>, vals: &mut Vals) -> usize {
    let mut todo: Vec<(u64, Build, Vec<usize>)> = Vec::new();
    let secs = c.sections();
    for (fp, b) in kids {
        let v = &vals[&(level, *fp)];
        let mut need: Vec<usize> = Vec::new();
        let groups: Vec<u8> = secs.iter().map(|&k| c.group(k)).collect::<std::collections::BTreeSet<_>>().into_iter().collect();
        for &g in &groups {
            let ok = |h: u64| lib.get(&h).is_some_and(|p| c.in_group(g, p));
            need.extend(v.top_in(&ok).into_iter().filter(|&i| v.vars[i].3.is_none() && !need.contains(&i)).collect::<Vec<_>>());
        }
        for &k in &secs {
            let ok = |h: u64| lib.get(&h).is_some_and(|p| c.admits(k, p));
            need.extend(v.top_in(&ok).into_iter().filter(|&i| v.vars[i].3.is_none() && !need.contains(&i)).collect::<Vec<_>>());
        }
        if !need.is_empty() {
            todo.push((*fp, b.clone(), need));
        }
    }
    if todo.is_empty() {
        return 0;
    }
    let builds: Vec<Build> = todo.iter().map(|t| t.1.clone()).collect();
    let variants: Vec<Vec<C::Play>> = todo
        .iter()
        .map(|(fp, b, need)| {
            let v = &vals[&(level, *fp)];
            need.iter().map(|&i| c.taught(&lib[&v.vars[i].0], b, level)[v.vars[i].1 as usize].clone()).collect()
        })
        .collect();
    for (t, xs) in todo.iter().zip(run(c, level, &builds, &variants, FINE_SEEDS, SECONDS)) {
        let v = vals.get_mut(&(level, t.0)).unwrap();
        for (&i, x) in t.2.iter().zip(xs) {
            v.vars[i].3 = Some(x);
        }
    }
    todo.len()
}

/// The places: the `width` best (hours, then XP an hour) and the `niche`
/// best of each tree split; one place for states the sim cannot tell apart;
/// and every state `force` holds (the earlier rounds' best routes).
/// Items: (state, section, hours by the section's plays, XP an hour, scenario,
/// hours by the best plays). Returns indices, best first.
type Item = (St, u8, f64, f64, u64, f64);
fn pick(items: &[Item], tal: &Talents, width: usize, niche: usize, force: &dyn Fn(St, u8) -> bool) -> Vec<usize> {
    let mut o: Vec<usize> = (0..items.len()).collect();
    o.sort_by(|&a, &b| items[a].2.total_cmp(&items[b].2).then(items[b].3.total_cmp(&items[a].3)));
    let mut seen = HashSet::new();
    o.retain(|&i| force(items[i].0, items[i].1) || seen.insert((items[i].4, items[i].0.cur, tal.per_tab(items[i].0), items[i].1)));
    let mut per: HashMap<([u8; 3], u8), usize> = HashMap::new();
    o.into_iter()
        .enumerate()
        .filter(|&(rank, i)| {
            let n = per.entry((tal.per_tab(items[i].0), items[i].1)).or_insert(0);
            *n += 1;
            rank < width || *n <= niche || force(items[i].0, items[i].1)
        })
        .map(|x| x.1)
        .collect()
}

fn same_in<C: Class>(c: &C, l: &[C::Play], q: &C::Play) -> bool {
    l.iter().any(|x| c.same(x, q))
}

/// Every level's plays, from the file and extended to `top` (levels 1-9 valued too).
pub fn library<C: Class>(c: &C, top: i32) -> (i32, Vec<Step<C::Play>>, HashMap<i32, Vec<C::Play>>) {
    let (lib_file, _) = c.files();
    let empty = Build::new();
    let kept: Option<Library<C::Play>> = std::fs::read_to_string(&lib_file).ok().and_then(|t| serde_json::from_str(&t).ok());
    let (from, mut early, mut lib) = match kept {
        Some(l) => (l.top + 1, l.early, l.lib.into_iter().map(|(lv, ps)| (lv, ps.into_iter().filter(|p| c.keeps(lv, p)).collect())).collect::<HashMap<i32, Vec<C::Play>>>()),
        None => (1, Vec::new(), HashMap::new()),
    };
    // the level below's plays from the empty build seed the next level's descent
    let mut below: Option<Vec<C::Play>> = lib.get(&(from - 1)).cloned();
    for level in from..=top {
        let ps: Vec<C::Play> = c.level_plays(level, &empty, below.as_deref()).into_iter().filter(|p| c.keeps(level, p)).collect();
        below = Some(ps.clone());
        if level < FIRST_TALENT {
            let (xp, play) = value(c, level, &empty, &ps);
            early.push(Step { level, build: empty.clone(), xp_per_hour: xp, hours: c.hours(level, &empty, xp), play });
        }
        lib.insert(level, ps);
    }
    (top.max(from - 1), early, lib)
}

pub fn save_lib<C: Class>(c: &C, top: i32, early: &[Step<C::Play>], lib: &HashMap<i32, Vec<C::Play>>) {
    let mut v: Vec<(i32, Vec<C::Play>)> = lib.clone().into_iter().collect();
    v.sort_by_key(|x| x.0);
    std::fs::write(c.files().0, serde_json::to_string(&Library { top, early: early.to_vec(), lib: v }).unwrap()).expect("the library file");
}

/// `seeds`: talent orders (from 10) kept in every section from the first
/// round, as an earlier round's best would be: a route the search would not
/// reach by itself (a talent whose worth comes points later) is still valued.
pub fn beam<C: Class>(c: &C, tabs: &[usize], width: usize, top: i32, seeds: &[Vec<String>]) -> BeamRoute<C::Play> {
    // a point a level from 10 to `top`; the time on `top` itself is not spent
    let points = (top - FIRST_TALENT + 1) as usize;
    let t0 = Instant::now();
    let mut log = Vec::new();
    let mut say = |s: String| {
        let line = format!("[{:6.1} s] {s}", t0.elapsed().as_secs_f64());
        eprintln!("{line}");
        log.push(line);
    };
    let tree = c.tree();
    let tal = Talents::new(tree, tabs);
    let (lib_top, early, mut lib) = library(c, top);
    save_lib(c, lib_top, &early, &lib);
    let values_file = c.files().1;
    let mut vals = load_vals(&values_file, &lib, c.sim_version());
    say(format!("levels 1-9: {:.2} h; plays for every level; {} stored values", early.iter().map(|s| s.hours).sum::<f64>(), vals.len()));
    let mut fps: HashMap<(i32, [u64; 3]), u64> = HashMap::new();
    let mut refined: HashSet<(i32, [u64; 3], u8)> = HashSet::new();
    // the earlier rounds' best routes stay in the beam (a round can only gain on
    // them); a respec route's builds after the respec are measured whatever happens
    let mut elite: HashSet<(i32, St, u8)> = HashSet::new();
    for order in seeds {
        for (i, s) in tal.walk(&order[..order.len().min(points)]).into_iter().enumerate() {
            for &k in &c.sections() {
                elite.insert((FIRST_TALENT + i as i32, s, k));
            }
        }
    }
    let mut extra: Vec<(i32, St)> = Vec::new();
    let mut result = None;
    let mut section_routes: Vec<(u8, f64, Vec<Step<C::Play>>)> = Vec::new();
    let hours = |level: i32, s: St, xp: f64| if level < top { c.hours(level, &tal.build(s), xp) } else { 0.0 };
    let sections = c.sections();
    for round in 1..=c.rounds() {
        let libh: HashMap<i32, Vec<(u64, C::Play)>> = lib.iter().map(|(l, ps)| (*l, ps.iter().map(|p| (play_hash(p), p.clone())).collect())).collect();
        let libm: HashMap<i32, HashMap<u64, C::Play>> = libh.iter().map(|(l, v)| (*l, v.iter().cloned().collect())).collect();
        let mut route: Vec<Node> = sections.iter().map(|&k| Node { s: St::START, k, hours: 0.0, true_h: 0.0, path: Vec::new() }).collect();
        let mut best_at: Vec<Node> = Vec::new();
        // every exactly measured child, by level and group: XP an hour (its group's best play)
        let mut measured: Vec<HashMap<(St, u8), f64>> = Vec::new();
        let groups: Vec<u8> = sections.iter().map(|&k| c.group(k)).collect::<std::collections::BTreeSet<_>>().into_iter().collect();
        let (mut screened, mut exact) = (0, 0);
        for p in 1..=points {
            let level = FIRST_TALENT + p as i32 - 1;
            let lm = &libm[&level];
            let in_sec = |k: u8| move |h: u64| lm.get(&h).is_some_and(|q| c.admits(k, q));
            let in_grp = |g: u8| move |h: u64| lm.get(&h).is_some_and(|q| c.in_group(g, q));
            // each (child, section)'s best parent
            let mut parent: HashMap<(St, u8), usize> = HashMap::new();
            for (i, n) in route.iter().enumerate() {
                for kid in tal.successors(tree, n.s) {
                    let e = parent.entry((kid, n.k)).or_insert(i);
                    if n.hours < route[*e].hours {
                        *e = i;
                    }
                }
            }
            let ex: Vec<St> = extra.iter().filter(|e| e.0 == level && !parent.keys().any(|k| k.0 == e.1)).map(|e| e.1).collect();
            let force = |s: St, k: u8| elite.contains(&(level, s, k));
            let new: Vec<[u64; 3]> = parent.keys().map(|k| k.0).chain(ex.iter().copied()).map(|s| s.r).filter(|r| !fps.contains_key(&(level, *r))).collect::<HashSet<_>>().into_iter().collect();
            let got: Vec<([u64; 3], u64)> = new.par_iter().map(|&r| (r, fingerprint(&c.scenario(level, &tal.build(St { r, cur: NONE }))))).collect();
            fps.extend(got.into_iter().map(|(r, f)| ((level, r), f)));
            let kids: Vec<(St, u8)> = parent.keys().copied().collect();
            let mut reps: HashMap<u64, Build> = HashMap::new();
            for s in kids.iter().map(|k| k.0).chain(ex.iter().copied()) {
                reps.entry(fps[&(level, s.r)]).or_insert_with(|| tal.build(s));
            }
            let reps: Vec<(u64, Build)> = reps.into_iter().collect();
            screened += screen(c, level, &reps, &libh[&level], &mut vals);
            // the places by the screen, then the contenders measured exactly
            let items: Vec<Item> = kids
                .iter()
                .map(|&(s, k)| {
                    let fp = fps[&(level, s.r)];
                    let v = &vals[&(level, fp)];
                    let (x, best) = (v.screen_in(&in_sec(k)), v.screen_in(&in_grp(c.group(k))));
                    let n = &route[parent[&(s, k)]];
                    (s, k, n.hours + hours(level, s, x), x, fp, n.true_h + hours(level, s, best))
                })
                .collect();
            let mut want: HashSet<u64> = pick(&items, &tal, SCREEN * width, SCREEN * c.niche(), &force).into_iter().map(|i| items[i].4).collect();
            want.extend(ex.iter().map(|s| fps[&(level, s.r)]));
            let todo: Vec<(u64, Build)> = reps.iter().filter(|r| want.contains(&r.0)).cloned().collect();
            exact += measure(c, level, &todo, &libm[&level], &mut vals);
            // the places by the exact values
            let items: Vec<Item> = kids
                .iter()
                .filter_map(|&(s, k)| {
                    let fp = fps[&(level, s.r)];
                    let v = &vals[&(level, fp)];
                    let n = &route[parent[&(s, k)]];
                    match (v.exact_in(&in_sec(k)), v.exact_in(&in_grp(c.group(k)))) {
                        (Some((x, _)), Some((best, _))) => Some((s, k, n.hours + hours(level, s, x), x, fp, n.true_h + hours(level, s, best))),
                        _ => None,
                    }
                })
                .collect();
            let mut here: HashMap<(St, u8), f64> = HashMap::new();
            for s in items.iter().map(|it| it.0).chain(ex.iter().copied()) {
                let v = &vals[&(level, fps[&(level, s.r)])];
                for &g in &groups {
                    if let Some((x, _)) = v.exact_in(&in_grp(g)) {
                        here.insert((s, g), x);
                    }
                }
            }
            measured.push(here);
            let next: Vec<Node> = pick(&items, &tal, width, c.niche(), &force)
                .into_iter()
                .map(|i| {
                    let (s, k, h, _, _, th) = items[i];
                    let n = &route[parent[&(s, k)]];
                    let mut path = n.path.clone();
                    path.push(s);
                    Node { s, k, hours: h, true_h: th, path }
                })
                .collect();
            route = next;
            best_at.push(route.iter().min_by(|a, b| a.true_h.total_cmp(&b.true_h)).expect("a route").clone());
            if level % 5 == 0 {
                say(format!("round {round}: level {level}, {} routes, {screened} screened, {exact} measured exactly so far", route.len()));
            }
        }
        // the respec: the best continuation from every measured build, backwards
        let idx = |level: i32| (level - FIRST_TALENT) as usize;
        let mut g: Vec<HashMap<(St, u8), (f64, Option<St>)>> = vec![HashMap::new(); points];
        g[points - 1] = measured[points - 1].keys().map(|&s| (s, (0.0, None))).collect();
        for level in (FIRST_TALENT..top).rev() {
            let (here, after) = g.split_at_mut(idx(level) + 1);
            let after = &after[0];
            here[idx(level)] = measured[idx(level)]
                .par_iter()
                .filter_map(|(&(s, gr), &x)| {
                    tal.successors(tree, s)
                        .into_iter()
                        .filter_map(|k| after.get(&(k, gr)).map(|v| (v.0, k)))
                        .min_by(|a, b| a.0.total_cmp(&b.0))
                        .map(|(h, k)| ((s, gr), (hours(level, s, x) + h, Some(k))))
                })
                .collect();
        }
        let by_respec: Vec<(i32, f64, (St, u8))> = (FIRST_TALENT + 1..top)
            .filter_map(|r| g[idx(r)].iter().filter(|(s, _)| c.respec_into(&tal.build(s.0))).min_by(|a, b| a.1 .0.total_cmp(&b.1 .0)).map(|(&s, v)| (r, best_at[idx(r) - 1].true_h + v.0, s)))
            .collect();
        let plain = best_at[points - 1].clone();
        let best = by_respec.iter().filter(|b| b.0 >= c.respec_from()).min_by(|a, b| a.1.total_cmp(&b.1)).filter(|b| c.respec() && b.1 < plain.true_h - 1e-9).cloned();
        let follow = |r: i32, (s, gr): (St, u8)| -> Vec<St> {
            let mut path = Vec::new();
            let mut at = Some(s);
            let mut level = r;
            while let Some(x) = at {
                path.push(x);
                at = g[idx(level)][&(x, gr)].1;
                level += 1;
            }
            path
        };
        // a route's group at each level (the respec may change it)
        let mut after_group: u8 = c.group(plain.k);
        let top_node = match &best {
            Some((r, h, s)) => {
                let pre = &best_at[idx(*r) - 1];
                let mut path = pre.path.clone();
                path.extend(follow(*r, *s));
                after_group = s.1;
                Node { s: *path.last().unwrap(), k: pre.k, hours: *h, true_h: *h, path }
            }
            None => plain.clone(),
        };
        let respec_at = best.map(|b| b.0);
        let step = |level: i32, s: &St, gr: u8| -> Step<C::Play> {
            let fp = fps[&(level, s.r)];
            let v = &vals[&(level, fp)];
            let lm = &libm[&level];
            let (x, i) = v.exact_in(&|h: u64| lm.get(&h).is_some_and(|q| c.in_group(gr, q))).expect("a measured build");
            let b = tal.build(*s);
            let play = c.taught(&lm[&v.vars[i].0], &b, level)[v.vars[i].1 as usize].clone();
            Step { level, build: b, xp_per_hour: x, hours: hours(level, *s, x), play }
        };
        let steps = |path: &[St], before: u8, after: u8, at: Option<i32>| -> Vec<Step<C::Play>> {
            path.iter().enumerate().map(|(i, s)| { let level = FIRST_TALENT + i as i32; step(level, s, if at.is_some_and(|r| level >= r) { after } else { before }) }).collect()
        };
        say(format!(
            "round {round}: {screened} screened, {exact} measured exactly; best {:.2} h{}, without a respec {:.2} h (section {}); the sections' best: {}",
            top_node.true_h,
            respec_at.map_or(String::new(), |r| format!(" (respec at {r})")),
            plain.true_h,
            plain.k,
            sections.iter().map(|&k| format!("{k}: {:.2}", route.iter().filter(|n| n.k == k).map(|n| n.true_h).fold(f64::INFINITY, f64::min))).collect::<Vec<_>>().join(", ")
        ));
        section_routes = sections
            .iter()
            .filter_map(|&k| {
                let n = route.iter().filter(|n| n.k == k).min_by(|a, b| a.hours.total_cmp(&b.hours))?;
                let steps = n.path.iter().enumerate().map(|(i, s)| {
                    let level = FIRST_TALENT + i as i32;
                    let v = &vals[&(level, fps[&(level, s.r)])];
                    let lm = &libm[&level];
                    let (x, j) = v.exact_in(&|h: u64| lm.get(&h).is_some_and(|q| c.admits(k, q))).expect("a measured build");
                    let b = tal.build(*s);
                    let play = c.taught(&lm[&v.vars[j].0], &b, level)[v.vars[j].1 as usize].clone();
                    Step { level, build: b, xp_per_hour: x, hours: hours(level, *s, x), play }
                }).collect();
                Some((k, n.hours, steps))
            })
            .collect();
        // each section's best route stays, and the respec route's start
        for &k in &sections {
            if let Some(n) = route.iter().filter(|n| n.k == k).min_by(|a, b| a.true_h.total_cmp(&b.true_h)) {
                for (i, s) in n.path.iter().enumerate() {
                    elite.insert((FIRST_TALENT + i as i32, *s, k));
                }
            }
        }
        for (i, s) in top_node.path.iter().enumerate() {
            let level = FIRST_TALENT + i as i32;
            if respec_at.map_or(true, |r| level < r) {
                elite.insert((level, *s, top_node.k));
            } else {
                extra.push((level, *s));
            }
        }
        // refine the plays on the routes' builds (each in its group's plays)
        let gr_of = |is_top: bool, n: &Node, level: i32| if is_top && respec_at.is_some_and(|r| level >= r) { after_group } else { c.group(n.k) };
        let mut new = 0;
        for (is_top, n) in [(true, &top_node), (false, &plain)] {
            for (i, s) in n.path.iter().enumerate() {
                let level = FIRST_TALENT + i as i32;
                if !refined.insert((level, s.r, gr_of(is_top, n, level))) {
                    continue;
                }
                let gr = if is_top && respec_at.is_some_and(|r| level >= r) { after_group } else { c.group(n.k) };
                let st = step(level, s, gr);
                let sc = c.scenario(level, &st.build);
                let seeds: Vec<u64> = (1..=DESCENT_SEEDS as u64).collect();
                for (q, _) in c.descend(&sc, level, &st.build, &st.play, &seeds, SECONDS) {
                    let l = lib.get_mut(&level).unwrap();
                    if c.keeps(level, &q) && !same_in(c, l, &q) {
                        l.push(q);
                        new += 1;
                    }
                }
            }
        }
        say(format!("round {round}: {new} new plays from the routes' builds"));
        save_lib(c, lib_top, &early, &lib);
        save_vals(&values_file, &vals, c.sim_version());
        // each level's ten best builds measured, by their own XP an hour
        let mut best_builds: Vec<(i32, Build, f64)> = Vec::new();
        for lv in FIRST_TALENT..=top {
            let mut at: Vec<(St, f64)> = measured[idx(lv)].iter().map(|(s, x)| (s.0, *x)).collect();
            at.sort_by(|a, b| b.1.total_cmp(&a.1));
            let mut seen = HashSet::new();
            best_builds.extend(at.into_iter().filter(|a| seen.insert(fps[&(lv, a.0.r)])).take(10).map(|(s, x)| (lv, tal.build(s), x)));
        }
        let best_to: Vec<(i32, f64, Vec<Build>)> =
            best_at.iter().enumerate().map(|(i, n)| (FIRST_TALENT + i as i32, n.true_h, n.path.iter().map(|s| tal.build(*s)).collect())).collect();
        let best_on: Vec<(i32, f64, Vec<Build>)> = (FIRST_TALENT..=top)
            .filter_map(|lv| g[idx(lv)].iter().min_by(|a, b| a.1 .0.total_cmp(&b.1 .0)).map(|(&s, v)| (lv, v.0, follow(lv, s).into_iter().map(|x| tal.build(x)).collect())))
            .collect();
        let plain_group = c.group(plain.k);
        result = Some((steps(&top_node.path, c.group(top_node.k), after_group, respec_at), top_node.true_h, respec_at, steps(&plain.path, plain_group, plain_group, None), plain.true_h, by_respec.iter().map(|b| (b.0, b.1)).collect(), best_builds, best_to, best_on));
        if new == 0 {
            break;
        }
    }
    let (path, hours_, respec_at, plain, plain_hours, by_respec, best_builds, best_to, best_on) = result.expect("a round");
    say("done".into());
    let names = c.tab_names();
    BeamRoute {
        trees: tabs.iter().map(|&t| names[t].to_string()).collect(),
        width,
        niche: c.niche(),
        early,
        path,
        hours: hours_,
        respec_at,
        plain,
        plain_hours,
        by_respec,
        best_builds,
        best_to,
        best_on,
        log,
        by_section: section_routes,
    }
}

/// One level of a route, climbed (2026-10-26): what its new point
/// adds at the same level (same mobs, spells and gear), what the level
/// itself adds to the build before it, and each talent's share.
#[derive(Serialize)]
#[serde(bound = "P: Serialize")]
pub struct ClimbRow<P> {
    pub level: i32,
    pub build: Build,
    /// the points new at this level
    pub new: Vec<String>,
    pub xp: f64,
    /// the level before's build at this level
    pub before_build: f64,
    /// the level before's build at the level before
    pub before_level: f64,
    /// each talent taken out whole: XP an hour without it
    pub without: Vec<(String, u32, f64)>,
    pub play: P,
    /// fresh seeds: the play, and the best a descent from it finds
    pub fresh: (f64, f64),
    pub better: P,
}

/// The climb of a route's levels (from 10): values as the beam's, from the
/// same library and store, so a route's own builds cost nothing.
pub fn climb<C: Class>(c: &C, steps: &[Step<C::Play>], early: &[Step<C::Play>], section: Option<u8>) -> Vec<ClimbRow<C::Play>> {
    let (lib_file, values_file) = c.files();
    let library: Library<C::Play> = serde_json::from_str(&std::fs::read_to_string(&lib_file).expect("a library: run a beam first")).expect("the library");
    let lib: HashMap<i32, Vec<C::Play>> = library.lib.into_iter().collect();
    let mut vals = load_vals(&values_file, &lib, c.sim_version());
    // every (level, build) the table needs
    let mut need: Vec<(i32, Build)> = Vec::new();
    for (i, s) in steps.iter().enumerate() {
        need.push((s.level, s.build.clone()));
        let before = if i == 0 { Build::new() } else { steps[i - 1].build.clone() };
        need.push((s.level, before));
        for t in s.build.keys() {
            let mut b = s.build.clone();
            b.remove(t);
            need.push((s.level, b));
        }
    }
    let mut value: HashMap<(i32, Build), (f64, C::Play)> = HashMap::new();
    let levels: Vec<i32> = need.iter().map(|n| n.0).collect::<std::collections::BTreeSet<_>>().into_iter().collect();
    for level in levels {
        let bs: Vec<Build> = need.iter().filter(|n| n.0 == level).map(|n| n.1.clone()).collect::<std::collections::BTreeSet<_>>().into_iter().collect();
        let fp: Vec<u64> = bs.par_iter().map(|b| fingerprint(&c.scenario(level, b))).collect();
        let mut reps: HashMap<u64, Build> = HashMap::new();
        for (f, b) in fp.iter().zip(&bs) {
            reps.entry(*f).or_insert_with(|| b.clone());
        }
        let reps: Vec<(u64, Build)> = reps.into_iter().collect();
        let libh: Vec<(u64, C::Play)> = lib[&level].iter().map(|p| (play_hash(p), p.clone())).collect();
        let libm: HashMap<u64, C::Play> = libh.iter().cloned().collect();
        screen(c, level, &reps, &libh, &mut vals);
        measure(c, level, &reps, &libm, &mut vals);
        for (f, b) in fp.iter().zip(&bs) {
            let v = &vals[&(level, *f)];
            let lm = &libm;
            let (x, i) = match section {
                Some(k) => v.exact_in(&|h: u64| lm.get(&h).is_some_and(|q| c.admits(k, q))).expect("measured"),
                None => v.exact().expect("measured"),
            };
            value.insert((level, b.clone()), (x, c.taught(&libm[&v.vars[i].0], b, level)[v.vars[i].1 as usize].clone()));
        }
    }
    save_vals(&values_file, &vals, c.sim_version());
    let fresh: Vec<u64> = FRESH.collect();
    steps
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let before = if i == 0 { Build::new() } else { steps[i - 1].build.clone() };
            // a play another build won with is tried here too, on the same seeds:
            // a value is never lower for the search having missed a known play
            let fine: Vec<u64> = (1..=FINE_SEEDS as u64).collect();
            let at_least = |b: &Build, from: (f64, C::Play), other: &C::Play| -> (f64, C::Play) {
                let sc = c.scenario(s.level, b);
                c.taught(other, b, s.level).into_iter().fold(from, |best, q| {
                    let x = xp_on(c, &sc, &q, &fine, SECONDS);
                    if x > best.0 { (x, q) } else { best }
                })
            };
            let (xp, play) = at_least(&s.build, value[&(s.level, s.build.clone())].clone(), &value[&(s.level, before.clone())].1);
            let before_level = if i == 0 { early.last().map_or(f64::NAN, |e| e.xp_per_hour) } else { value[&(s.level - 1, before.clone())].0 };
            let new = s.build.iter().filter(|(k, v)| before.get(*k) != Some(v)).map(|(k, v)| format!("{k} {v}")).collect();
            let without = s
                .build
                .iter()
                .map(|(t, r)| {
                    let mut b = s.build.clone();
                    b.remove(t);
                    (t.clone(), *r, at_least(&b, value[&(s.level, b.clone())].clone(), &play).0)
                })
                .collect();
            let sc = c.scenario(s.level, &s.build);
            let start = xp_on(c, &sc, &play, &fresh, SECONDS);
            let (better, best) = c.descend(&sc, s.level, &s.build, &play, &fresh, SECONDS).remove(0);
            ClimbRow { level: s.level, build: s.build.clone(), new, xp, before_build: value[&(s.level, before)].0, before_level, without, play, fresh: (start, best), better }
        })
        .collect()
}

/// A given talent order valued level by level as the beam values a build
/// (its level's library plays, screened, the best two on 24 seeds): the
/// steps and the hours from 10.
pub fn path<C: Class>(c: &C, order: &[String], top: i32) -> Vec<Step<C::Play>> {
    let (got, early, lib) = library(c, top);
    // levels 1-9 again, on plays the library may have gained since
    let empty = Build::new();
    let early: Vec<Step<C::Play>> = early
        .iter()
        .map(|e| {
            let (xp, play) = value(c, e.level, &empty, &lib[&e.level]);
            Step { level: e.level, build: empty.clone(), xp_per_hour: xp, hours: c.hours(e.level, &empty, xp), play }
        })
        .collect();
    save_lib(c, got, &early, &lib);
    let mut b = Build::new();
    let mut out = Vec::new();
    for (i, level) in (FIRST_TALENT..=top).enumerate() {
        if let Some(t) = order.get(i) {
            *b.entry(t.clone()).or_insert(0) += 1;
        }
        let (x, play) = value(c, level, &b, &lib[&level]);
        out.push(Step { level, build: b.clone(), xp_per_hour: x, hours: if level < top { c.hours(level, &b, x) } else { 0.0 }, play });
    }
    out
}
