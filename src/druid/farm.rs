//! The druid farming mobs of its level, pull after pull, for a session: the
//! port of `levelling.py`'s `cat_farm`, `bear_farm` and `balance_run` (with
//! `_Body`), carried to any level (README *Levelling*, the plays).
//!
//! - **Mobs** of the druid's level, 30 yards apart; a mob swings every 2 s
//!   against the form's armor and dodge (1.12's table) and parries 5% from
//!   the front. Thorns hits back.
//! - **Health and mana.** Between pulls, below the play's threshold, the
//!   druid shifts out (a global cooldown), heals up to its target counting
//!   the HoTs still to tick (Rejuvenation and Regrowth; Healing Touch before
//!   them), and shifts back (its mana; the cat's energy and the bear's rage
//!   lost, Furor gives some back). It eats and drinks when the mana will not
//!   cover the heal. Mana regenerates in any form; health out of combat.
//!   Read Leyline (High Order skyborne) is cast as the druid sits to rest,
//!   when ready (or at each pull, a play's choice): +50% on all its
//!   regeneration for 15 s, drinking, eating, spirit (the share that flows)
//!   and mp5.
//! - **A death** costs two minutes back to the corpse and healing to full
//!   (2026-10-31); the pull in hand earns nothing, the session keeps the rest.
//! - **The cat** walks 30 yards (Feral Swiftness), leaps if talented, and
//!   fights one mob from the front: Rake, Rip or Ferocious Bite at their
//!   combo points, Claw; a mob its bleeds will kill is left to follow.
//! - **The bear** pulls its triangle: the first mob charged (or walked to),
//!   the others in from 10 yards; Enrage, Demoralizing Roar on two or more,
//!   Swipe while enough stand, Primal Bite on its cooldown, Lacerate up to the
//!   play's applications, Maul with the rage to spare.
//! - **The caster** walks on 30 yards after a kill and opens from 30 yards as the mob runs in: Starfire or
//!   Wrath, Moonfire and Insect Swarm kept up, Wrath; Clearcasting on
//!   Starfire. A hit pushes a cast back 0.5 s (twice at most; Nature's Focus
//!   saves it). Below the play's mana it melees with its weapon instead.

use super::launch;
use super::play::{Play, LACERATE_SPAM};
use super::scenario::{Form, Scenario, Sp};
use crate::assumptions as A;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use serde::Serialize;

/// the caster's wait when nothing else is due; the event loops' shortest step
const STEP: f64 = 0.1;
const MIN_STEP: f64 = 0.001;
/// how soon a bear with a free global cooldown looks again while Enrage's rage comes in
const RAGE_LOOK: f64 = 0.5;
/// the last yards of an approach under Prowl (assumption: a mob's aggro range and a margin)
const PROWL_YARDS: f64 = 15.0;
/// summed steps drift below a timer's sum: a timer due within this is due
const EPS: f64 = 1e-9;
const DISTANCE: f64 = 30.0;
const CASTING_RANGE: f64 = 30.0;
const OUTER: f64 = 10.0;
const CAT_SPEED: f64 = 1.0;
const BEAR_SPEED: f64 = 2.5;
const CAT_GCD: f64 = 1.0;
const BEAR_GCD: f64 = 1.5;
const ENERGY_PER_SECOND: f64 = 10.0;
const PUSHBACK: f64 = 0.5;
const PUSHBACK_HITS: u32 = 2;
const MAX_MOBS: usize = 4;

#[derive(Serialize, Clone, Default, Debug)]
pub struct Tally {
    pub seconds: f64,
    pub xp: f64,
    pub kills: u32,
    pub deaths: u32,
    /// seconds healing between pulls, eating and drinking, walking between pulls
    pub healing: f64,
    pub resting: f64,
    pub walking: f64,
    pub taken: f64,
    /// the lowest the mana went: below 0, something was paid for without the mana
    pub lowest_mana: f64,
}

#[derive(Clone, Copy, PartialEq)]
enum Out {
    Miss,
    Dodge,
    Parry,
    Glance,
    Crit,
    Hit,
}

impl Out {
    fn landed(self) -> bool {
        !matches!(self, Out::Miss | Out::Dodge | Out::Parry)
    }
}

struct Hot {
    regrowth: bool,
    next: f64,
    end: f64,
    per: f64,
    period: f64,
}

/// The druid's health and mana through a session.
struct Body<'a> {
    sc: &'a Scenario,
    hp: f64,
    max: f64,
    /// health in caster form: a heal cast there, scaled to the form's
    caster_max: f64,
    mana: f64,
    mana_t: f64,
    last_spend: f64,
    hp_t: f64,
    in_combat: bool,
    hots: Vec<Hot>,
    shift: f64,
    swiftmend_ready: f64,
    swiftness_ready: f64,
    deaths: u32,
    healing: f64,
    resting: f64,
    taken: f64,
    dodged: bool,
    /// Read Leyline's buff, [from, to), and when it is ready again
    ley_from: f64,
    ley_to: f64,
    ley_ready: f64,
    /// Elune's Light: its buff ends, and when it is ready again
    elune_to: f64,
    elune_ready: f64,
    /// Innervate's buff, [from, to), and when it is ready again
    inn_from: f64,
    inn_to: f64,
    inn_ready: f64,
    /// Barkskin up: a swing's damage multiplied by this
    bark: f64,
    lowest_mana: f64,
}

impl<'a> Body<'a> {
    fn new(sc: &'a Scenario, form: &Form) -> Self {
        Body {
            sc,
            hp: form.health,
            max: form.health,
            caster_max: sc.caster.health,
            mana: sc.mana,
            mana_t: 0.0,
            last_spend: -99.0,
            hp_t: 0.0,
            in_combat: false,
            hots: Vec::new(),
            shift: form.shift,
            swiftmend_ready: 0.0,
            swiftness_ready: 0.0,
            deaths: 0,
            healing: 0.0,
            resting: 0.0,
            taken: 0.0,
            dodged: false,
            ley_from: 0.0,
            ley_to: 0.0,
            ley_ready: 0.0,
            elune_to: 0.0,
            elune_ready: 0.0,
            inn_from: 0.0,
            inn_to: 0.0,
            inn_ready: 0.0,
            bark: 1.0,
            lowest_mana: sc.mana,
        }
    }

    /// Innervate at `t`, when ready and known. Whether it was cast.
    fn press_innervate(&mut self, t: f64) -> bool {
        let inn = &self.sc.innervate;
        self.advance_mana(t);
        if !inn.known || t < self.inn_ready || self.mana < inn.cost {
            return false;
        }
        self.spend(t, inn.cost);
        (self.inn_from, self.inn_to) = (t, t + inn.duration);
        self.inn_ready = t + inn.cooldown;
        true
    }

    /// Read Leyline cast at `t` when ready: when the cast ends.
    fn read_leyline(&mut self, t: f64) -> f64 {
        let sc = self.sc;
        // it cannot be cast under Innervate
        if sc.leyline_bonus <= 0.0 || t < self.ley_ready || t < self.inn_to {
            return t;
        }
        let end = t + sc.leyline_cast / sc.haste;
        self.tick(end);
        (self.ley_from, self.ley_to) = (end, end + sc.leyline_seconds);
        self.ley_ready = end + sc.leyline_cooldown;
        end
    }

    /// Elune's Light at a pull's start, when ready (instant, off the global cooldown).
    fn press_elune(&mut self, t: f64) {
        if self.sc.elune_crit > 0.0 && t >= self.elune_ready {
            self.elune_to = t + self.sc.elune_seconds;
            self.elune_ready = t + self.sc.elune_cooldown;
        }
    }

    /// Elune's Light's crit at `t`.
    fn elune(&self, t: f64) -> f64 {
        if t < self.elune_to { self.sc.elune_crit } else { 0.0 }
    }

    fn dead(&self) -> bool {
        self.hp <= 0.0
    }

    /// Read Leyline's multiplier over [a, b): the share of it under the buff.
    fn leyline_over(&self, a: f64, b: f64) -> f64 {
        if b <= a {
            return 1.0;
        }
        1.0 + self.sc.leyline_bonus * self.cast_share(a, b)
    }

    /// The share of [a, b) under the cast Read Leyline's buff.
    fn cast_share(&self, a: f64, b: f64) -> f64 {
        if b <= a {
            return 0.0;
        }
        (self.ley_to.min(b) - self.ley_from.max(a)).max(0.0) / (b - a)
    }

    /// Seconds to regain `need` at `rate` from `t`, the rate lifted by Read Leyline (on
    /// `lifted` of it) while its buff lasts and by `inn` a second while Innervate's does.
    fn regain(&self, t: f64, need: f64, rate: f64, lifted: f64, inn: f64) -> f64 {
        if need <= 0.0 {
            return 0.0;
        }
        let (mut now, mut left) = (t, need);
        let mut cuts = [self.ley_from, self.ley_to, self.inn_to, f64::INFINITY];
        cuts.sort_by(f64::total_cmp);
        for cut in cuts {
            if cut <= now {
                continue;
            }
            let ley = if now >= self.ley_from && now < self.ley_to { self.sc.leyline_bonus * lifted } else { 0.0 };
            let r = rate + ley + if now < self.inn_to { inn } else { 0.0 };
            if cut.is_infinite() || left <= r * (cut - now) {
                return now + left / r - t;
            }
            left -= r * (cut - now);
            now = cut;
        }
        now - t
    }

