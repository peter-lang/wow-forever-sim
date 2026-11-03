//! The launch druid: the tree as foreverchanges.pro reads the hotfix of
//! 2026-10-30, the spell ranks it brought, and the rules that follow from them.

use forever::druid::beam::xp_of;
use forever::druid::farm::session;
use forever::druid::levels::{Data, Kit};
use forever::druid::play::Play;
use forever::talents::Build;

fn build(ranks: &[(&str, u32)]) -> Build {
    ranks.iter().map(|(k, v)| (k.to_string(), *v)).collect()
}

#[test]
fn the_tree_is_the_launch_tree() {
    let d = Data::load(Kit::Feral);
    assert_eq!(d.tree.talents.len(), 52);
    for gone in ["Berserk", "Gift of the Earthmother", "Improved Shifting Power"] {
        assert!(!d.tree.talents.iter().any(|t| t.name == gone), "{gone}");
    }
    for (name, tab, row) in [("Furor", 1, 0), ("Heart of the Wild", 2, 0), ("Lacerate", 1, 3), ("Primal Bite", 1, 6), ("Shifting Power", 1, 6), ("Bloom", 2, 2), ("Reflection", 2, 1)] {
        let t = d.tree.get(name);
        assert_eq!((t.tab(), t.row()), (tab, row), "{name}");
    }
    // moved and unmoved talents on one grid: each its own cell, the site's columns
    let cells: std::collections::HashSet<_> = d.tree.talents.iter().map(|t| (t.tab(), t.row(), t.col())).collect();
    assert_eq!(cells.len(), 52);
    assert!(d.tree.talents.iter().all(|t| (0..4).contains(&t.col())));
    for (name, col) in [("Rend and Tear", 0), ("Feral Swiftness", 0), ("Primal Bite", 1), ("Leader of the Pack", 1), ("Shifting Power", 2), ("Predatory Discipline", 2), ("Natural Reaction", 3), ("Bloom", 3)] {
        assert_eq!(d.tree.get(name).col(), col, "{name}");
    }
    assert_eq!(d.tree.value(&d.c, "Genesis", 0, 5), 10.0);
    assert_eq!(d.tree.value(&d.c, "Naturalist", 1, 5), 10.0);
}

#[test]
fn row_seven_needs_thirty_points_and_its_parent() {
    let d = Data::load(Kit::Feral);
    assert!(!d.tree.legal(&build(&[("Primal Bite", 1)])));
    let full = build(&[("Ferocity", 5), ("Feral Instinct", 3), ("Feral Swiftness", 2), ("Savage Fury", 2), ("Sharpened Claws", 2), ("Feral Charge", 1),
        ("Predatory Strikes", 3), ("Blood Frenzy", 2), ("Natural Instinct", 2), ("Thick Hide", 3), ("Rend and Tear", 5)]);
    assert_eq!(full.values().sum::<u32>(), 30);
    let mut bite = full.clone();
    bite.insert("Primal Bite".into(), 1);
    assert!(!d.tree.legal(&bite), "without Leader of the Pack");
}

#[test]
fn spells_come_at_the_launch_levels() {
    let d = Data::load(Kit::Feral);
    let bite = |lv| d.scenario(lv, &Build::new()).bite;
    assert!(!bite(23).known);
    // the tooltips' numbers, with the 2026-11-03 notes' +25%
    let k = forever::druid::launch::CAT_SCALE;
    assert_eq!((bite(24).base, bite(24).extra), (14.0 * k, 24.0 * k));
    assert_eq!(bite(32).extra, 36.0 * k);
    // Lacerate and Primal Bite only with the talent, at the level's rank
    assert!(!d.scenario(50, &Build::new()).lacerate.known);
    let t = build(&[("Lacerate", 1), ("Primal Bite", 1)]);
    let (a, b) = (d.scenario(34, &t), d.scenario(50, &t));
    assert_eq!((a.lacerate.tick, b.lacerate.tick), (6.0, 12.0));
    assert_eq!(b.primal_bite.base, 62.0);
    assert_eq!(d.scenario(40, &t).enrage_armor, 0.16);
    assert_eq!(d.scenario(39, &t).enrage_armor, 0.27);
}

