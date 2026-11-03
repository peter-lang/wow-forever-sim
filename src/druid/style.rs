//! A levelling route: talent orders written by hand (`Style::order`), one gear set throughout
//! (2026-10-31), a respec between two ways of fighting at a chosen level. The simulation
//! only checks them: where the time goes and what each talent adds is in `druid-level understand`.

use super::launch::{Route, Style};
use super::levels::Data;
use super::peak::Peak;
use super::play::Play;
use crate::route::{library, save_lib, Step, FIRST_TALENT};
use crate::talents::Build;
use rayon::prelude::*;
use serde::Serialize;

const TOP: i32 = 60;

#[derive(Serialize)]
pub struct Walked {
    pub route: String,
    pub goal: Build,
    pub early: Vec<Step<Play>>,
    pub early_hours: f64,
    pub path: Vec<Step<Play>>,
    pub hours: f64,
    pub respec_at: Option<i32>,
    pub rows: Vec<Row>,
}

/// One level of a route, for the page.
#[derive(Serialize)]
pub struct Row {
    pub level: i32,
    pub build: Build,
    pub xp: f64,
    /// the level before's build at this level (at a respec: this build)
    pub before_build: f64,
    /// each talent of the build taken out whole: its ranks, XP an hour and the level's hours without it
    pub without: Vec<(String, u32, f64, f64)>,
    pub play: Play,
    pub deaths_a_day: f64,
}

/// The first `points` of an order.
pub fn build_of(order: &[&str], points: usize) -> Build {
    order[..points].iter().fold(Build::new(), |mut b, n| {
        *b.entry(n.to_string()).or_insert(0) += 1;
        b
    })
}

pub fn walk(d: &Data, r: &Route) -> Walked {
    let (got, early, lib) = library(d, FIRST_TALENT - 1);
    save_lib(d, got, &early, &lib);
    let early_hours = early.iter().map(|x| x.hours).sum();
    let legs: Vec<(Style, i32, Vec<&'static str>, Peak)> = r.legs.iter().enumerate().map(|(i, &(order, fights, from))| (fights, from, if i == 0 { order.order() } else { order.after_respec() }, Peak::new(d, fights))).collect();
    let leg = |level: i32| legs.iter().rev().find(|l| l.1 <= level).expect("a leg");
    let respec_at = legs.get(1).map(|l| l.1);
    let levels: Vec<i32> = (FIRST_TALENT..=TOP).collect();
    let mut rows: Vec<(Step<Play>, Row)> = levels
        .par_iter()
        .map(|&level| {
            let l = leg(level);
            let b = build_of(&l.2, (level - FIRST_TALENT + 1) as usize);
            assert!(d.tree.legal(&b), "{}: level {level} breaks the rules: {b:?}", r.name);
            let s = l.3.step(level, &b);
            let before_build = if level == FIRST_TALENT || Some(level) == respec_at {
                s.xp_per_hour
            } else {
                l.3.xp(level, &build_of(&l.2, (level - FIRST_TALENT) as usize)).0
            };
            // every level played: the page sums them into a talent's worth over a whole leg
            let without = if level < TOP {
                b.iter().map(|(n, &rank)| { let mut o = b.clone(); o.remove(n); let w = l.3.step(level, &o); (n.clone(), rank, w.xp_per_hour, w.hours) }).collect()
            } else {
                Vec::new()
            };
            let row = Row { level, build: b.clone(), xp: s.xp_per_hour, before_build, without, play: s.play.clone(), deaths_a_day: l.3.deaths(level, &b, &s.play) };
            (s, row)
        })
        .collect();
    rows.sort_by_key(|x| x.0.level);
    let hours = rows.iter().map(|x| x.0.hours).sum();
    let goal = rows.last().expect("a level").0.build.clone();
    let (path, rows): (Vec<_>, Vec<_>) = rows.into_iter().unzip();
    Walked { route: r.name.into(), goal, early, early_hours, path, hours, respec_at, rows }
}