    /// Mana up to `t`: spirit regeneration out of the five-second rule
    /// (Reflection's share inside it), mp5 always; Read Leyline on spirit.
    fn advance_mana(&mut self, t: f64) {
        if t <= self.mana_t {
            return;
        }
        let sc = self.sc;
        let full = (self.last_spend + A::FIVE_SECOND_RULE).max(self.mana_t);
        let (a, b) = (self.mana_t, t);
        let inside = (full.min(b) - a).max(0.0);
        let outside = (b - full.max(a)).max(0.0);
        let ley = self.leyline_over(a, b);
        let mut spirit = sc.regen * ley * (outside + inside * sc.reflection);
        // Innervate: all of it flows while casting, and +400%
        let under = (self.inn_to.min(b) - self.inn_from.max(a)).max(0.0);
        if under > 0.0 {
            let flowing = (outside + inside * sc.reflection) / (b - a);
            spirit += sc.regen * under * (1.0 + sc.innervate.extra - flowing);
        }
        // the cast buff lifts mp5 too, as the last factor on all of it (2026-10-28)
        let mp5 = sc.mp5 / 5.0 * (b - a) * (1.0 + sc.leyline_bonus * self.cast_share(a, b));
        self.mana = (self.mana + spirit + mp5).min(sc.mana);
        self.mana_t = t;
    }

    fn spend(&mut self, t: f64, cost: f64) {
        self.advance_mana(t);
        if cost > 0.0 {
            self.mana -= cost;
            self.lowest_mana = self.lowest_mana.min(self.mana);
            self.last_spend = t;
        }
    }

    /// HoT ticks up to `t`, mana, and health out of combat.
    fn tick(&mut self, t: f64) {
        if t > self.hp_t {
            if !self.in_combat && self.hp > 0.0 {
                let gain = self.sc.hp_regen * self.leyline_over(self.hp_t, t) * (t - self.hp_t);
                self.hp = (self.hp + gain).min(self.max);
            }
            self.hp_t = t;
        }
        for h in &mut self.hots {
            while h.next <= t.min(h.end) + 1e-9 {
                self.hp = (self.hp + h.per).min(self.max);
                h.next += h.period;
            }
        }
        let (before, bloom) = (self.hots.len(), self.sc.bloom * self.max / self.caster_max);
        self.hots.retain(|h| h.next <= h.end + 1e-9);
        // a HoT run out: Bloom heals for the druid's spirit
        self.hp = (self.hp + bloom * (before - self.hots.len()) as f64).min(self.max);
        self.advance_mana(t);
    }

    fn pending(&self) -> f64 {
        self.hots.iter().map(|h| h.per * (((h.end - h.next) / h.period + 1e-9).floor() + 1.0)).sum()
    }

    /// A mob's swing at the druid in a form with `armor` and `dodge`.
    fn swing(&mut self, rng: &mut StdRng, armor: f64, dodge: f64, roar_ap: f64, swarm: bool) -> f64 {
        let sc = self.sc;
        let r = rng.gen::<f64>() * 100.0;
        let miss = A::BOSS_MISS + if swarm { sc.swarm_miss } else { 0.0 };
        self.dodged = miss <= r && r < miss + dodge;
        if r < miss + dodge {
            return 0.0;
        }
        let k = armor.max(0.0);
        let red = (k / (k + A::ARMOR_K.0 + A::ARMOR_K.1 * sc.mob.level as f64)).min(A::ARMOR_CAP);
        let mut hit = (sc.mob.dps - roar_ap / 14.0).max(0.0) * A::MOB_SWING * (1.0 - red) * self.bark;
        if r < miss + dodge + A::BOSS_CRIT {
            hit *= A::MELEE_CRIT_MULTIPLIER;
        }
        self.hp -= hit;
        self.taken += hit;
        hit
    }

    fn die(&mut self, t: f64) -> f64 {
        self.deaths += 1;
        let t = t + A::DRUID_DEATH_SECONDS;
        self.hp = self.max;
        self.hots.clear();
        self.in_combat = false;
        self.hp_t = t;
        self.mana = self.sc.mana;
        self.mana_t = t;
        self.last_spend = -99.0;
        t
    }

    /// A HoT's ticks crit (Blizzard's notes, 2026-10-01): their average with the caster's chance.
    fn hot_crit(&self, extra: f64) -> f64 {
        1.0 + (self.sc.spell_crit + extra).clamp(0.0, 100.0) / 100.0 * (self.sc.heal_crit_mult - 1.0)
    }

    /// Rejuvenation lands at `t` (instant): its HoT, cast in caster form, scaled to the form's health.
    fn rejuvenate(&mut self, t: f64) {
        let sc = self.sc;
        let rj = &sc.rejuvenation;
        let per = (rj.tick + rj.tick_coef * sc.healing) * sc.periodic_mult * rj.extra * self.max / self.caster_max * self.hot_crit(0.0);
        self.hots.push(Hot { regrowth: false, next: t + rj.period, end: t + rj.duration, per, period: rj.period });
    }

    /// Regrowth (or Healing Touch before it) lands at `t`.
    fn direct_heal(&mut self, t: f64, rng: &mut StdRng) {
        let sc = self.sc;
        let scale = self.max / self.caster_max;
        let crit = |rng: &mut StdRng, extra: f64| if rng.gen::<f64>() * 100.0 < sc.spell_crit + extra { sc.heal_crit_mult } else { 1.0 };
        let rg = &sc.regrowth;
        if rg.known {
            let direct = (rg.base + rng.gen_range(-1.0..=1.0) * rg.spread + rg.coef * sc.healing) * rg.extra * crit(rng, sc.regrowth_crit + self.elune(t)) * scale;
            self.hp = (self.hp + direct).min(self.max);
            self.hots.retain(|h| !h.regrowth);
            let per = (rg.tick + rg.tick_coef * sc.healing) * sc.periodic_mult * rg.extra * scale * self.hot_crit(sc.regrowth_crit);
            self.hots.push(Hot { regrowth: true, next: t + rg.period, end: t + rg.duration, per, period: rg.period });
        } else {
            let ht = &sc.healing_touch;
            let amount = (ht.base + rng.gen_range(-1.0..=1.0) * ht.spread + ht.coef * sc.healing) * ht.extra * crit(rng, self.elune(t)) * scale;
            self.hp = (self.hp + amount).min(self.max);
        }
    }

    /// Eat and drink until health and mana are full, Read Leyline cast first
    /// when it is ready and saves more than its cast.
    fn rest(&mut self, mut t: f64) -> f64 {
        let sc = self.sc;
        self.advance_mana(t);
        let hp = (self.max - self.hp) / self.max * self.caster_max;
        // Innervate on a long drink: +400% of spirit regeneration for its 20 s
        if sc.innervate.known && (sc.mana - self.mana) > 0.3 * sc.mana {
            self.press_innervate(t);
        }
        let mana = sc.mana - self.mana;
        let flow = sc.drink + sc.regen + sc.mp5 / 5.0;
        let inn = sc.regen * sc.innervate.extra;
        let wait = |b: &Self, t: f64| b.regain(t, hp, sc.food, sc.food, 0.0).max(b.regain(t, mana, flow, flow, inn));
        let mut w = wait(self, t);
        // it cannot be cast under Innervate
        if sc.leyline_bonus > 0.0 && t >= self.ley_ready && t >= self.inn_to {
            let cast = sc.leyline_cast / sc.haste;
            let (from, to) = (self.ley_from, self.ley_to);
            (self.ley_from, self.ley_to) = (t + cast, t + cast + sc.leyline_seconds);
            let lifted = wait(self, t + cast);
            if cast + lifted < w {
                self.ley_ready = t + cast + sc.leyline_cooldown;
                self.resting += cast;
                t += cast;
                w = lifted;
            } else {
                (self.ley_from, self.ley_to) = (from, to);
            }
        }
        let wait = w;
        self.resting += wait;
        let t = t + wait;
        self.hp = self.max;
        self.mana = sc.mana;
        self.mana_t = t;
        self.hp_t = t;
        self.last_spend = -99.0;
        self.hots.retain(|h| h.end > t);
        t
    }

    /// Between pulls: heal when below `below` (shifting out of a form and,
    /// with `back`, into it again), eat and drink when the mana will not
    /// cover it or `mana_need`. Returns when it ends and whether the druid
    /// left its form.
    fn recover(&mut self, mut t: f64, below: f64, to: f64, shapeshifted: bool, mana_need: f64, rng: &mut StdRng) -> (f64, bool) {
        let sc = self.sc;
        self.tick(t);
        let low = self.hp / self.max < below;
        if !low && self.mana >= mana_need {
            return (t, false);
        }
        let start = t;
        let gcd = (A::SPELL_GCD / sc.haste).max(A::MIN_GCD);
        if low {
            if shapeshifted {
                t += gcd;
            }
            let scale = self.max / self.caster_max;
            let crit = |rng: &mut StdRng, extra: f64| if rng.gen::<f64>() * 100.0 < sc.spell_crit + extra { sc.heal_crit_mult } else { 1.0 };
            while (self.hp + self.pending()) / self.max < to {
                let rejuv_up = self.hots.iter().any(|h| !h.regrowth);
                let (rj, rg, ht) = (&sc.rejuvenation, &sc.regrowth, &sc.healing_touch);
                // Swiftmend consumes the Rejuvenation: worth it on one about to run out
                let ending = self.hots.iter().any(|h| !h.regrowth && h.end - h.next < h.period);
                let mend = sc.swiftmend.known && ending && t >= self.swiftmend_ready;
                // Rejuvenation first; then Swiftmend, Regrowth, or Healing Touch before Regrowth
                let (kind, cost) = if rj.known && !rejuv_up {
                    (0, rj.cost)
                } else if mend {
                    (1, sc.swiftmend.cost)
                } else if rg.known {
                    (2, rg.cost)
                } else {
                    (3, ht.cost)
                };
                if self.mana < cost + if shapeshifted { self.shift } else { 0.0 } {
                    t = self.rest(t);
                    break;
                }
                self.spend(t, cost);
                match kind {
                    0 => {
                        self.rejuvenate(t);
                        t += gcd;
                    }
                    1 => {
                        // the whole Rejuvenation at once, consumed (its tooltip, 2026-10-30)
                        let ticks = (rj.duration / rj.period).round();
                        let amount = (rj.tick + rj.tick_coef * sc.healing) * rj.extra * ticks * crit(rng, self.elune(t)) * scale;
                        self.hp = (self.hp + amount).min(self.max);
                        self.hots.retain(|h| h.regrowth);
                        self.swiftmend_ready = t + sc.swiftmend.cooldown;
                        t += gcd;
                    }
                    2 => {
                        let mut cast = rg.cast / sc.haste;
                        if sc.swiftness.known && t >= self.swiftness_ready {
                            cast = 0.0;
                            self.swiftness_ready = t + sc.swiftness.cooldown;
                        }
                        t += cast.max(gcd);
                        self.direct_heal(t, rng);
                    }
                    _ => {
                        t += (ht.cast / sc.haste).max(gcd);
                        self.direct_heal(t, rng);
                    }
                }
                self.tick(t);
            }
            if shapeshifted {
                // back into form only with the mana for it
                if self.mana < self.shift {
                    t = self.rest(t);
                }
                self.spend(t, self.shift);
                t += gcd;
            }
            self.healing += t - start;
        }
        if self.mana < mana_need {
            t = self.rest(t);
        }
        self.tick(t);
        (t, low && shapeshifted)
    }
}

