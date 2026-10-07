//! Fire (Vol. III Ch. 1 §1.9, "fire spreads because nearby materials satisfy ignition
//! conditions"; Amendment A-28).
//!
//! Burning is a level on a thing or a walled place ([`BURNING`]). Each step of the world's fire
//! cadence:
//!
//! - **a fire takes its course**: it grows toward what its most flammable material allows, and
//!   eats its integrity at the world's rate, more slowly the more there is of it. One with nothing
//!   left goes out, and what it burned is broken (Amendment A-27). In the open, under rain, it is
//!   put out. A kept fire — a tended hearth — neither grows, nor eats, nor dies down;
//! - **it gives heat**: to everything within its reach — more the nearer, and the fiercer the
//!   fire — that it has a line to, as light travels: walls keep it out until they burn. A fire
//!   inside a walled place heats the walls, as near as it is to them; a walled place on fire
//!   heats all it holds;
//! - **heat catches**: a thing that can burn, given heat enough for its flammability to reach the
//!   world's ignition line, catches fire;
//! - **heat is reported**: on every body that moves by itself, as heat exposure, for Living
//!   Systems to judge what it does (Appendix A, Ruling 9).
//!
//! Only what burns, and what lies within reach of it, is looked at: a world with no fire pays
//! nothing for this.

use crate::index::{container_of, size_of};
use crate::schema::{
    BODY_SIZE, BROKEN, BURNING, CONTAINED_IN, ENCLOSED, EXPOSURE, HAS_PORTAL, HEADING,
    HEAT_EXPOSURE, HUMIDITY, INTEGRITY, KEPT_FIRE, LEADS_TO, MADE_OF, MATERIAL_DENSITY,
    MATERIAL_FLAMMABILITY, MOBILE, MOTION_END, MOTION_START, MOTION_TARGET, OPAQUE, PERCENT_FULL,
    PORTAL_FAR_SIDE, PORTAL_OPEN, POSITION, TERRAIN_SAMPLE, TERRAIN_SPACING,
};
use crate::terrain::is_true;
use kernel::fact::{Cause, FactKey, FactType, SystemId};
use kernel::fixed::div_dither;
use kernel::identity::EntityId;
use kernel::proposal::{Change, Proposal};
use kernel::rng::Rng;
use kernel::system::{Cadence, CommittedView, System, TickContext};
use kernel::time::Step;
use kernel::value::Value;
use std::collections::BTreeMap;

/// How fire behaves, all world rules (Vol. IV Ch. 2 §2.2).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct FireRules {
    /// How often fire takes its course, in seconds of simulated time.
    pub step_seconds: u64,
    /// How fast a fire grows toward what its material allows, per hour.
    pub grow_per_hour: i64,
    /// How much integrity a fire at full fierceness eats per hour, from a hundred kilograms of
    /// fuel; more of it burns more slowly, by the cube root of how much more.
    pub burn_per_hour: i64,
    /// How far a fire at full fierceness reaches beyond its own extent, in centimetres.
    pub reach_cm: i64,
    /// The heat, times flammability, at which a thing catches fire.
    pub ignition_heat: i64,
    /// How fiercely a fire burns when it first catches.
    pub ignite_intensity: i64,
    /// The humidity at and above which rain falls on what is open to the sky.
    pub rain_above: i64,
    /// How fast rain puts out a fire in the open, per hour.
    pub douse_per_hour: i64,
    /// How thick a walled place's walls, floor, and roof are, in centimetres: what of it burns.
    pub wall_thickness_cm: i64,
}

/// Every placement and line-of-sight fact, besides fire's own.
const FIRE_READS: &[FactType] = &[
    BURNING,
    KEPT_FIRE,
    INTEGRITY,
    BROKEN,
    MADE_OF,
    MATERIAL_FLAMMABILITY,
    MATERIAL_DENSITY,
    HUMIDITY,
    ENCLOSED,
    EXPOSURE,
    MOBILE,
    HEAT_EXPOSURE,
    CONTAINED_IN,
    POSITION,
    BODY_SIZE,
    HEADING,
    MOTION_TARGET,
    MOTION_START,
    MOTION_END,
    LEADS_TO,
    HAS_PORTAL,
    OPAQUE,
    PORTAL_OPEN,
    PORTAL_FAR_SIDE,
    TERRAIN_SAMPLE,
    TERRAIN_SPACING,
];
const FIRE_WRITES: &[FactType] = &[BURNING, INTEGRITY, HEAT_EXPOSURE];

/// Fire's course, its heat, and what catches (Amendment A-28).
pub struct Fire {
    step: Step,
    rules: FireRules,
}

