//! Wet and snow (Vol. III Ch. 1 §1.10, "soil moisture"; Amendment A-38).
//!
//! On the environment's cadence:
//!
//! - **The ground** of every climate open to the sky holds water ([`GROUND_WATER`]): rain soaks
//!   in, and so does melting snow; it dries as the air is dry, the sun shines, and the wind
//!   blows — but not under snow.
//! - **Snow** lies ([`SNOW_DEPTH`]): what falls as snow builds up, ten times as deep as the water
//!   in it, and melts above freezing, the faster the warmer, into the ground.
//! - **Things** get wet ([`WETNESS`]): whatever stands in the open while rain or snow falls soaks,
//!   as fast as it falls; and everything wet dries, as the ground does — indoors too, in the
//!   still, dim air of a room, slowly. Wet things burn less readily ([`crate::fire`]). What
//!   wet does to a body is Living Systems' to judge (Appendix A, Ruling 9).
//!
//! A world where nothing falls pays only for drying what is already wet.

use crate::schema::{
    CONTAINED_IN, ENCLOSED, EXPOSURE, GROUND_WATER, HUMIDITY, ILLUMINATION, MADE_OF, MOBILE,
    PERCENT_FULL, RAIN, SNOW, SNOW_DEPTH, TEMPERATURE, WETNESS, WIND_SPEED,
};
use kernel::fact::{Cause, FactKey, FactType, SystemId};
use kernel::fixed::div_dither;
use kernel::identity::EntityId;
use kernel::proposal::{Change, Proposal};
use kernel::rng::Rng;
use kernel::system::{Cadence, CommittedView, System, TickContext};
use kernel::time::Step;
use std::collections::BTreeSet;

/// How things get wet and dry, all world rules (Vol. IV Ch. 2 §2.2).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct WetRules {
    /// How much wetter the ground grows for each millimetre of water that reaches it, in
    /// hundredths of a percent.
    pub soak_per_mm: i64,
    /// How fast the ground dries per hour, in hundredths of a percent, in air with no water in
    /// it, no sun, and no wind; twice as fast in full sun, and faster in wind.
    pub ground_drying_per_hour: i64,
    /// How many millimetres of snow melt per hour for each degree above freezing.
    pub snow_melt_mm_per_degree_hour: i64,
    /// How fast a thing in the open soaks in the world's hardest rain, per hour, in hundredths of
    /// a percent; in proportion, in lighter rain.
    pub wetting_per_hour: i64,
    /// How fast a wet thing dries per hour, as the ground does.
    pub drying_per_hour: i64,
    /// The world's hardest rain, in hundredths of a millimetre an hour (A-35's
    /// `rain_per_hour`): what soaks a thing at the full `wetting_per_hour`.
    pub hardest_rain: i64,
}

const WET_READS: &[FactType] = &[
    TEMPERATURE,
    EXPOSURE,
    ENCLOSED,
    CONTAINED_IN,
    RAIN,
    SNOW,
    HUMIDITY,
    ILLUMINATION,
    WIND_SPEED,
    GROUND_WATER,
    SNOW_DEPTH,
    WETNESS,
    MADE_OF,
    MOBILE,
    crate::schema::WATER_LEVEL,
    crate::schema::FLOOD_DEPTH,
    crate::schema::ICE,
    crate::schema::POSITION,
    crate::schema::BODY_SIZE,
    crate::schema::HEADING,
    crate::schema::MOTION_TARGET,
    crate::schema::MOTION_START,
    crate::schema::MOTION_END,
];
const WET_WRITES: &[FactType] = &[GROUND_WATER, SNOW_DEPTH, WETNESS];

/// Wet and snow: the ground, the snow on it, and what stands in the rain (Amendment A-38).
pub struct Wet {
    step: Step,
    rules: WetRules,
}

impl Wet {
    /// Wet and dry by `rules`, stepping as `step` says.
    pub const fn new(step: Step, rules: WetRules) -> Self {
        Self { step, rules }
    }

    /// `per_hour` over one step, dithered so nothing is lost to rounding over many.
    fn over(&self, per_hour: i64, rng: &mut Rng) -> i64 {
        div_dither(per_hour as i128 * self.step.dt_ms as i128, 3_600_000, rng) as i64
    }

    /// How fast something dries per hour at `place`, from `per_hour` in still, dark, saturated-
    /// free air: as dry as the air there is, twice as fast in full light, and faster in wind.
    fn drying(view: &dyn CommittedView, place: EntityId, per_hour: i64, wind: i64) -> i64 {
        let ambient = |fact| crate::climate::ambient(view, place, fact).map_or(0, |(_, v)| v);
        let dry = (PERCENT_FULL - ambient(HUMIDITY).clamp(0, PERCENT_FULL)) as i128;
        let light = ambient(ILLUMINATION).clamp(0, PERCENT_FULL) as i128;
        let full = PERCENT_FULL as i128;
        (per_hour as i128 * dry / full * (full + light) / full * (1_000 + wind.max(0) as i128)
            / 1_000) as i64
    }
}

fn int(view: &dyn CommittedView, key: FactKey) -> i64 {
    view.read(key).and_then(|f| f.value.as_int()).unwrap_or(0)
}

/// The water falling where `thing` is, if it stands open to the sky, in hundredths of a
/// millimetre an hour: rain and snow alike.
fn falling_on(view: &dyn CommittedView, thing: EntityId) -> i64 {
    if !crate::fire::Fire::in_the_open(view, thing) {
        return 0;
    }
    [RAIN, SNOW]
        .into_iter()
        .map(|fact| crate::climate::ambient(view, thing, fact).map_or(0, |(_, r)| r.max(0)))
        .sum()
}