/// Faerie Fire on one mob. Whether the play casts it and whether it is on are two
/// things: only `on` cuts armor.
#[derive(Clone, Copy)]
struct FaerieFire {
    wanted: bool,
    on: bool,
}

impl FaerieFire {
    fn new(sc: &Scenario, p: &Play) -> Self {
        FaerieFire { wanted: p.faerie_fire && sc.faerie_fire.known, on: false }
    }

    /// Cast it now, if the play wants it, it is not on and the mana is there.
    fn cast(&mut self, body: &mut Body, t: f64) -> bool {
        if !self.wanted || self.on || body.mana < body.sc.faerie_fire.cost {
            return false;
        }
        body.spend(t, body.sc.faerie_fire.cost);
        self.on = true;
        true
    }
}

struct Omen {
    ready: f64,
    up: bool,
}

impl Omen {
    /// What an ability costs now: nothing under Clearcasting, which it uses up. Every
    /// ability that consumes Clearcasting pays through here.
    fn pay(&mut self, cost: f64) -> f64 {
        if std::mem::take(&mut self.up) { 0.0 } else { cost }
    }

    fn roll(&mut self, sc: &Scenario, rng: &mut StdRng, t: f64, interval: f64, mult: f64) {
        if sc.omen && t >= self.ready && rng.gen::<f64>() < sc.omen_ppm * mult * interval / 60.0 {
            self.up = true;
            self.ready = t + sc.omen_icd;
        }
    }
}

/// The melee table against a mob of the level from the front.
fn melee_roll(sc: &Scenario, rng: &mut StdRng, crit: f64, white: bool) -> Out {
    let mut r = rng.gen::<f64>();
    for (o, chance) in [
        (Out::Miss, (A::MELEE_MISS - sc.melee_hit).max(0.0) / 100.0),
        (Out::Dodge, A::MOB_DODGE / 100.0),
        (Out::Parry, A::MOB_PARRY / 100.0),
        (Out::Glance, if white { A::GLANCE_BASE / 100.0 } else { 0.0 }),
        (Out::Crit, crit.max(0.0) / 100.0),
    ] {
        if r < chance {
            return o;
        }
        r -= chance;
    }
    Out::Hit
}

fn melee_mult(sc: &Scenario, o: Out) -> f64 {
    match o {
        Out::Miss | Out::Dodge | Out::Parry => 0.0,
        Out::Glance => A::GLANCE_DAMAGE,
        Out::Crit => sc.melee_crit_mult,
        Out::Hit => 1.0,
    }
}

/// A DoT's ticks crit (Blizzard's notes, 2026-10-01; bleeds do not): their average.
fn dot_crit(sc: &Scenario, crit: f64) -> f64 {
    1.0 + crit.clamp(0.0, 100.0) / 100.0 * (sc.spell_crit_mult - 1.0)
}

/// The share of a mob's swings that land on the druid.
fn landed_share(sc: &Scenario, dodge: f64, swarm: bool) -> f64 {
    1.0 - (A::BOSS_MISS + if swarm { sc.swarm_miss } else { 0.0 } + dodge).clamp(0.0, 100.0) / 100.0
}

/// a weapon's normal damage in a form: its DPS and the attack power over the swing speed
fn normal(sc: &Scenario, ap: f64, speed: f64) -> f64 {
    (sc.weapon_dps + ap / 14.0) * speed
}

/// armor's cut to the druid's melee against the mob, and Naturalist
fn mitig(sc: &Scenario, ff: Option<&FaerieFire>) -> f64 {
    let k = (sc.mob.armor - if ff.is_some_and(|f| f.on) { sc.faerie_fire.extra } else { 0.0 }).max(0.0);
    let red = (k / (k + A::ARMOR_K.0 + A::ARMOR_K.1 * sc.level as f64)).min(A::ARMOR_CAP);
    (1.0 - red) * sc.damage_mult
}

fn thorns(sc: &Scenario) -> f64 {
    if sc.thorns.known {
        (sc.thorns.base + A::THORNS_SPELL_POWER * sc.spell_power) * sc.damage_mult
    } else {
        0.0
    }
}

/// Seconds to the next pull at `speed`, and when the charge is ready again:
/// walk into range, then charge if it is ready, else walk the rest.
fn approach(distance: f64, speed: f64, t: f64, charge: &Sp, ready: f64) -> (f64, f64) {
    if charge.known && distance > 0.0 {
        let run = (distance - charge.reach).max(0.0) / speed;
        if distance >= charge.extra && t + run >= ready {
            return (run, t + run + charge.cooldown);
        }
    }
    (distance / speed, ready)
}

struct Bleed {
    next: f64,
    end: f64,
    per: f64,
    period: f64,
}

fn pending(bs: &[Bleed]) -> f64 {
    bs.iter().filter(|b| b.next <= b.end + 1e-9).map(|b| b.per * (((b.end - b.next) / b.period + 1e-9).floor() + 1.0)).sum()
}

/// A mob left to its bleeds or DoTs: when it dies, its next swing.
struct Chaser {
    dies: f64,
    swing: f64,
}

fn chase(body: &mut Body, rng: &mut StdRng, chasers: &mut [Chaser], until: f64, armor: f64, dodge: f64) {
    for ch in chasers.iter_mut() {
        while ch.swing <= until.min(ch.dies) && !body.dead() {
            body.swing(rng, armor, dodge, 0.0, false);
            ch.swing += A::MOB_SWING;
        }
    }
    if !chasers.iter().any(|c| c.dies > until) {
        body.in_combat = false;
    }
}

pub fn session(sc: &Scenario, p: &Play, seed: u64, seconds: f64) -> Tally {
    let mut rng = StdRng::seed_from_u64(seed);
    match p.form.as_str() {
        "cat" if sc.cat.known => cat(sc, p, &mut rng, seconds),
        "bear" if sc.bear.known => bear(sc, p, &mut rng, seconds),
        _ if p.hurricane > 0 && sc.hurricane.known => storm(sc, p, &mut rng, seconds),
        _ => caster(sc, p, &mut rng, seconds),
    }
}

// --- cat -----------------------------------------------------------------------

/// An energy cost with Poised's discount, while it lasts.
fn less(cost: f64, poised: f64) -> f64 {
    (cost - poised).max(0.0)
}

