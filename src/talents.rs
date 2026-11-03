//! Forever's talent trees from the client's trait tables (`forever_sim.talents`):
//! a node's entry names a definition, which names the talent's spell; an
//! effect that grows with rank has a curve (`CurvePoint(rank)`), the others
//! keep the spell's base points. Classic's rules on top: a row opens at 5
//! points in its own tab, an arrowed parent must be full.

use crate::client::Client;
use std::collections::{BTreeMap, HashMap};

/// A build: talent name -> rank.
pub type Build = BTreeMap<String, u32>;

pub struct Talent {
    pub name: String,
    pub spell_id: i64,
    pub max_rank: u32,
    pub x: i64,
    pub y: i64,
    curves: HashMap<i64, HashMap<i64, f64>>,
    node: i64,
}

pub const TAB_NAMES: [&str; 3] = ["Affliction", "Demonology", "Destruction"];
const TAB_WIDTH: i64 = 4000;
const ROW_TOP: f64 = 2130.0;
const ROW_STEP: f64 = 600.0;
pub const ROW_GATE: u32 = 5;

impl Talent {
    pub fn tab(&self) -> usize {
        (self.x / TAB_WIDTH) as usize
    }

    /// Its column in the tab, 0 to 3 (the client's canvas starts a column in).
    pub fn col(&self) -> i64 {
        (self.x - self.tab() as i64 * TAB_WIDTH) / ROW_STEP as i64 - 1
    }

    pub fn row(&self) -> u32 {
        ((self.y as f64 - ROW_TOP) / ROW_STEP).round() as u32
    }
}

pub struct Tree {
    pub talents: Vec<Talent>,
    index: HashMap<String, usize>,
    /// each talent's arrowed parents (`TraitEdge` type 2 or 3)
    pub parents: HashMap<usize, Vec<usize>>,
}

impl Tree {
    /// The class tree that holds the talent spell `spell_in_tree`.
    pub fn load(c: &Client, spell_in_tree: i64) -> Tree {
        let mut points: HashMap<i64, Vec<(f64, f64)>> = HashMap::new();
        for r in c.t("CurvePoint").iter() {
            points.entry(r.i("CurveID")).or_default().push((r.f("Pos_0"), r.f("Pos_1")));
        }
        let mut curves: HashMap<i64, HashMap<i64, HashMap<i64, f64>>> = HashMap::new();
        for r in c.t("TraitDefinitionEffectPoints").iter() {
            let by_rank = points.get(&r.i("CurveID")).map(|v| v.iter().map(|&(k, v)| (k as i64, v)).collect()).unwrap_or_default();
            curves.entry(r.i("TraitDefinitionID")).or_default().insert(r.i("EffectIndex"), by_rank);
        }
        let (nodes, entries, defs) = (c.t("TraitNode"), c.t("TraitNodeEntry"), c.t("TraitDefinition"));
        // tree id -> its talents in the order first seen (a dict by name)
        let mut by_tree: HashMap<String, (Vec<Talent>, HashMap<String, usize>)> = HashMap::new();
        let mut tree_of_spell: HashMap<i64, String> = HashMap::new();
        for link in c.t("TraitNodeXTraitNodeEntry").iter() {
            let (Some(node), Some(entry)) = (nodes.get(link.i("TraitNodeID")), entries.get(link.i("TraitNodeEntryID"))) else { continue };
            let Some(d) = defs.get(entry.i("TraitDefinitionID")) else { continue };
            let sid = d.i("SpellID");
            let name = match d.s("OverrideName_lang") {
                "" => c.spell_name(sid).map(str::to_owned).unwrap_or_else(|| sid.to_string()),
                n => n.to_owned(),
            };
            let t = Talent {
                name: name.clone(),
                spell_id: sid,
                max_rank: entry.i("MaxRanks") as u32,
                x: node.i("PosX"),
                y: node.i("PosY"),
                curves: curves.get(&d.i("ID")).cloned().unwrap_or_default(),
                node: link.i("TraitNodeID"),
            };
            let tid = node.s("TraitTreeID").to_owned();
            let (list, idx) = by_tree.entry(tid.clone()).or_default();
            match idx.get(&name) {
                Some(&i) => list[i] = t,
                None => {
                    idx.insert(name, list.len());
                    list.push(t);
                }
            }
            tree_of_spell.insert(sid, tid);
        }
        let (talents, index) = by_tree.remove(&tree_of_spell[&spell_in_tree]).expect("the tree");
        let by_node: HashMap<i64, usize> = talents.iter().enumerate().map(|(i, t)| (t.node, i)).collect();
        let mut parents: HashMap<usize, Vec<usize>> = HashMap::new();
        for r in c.t("TraitEdge").iter() {
            let (a, b) = (r.i("LeftTraitNodeID"), r.i("RightTraitNodeID"));
            if let (Some(&a), Some(&b), true) = (by_node.get(&a), by_node.get(&b), matches!(r.s("Type"), "2" | "3")) {
                parents.entry(b).or_default().push(a);
            }
        }
        Tree { talents, index, parents }
    }