#[test]
fn haste_shortens_a_swing_below_a_step() {
    // the old timer restarted on the step it landed on: 2% haste did nothing
    let d = Data::load(Kit::Feral);
    let mut sc = d.scenario(30, &Build::new());
    let p = Play::cat_default();
    sc.haste = 1.0;
    let slow = xp_of(&sc, &p);
    sc.haste = 1.02;
    assert!(xp_of(&sc, &p) > slow * 1.002);
}

#[test]
fn predatory_discipline_keeps_the_energy_and_pays() {
    let d = Data::load(Kit::Feral);
    let p = Play { rip_cp: 0, bite_cp: 5, ..Play::cat_default() };
    // a cat that Bites at 35 energy has none extra to keep: only a surplus shows it
    let surplus = |b: &Build| {
        let mut sc = d.scenario(40, b);
        sc.claw.cost = 5.0;
        sc.rake.known = false; // a Rake it can now afford would take the cheap Claw's global cooldowns
        xp_of(&sc, &p)
    };
    let plain = surplus(&Build::new());
    let kept = surplus(&build(&[("Predatory Discipline", 1)]));
    assert!(kept > plain, "{kept} against {plain}");
}

#[test]
fn lacerate_opens_rend_and_tear_for_the_bear() {
    let d = Data::load(Kit::Feral);
    let sc = d.scenario(45, &build(&[("Lacerate", 1), ("Rend and Tear", 5)]));
    let p = Play { form: "bear".into(), mobs: 1, drag: false, ..Play::cat_default() };
    assert!(xp_of(&sc, &Play { lacerate: 1, ..p.clone() }) > xp_of(&sc, &p));
}

#[test]
fn a_held_bite_pays_with_predatory_discipline() {
    let d = Data::load(Kit::Feral);
    let sc = d.scenario(48, &build(&[("Predatory Discipline", 1)]));
    let p = Play { rip_cp: 0, bite_cp: 5, ..Play::cat_default() };
    let (at_once, held) = (xp_of(&sc, &p), xp_of(&sc, &Play { bite_pool: 85.0, ..p.clone() }));
    println!("held {held:.0}, at once {at_once:.0}");
    assert!(held != at_once);
}

#[test]
fn the_requested_spells_come_at_their_levels() {
    let d = Data::load(Kit::Balance);
    let (a, b) = (d.scenario(39, &Build::new()), d.scenario(40, &Build::new()));
    assert!(!a.innervate.known && !a.hurricane.known);
    assert_eq!((b.innervate.cooldown, b.innervate.extra, b.hurricane.tick), (360.0, 4.0, 68.0));
    // Hurricane's tick grows 0.2 a level to its rank's top level, 46
    assert!((d.scenario(46, &Build::new()).hurricane.tick - 69.2).abs() < 1e-6);
    assert!((d.scenario(49, &Build::new()).hurricane.tick - 69.2).abs() < 1e-6);
    assert!(!d.scenario(35, &Build::new()).frenzied.known && d.scenario(36, &Build::new()).frenzied.known);
    assert!(d.scenario(32, &Build::new()).ravage.known && !d.scenario(32, &Build::new()).pounce.known);
    let roots = d.scenario(30, &build(&[("Improved Entangling Roots", 3)])).root_break;
    assert!((roots - 0.5 / 1.75).abs() < 1e-9);
}

#[test]
fn innervate_saves_a_mana_bound_balance_druid_time() {
    let d = Data::load(Kit::Balance);
    let mut sc = d.scenario(45, &Build::new());
    let p = Play { form: "caster".into(), ..Play::cat_default() };
    let with = xp_of(&sc, &p);
    sc.innervate.known = false;
    assert!(with > xp_of(&sc, &p));
}

// The review of 2026-10-31: each of these was wrong.

#[test]
fn faerie_fire_cuts_armor_only_when_the_play_casts_it() {
    let d = Data::load(Kit::Feral);
    let sc = d.scenario(30, &Build::new());
    let mut none = sc.clone();
    none.faerie_fire.extra = 0.0;
    for p in [Play::cat_default(), Play { form: "bear".into(), mobs: 1, drag: false, ..Play::cat_default() }] {
        assert_eq!(xp_of(&sc, &p), xp_of(&none, &p), "{}", p.form);
    }
}