fn cat(sc: &Scenario, p: &Play, rng: &mut StdRng, until: f64) -> Tally {
    let form = &sc.cat;
    let mut body = Body::new(sc, form);
    let (armor, dodge) = (form.armor, form.dodge);
    let mut tally = Tally::default();
    let mut t = 0.0;
    let mut energy = 100.0;
    let (mut swing_at, mut gcd_at) = (0.0f64, 0.0f64);
    let mut omen = Omen { ready: 0.0, up: false };
    let speed = A::RUN_SPEED * (1.0 + (sc.cat_speed + sc.run_speed) / 100.0);
    let mut leap_ready = 0.0;
    let (mut shifting_ready, mut prowl_ready) = (0.0f64, 0.0f64);
    let shifting_reserve = sc.regrowth.cost + sc.rejuvenation.cost + form.shift + sc.shifting.cost;
    let mut chasers: Vec<Chaser> = Vec::new();
    let thorn = thorns(sc);
    let haste = sc.haste;
    let stealth = (p.opener == 1 && sc.pounce.known) || (p.opener == 2 && sc.ravage.known) || (p.opener == 3 && sc.shred.known && sc.prowl.known);
    let holds = p.bite_pool > 0.0 && sc.bite.known && p.bite_cp > 0;
    let gain = |e: f64, dt: f64| (e + ENERGY_PER_SECOND * dt).min(100.0);
    while t < until {
        if tally.kills > 0 {
            let (t_was, energy_was) = (t, energy);
            let (t2, shifted) = body.recover(t, p.heal_below, p.heal_to, true, 0.0, rng);
            t = t2;
            chase(&mut body, rng, &mut chasers, t, armor, dodge);
            energy = if shifted { (100.0 * sc.furor_energy).min(sc.furor_energy * (energy_was + ENERGY_PER_SECOND * (t - t_was))) } else { gain(energy, t - t_was) };
            let (mut way, ready) = approach(DISTANCE, speed, t, &sc.cat_leap, leap_ready);
            leap_ready = ready;
            if stealth {
                // out of combat first, Prowl ready, then the last yards at its pace
                let free = chasers.iter().map(|c| c.dies).fold(t, f64::max).max(prowl_ready);
                way = (free - t).max(way - PROWL_YARDS / speed) + PROWL_YARDS / (speed * sc.prowl.extra);
            }
            energy = gain(energy, way);
            chase(&mut body, rng, &mut chasers, t + way, armor, dodge);
            body.tick(t + way);
            t += way;
            tally.walking += way;
            swing_at = swing_at.max(t);
            gcd_at = gcd_at.max(t);
            chasers.retain(|c| c.dies > t);
        }
        if body.dead() {
            t = body.die(t);
            chasers.clear();
            continue;
        }
        let mut hp = sc.mob.health;
        let mut ff = FaerieFire::new(sc, p);
        let (mut rake_until, mut rip_until) = (-1.0f64, -1.0f64);
        // Poised: full energy and nothing on the druid; the next ability, Omen's free one too, takes it
        let mut poised = if chasers.is_empty() && energy >= 100.0 - EPS { sc.poised } else { 0.0 };
        let mut bleeds: Vec<Bleed> = Vec::new();
        let mut cp = 0u32;
        let mut mob_at = t + rng.gen_range(0.0..A::MOB_SWING);
        body.press_elune(t);
        body.tick(t);
        if stealth {
            // pooled in Prowl; the opener lands from behind, unseen
            let sp = match p.opener { 1 => &sc.pounce, 2 => &sc.ravage, _ => &sc.shred };
            let cost = less(sp.cost, poised);
            poised = 0.0;
            if energy < cost {
                let wait = (cost - energy) / ENERGY_PER_SECOND;
                body.tick(t + wait);
                t += wait;
                energy = cost;
                tally.walking += wait;
            }
            energy -= cost;
            let crit = form.crit + body.elune(t);
            if p.opener == 1 {
                let per = sp.tick * sc.damage_mult * sc.periodic_mult;
                bleeds.push(Bleed { next: t + sp.period, end: t + sp.duration, per, period: sp.period });
                mob_at = t + launch::POUNCE_STUN + rng.gen_range(0.0..A::MOB_SWING);
            } else {
                let o = melee_roll(sc, rng, crit, false);
                hp -= (normal(sc, form.attack_power, CAT_SPEED) * sp.pct + sp.base) * melee_mult(sc, o) * mitig(sc, Some(&ff));
                mob_at = t + A::MOB_SWING;
            }
            cp = 1;
            prowl_ready = t + sc.prowl.cooldown;
            gcd_at = t + CAT_GCD;
        }
        body.in_combat = true;
        while hp > 0.0 && !body.dead() {
            for b in &mut bleeds {
                while b.next <= t.min(b.end) + EPS && hp > 0.0 {
                    hp -= b.per;
                    b.next += b.period;
                }
            }
            if hp <= 0.0 {
                break;
            }
            if p.drag && !stealth && hp <= pending(&bleeds) {
                // its bleeds finish it: on to the next while it follows, hitting
                let mut last = t;
                for b in &mut bleeds {
                    while b.next <= b.end + EPS && hp > 0.0 {
                        hp -= b.per;
                        last = last.max(b.next);
                        b.next += b.period;
                    }
                }
                chasers.push(Chaser { dies: last, swing: mob_at });
                break;
            }
            body.tick(t);
            if t + EPS >= mob_at {
                if body.swing(rng, armor, dodge, 0.0, false) > 0.0 {
                    hp -= thorn;
                }
                mob_at += A::MOB_SWING;
            }
            chase(&mut body, rng, &mut chasers, t, armor, dodge);
            let crit = form.crit + body.elune(t);
            let m = mitig(sc, Some(&ff));
            let torn = if t < rake_until || t < rip_until { sc.rend } else { 1.0 };
            if t + EPS >= swing_at {
                let o = melee_roll(sc, rng, crit, true);
                hp -= normal(sc, form.attack_power, CAT_SPEED) * melee_mult(sc, o) * m;
                if o.landed() {
                    omen.roll(sc, rng, t, CAT_SPEED / haste, 1.0);
                }
                swing_at += CAT_SPEED / haste;
            }
            let ripping = p.rip_cp > 0 && sc.rip.known;
            // the energy the next ability waits for, when nothing is affordable
            let mut wants = f64::INFINITY;
            if hp > 0.0 && t + EPS >= gcd_at {
                let free = omen.up;
                let combo = |rng: &mut StdRng, o: Out| if o == Out::Crit && sc.frenzy_cp > 0.0 && rng.gen::<f64>() < sc.frenzy_cp { 2 } else { 1 };
                let biting = p.bite_cp > 0 && sc.bite.known && cp >= p.bite_cp && (!ripping || t < rip_until);
                let (rip_cost, bite_cost, rake_cost, claw_cost) = (less(sc.rip.cost, poised), less(sc.bite.cost, poised), less(sc.rake.cost, poised), less(sc.claw.cost, poised));
                let bite_at = bite_cost.max(p.bite_pool);
                if ff.cast(&mut body, t) {
                    gcd_at = t + CAT_GCD;
                } else if ripping && cp >= p.rip_cp && t >= rip_until {
                    if free || energy >= rip_cost {
                        energy -= omen.pay(rip_cost);
                        poised = 0.0;
                        omen.roll(sc, rng, t, 1.5, 1.0);
                        let o = melee_roll(sc, rng, crit, false);
                        if o.landed() {
                            let dur = sc.rip.duration;
                            let mut per = sc.rip.base + sc.rip.extra * cp as f64;
                            per += A::RIP_AP_AT_5 * launch::CAT_SCALE * cp as f64 / 5.0 * form.attack_power / (dur / sc.rip.period);
                            bleeds.push(Bleed { next: t + sc.rip.period, end: t + dur, per: per * sc.damage_mult * sc.periodic_mult, period: sc.rip.period });
                            rip_until = t + dur;
                            cp = 0;
                        }
                        gcd_at = t + CAT_GCD;
                    } else {
                        wants = rip_cost;
                    }
                } else if biting && (energy >= bite_at || (free && !holds)) {
                    energy -= omen.pay(bite_cost);
                    poised = 0.0;
                    omen.roll(sc, rng, t, 1.5, 1.0);
                    let o = melee_roll(sc, rng, crit, false);
                    if o.landed() {
                        let roll = rng.gen_range(-1.0..=1.0) * sc.bite.spread;
                        let dmg = sc.bite.base + sc.bite.extra * cp as f64 + roll + A::BITE_AP_PER_CP * launch::CAT_SCALE * cp as f64 * form.attack_power + sc.bite_energy * energy;
                        hp -= dmg * melee_mult(sc, o) * m * torn;
                        cp = 0;
                        // every extra point spent, unless Predatory Discipline keeps them
                        if !sc.discipline {
                            energy = 0.0;
                        }
                    }
                    gcd_at = t + CAT_GCD;
                } else if biting && (holds || cp == 5) {
                    // the combo points are ready: pool for the Bite, nothing else spends
                    wants = bite_at;
                } else if sc.rake.known && t >= rake_until && (free || energy >= rake_cost) {
                    energy -= omen.pay(rake_cost);
                    poised = 0.0;
                    let o = melee_roll(sc, rng, crit, false);
                    if o.landed() {
                        hp -= sc.rake.base * sc.savage * melee_mult(sc, o) * m * torn;
                        let per = sc.rake.tick * sc.savage * sc.damage_mult * sc.periodic_mult;
                        bleeds.push(Bleed { next: t + sc.rake.period, end: t + sc.rake.duration, per, period: sc.rake.period });
                        rake_until = t + sc.rake.duration;
                        cp = (cp + combo(rng, o)).min(5);
                    }
                    omen.roll(sc, rng, t, 1.5, 1.0);
                    gcd_at = t + CAT_GCD;
                } else if sc.claw.known && (free || energy >= claw_cost) {
                    energy -= omen.pay(claw_cost);
                    poised = 0.0;
                    let o = melee_roll(sc, rng, crit, false);
                    if o.landed() {
                        let dmg = (normal(sc, form.attack_power, CAT_SPEED) * sc.claw.pct + sc.claw.base) * sc.savage;
                        hp -= dmg * melee_mult(sc, o) * m * torn;
                        cp = (cp + combo(rng, o)).min(5);
                    }
                    omen.roll(sc, rng, t, 1.5, 1.0);
                    gcd_at = t + CAT_GCD;
                } else if p.shifting && sc.shifting.known && t >= shifting_ready && energy + sc.shifting.extra <= 100.0 && body.mana >= shifting_reserve {
                    body.spend(t, sc.shifting.cost);
                    energy += sc.shifting.extra;
                    shifting_ready = t + sc.shifting.cooldown;
                    gcd_at = t + CAT_GCD;
                } else {
                    wants = if sc.rake.known && t >= rake_until { claw_cost.min(rake_cost) } else { claw_cost };
                }
            }
            // on to the next thing that can happen
            let mut next = swing_at.min(mob_at);
            if gcd_at > t + EPS {
                next = next.min(gcd_at);
            } else if wants.is_finite() {
                next = next.min(t + ((wants - energy) / ENERGY_PER_SECOND).max(MIN_STEP));
                if p.shifting && sc.shifting.known && shifting_ready > t {
                    next = next.min(shifting_ready);
                }
            }
            for b in &bleeds {
                if b.next <= b.end + EPS {
                    next = next.min(b.next);
                }
            }
            for c in &chasers {
                if c.dies > t {
                    next = next.min(c.swing.max(t + MIN_STEP));
                }
            }
            let next = next.max(t + MIN_STEP);
            energy = gain(energy, next - t);
            t = next;
        }
        if body.dead() {
            t = body.die(t);
            chasers.clear();
            continue;
        }
        tally.kills += 1;
        body.in_combat = chasers.iter().any(|c| c.dies > t);
    }
    tally.seconds = chasers.iter().map(|c| c.dies).fold(t, f64::max);
    finish(tally, &body, sc)
}

fn finish(mut tally: Tally, body: &Body, sc: &Scenario) -> Tally {
    tally.deaths = body.deaths;
    tally.healing = body.healing;
    tally.resting = body.resting;
    tally.taken = body.taken;
    tally.lowest_mana = body.lowest_mana;
    tally.xp = tally.kills as f64 * sc.mob.xp;
    tally
}

// --- bear ----------------------------------------------------------------------

