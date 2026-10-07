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
    ACT_PUSH, BODY_SIZE, BROKEN, CONTAINED_IN, ENCLOSED, EXPOSURE, HEADING, INTEGRITY, LEADS_TO,
    MADE_OF, MATERIAL_DENSITY, MATERIAL_HARDNESS, MOBILE, MOTION_END, MOTION_START, MOTION_TARGET,
    PERCENT_FULL, POSITION, SOLID, WIND_SPEED, WIND_TOWARD,
};
use crate::terrain::is_true;
use kernel::fact::{Cause, FactKey, FactType, SystemId};
use kernel::fixed::{div_dither, isqrt};
use kernel::identity::EntityId;
use kernel::proposal::{Change, Proposal};
use kernel::rng::Rng;
use kernel::system::{Cadence, CommittedView, System, TickContext};
use kernel::time::Step;
use kernel::value::Value;

/// How wind bears on what stands in it (Amendments A-29, A-33), all world rules.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct WindRules {
    /// The wind's pressure is its speed squared over this, in the units of hardness.
    pub force_divisor: i64,
    /// Integrity lost per hour for each unit the pressure exceeds what a structure bears, from a
    /// hundred kilograms of it; more of it fails more slowly, by the cube root of how much more.
    pub damage_scale: i64,
    /// How much pressure each kilogram of a loose thing holds it in place against (A-33).
    pub hold_per_kg: i64,
    /// How far, in centimetres an hour, a wind that overwhelms what holds a thing carries it.
    pub carry_cm_per_hour: i64,
}

const WIND_LOAD_READS: &[FactType] = &[
    WIND_SPEED,
    WIND_TOWARD,
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
    SOLID,
    POSITION,
    HEADING,
    MOTION_TARGET,
    MOTION_START,
    MOTION_END,
];
const WIND_LOAD_WRITES: &[FactType] = &[INTEGRITY, ACT_PUSH];

/// Which way, as a vector of length `distance`, the wind over `region` carries things: toward the
/// region it blows to, where both have a place in a common frame; else a gust's way, drawn from
/// `rng` (Amendment A-33).
fn downwind(view: &dyn CommittedView, region: EntityId, distance: i64, rng: &mut Rng) -> [i64; 2] {
    let toward = view
        .read(FactKey::new(region, WIND_TOWARD))
        .and_then(|f| match f.value {
            Value::Entity(e) => Some(e),
            _ => None,
        });
    let way = toward.and_then(|t| {
        let common = kernel::hierarchy::lowest_common_ancestor(view, region, t, CONTAINED_IN)?;
        let a = crate::space::position_in(view, region, common)?;
        let b = crate::space::position_in(view, t, common)?;
        let (dx, dy) = ((b[0] - a[0]) as i128, (b[1] - a[1]) as i128);
        let len = isqrt((dx * dx + dy * dy) as u128) as i128;
        (len > 0).then(|| {
            [
                (dx * distance as i128 / len) as i64,
                (dy * distance as i128 / len) as i64,
            ]
        })
    });
    way.unwrap_or_else(|| {
        let bearing = rng.below(36_000) as i64;
        let v = crate::space::rotate(bearing, [0, distance, 0]);
        [v[0], v[1]]
    })
}

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
            let Some((region, wind)) = crate::climate::ambient(view, thing, WIND_SPEED) else {
                continue;
            };
            let mut rng = ctx.rng(thing.raw());
            // A loose thing too light to hold its place is carried downwind (Amendment A-33): the
            // further, the more the wind overwhelms what its weight holds.
            let presses = pressure(wind);
            if !is_true(view, thing, SOLID) {
                let held = crate::materials::weight_g(view, thing)
                    .map_or(i64::MAX, |g| g / 1000 * r.hold_per_kg);
                if presses > held {
                    let distance = div_dither(
                        (presses - held) as i128
                            * r.carry_cm_per_hour as i128
                            * self.step.dt_ms as i128,
                        presses as i128 * 3_600_000,
                        &mut rng,
                    ) as i64;
                    if distance > 0 {
                        let [dx, dy] = downwind(view, region, distance, &mut rng);
                        out.push(Proposal::new(
                            self.id(),
                            FactKey::new(thing, ACT_PUSH),
                            ctx.basis_tick(),
                            Change::Set(Value::Vec3([dx, dy, 0])),
                            Cause::new("blown"),
                        ));
                    }
                }
            }
            let bears = crate::materials::structural_hardness(view, thing).unwrap_or(PERCENT_FULL);
            let over_by = presses - bears;
            if over_by <= 0 {
                continue;
            }
            let slower = cube_root(structure_kg(view, thing, self.wall_thickness_cm) / 100).max(1);
            let torn = div_dither(
                over_by as i128 * r.damage_scale as i128 * self.step.dt_ms as i128,
                3_600_000 * slower as i128,
                &mut rng,
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