#[test]
fn every_form_walks_between_pulls() {
    let f = Data::load(Kit::Feral);
    let b = Data::load(Kit::Balance);
    let fsc = f.scenario(45, &Build::new());
    let mut bsc = b.scenario(45, &Build::new());
    bsc.caster.health = 1.0e7; // alive to walk: an untalented druid under three mobs dies first
    let base = Play::cat_default();
    let plays = [
        (&fsc, base.clone()),
        (&fsc, Play { form: "bear".into(), mobs: 2, drag: false, ..base.clone() }),
        (&bsc, Play { form: "caster".into(), ..base.clone() }),
        (&bsc, Play { form: "caster".into(), hurricane: 3, ..base.clone() }),
    ];
    for (sc, p) in plays {
        let t = session(sc, &p, 1, 1800.0);
        assert!(t.kills > 0 && t.walking / (t.kills as f64) > 1.0, "{} walked {:.0} s for {} kills", p.form, t.walking, t.kills);
    }
}

#[test]
fn hurricane_waits_for_the_mobs() {
    // one tick cannot be the whole of a mob: with ticks that kill at once, a pull still takes the run in
    let b = Data::load(Kit::Balance);
    let mut sc = b.scenario(45, &Build::new());
    sc.hurricane.tick = 1.0e7;
    sc.hurricane.cost = 1.0;
    sc.heal_crit_mult = 1.0;
    let p = Play { form: "caster".into(), hurricane: 3, ..Play::cat_default() };
    let t = session(&sc, &p, 1, 600.0);
    let pulls = t.kills as f64 / 3.0;
    assert!(t.seconds / pulls > 30.0 / 7.0, "{:.1} s a pull", t.seconds / pulls);
}

#[test]
fn clearcasting_helps_the_bear() {
    let d = Data::load(Kit::Feral);
    let mut sc = d.scenario(45, &Build::new());
    let p = Play { form: "bear".into(), mobs: 2, drag: false, heal_below: 0.75, emergency: 0.3, ..Play::cat_default() };
    let with = xp_of(&sc, &p);
    sc.omen = false;
    assert!(with > xp_of(&sc, &p));
}

#[test]
fn dot_ticks_crit() {
    // more crit, the same direct hits removed: the only thing left that it can lift is the ticks
    let d = Data::load(Kit::Balance);
    let mut sc = d.scenario(45, &Build::new());
    sc.wrath.base = 0.0;
    sc.wrath.coef = 0.0;
    sc.starfire.known = false;
    sc.moonfire.base = 0.0;
    sc.moonfire.spread = 0.0;
    sc.moonfire.coef = 0.0;
    sc.caster.health = 1.0e7;
    // no mana or melee in the way: only the ticks do damage
    sc.wrath.cost = 0.0;
    sc.moonfire.cost = 0.0;
    sc.thorns.known = false;
    let p = Play { form: "caster".into(), starfire_opener: false, moonfire: true, drag: false, heal_below: 0.0, ..Play::cat_default() };
    let low = session(&sc, &p, 1, 7200.0);
    sc.spell_crit += 40.0;
    let high = session(&sc, &p, 1, 7200.0);
    assert!(low.kills > 0, "no kills: {low:?}");
    assert!(high.kills as f64 > low.kills as f64 * 1.05, "{} kills against {}", high.kills, low.kills);
}

#[test]
fn no_session_stalls_or_earns_from_nothing() {
    // the event loops: every play ends and spends its time, at low and high level
    let (f, b) = (Data::load(Kit::Feral), Data::load(Kit::Balance));
    for lv in [10, 25, 40, 59] {
        let (fsc, bsc) = (f.scenario(lv, &Build::new()), b.scenario(lv, &Build::new()));
        for p in [Play::cat_default(), Play { form: "bear".into(), mobs: 3, drag: false, ..Play::cat_default() }, Play { form: "caster".into(), ..Play::cat_default() }, Play { form: "caster".into(), roots: true, ..Play::cat_default() }] {
            let sc = if p.form == "caster" { &bsc } else { &fsc };
            let t = session(sc, &p, 7, 1800.0);
            assert!(t.seconds >= 1800.0 && t.seconds < 3600.0, "{} at {lv}: {:.0} s", p.form, t.seconds);
            assert!(t.kills > 0, "{} at {lv}: no kills", p.form);
        }
    }
}