fn bear(sc: &Scenario, p: &Play, rng: &mut StdRng, until: f64) -> Tally {
    let form = &sc.bear;
    let mut body = Body::new(sc, form);
    let mut tally = Tally::default();
    let n = p.mobs.clamp(1, MAX_MOBS);
    let mut t = 0.0;
    let thorn = thorns(sc);
    let haste = sc.haste;
    let mut omen = Omen { ready: 0.0, up: false };
    let mut rage = sc.furor_rage;
    let (mut enrage_ready, mut enrage_until) = (0.0f64, -1.0f64);
    let enrage_rate = if sc.enrage.known { sc.enrage.base / sc.enrage.duration } else { 0.0 };
    let (mut swing_at, mut gcd_at, mut bite_at) = (0.0f64, 0.0f64, 0.0f64);
    let mut maul_queued = false;
    let mut charge_ready = 0.0;
    let (mut frenzied_ready, mut frenzied_next, mut frenzied_end) = (0.0f64, f64::INFINITY, -1.0f64);
    let speed = A::RUN_SPEED * (1.0 + sc.run_speed / 100.0);
    let frenzied_rage = |rng: &mut StdRng, o: Out| {
        if o != Out::Crit || sc.frenzy_rage <= 0.0 {
            0.0
        } else if sc.frenzy_rage >= 1.0 || rng.gen::<f64>() < sc.frenzy_rage {
            sc.frenzy_amount
        } else {
            0.0
        }
    };
    let swipe_dmg = (sc.swipe.base + A::SWIPE_AP * form.attack_power) * sc.swipe_mult * sc.savage;
    // an emergency mid-pull: out of form (until `out_until`), Rejuvenation, then Regrowth or
    // Healing Touch landing at `heal_at`, back into form
    let gcd = (A::SPELL_GCD / haste).max(A::MIN_GCD);
    let main_heal = if sc.regrowth.known { &sc.regrowth } else { &sc.healing_touch };
    let emergency_cost = main_heal.cost + if sc.rejuvenation.known { sc.rejuvenation.cost } else { 0.0 } + form.shift;
    let (mut out_until, mut heal_at) = (-1.0f64, f64::INFINITY);
    let lacerating = p.lacerate.min(launch::LACERATE_STACKS);
    let mut last = 0.0f64;
    while t < until {
        if tally.kills > 0 {
            let was = t;
            let (t2, shifted) = body.recover(t, p.heal_below, p.heal_to, true, 0.0, rng);
            t = t2;
            if shifted {
                rage = sc.furor_rage;
            }
            // a charge costs its rage: without it the bear walks
            let no_charge = Sp::default();
            let charge = if rage >= sc.bear_charge.cost { &sc.bear_charge } else { &no_charge };
            let (way, ready) = approach(DISTANCE, speed, t, charge, charge_ready);
            if ready != charge_ready {
                rage -= sc.bear_charge.cost;
            }
            charge_ready = ready;
            let way = way.max(BEAR_GCD);
            body.tick(t + way);
            t += way;
            rage = (rage - A::RAGE_DECAY * (t - was)).max(0.0);
            tally.walking += way;
            swing_at = swing_at.max(t);
            gcd_at = gcd_at.max(t);
        }
        if body.dead() {
            t = body.die(t);
            continue;
        }
        let mut hp = [sc.mob.health; MAX_MOBS];
        hp[n..].fill(0.0);
        // Faerie Fire on each mob (a global cooldown a mob; the first pulled with it)
        let mut ff = [FaerieFire::new(sc, p); MAX_MOBS];
        ff[0].cast(&mut body, t);
        let arrive: [f64; MAX_MOBS] = std::array::from_fn(|i| if i == 0 { t } else { t + OUTER / A::MOB_RUN_SPEED });
        let mut mob_at: [f64; MAX_MOBS] = std::array::from_fn(|i| arrive[i] + rng.gen_range(0.0..A::MOB_SWING));
        let mut roar_until = -1.0f64;
        // Lacerate on each mob: applications, its next tick, its end
        let mut lac = [(0u32, 0.0f64, -1.0f64); MAX_MOBS];
        body.press_elune(t);
        if p.enrage && sc.enrage.known && t >= enrage_ready {
            rage = (rage + sc.enrage.extra).min(A::RAGE_CAP);
            enrage_until = t + sc.enrage.duration;
            enrage_ready = t + sc.enrage.cooldown;
        }
        body.tick(t);
        body.in_combat = true;
        while hp.iter().any(|&h| h > 0.0) && !body.dead() {
            body.tick(t);
            // Enrage's rage since the last event
            if last < enrage_until {
                rage = (rage + enrage_rate * (t.min(enrage_until) - last).max(0.0)).min(A::RAGE_CAP);
            }
            last = t;
            for i in 0..n {
                let (stacks, next, end) = &mut lac[i];
                while *stacks > 0 && *next <= t.min(*end) + EPS && hp[i] > 0.0 {
                    hp[i] -= sc.lacerate.tick * *stacks as f64 * sc.damage_mult * sc.periodic_mult;
                    *next += sc.lacerate.period;
                }
            }
            // Frenzied Regeneration's second: rage into health
            while frenzied_next <= t + EPS {
                let used = rage.min(sc.frenzied.base);
                rage -= used;
                body.hp = (body.hp + used * sc.frenzied.extra * body.max).min(body.max);
                frenzied_next = if frenzied_next + 1.0 <= frenzied_end + EPS { frenzied_next + 1.0 } else { f64::INFINITY };
            }
            let out = t < out_until;
            // Enrage cuts the base (agility's) armor, as `levelling.py`
            let armor = if out { sc.caster.armor } else { form.armor - if t < enrage_until { sc.enrage_armor * sc.base_armor } else { 0.0 } };
            let dodge = if out { sc.caster.dodge } else { form.dodge };
            let roar_ap = if t < roar_until { sc.roar.extra } else { 0.0 };
            if t + EPS >= heal_at {
                body.direct_heal(t, rng);
                body.spend(t, form.shift);
                heal_at = f64::INFINITY;
            }
            if !out && p.frenzied > 0.0 && sc.frenzied.known && body.hp / body.max < p.frenzied && t >= frenzied_ready && rage >= sc.frenzied.base && t + EPS >= gcd_at {
                frenzied_ready = t + sc.frenzied.cooldown;
                frenzied_next = t + 1.0;
                frenzied_end = t + sc.frenzied.duration;
                gcd_at = t + BEAR_GCD;
            }
            if !out && p.emergency > 0.0 && body.hp / body.max < p.emergency && t + EPS >= gcd_at && body.mana >= emergency_cost {
                // out of form, Rejuvenation, the direct heal, back in
                let mut at = t + gcd;
                if sc.rejuvenation.known && !body.hots.iter().any(|h| !h.regrowth) {
                    body.spend(at, sc.rejuvenation.cost);
                    body.rejuvenate(at);
                    at += gcd;
                }
                body.spend(at, main_heal.cost);
                heal_at = at + (main_heal.cast / haste).max(gcd);
                out_until = heal_at + gcd;
                rage = sc.furor_rage;
                frenzied_next = f64::INFINITY;
                gcd_at = out_until;
                swing_at = swing_at.max(out_until);
                maul_queued = false;
            }
            for i in 0..n {
                if hp[i] > 0.0 && t >= arrive[i] && t + EPS >= mob_at[i] {
                    let hit = body.swing(rng, armor, dodge, roar_ap, false);
                    if hit > 0.0 {
                        rage = (rage + A::RAGE_TAKEN * hit / sc.rage_c).min(A::RAGE_CAP);
                        hp[i] -= thorn;
                    } else if body.dodged && sc.reaction > 0.0 && rng.gen::<f64>() < sc.reaction {
                        rage = (rage + sc.reaction_rage).min(A::RAGE_CAP);
                    }
                    mob_at[i] += A::MOB_SWING;
                }
            }
            let target = (0..n).find(|&i| hp[i] > 0.0 && t >= arrive[i]);
            if let (Some(target), false) = (target, t < out_until) {
                let crit = form.crit + body.elune(t);
                let torn = |lac: &[(u32, f64, f64); MAX_MOBS], i: usize| if lac[i].0 > 0 && t < lac[i].2 { sc.rend } else { 1.0 };
                if t + EPS >= swing_at {
                    if maul_queued && sc.maul.known && (omen.up || rage >= sc.maul.cost) {
                        rage -= omen.pay(sc.maul.cost);
                        let o = melee_roll(sc, rng, crit, false);
                        if o.landed() {
                            omen.roll(sc, rng, t, BEAR_SPEED / haste, 1.0);
                        }
                        hp[target] -= (normal(sc, form.attack_power, BEAR_SPEED) + sc.maul.base * sc.savage) * melee_mult(sc, o) * mitig(sc, Some(&ff[target])) * torn(&lac, target);
                        rage = (rage + frenzied_rage(rng, o)).min(A::RAGE_CAP);
                    } else {
                        let o = melee_roll(sc, rng, crit, true);
                        let dmg = normal(sc, form.attack_power, BEAR_SPEED) * melee_mult(sc, o) * mitig(sc, Some(&ff[target]));
                        hp[target] -= dmg;
                        if o.landed() {
                            omen.roll(sc, rng, t, BEAR_SPEED / haste, 1.0);
                        }
                        let gain = A::RAGE_DEALT * dmg / sc.rage_c * if o == Out::Crit { A::CRIT_RAGE } else { 1.0 };
                        rage = (rage + gain + frenzied_rage(rng, o)).min(A::RAGE_CAP);
                    }
                    swing_at += BEAR_SPEED / haste;
                }
                let (mut alive, mut n_up) = ([0usize; MAX_MOBS], 0);
                for i in 0..n {
                    if hp[i] > 0.0 && t >= arrive[i] {
                        alive[n_up] = i;
                        n_up += 1;
                    }
                }
                let alive = &alive[..n_up];
                let standing = (0..n).filter(|&i| hp[i] > 0.0).count();
                if !alive.is_empty() && t + EPS >= gcd_at {
                    let target = alive[0];
                    let bite = &sc.primal_bite;
                    let free = omen.up;
                    let lacerate = sc.lacerate.known && (free || rage >= sc.lacerate.cost);
                    // Clearcasting: a free offensive ability, rolling Omen's chance again
                    let cast = |rage: &mut f64, omen: &mut Omen, rng: &mut StdRng, cost: f64| {
                        *rage -= omen.pay(cost);
                        omen.roll(sc, rng, t, BEAR_GCD, 1.0);
                    };
                    if p.roar && sc.roar.known && standing >= 2 && t >= roar_until && rage >= sc.roar.cost {
                        rage -= sc.roar.cost;
                        roar_until = t + sc.roar.duration;
                        gcd_at = t + BEAR_GCD;
                    } else if sc.swipe.known && alive.len() >= p.swipe_at && (free || rage >= sc.swipe.cost) {
                        cast(&mut rage, &mut omen, rng, sc.swipe.cost);
                        for &i in alive.iter().take(3) {
                            let o = melee_roll(sc, rng, crit, false);
                            hp[i] -= swipe_dmg * melee_mult(sc, o) * mitig(sc, Some(&ff[i])) * torn(&lac, i);
                            rage = (rage + frenzied_rage(rng, o)).min(A::RAGE_CAP);
                        }
                        gcd_at = t + BEAR_GCD;
                    } else if bite.known && t + EPS >= bite_at && (free || rage >= bite.cost) {
                        cast(&mut rage, &mut omen, rng, bite.cost);
                        let o = melee_roll(sc, rng, crit, false);
                        hp[target] -= (normal(sc, form.attack_power, BEAR_SPEED) * bite.pct + bite.base) * melee_mult(sc, o) * mitig(sc, Some(&ff[target])) * torn(&lac, target);
                        rage = (rage + frenzied_rage(rng, o)).min(A::RAGE_CAP);
                        bite_at = t + bite.cooldown;
                        gcd_at = t + BEAR_GCD;
                    } else if lacerate && (lac[target].0 < lacerating || (p.lacerate > 0 && lac[target].2 - t < BEAR_GCD)) {
                        cast(&mut rage, &mut omen, rng, sc.lacerate.cost);
                        rage = apply_lacerate(sc, form, rng, crit, rage, &mut lac[target], &mut hp[target], &ff[target], t, &frenzied_rage);
                        gcd_at = t + BEAR_GCD;
                    } else if lacerate && p.lacerate == LACERATE_SPAM {
                        cast(&mut rage, &mut omen, rng, sc.lacerate.cost);
                        rage = apply_lacerate(sc, form, rng, crit, rage, &mut lac[target], &mut hp[target], &ff[target], t, &frenzied_rage);
                        gcd_at = t + BEAR_GCD;
                    } else if ff[target].cast(&mut body, t) {
                        gcd_at = t + BEAR_GCD;
                    }
                }
                // Maul with what is left after the next Swipe or Primal Bite
                let reserve = p.maul_spare
                    + if sc.swipe.known && n_up >= p.swipe_at {
                        sc.swipe.cost
                    } else if sc.primal_bite.known && bite_at - t < BEAR_SPEED {
                        sc.primal_bite.cost
                    } else {
                        0.0
                    };
                maul_queued = rage >= sc.maul.cost + reserve;
            }
            // on to the next thing that can happen
            let mut next = heal_at.min(frenzied_next);
            if t < out_until {
                next = next.min(out_until);
            } else {
                next = next.min(swing_at);
            }
            if gcd_at > t + EPS {
                next = next.min(gcd_at);
            } else if t < enrage_until {
                // rage comes in steadily: look again soon
                next = next.min(t + RAGE_LOOK);
            }
            if bite_at > t + EPS {
                next = next.min(bite_at);
            }
            for i in 0..n {
                if hp[i] > 0.0 {
                    next = next.min(mob_at[i]);
                    if arrive[i] > t {
                        next = next.min(arrive[i]);
                    }
                    if lac[i].0 > 0 && lac[i].1 <= lac[i].2 + EPS {
                        next = next.min(lac[i].1);
                    }
                }
            }
            t = next.max(t + MIN_STEP);
        }
        if heal_at.is_finite() {
            // the pull died while the heal was being cast: it lands, then back into form
            t = t.max(heal_at);
            body.direct_heal(t, rng);
            body.spend(t, form.shift);
            heal_at = f64::INFINITY;
            t = t.max(out_until);
        }
        frenzied_next = f64::INFINITY;
        if body.dead() {
            out_until = -1.0;
            t = body.die(t);
            continue;
        }
        tally.kills += n as u32;
        body.in_combat = false;
    }
    tally.seconds = t;
    finish(tally, &body, sc)
}