impl Fire {
    /// Fire by `rules`, stepping as `step` says.
    pub const fn new(step: Step, rules: FireRules) -> Self {
        Self { step, rules }
    }

    /// `per_hour` over one step, dithered so nothing is lost to rounding over many.
    fn over(&self, per_hour: i64, rng: &mut Rng) -> i64 {
        div_dither(per_hour as i128 * self.step.dt_ms as i128, 3_600_000, rng) as i64
    }

    /// How much of `thing` there is to burn, in kilograms.
    fn fuel_kg(&self, view: &dyn CommittedView, thing: EntityId) -> i64 {
        structure_kg(view, thing, self.rules.wall_thickness_cm)
    }

    /// Whether `thing` lies open to the sky: no walled place holds it.
    pub fn in_the_open(view: &dyn CommittedView, thing: EntityId) -> bool {
        kernel::hierarchy::ancestry(view, thing, CONTAINED_IN)
            .into_iter()
            .skip(1)
            .all(|p| !is_true(view, p, ENCLOSED) || view.read(FactKey::new(p, EXPOSURE)).is_some())
    }
}

/// How much of `thing` there is as a structure, in kilograms (Amendments A-28, A-29): a walled
/// place's walls, floor, and roof, `wall_thickness_cm` thick, of its densest material; anything
/// else, its weight. What a fire has to eat, and what a wind has to tear down.
pub fn structure_kg(view: &dyn CommittedView, thing: EntityId, wall_thickness_cm: i64) -> i64 {
    if is_true(view, thing, ENCLOSED) {
        let density = crate::materials::materials_of(view, thing)
            .into_iter()
            .filter_map(|m| crate::materials::density(view, m))
            .max()
            .unwrap_or(0) as i128;
        let Some([hw, hd, h]) = size_of(view, thing) else {
            return 0;
        };
        let (w, d, h) = (2 * hw as i128, 2 * hd as i128, h as i128);
        let shell_cm2 = 2 * (w + d) * h + 2 * w * d;
        (shell_cm2 * wall_thickness_cm as i128 * density / 1_000_000) as i64
    } else {
        crate::materials::weight_g(view, thing).unwrap_or(0) / 1000
    }
}

/// How far `thing` extends from where it stands, across: half its widest side.
fn half(view: &dyn CommittedView, thing: EntityId) -> i64 {
    size_of(view, thing).map_or(0, |[w, d, _]| w.max(d))
}

fn int(view: &dyn CommittedView, key: FactKey) -> Option<i64> {
    view.read(key).and_then(|f| f.value.as_int())
}

/// Whether `thing` can catch fire: made of something that burns, with something left to burn,
/// and not burning already.
pub fn can_catch(view: &dyn CommittedView, thing: EntityId) -> bool {
    crate::materials::flammability_of(view, thing).is_some_and(|f| f > 0)
        && int(view, FactKey::new(thing, INTEGRITY)).unwrap_or(PERCENT_FULL) > 0
        && !is_true(view, thing, BROKEN)
        && view.read(FactKey::new(thing, BURNING)).is_none()
}

/// The integer cube root of `n`, rounded down.
pub fn cube_root(n: i64) -> i64 {
    let mut r = (n.max(0) as f64).cbrt() as i64;
    while (r + 1).pow(3) <= n {
        r += 1;
    }
    while r > 0 && r.pow(3) > n {
        r -= 1;
    }
    r
}