/// Every play of every level's grid, on both kits with a full build: the books must balance.
#[test]
fn every_play_keeps_the_books() {
    use forever::druid::play::Grid;
    let builds = [
        (Kit::Feral, build(&[("Ferocity", 5), ("Furor", 5), ("Sharpened Claws", 2), ("Savage Fury", 2), ("Lacerate", 1), ("Predatory Discipline", 1), ("Shifting Power", 1), ("Primal Bite", 1), ("Bloom", 1), ("Swiftmend", 1)])),
        (Kit::Balance, build(&[("Moonkin Form", 1), ("Insect Swarm", 1), ("Nature's Grace", 1), ("Eclipse", 3), ("Nature's Focus", 5), ("Reflection", 3), ("Bloom", 1), ("Swiftmend", 1)])),
    ];
    for (kit, b) in &builds {
        let d = Data::load(*kit);
        for level in [12, 24, 36, 45, 59] {
            let sc = d.scenario(level, b);
            let g = Grid::new(&sc, *kit == Kit::Balance, b);
            let plays: Vec<Play> = g.indices().iter().filter_map(|ix| g.make(ix)).collect();
            for base in plays.iter().step_by(7) {
                for p in forever::route::Class::taught(&d, base, b, level) {
                    let t = session(&sc, &p, 3, 900.0);
                    assert!(t.seconds >= 900.0 && t.seconds.is_finite(), "{level}: time {t:?} {p:?}");
                    assert!(t.lowest_mana >= -1e-6, "{level}: mana {:.0}, {p:?}", t.lowest_mana);
                    assert!(t.healing + t.resting + t.walking <= t.seconds + 1e-6, "{level}: more time accounted than passed {t:?}");
                    assert!(t.xp == t.kills as f64 * sc.mob.xp);
                }
            }
        }
    }
}

#[test]
fn every_route_keeps_the_rules_and_the_points() {
    use forever::druid::launch::{Style, ROUTES};
    use forever::druid::style::build_of;
    let d = Data::load(Kit::Feral);
    for st in Style::ALL {
        assert_eq!(st.order().len(), 51, "{st:?}");
        assert_eq!(st.after_respec().len(), 51, "{st:?}");
    }
    for r in &ROUTES {
        for (i, &(st, _, from)) in r.legs.iter().enumerate() {
            let order = if i == 0 { st.order() } else { st.after_respec() };
            let upto = r.legs.get(i + 1).map_or(60, |x| x.2 - 1);
            for level in from..=upto {
                assert!(d.tree.legal(&build_of(&order, (level - 9) as usize)), "{}: level {level}", r.name);
            }
        }
    }
}

#[test]
fn lacerate_does_not_cost_a_pull_its_swipes() {
    use forever::druid::launch::Style;
    use forever::druid::peak::rotations;
    let d = Data::load(Kit::Feral);
    let mut b = Build::new();
    for n in Style::BearAoe.order() {
        *b.entry(n.to_string()).or_insert(0) += 1;
    }
    let sc = d.scenario(60, &b);
    let p = rotations(&d, Style::BearAoe, 60, &b).remove(0);
    assert!(p.mobs >= 3 && p.lacerate == 0);
    let (with, without) = (xp_of(&sc, &Play { lacerate: 5, ..p.clone() }), xp_of(&sc, &p));
    assert!(with > without * 0.97, "{with} against {without}");
    // alone with a mob it pays
    let one = Play { mobs: 1, lacerate: 5, ..p };
    let (a, c) = (xp_of(&sc, &one), xp_of(&sc, &Play { lacerate: 0, ..one.clone() }));
    assert!(a > c, "{a} against {c}");
}

#[test]
fn dash_speeds_the_road_until_the_mount() {
    let d = Data::load(Kit::Feral);
    let b = build(&[("Feral Swiftness", 2)]);
    assert_eq!(d.walk_speed(25, &b), 1.3);
    assert!(d.walk_speed(26, &b) > 1.3 && d.walk_speed(26, &b) < 1.35);
    assert_eq!(d.walk_speed(40, &b), 1.6);
}