/// A Lacerate on one mob: 10% of the weapon's damage per application already there,
/// one more application, the bleed's timer restarted. The rage after it (its cost already paid).
#[allow(clippy::too_many_arguments)]
fn apply_lacerate(sc: &Scenario, form: &Form, rng: &mut StdRng, crit: f64, rage: f64, lac: &mut (u32, f64, f64), hp: &mut f64, ff: &FaerieFire, t: f64, frenzied: &dyn Fn(&mut StdRng, Out) -> f64) -> f64 {
    let mut rage = rage;
    let o = melee_roll(sc, rng, crit, false);
    if o.landed() {
        let (stacks, next, end) = lac;
        let live = if t >= *end { 0 } else { *stacks };
        *hp -= normal(sc, form.attack_power, BEAR_SPEED) * sc.lacerate.extra * live as f64 * melee_mult(sc, o) * mitig(sc, Some(ff)) * if live > 0 { sc.rend } else { 1.0 };
        if live == 0 {
            *next = t + sc.lacerate.period;
        }
        *stacks = (live + 1).min(launch::LACERATE_STACKS);
        *end = t + sc.lacerate.duration;
        rage = (rage + frenzied(rng, o)).min(A::RAGE_CAP);
    }
    rage
}

// --- caster --------------------------------------------------------------------

/// A mob's swings up to `until`, `every` seconds apart; `hit` on each that lands.
#[allow(clippy::too_many_arguments)]
fn swings_to(body: &mut Body, rng: &mut StdRng, hp: &mut f64, mob_at: &mut f64, until: f64, armor: f64, dodge: f64, thorn: f64, every: f64, mut hit: impl FnMut(&mut StdRng)) {
    while *hp > 0.0 && *mob_at <= until && !body.dead() {
        if body.swing(rng, armor, dodge, 0.0, false) > 0.0 {
            *hp -= thorn;
            hit(rng);
        }
        *mob_at += every;
    }
}

/// A DoT's ticks (when it ends, its next, a tick) up to `until`.
fn dot_ticks(dot: &mut (f64, f64, f64), period: f64, hp: &mut f64, until: f64) {
    while dot.1 <= until.min(dot.0) + EPS && *hp > 0.0 {
        *hp -= dot.2;
        dot.1 += period;
    }
}