/// Whether `thing` stands in water (Amendment A-39): in a pond, river, or sea not frozen hard, or
/// on flooded ground.
fn in_water(view: &dyn CommittedView, thing: EntityId) -> bool {
    crate::index::container_of(view, thing).is_some_and(|place| {
        let level = int(view, FactKey::new(place, crate::schema::WATER_LEVEL));
        let ice = int(view, FactKey::new(place, crate::schema::ICE));
        (level > 0 && ice == 0) || int(view, FactKey::new(place, crate::schema::FLOOD_DEPTH)) > 0
    })
}

/// The wind on a thing: its climate's, in the open; none behind walls.
fn wind_on(view: &dyn CommittedView, body: EntityId) -> i64 {
    if crate::fire::Fire::in_the_open(view, body) {
        crate::climate::ambient(view, body, WIND_SPEED).map_or(0, |(_, w)| w.max(0))
    } else {
        0
    }
}

impl System for Wet {
    fn id(&self) -> SystemId {
        SystemId::new("physical.wet")
    }
    fn reads(&self) -> &'static [FactType] {
        WET_READS
    }
    fn writes(&self) -> &'static [FactType] {
        WET_WRITES
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
        // These are levels that vanish at zero, and others may add to them in the same tick
        // (fire dries what it heats): always a difference.
        let set = |held: i64, next: i64| Change::Delta(next - held);

        // 1. The ground and its snow, climate by climate.
        let mut falling_anywhere = false;
        for region in view.entities_with(TEMPERATURE) {
            if crate::climate::is_sheltered(view, region) {
                continue;
            }
            let mut rng = ctx.rng(region.raw() ^ 0x3e7);
            let rain = int(view, FactKey::new(region, RAIN)).max(0);
            let snow = int(view, FactKey::new(region, SNOW)).max(0);
            falling_anywhere |= rain > 0 || snow > 0;
            let depth = int(view, FactKey::new(region, SNOW_DEPTH)).max(0);
            // Snow builds ten times as deep as its water (hundredths of a millimetre of water an
            // hour make tenths of a millimetre of snow), and melts above freezing.
            let fallen = self.over(snow / 10, &mut rng);
            let warmth = int(view, FactKey::new(region, TEMPERATURE)).max(0);
            let melted = self
                .over(r.snow_melt_mm_per_degree_hour * warmth / 100, &mut rng)
                .min(depth + fallen);
            let next_depth = depth + fallen - melted;
            if next_depth != depth {
                push(
                    FactKey::new(region, SNOW_DEPTH),
                    set(depth, next_depth),
                    if next_depth > depth {
                        "snowing"
                    } else {
                        "melting"
                    },
                );
            }
            // The ground: rain and melt-water soak in; it dries, unless snow lies on it.
            let ground = int(view, FactKey::new(region, GROUND_WATER)).clamp(0, PERCENT_FULL);
            // Rain is a rate, in hundredths of a millimetre an hour; what melted this step is an
            // amount of snow, a tenth of it water.
            let soaked =
                self.over(rain * r.soak_per_mm / 100, &mut rng) + melted * r.soak_per_mm / 10;
            let wind = int(view, FactKey::new(region, WIND_SPEED));
            let dried = if next_depth > 0 {
                0
            } else {
                self.over(
                    Self::drying(view, region, r.ground_drying_per_hour, wind),
                    &mut rng,
                )
            };
            let next_ground = (ground + soaked - dried).clamp(0, PERCENT_FULL);
            if next_ground != ground {
                push(
                    FactKey::new(region, GROUND_WATER),
                    set(ground, next_ground),
                    if next_ground > ground {
                        "soaked"
                    } else {
                        "dried"
                    },
                );
            }
        }

        // 2. Things: those in the rain, while it falls anywhere; whatever stands in water or on
        //    flooded ground (Amendment A-39); and whatever is wet.
        let mut things: BTreeSet<EntityId> = view.entities_with(WETNESS).into_iter().collect();
        if falling_anywhere {
            things.extend(view.entities_with(MADE_OF));
            things.extend(view.entities_with(MOBILE));
        }
        for place in view
            .entities_with(crate::schema::WATER_LEVEL)
            .into_iter()
            .chain(view.entities_with(crate::schema::FLOOD_DEPTH))
        {
            things.extend(crate::nearby::contents(view, place));
        }
        for thing in things {
            if view.read(FactKey::new(thing, CONTAINED_IN)).is_none() {
                continue;
            }
            let mut rng = ctx.rng(thing.raw() ^ 0x3e7);
            let wet = int(view, FactKey::new(thing, WETNESS)).clamp(0, PERCENT_FULL);
            let falling = falling_on(view, thing);
            let soaking = in_water(view, thing);
            let gained = if soaking {
                PERCENT_FULL
            } else {
                self.over(
                    r.wetting_per_hour * falling / r.hardest_rain.max(1),
                    &mut rng,
                )
            };
            let dried = if wet > 0 && falling == 0 && !soaking {
                let wind = wind_on(view, thing);
                self.over(Self::drying(view, thing, r.drying_per_hour, wind), &mut rng)
            } else {
                0
            };
            let next = (wet + gained - dried).clamp(0, PERCENT_FULL);
            if next != wet {
                push(
                    FactKey::new(thing, WETNESS),
                    set(wet, next),
                    if next > wet { "rained_on" } else { "dried" },
                );
            }
        }

        out
    }
}
