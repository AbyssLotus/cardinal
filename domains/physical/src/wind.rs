//! Wind load (Vol. III Ch. 1 §1.9, "a bridge fails because the material cannot support the required
//! stress"; §1.10, wind; Amendment A-29): what stands in the open bears the wind, and what its
//! weakest material cannot bear, the wind tears down.
//!
//! On the environment's cadence, every structure in the open — a thing made of something, or a
//! walled place's outer walls, standing in a place with no walled place around it, not inside
//! another thing that shelters it — bears the wind of the region it stands in. The wind presses with the square of its speed, over the world's `force_divisor`, in
//! the units of hardness. Where that pressure exceeds what the structure's weakest material bears,
//! it loses integrity at the world's `damage_scale` per hour for each unit it exceeds by, more
//! slowly the more of it there is. Breaking is then Physical Reality's as any breaking is
//! (Amendment A-27): a shed becomes its wreckage, a house a ruin — and what the house held is in
//! the open from then on.
//!
//! In calm weather nothing is looked at; in a storm, only what is made of something.

use crate::fire::{cube_root, structure_kg, Fire};
use crate::schema::{
    BODY_SIZE, BROKEN, CONTAINED_IN, ENCLOSED, EXPOSURE, INTEGRITY, LEADS_TO, MADE_OF,
    MATERIAL_DENSITY, MATERIAL_HARDNESS, MOBILE, PERCENT_FULL, WIND_SPEED,
};
use crate::terrain::is_true;
use kernel::fact::{Cause, FactKey, FactType, SystemId};
use kernel::fixed::div_dither;
use kernel::proposal::{Change, Proposal};
use kernel::system::{Cadence, CommittedView, System, TickContext};
use kernel::time::Step;

/// How wind bears on what stands in it (Amendment A-29), all world rules.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct WindRules {
    /// The wind's pressure is its speed squared over this, in the units of hardness.
    pub force_divisor: i64,
    /// Integrity lost per hour for each unit the pressure exceeds what a structure bears, from a
    /// hundred kilograms of it; more of it fails more slowly, by the cube root of how much more.
    pub damage_scale: i64,
}

const WIND_LOAD_READS: &[FactType] = &[
    WIND_SPEED,
    CONTAINED_IN,
    ENCLOSED,
    EXPOSURE,
    MADE_OF,
    MATERIAL_HARDNESS,
    MATERIAL_DENSITY,
    BODY_SIZE,
    INTEGRITY,
    BROKEN,
    MOBILE,
    LEADS_TO,
];
const WIND_LOAD_WRITES: &[FactType] = &[INTEGRITY];

/// What the wind tears at (Amendment A-29).
pub struct WindLoad {
    step: Step,
    rules: WindRules,
    wall_thickness_cm: i64,
}

impl WindLoad {
    /// Wind load by `rules`, on the environment's `step`, judging walled places by walls
    /// `wall_thickness_cm` thick.
    pub const fn new(step: Step, rules: WindRules, wall_thickness_cm: i64) -> Self {
        Self {
            step,
            rules,
            wall_thickness_cm,
        }
    }
}

impl System for WindLoad {
    fn id(&self) -> SystemId {
        SystemId::new("physical.wind_load")
    }
    fn reads(&self) -> &'static [FactType] {
        WIND_LOAD_READS
    }
    fn writes(&self) -> &'static [FactType] {
        WIND_LOAD_WRITES
    }
    fn cadence(&self) -> Cadence {
        self.step.cadence()
    }
    fn evaluate(&self, view: &dyn CommittedView, ctx: &TickContext) -> Vec<Proposal> {
        let r = &self.rules;
        let pressure = |v: i64| (v as i128 * v as i128 / r.force_divisor.max(1) as i128) as i64;
        // Calm everywhere — no wind that presses at all — and there is nothing to look at.
        let strongest = view
            .entities_with(WIND_SPEED)
            .into_iter()
            .filter_map(|region| view.read(FactKey::new(region, WIND_SPEED)))
            .filter_map(|f| f.value.as_int())
            .max()
            .unwrap_or(0);
        if pressure(strongest) <= 0 {
            return Vec::new();
        }
        let mut out = Vec::new();
        // Standing in a place — not inside another thing, which shelters it (an apple in a crate,
        // a hammer in a hand) — though a ruin, its walls down, is a place like any.
        let in_a_place = |thing| match crate::index::container_of(view, thing) {
            Some(c) => view.read(FactKey::new(c, MADE_OF)).is_none() || is_true(view, c, BROKEN),
            None => false,
        };
        for thing in view.entities_with(MADE_OF) {
            if is_true(view, thing, MOBILE)
                || is_true(view, thing, BROKEN)
                || view.read(FactKey::new(thing, LEADS_TO)).is_some()
                || view.read(FactKey::new(thing, BODY_SIZE)).is_none()
                || !in_a_place(thing)
                || !Fire::in_the_open(view, thing)
            {
                continue;
            }
            let Some((_, wind)) = crate::climate::ambient(view, thing, WIND_SPEED) else {
                continue;
            };
            let bears = crate::materials::structural_hardness(view, thing).unwrap_or(PERCENT_FULL);
            let over_by = pressure(wind) - bears;
            if over_by <= 0 {
                continue;
            }
            let slower = cube_root(structure_kg(view, thing, self.wall_thickness_cm) / 100).max(1);
            let torn = div_dither(
                over_by as i128 * r.damage_scale as i128 * self.step.dt_ms as i128,
                3_600_000 * slower as i128,
                &mut ctx.rng(thing.raw()),
            ) as i64;
            if torn > 0 {
                out.push(Proposal::new(
                    self.id(),
                    FactKey::new(thing, INTEGRITY),
                    ctx.basis_tick(),
                    Change::Delta(-torn),
                    Cause::new("wind"),
                ));
            }
        }
        out
    }
}