/// Pull `p.hurricane` mobs (the first tagged, the others in from 10 yards) and
/// channel Hurricane on them until the mana is out, then Wrath the rest. A hit
/// cuts the channel short by a tick, unless Nature's Focus saves it (as a cast's pushback).
fn storm(sc: &Scenario, p: &Play, rng: &mut StdRng, until: f64) -> Tally {
    let moonkin = p.moonkin && sc.moonkin.known;
    let form = if moonkin { &sc.moonkin } else { &sc.caster };
    let mut body = Body::new(sc, form);
    let (armor, dodge) = (form.armor, form.dodge);
    let mut tally = Tally::default();
    let n = p.hurricane.clamp(2, MAX_MOBS);
    let dm = sc.damage_mult * sc.balance_mult;
    // a periodic aura (client: aura 226), so Genesis counts
    let tick = (sc.hurricane.tick + sc.hurricane.tick_coef * sc.spell_power) * dm * sc.periodic_mult;
    let gcd_of = |t: f64, grace_until: f64| (A::SPELL_GCD / sc.haste * if t < grace_until { 0.9 } else { 1.0 }).max(A::MIN_GCD);
    let mut grace_until = -1.0f64;
    let crit = sc.spell_crit + if moonkin { sc.moonkin_crit } else { 0.0 };
    let thorn = thorns(sc);
    let mut omen = Omen { ready: 0.0, up: false };
    let omen_mult = if moonkin { sc.moonkin_omen } else { 1.0 };
    let (mut bark_ready, mut bark_to) = (0.0f64, -1.0f64);
    let barking = p.barkskin && sc.barkskin.known;
    let mut t = 0.0;
    while t < until {
        if tally.kills > 0 {
            let (t2, _) = body.recover(t, p.heal_below, p.heal_to, moonkin, sc.hurricane.cost * 2.0, rng);
            let way = DISTANCE / (A::RUN_SPEED * (1.0 + sc.run_speed / 100.0));
            t = t2 + way;
            tally.walking += way;
            body.tick(t);
        }
        if body.dead() {
            t = body.die(t);
            continue;
        }
        let mut hp = [sc.mob.health; MAX_MOBS];
        hp[n..].fill(0.0);
        // tagged from range: they run in, and Hurricane waits for them
        let arrive = t + CASTING_RANGE / A::MOB_RUN_SPEED;
        let mut mob_at: [f64; MAX_MOBS] = std::array::from_fn(|_| arrive + rng.gen_range(0.0..A::MOB_SWING));
        body.press_elune(t);
        // the tag: Moonfire on the first, its hit and its ticks; a crit brings Nature's Grace
        // (Hurricane's own ticks are periodic and do not)
        // (Clearcasting goes on the first damage spell that costs anything)
        body.spend(t, omen.pay(sc.moonfire.cost));
        let mut dot = (-1.0f64, 0.0f64, 0.0f64);
        if sc.moonfire.known && rng.gen::<f64>() * 100.0 < sc.spell_hit {
            let mf = &sc.moonfire;
            let c = rng.gen::<f64>() * 100.0 < crit + sc.moonfire_crit + body.elune(t);
            hp[0] -= (mf.base + rng.gen_range(-1.0..=1.0) * mf.spread + mf.coef * sc.spell_power) * dm * if c { sc.spell_crit_mult } else { 1.0 };
            let per = (mf.tick + mf.tick_coef * sc.spell_power) * mf.extra * dm * sc.periodic_mult * dot_crit(sc, crit + sc.moonfire_crit);
            dot = (t + mf.duration, t + mf.period, per);
            if c && sc.grace > 0.0 {
                grace_until = t + sc.grace;
            }
        }
        omen.roll(sc, rng, t, A::SPELL_GCD, omen_mult);
        t += gcd_of(t, grace_until);
        body.in_combat = true;
        body.tick(t);
        if body.mana < 0.5 * sc.mana && body.press_innervate(t) {
            t += gcd_of(t, grace_until);
        }
        while hp.iter().any(|&h| h > 0.0) && !body.dead() {
            body.tick(t);
            dot_ticks(&mut dot, sc.moonfire.period.max(1.0), &mut hp[0], t);
            body.bark = if t < bark_to { 1.0 - sc.barkskin.extra } else { 1.0 };
            if t >= arrive && (omen.up || body.mana >= sc.hurricane.cost) {
                if barking && t >= bark_ready {
                    // its global cooldown, the mobs swinging
                    let next = t + gcd_of(t, grace_until);
                    for i in 0..n {
                        swings_to(&mut body, rng, &mut hp[i], &mut mob_at[i], next, armor, dodge, thorn, A::MOB_SWING, |_| {});
                    }
                    t = next;
                    body.tick(t);
                    bark_to = t + sc.barkskin.duration;
                    bark_ready = t + sc.barkskin.cooldown;
                }
                // Clearcasting pays for a channel
                body.spend(t, omen.pay(sc.hurricane.cost));
                let started = t;
                // the ticks; the mobs' swings, slowed by it, between them
                let mut left = sc.hurricane.duration;
                while left > 0.0 && hp.iter().any(|&h| h > 0.0) && !body.dead() {
                    let next = t + sc.hurricane.period;
                    let barked = t < bark_to;
                    body.bark = if barked { 1.0 - sc.barkskin.extra } else { 1.0 };
                    for i in 0..n {
                        // a hit takes a tick off the channel, unless Barkskin or Nature's Focus saves it
                        let slowed = A::MOB_SWING * (1.0 + sc.hurricane.extra);
                        swings_to(&mut body, rng, &mut hp[i], &mut mob_at[i], next, armor, dodge, thorn, slowed, |rng| {
                            if !barked && rng.gen::<f64>() >= sc.focus {
                                left -= sc.hurricane.period;
                            }
                        });
                        if rng.gen::<f64>() * 100.0 < sc.spell_hit {
                            hp[i] -= tick * if rng.gen::<f64>() * 100.0 < crit + body.elune(t) { sc.spell_crit_mult } else { 1.0 };
                        }
                    }
                    t = next;
                    body.tick(t);
                    dot_ticks(&mut dot, sc.moonfire.period.max(1.0), &mut hp[0], t);
                    left -= sc.hurricane.period;
                }
                // Omen's rate a minute over the channel's time (assumption)
                omen.roll(sc, rng, t, t - started, omen_mult);
                body.bark = 1.0;
            } else {
                // dry: Wrath on the first standing, with what regenerates, else the weapon
                let Some(i) = (0..n).find(|&i| hp[i] > 0.0) else { break };
                if t < arrive && body.mana < sc.wrath.cost {
                    t = arrive;
                    body.tick(t);
                    continue;
                }
                let cast = (sc.wrath.cast / (sc.haste * if t < grace_until { 1.1 } else { 1.0 })).max(gcd_of(t, grace_until));
                let mut next = t + cast;
                let mut pushes = 0;
                let barked = t < bark_to;
                let mut swung = true;
                while swung {
                    swung = false;
                    for k in 0..n {
                        while hp[k] > 0.0 && mob_at[k] <= next && !body.dead() {
                            swung = true;
                            if body.swing(rng, armor, dodge, 0.0, false) > 0.0 {
                                hp[k] -= thorn;
                                // a hit pushes the cast back, as the caster's
                                if !barked && pushes < PUSHBACK_HITS && rng.gen::<f64>() >= sc.focus {
                                    next += PUSHBACK;
                                    pushes += 1;
                                }
                            }
                            mob_at[k] += A::MOB_SWING;
                        }
                    }
                }
                t = next;
                body.tick(t);
                if body.mana >= sc.wrath.cost {
                    body.spend(t, sc.wrath.cost);
                    omen.roll(sc, rng, t, cast, omen_mult);
                    if rng.gen::<f64>() * 100.0 < sc.spell_hit {
                        let c = rng.gen::<f64>() * 100.0 < crit + body.elune(t);
                        hp[i] -= (sc.wrath.base + sc.wrath.coef * sc.spell_power) * dm * if c { sc.spell_crit_mult } else { 1.0 };
                        if c && sc.grace > 0.0 {
                            grace_until = t + sc.grace;
                        }
                    }
                } else {
                    let o = melee_roll(sc, rng, form.crit, true);
                    hp[i] -= normal(sc, form.attack_power, sc.weapon_speed) * melee_mult(sc, o) * mitig(sc, None);
                }
            }
        }
        if body.dead() {
            t = body.die(t);
            continue;
        }
        tally.kills += n as u32;
        body.in_combat = false;
    }
    tally.seconds = t;
    finish(tally, &body, sc)
}