    /// A hand-made change to a client's tree (`druid::launch`): a talent moved,
    /// taken out or added, or an effect's values by rank, and the arrows anew.
    pub fn place(&mut self, name: &str, tab: i64, row: i64, col: i64) {
        let i = self.id(name);
        self.talents[i].x = tab * TAB_WIDTH + (col + 1) * ROW_STEP as i64;
        self.talents[i].y = ROW_TOP as i64 + row * ROW_STEP as i64;
    }

    pub fn add(&mut self, name: &str, max_rank: u32, tab: i64, row: i64, col: i64) {
        self.talents.push(Talent { name: name.to_owned(), spell_id: 0, max_rank, x: 0, y: 0, curves: HashMap::new(), node: -1 });
        self.index.insert(name.to_owned(), self.talents.len() - 1);
        self.place(name, tab, row, col);
    }

    pub fn remove(&mut self, name: &str) {
        let i = self.id(name);
        self.talents.remove(i);
        self.parents.clear();
        self.index = self.talents.iter().enumerate().map(|(i, t)| (t.name.clone(), i)).collect();
    }

    pub fn set_curve(&mut self, name: &str, index: i64, values: &[f64]) {
        let i = self.id(name);
        self.talents[i].curves.insert(index, values.iter().enumerate().map(|(k, v)| (k as i64 + 1, *v)).collect());
    }

    pub fn set_parents(&mut self, arrows: &[(&str, &str)]) {
        self.parents.clear();
        for (a, b) in arrows {
            let (a, b) = (self.id(a), self.id(b));
            self.parents.entry(b).or_default().push(a);
        }
    }

    pub fn id(&self, name: &str) -> usize {
        *self.index.get(name).unwrap_or_else(|| panic!("no talent {name}"))
    }

    pub fn get(&self, name: &str) -> &Talent {
        &self.talents[self.id(name)]
    }

    /// The talent's effect `index` at `rank` (0: not taken).
    pub fn value(&self, c: &Client, name: &str, index: i64, rank: u32) -> f64 {
        if rank == 0 {
            return 0.0;
        }
        let t = self.get(name);
        assert!(rank <= t.max_rank, "{name} has {} ranks, not {rank}", t.max_rank);
        match t.curves.get(&index) {
            Some(curve) if !curve.is_empty() => curve[&(rank as i64)],
            _ => c.effect(t.spell_id, index).f("EffectBasePointsF"),
        }
    }

    /// Whether a build breaks Classic's rules (a row opens at 5 points in its
    /// own tab; an arrowed parent must be at full rank).
    pub fn legal(&self, b: &Build) -> bool {
        for (name, &rank) in b {
            let t = self.get(name);
            if rank == 0 || rank > t.max_rank {
                return false;
            }
            let below: u32 = b.iter().filter(|(m, _)| self.get(m).tab() == t.tab() && self.get(m).row() < t.row()).map(|(_, v)| v).sum();
            if below < ROW_GATE * t.row() {
                return false;
            }
            for &p in self.parents.get(&self.id(name)).map(|v| v.as_slice()).unwrap_or(&[]) {
                if b.get(&self.talents[p].name).copied().unwrap_or(0) != self.talents[p].max_rank {
                    return false;
                }
            }
        }
        true
    }
}

/// A spell's class mask (`SpellClassOptions`).
pub fn class_mask(c: &Client, sid: i64) -> [u32; 4] {
    match c.rows("SpellClassOptions", sid).first() {
        Some(r) => std::array::from_fn(|i| (r.i(&format!("SpellClassMask_{i}")) & 0xFFFF_FFFF) as u32),
        None => [0; 4],
    }
}

/// One spell modifier of a build: (op, flat, value, class mask).
pub type Mod = (i64, bool, f64, [u32; 4]);

/// Every spell modifier a build's talents carry (aura 107 flat, 108 percent).
pub fn mods(c: &Client, tree: &Tree, b: &Build) -> Vec<Mod> {
    let mut out = Vec::new();
    for (name, &rank) in b {
        let t = tree.get(name);
        for e in c.rows("SpellEffect", t.spell_id) {
            let aura = e.s("EffectAura");
            if aura != "107" && aura != "108" {
                continue;
            }
            let mask = std::array::from_fn(|i| (e.i(&format!("EffectSpellClassMask_{i}")) & 0xFFFF_FFFF) as u32);
            out.push((e.i("EffectMiscValue_0"), aura == "107", tree.value(c, name, e.i("EffectIndex"), rank), mask));
        }
    }
    out
}

/// (flat, percent) of modifier `op` on a spell of class mask `mask`.
pub fn modifier(mods: &[Mod], mask: [u32; 4], op: i64) -> (f64, f64) {
    let (mut flat, mut pct) = (0.0, 0.0);
    for &(o, is_flat, v, m) in mods {
        if o == op && mask.iter().zip(m.iter()).any(|(a, b)| a & b != 0) {
            if is_flat {
                flat += v;
            } else {
                pct += v;
            }
        }
    }
    (flat, pct)
}