impl System for Fire {
    fn id(&self) -> SystemId {
        SystemId::new("physical.fire")
    }
    fn reads(&self) -> &'static [FactType] {
        FIRE_READS
    }
    fn writes(&self) -> &'static [FactType] {
        FIRE_WRITES
    }
    fn cadence(&self) -> Cadence {
        self.step.cadence()
    }
    fn evaluate(&self, view: &dyn CommittedView, ctx: &TickContext) -> Vec<Proposal> {
        let r = &self.rules;
        let mut out = Vec::new();
        let mut push = |key: FactKey, change: Change, why: &'static str| {
            out.push(Proposal::new(
                self.id(),
                key,
                ctx.basis_tick(),
                change,
                Cause::new(why),
            ));
        };
        let burning: Vec<(EntityId, i64)> = view
            .entities_with(BURNING)
            .into_iter()
            .filter_map(|b| int(view, FactKey::new(b, BURNING)).map(|i| (b, i)))
            .collect();

        // 1. Each fire takes its course.
        for &(b, i) in &burning {
            let mut rng = ctx.rng(b.raw());
            if is_true(view, b, KEPT_FIRE) {
                continue;
            }
            let whole = int(view, FactKey::new(b, INTEGRITY)).unwrap_or(PERCENT_FULL);
            if whole <= 0 || is_true(view, b, BROKEN) {
                push(FactKey::new(b, BURNING), Change::Tombstone, "burnt_out");
                continue;
            }
            let allows = crate::materials::flammability_of(view, b).unwrap_or(0);
            let raining = Self::in_the_open(view, b)
                && crate::climate::ambient(view, b, HUMIDITY)
                    .is_some_and(|(_, h)| h >= r.rain_above);
            let next = if raining {
                i - self.over(r.douse_per_hour, &mut rng)
            } else if i < allows {
                (i + self.over(r.grow_per_hour, &mut rng)).min(allows)
            } else {
                allows
            };
            if next <= 0 {
                let why = if raining { "rained_out" } else { "burnt_out" };
                push(FactKey::new(b, BURNING), Change::Tombstone, why);
            } else if next != i {
                push(
                    FactKey::new(b, BURNING),
                    Change::Set(Value::Int(next)),
                    "burning",
                );
            }
            // It eats what it burns in: more slowly, the more of it there is.
            let slower = cube_root(self.fuel_kg(view, b) / 100).max(1);
            let eaten = self.over(
                i as i128 as i64 * r.burn_per_hour / PERCENT_FULL / slower,
                &mut rng,
            );
            if eaten > 0 {
                push(FactKey::new(b, INTEGRITY), Change::Delta(-eaten), "burning");
            }
        }

        // 2. The heat each fire gives, to what it reaches.
        let mut heat: BTreeMap<EntityId, i64> = BTreeMap::new();
        let mut felt: BTreeMap<EntityId, i64> = BTreeMap::new();
        for &(b, i) in &burning {
            if i <= 0 {
                continue;
            }
            let reach = r.reach_cm * i / PERCENT_FULL + half(view, b);
            let holds = |c: EntityId| {
                kernel::hierarchy::ancestry(view, c, CONTAINED_IN)
                    .into_iter()
                    .skip(1)
                    .any(|p| p == b)
            };
            for (c, d) in crate::nearby::within(view, b, reach + 1_000) {
                let from_edge = (d - half(view, c)).max(0);
                if from_edge > reach {
                    continue;
                }
                let given = i * (reach - from_edge) / reach.max(1);
                // In the fire, or in its line: walls keep heat out until they burn.
                if !(holds(c) || crate::sight::line_of_heat(view, b, c)) {
                    continue;
                }
                if is_true(view, c, MOBILE) {
                    *felt.entry(c).or_insert(0) += given;
                } else if can_catch(view, c) {
                    *heat.entry(c).or_insert(0) += given;
                }
            }
            // A fire inside a walled place heats its walls, from as near as it is to the nearest
            // of them, or the ceiling.
            if let Some(walls) = container_of(view, b).filter(|w| is_true(view, *w, ENCLOSED)) {
                if let (true, Some([hw, hd, h])) = (can_catch(view, walls), size_of(view, walls)) {
                    let [x, y, z] = crate::space::local_position(view, b);
                    let tall = size_of(view, b).map_or(0, |s| s[2]);
                    let nearest = (hw - x.abs()).min(hd - y.abs()).min(h - z - tall);
                    let from_edge = (nearest - half(view, b)).max(0);
                    if from_edge <= reach {
                        *heat.entry(walls).or_insert(0) += i * (reach - from_edge) / reach.max(1);
                    }
                }
            }
        }

        // 3. What is given heat enough for its flammability catches.
        for (c, h) in heat {
            let f = crate::materials::flammability_of(view, c).unwrap_or(0);
            if h * f / PERCENT_FULL >= r.ignition_heat {
                push(
                    FactKey::new(c, BURNING),
                    Change::Set(Value::Int(r.ignite_intensity)),
                    "caught_fire",
                );
            }
        }

        // 4. The heat bodies are taking, reported where it changed.
        for (c, h) in &felt {
            let h = (*h).min(PERCENT_FULL);
            if int(view, FactKey::new(*c, HEAT_EXPOSURE)) != Some(h) {
                push(
                    FactKey::new(*c, HEAT_EXPOSURE),
                    Change::Set(Value::Int(h)),
                    "heat",
                );
            }
        }
        for c in view.entities_with(HEAT_EXPOSURE) {
            if !felt.contains_key(&c) {
                push(FactKey::new(c, HEAT_EXPOSURE), Change::Tombstone, "cooled");
            }
        }
        out
    }
}