fn caster(sc: &Scenario, p: &Play, rng: &mut StdRng, until: f64) -> Tally {
    let moonkin = p.moonkin && sc.moonkin.known;
    let form = if moonkin { &sc.moonkin } else { &sc.caster };
    let mut body = Body::new(sc, form);
    let (armor, dodge) = (form.armor, form.dodge);
    let mut tally = Tally::default();
    let mut omen = Omen { ready: 0.0, up: false };
    let omen_mult = if moonkin { sc.moonkin_omen } else { 1.0 };
    let crit_bonus = if moonkin { sc.moonkin_crit } else { 0.0 };
    let thorn = thorns(sc);
    let haste = sc.haste;
    let dm = sc.damage_mult * sc.balance_mult;
    let mut t = 0.0;
    let mut spent: Vec<f64> = Vec::new();
    let mut chasers: Vec<Chaser> = Vec::new();
    let (mut eclipse, mut eclipse_until) = (0.0f64, -1.0f64);
    let mut grace_until = -1.0f64;
    let gcd_of = |grace: bool| (A::SPELL_GCD / haste * if grace { 0.9 } else { 1.0 }).max(A::MIN_GCD);
    let (mut grasp_ready, mut grasp_on) = (0.0f64, false);
    let mut bark_to = -1.0f64;
    let mut bark_ready = 0.0f64;
    while t < until {
        if tally.kills > 0 {
            let k = spent.len().min(5);
            let need = if k > 0 { spent[spent.len() - k..].iter().sum::<f64>() / k as f64 } else { 0.0 };
            let need = need * (1.0 - p.melee_below.min(0.9));
            let (t2, _) = body.recover(t, p.heal_below, p.heal_to, moonkin, need, rng);
            t = t2;
            chase(&mut body, rng, &mut chasers, t, armor, dodge);
            // on along the line of mobs: each kill moves the fight on by the distance between them
            let way = DISTANCE / (A::RUN_SPEED * (1.0 + sc.run_speed / 100.0));
            chase(&mut body, rng, &mut chasers, t + way, armor, dodge);
            body.tick(t + way);
            t += way;
            tally.walking += way;
            chasers.retain(|c| c.dies > t);
        }
        if body.dead() {
            t = body.die(t);
            chasers.clear();
            continue;
        }
        if p.grasp && sc.grasp.known && !grasp_on && t >= grasp_ready && body.mana >= sc.grasp.cost {
            body.spend(t, sc.grasp.cost);
            grasp_on = true;
            grasp_ready = t + sc.grasp.cooldown;
        }
        // on the way to the pull, so it takes no time from the fight
        if p.barkskin && sc.barkskin.known && t >= bark_ready {
            bark_to = t + sc.barkskin.duration;
            bark_ready = t + sc.barkskin.cooldown;
        }
        let start_mana = body.mana;
        let mut hp = sc.mob.health;
        let mut opened = false;
        // dots: Moonfire, Insect Swarm (end, next, per, period)
        let mut dots: [(f64, f64, f64, f64); 2] = [(-1.0, 0.0, 0.0, 1.0); 2];
        if p.leyline_cd && !moonkin {
            t = body.read_leyline(t);
        }
        let mut mob_at = t + CASTING_RANGE / A::MOB_RUN_SPEED; // it runs in from 30 yards
        body.press_elune(t);
        let mut arrive = mob_at;
        let mut swing_at = arrive;
        body.tick(t);
        body.in_combat = true;
        // Entangling Roots on the mob running in: it stands until enough damage breaks it
        let (mut rooted, mut run_left, mut root_end, mut root_broke) = (false, 0.0f64, 0.0f64, false);
        let mut rooted_once = false;
        if body.mana < 0.5 * sc.mana && body.press_innervate(t) {
            t += gcd_of(false);
        }
        if p.roots && sc.roots.known && body.mana >= sc.roots.cost {
            let land = t + (sc.roots.cast / haste).max(gcd_of(false));
            if land < arrive {
                body.spend(t, sc.roots.cost);
                body.tick(land);
                rooted = true;
                run_left = arrive - land;
                root_end = land + sc.roots.duration;
                arrive = f64::INFINITY;
                mob_at = f64::INFINITY;
                swing_at = f64::INFINITY;
                t = land;
            }
        }
        let tick_dots = |dots: &mut [(f64, f64, f64, f64); 2], hp: &mut f64, until: f64| {
            for d in dots.iter_mut() {
                while d.1 <= until.min(d.0) + 1e-9 && *hp > 0.0 {
                    *hp -= d.2;
                    d.1 += d.3;
                }
            }
        };
        let dot_pending = |dots: &[(f64, f64, f64, f64); 2]| -> f64 {
            dots.iter().filter(|d| d.1 <= d.0 + 1e-9).map(|d| d.2 * (((d.0 - d.1) / d.3 + 1e-9).floor() + 1.0)).sum()
        };
        while hp > 0.0 && !body.dead() {
            if p.drag && hp <= dot_pending(&dots) {
                let mut last = t;
                for d in dots.iter_mut() {
                    while d.1 <= d.0 + 1e-9 && hp > 0.0 {
                        hp -= d.2;
                        last = last.max(d.1);
                        d.1 += d.3;
                    }
                }
                chasers.push(Chaser { dies: last, swing: mob_at.max(t) });
                break;
            }
            if rooted && (root_broke || t >= root_end) {
                root_broke = false;
                rooted = false;
                arrive = t + run_left;
                mob_at = arrive;
                swing_at = arrive;
            }
            if p.roots_after > 0 && opened && !rooted && (!rooted_once || p.roots_after == 2) && sc.roots.known && body.mana >= sc.roots.cost {
                let land = t + (sc.roots.cast / haste).max(gcd_of(false));
                if land < arrive {
                    body.spend(t, sc.roots.cost);
                    tick_dots(&mut dots, &mut hp, land);
                    chase(&mut body, rng, &mut chasers, land, armor, dodge);
                    body.tick(land);
                    rooted = true;
                    rooted_once = true;
                    run_left = arrive - land;
                    root_end = land + sc.roots.duration;
                    arrive = f64::INFINITY;
                    mob_at = f64::INFINITY;
                    swing_at = f64::INFINITY;
                    t = land;
                    continue;
                }
            }
            let barked = t < bark_to;
            body.bark = if barked { 1.0 - sc.barkskin.extra } else { 1.0 };
            let swarm_up = dots[1].0 > t;
            let mob_swings = |body: &mut Body, rng: &mut StdRng, mob_at: &mut f64, hp: &mut f64, until: f64| -> u32 {
                let mut hits = 0;
                while *mob_at <= until && !body.dead() {
                    if body.swing(rng, armor, dodge, 0.0, swarm_up) > 0.0 {
                        *hp -= thorn;
                        hits += 1;
                    }
                    *mob_at += A::MOB_SWING;
                }
                hits
            };
            // choose: melee below the play's mana, else the spell
            let mf_up = dots[0].0 > t;
            // Clearcasting goes on Starfire; Wrath does not consume it (Omen's tooltip)
            let (sp, which): (&Sp, u8) = if !opened || (omen.up && sc.starfire.known) {
                if sc.starfire.known && (p.starfire_opener || opened) { (&sc.starfire, 1) } else { (&sc.wrath, 0) }
            } else if p.moonfire && sc.moonfire.known && !mf_up {
                (&sc.moonfire, 2)
            } else if p.swarm && sc.insect_swarm.known && !swarm_up {
                (&sc.insect_swarm, 3)
            } else if sc.starfire.known && (p.starfire_filler == 1 || (p.starfire_filler == 2 && eclipse > 0.0 && t < eclipse_until)) {
                (&sc.starfire, 1)
            } else {
                (&sc.wrath, 0)
            };
            let free = omen.up && which != 0;
            body.advance_mana(t);
            let dry = !free && body.mana < sp.cost;
            let melee = t >= arrive && (dry || body.mana < p.melee_below * sc.mana);
            if melee || dry {
                // a swing with the weapon when one is due, else wait for it (or the mob)
                let next = if t >= arrive { swing_at.max(t) } else { arrive.min(t + 1.0) };
                let step = (next - t).max(STEP);
                tick_dots(&mut dots, &mut hp, t + step);
                mob_swings(&mut body, rng, &mut mob_at, &mut hp, t + step);
                chase(&mut body, rng, &mut chasers, t + step, armor, dodge);
                body.tick(t + step);
                t += step;
                if t >= arrive && t >= swing_at && hp > 0.0 {
                    let o = melee_roll(sc, rng, form.crit + body.elune(t), true);
                    hp -= normal(sc, form.attack_power, sc.weapon_speed) * melee_mult(sc, o) * mitig(sc, None);
                    if o.landed() {
                        omen.roll(sc, rng, t, sc.weapon_speed / haste, omen_mult);
                    }
                    swing_at = t + sc.weapon_speed / haste;
                }
                continue;
            }
            let grace = t < grace_until;
            let mut cast = sp.cast;
            if which == 1 && eclipse > 0.0 && t < eclipse_until {
                cast = (cast - sc.eclipse_cut).max(0.0);
            }
            let mut cast_time = cast / (haste * if grace { 1.1 } else { 1.0 });
            // the mob sets off when the first spell lands, not when it starts
            if !opened && !rooted {
                arrive = t + cast_time + CASTING_RANGE / A::MOB_RUN_SPEED;
                mob_at = arrive;
                swing_at = arrive;
            }
            // a hit pushes the cast back (twice at most), unless Nature's Focus saves it
            let mut land = t + cast_time;
            if cast_time > 0.0 {
                let mut pushes = 0;
                let mut at = mob_at;
                let lands = landed_share(sc, dodge, dots[1].0 > t);
                while at <= land && pushes < PUSHBACK_HITS && at >= arrive {
                    if !barked && rng.gen::<f64>() < lands && rng.gen::<f64>() >= sc.focus {
                        land += PUSHBACK;
                        cast_time += PUSHBACK;
                        pushes += 1;
                    }
                    at += A::MOB_SWING;
                }
            }
            let done = t + cast_time.max(gcd_of(grace));
            tick_dots(&mut dots, &mut hp, land);
            let hits = mob_swings(&mut body, rng, &mut mob_at, &mut hp, done);
            chase(&mut body, rng, &mut chasers, done, armor, dodge);
            body.tick(land);
            if hp <= 0.0 {
                t = land;
                break;
            }
            let grasped = grasp_on && hits > 0;
            // Wrath neither uses Clearcasting up nor is free under it
            body.spend(land, if which == 0 { sp.cost } else { omen.pay(sp.cost) });
            if which == 1 && eclipse > 0.0 && t < eclipse_until {
                eclipse -= 1.0;
            }
            opened = true;
            omen.roll(sc, rng, land, cast_time.max(1.5), omen_mult);
            if rng.gen::<f64>() * 100.0 < sc.spell_hit {
                let crit = sc.spell_crit + crit_bonus + body.elune(land);
                let roll = |rng: &mut StdRng, s: &Sp, crit: f64| -> (f64, bool) {
                    let amount = s.base + rng.gen_range(-1.0..=1.0) * s.spread + s.coef * sc.spell_power;
                    let c = rng.gen::<f64>() * 100.0 < crit;
                    (amount * if c { sc.spell_crit_mult } else { 1.0 }, c)
                };
                let was = match which {
                    0 | 1 => {
                        let (dmg, c) = roll(rng, sp, crit);
                        hp -= dmg * dm;
                        // Wrath or Starfire on a rooted mob may break it; a tick does not
                        if rooted && rng.gen::<f64>() < sc.root_break {
                            root_broke = true;
                        }
                        if which == 0 && sc.eclipse_cut > 0.0 {
                            eclipse = (eclipse + sc.eclipse_per).min(sc.eclipse_max.max(1.0));
                            eclipse_until = land + sc.eclipse_secs;
                        }
                        c
                    }
                    2 => {
                        let (dmg, c) = roll(rng, sp, crit + sc.moonfire_crit);
                        hp -= dmg * dm;
                        let per = (sp.tick + sp.tick_coef * sc.spell_power) * sp.extra * dm * sc.periodic_mult * dot_crit(sc, crit + sc.moonfire_crit);
                        dots[0] = (land + sp.duration, land + sp.period, per, sp.period);
                        c
                    }
                    _ => {
                        let per = (sp.tick + sp.tick_coef * sc.spell_power) * dm * sc.periodic_mult * dot_crit(sc, crit);
                        dots[1] = (land + sp.duration, land + sp.period, per, sp.period);
                        false
                    }
                };
                if was && sc.grace > 0.0 {
                    grace_until = land + sc.grace;
                }
            }
            tick_dots(&mut dots, &mut hp, done);
            body.tick(done);
            t = done;
            if grasped && !rooted && hp > 0.0 {
                // rooted by the hit: a few steps out of its reach; it stands until enough damage breaks it
                grasp_on = false;
                tick_dots(&mut dots, &mut hp, t + launch::GRASP_STEP);
                chase(&mut body, rng, &mut chasers, t + launch::GRASP_STEP, armor, dodge);
                body.tick(t + launch::GRASP_STEP);
                t += launch::GRASP_STEP;
                rooted = true;
                run_left = launch::GRASP_STEP;
                root_end = t + sc.roots.duration;
                arrive = f64::INFINITY;
                mob_at = f64::INFINITY;
                swing_at = f64::INFINITY;
            }
        }
        if body.dead() {
            t = body.die(t);
            chasers.clear();
            continue;
        }
        tally.kills += 1;
        body.in_combat = chasers.iter().any(|c| c.dies > t);
        spent.push((start_mana - body.mana).max(0.0));
    }
    tally.seconds = chasers.iter().map(|c| c.dies).fold(t, f64::max);
    finish(tally, &body, sc)
}

/// XP an hour of sessions (a death's cost is the time it took).
pub fn xp_per_hour(ts: &[Tally]) -> f64 {
    let secs: f64 = ts.iter().map(|t| t.seconds).sum();
    ts.iter().map(|t| t.xp).sum::<f64>() / secs.max(1.0) * 3600.0
}
