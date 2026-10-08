//! Water (Vol. III Ch. 1 §1.10, "water level", "rivers flood", "tides"; §1.11, "water flows
//! downhill"; Amendment A-39).
//!
//! A world declares its water: ponds, rivers, and the sea, each a place with a normal depth, the
//! depth at which it tops its banks, the land that drains into it, and how fast it drains; which
//! water it flows on into; and the low ground it floods. On the environment's cadence:
//!
//! - **The sea** stands at its normal depth plus the tide: a swing of the world's amplitude over
//!   the world's tidal period, largest at new and full moon, smallest at the quarters.
//! - **Ponds and rivers** take in the rain that falls on them, the rain and melting snow that run
//!   off their land — more, the wetter that land already is — and whatever flows on into them.
//!   What stands above their normal depth flows on, as fast as each drains, into the water
//!   downstream, within the same step, upstream first: water flows downhill, and what leaves one
//!   arrives in the next. Each level is solved in the step's own end, so a river stands as high
//!   at any tick length (Amendment A-1).
//! - **Floods.** Water above its banks spills over the low ground the world says it floods, as
//!   deep as it stands above them, and drains off it as the water falls.
//! - **Ice** forms on water in frost, the thicker the harder it freezes, and melts in a thaw.
//!
//! Water stands in the way of walkers ([`crate::travel`]): no one walks into water, or onto
//! flooded ground, deeper than the world's wading depth, unless the ice will bear them. What stands
//! in water is soaked ([`crate::wet`]).

use crate::schema::{
    BODY_SIZE, CATCHMENT, DRAINS_TO, FLOODS, FLOOD_DEPTH, GROUND_WATER, ICE, PERCENT_FULL, RAIN,
    SNOW_DEPTH, TEMPERATURE, WATER_BANK, WATER_DRAIN_SECONDS, WATER_KIND, WATER_LEVEL,
    WATER_NORMAL,
};
use kernel::fact::{Cause, FactKey, FactType, SystemId};
use kernel::fixed::{div_dither, sin_cos, FULL_TURN, TRIG_ONE};
use kernel::identity::EntityId;
use kernel::proposal::{Change, Proposal};
use kernel::rng::Rng;
use kernel::system::{Cadence, CommittedView, System, TickContext};
use kernel::time::{SimClock, Step};
use kernel::value::Value;
use std::collections::BTreeMap;

/// A pond: still water that fills and drains slowly.
pub const POND: i64 = 1;
/// A river: water that flows on.
pub const RIVER: i64 = 2;
/// The sea: it rises and falls with the tide.
pub const SEA: i64 = 3;

/// How water behaves, all world rules (Vol. IV Ch. 2 §2.2).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct WaterRules {
    /// Half the sea's rise and fall at a spring tide, in centimetres.
    pub tide_amplitude_cm: i64,
    /// From one high tide to the next, in milliseconds.
    pub tidal_period_ms: u64,
    /// How many millimetres of ice form an hour for each degree of frost, and melt for each degree
    /// of thaw.
    pub ice_mm_per_degree_hour: i64,
    /// The deepest water, in centimetres, one walks through.
    pub wade_depth_cm: i64,
    /// How thick ice must be, in millimetres, to bear a walker.
    pub ice_bears_mm: i64,
    /// How many millimetres of snow melt an hour for each degree above freezing (A-38's rule):
    /// the melt-water that runs off the land.
    pub snow_melt_mm_per_degree_hour: i64,
}

fn int(view: &dyn CommittedView, key: FactKey) -> Option<i64> {
    view.read(key).and_then(|f| f.value.as_int())
}

/// How deep the water stands over `place`, in centimetres: its own water, or the flood over it.
pub fn depth(view: &dyn CommittedView, place: EntityId) -> i64 {
    int(view, FactKey::new(place, WATER_LEVEL))
        .or_else(|| int(view, FactKey::new(place, FLOOD_DEPTH)))
        .unwrap_or(0)
        .max(0)
}

/// Whether one can walk into `place` (Amendment A-39): its water, or the flood over it, no deeper
/// than one wades, or frozen thick enough to bear one.
pub fn passable(view: &dyn CommittedView, place: EntityId, wade_cm: i64, bears_mm: i64) -> bool {
    depth(view, place) <= wade_cm || int(view, FactKey::new(place, ICE)).unwrap_or(0) >= bears_mm
}

/// How much of `place`'s footprint there is, in square centimetres; none without a size.
fn area_cm2(view: &dyn CommittedView, place: EntityId) -> i128 {
    view.read(FactKey::new(place, BODY_SIZE))
        .and_then(|f| f.value.as_vec3())
        .map_or(0, |[hw, hd, _]| 4 * hw.max(0) as i128 * hd.max(0) as i128)
}

const WATER_READS: &[FactType] = &[
    WATER_KIND,
    WATER_LEVEL,
    WATER_NORMAL,
    WATER_BANK,
    WATER_DRAIN_SECONDS,
    CATCHMENT,
    DRAINS_TO,
    FLOODS,
    FLOOD_DEPTH,
    ICE,
    BODY_SIZE,
    RAIN,
    SNOW_DEPTH,
    GROUND_WATER,
    TEMPERATURE,
    crate::schema::CONTAINED_IN,
];
const WATER_WRITES: &[FactType] = &[WATER_LEVEL, FLOOD_DEPTH, ICE];

/// Water: the tide, rivers and ponds filling and draining into one another, floods, and ice
/// (Amendment A-39).
pub struct Water {
    clock: SimClock,
    step: Step,
    rules: WaterRules,
    calendar: crate::calendar::Calendar,
}

impl Water {
    /// Water by `rules`, stepping as `step` says on `clock`; the tide follows `calendar`'s moon.
    pub const fn new(
        clock: SimClock,
        step: Step,
        rules: WaterRules,
        calendar: crate::calendar::Calendar,
    ) -> Self {
        Self {
            clock,
            step,
            rules,
            calendar,
        }
    }

    /// `per_hour` over one step, dithered.
    fn over(&self, per_hour: i64, rng: &mut Rng) -> i64 {
        div_dither(per_hour as i128 * self.step.dt_ms as i128, 3_600_000, rng) as i64
    }

    /// The tide at `at_ms`, in centimetres above the sea's normal: a cosine over the tidal period,
    /// scaled from three-quarters at the moon's quarters to all of it at new and full moon.
    pub fn tide(&self, at_ms: u64) -> i64 {
        let r = &self.rules;
        if r.tidal_period_ms == 0 {
            return 0;
        }
        let period = r.tidal_period_ms as i128;
        let angle = ((at_ms as i128).rem_euclid(period) * FULL_TURN as i128 / period) as i64;
        let one = TRIG_ONE as i128;
        let spring = self.calendar.moon_phase(at_ms).map_or(one, |phase| {
            let twice = 2 * phase * FULL_TURN / 10_000;
            3 * one / 4 + sin_cos(twice).1 as i128 / 4
        });
        (r.tide_amplitude_cm as i128 * sin_cos(angle).1 as i128 / one * spring / one) as i64
    }

    /// The water running into `body` over one step from the sky and its land, in cubic
    /// centimetres: the rain on its own face, all of it; and the rain and melt-water on its land,
    /// as much of it as the land is wet already — dry ground drinks it.
    fn from_the_sky(&self, view: &dyn CommittedView, body: EntityId, area: i128) -> i128 {
        let ambient = |fact| crate::climate::ambient(view, body, fact).map_or(0, |(_, v)| v);
        let rain = ambient(RAIN).max(0) as i128; // hundredths of a millimetre an hour
        let melt = if ambient(SNOW_DEPTH) > 0 {
            // Millimetres of snow an hour; a tenth of it water, in hundredths of a millimetre.
            (self.rules.snow_melt_mm_per_degree_hour * ambient(TEMPERATURE).max(0) / 100 * 10)
                as i128
        } else {
            0
        };
        let wet = ambient(GROUND_WATER).clamp(0, PERCENT_FULL) as i128;
        let land = int(view, FactKey::new(body, CATCHMENT)).unwrap_or(0).max(0) as i128 * 10_000;
        let dt = self.step.dt_ms as i128;
        // Hundredths of a millimetre an hour over square centimetres: a thousandth of a cubic
        // centimetre an hour.
        let on_face = rain * area;
        let off_land = (rain + melt) * land * wet / PERCENT_FULL as i128;
        (on_face + off_land) * dt / (1_000 * 3_600_000)
    }
}

impl System for Water {
    fn id(&self) -> SystemId {
        SystemId::new("physical.water")
    }
    fn reads(&self) -> &'static [FactType] {
        WATER_READS
    }
    fn writes(&self) -> &'static [FactType] {
        WATER_WRITES
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
        // Upstream first: each body's water flows on within the step into the one below it, so a
        // river's level is the same whatever the step's length.
        let downstream =
            |body: EntityId| match view.read(FactKey::new(body, DRAINS_TO)).map(|f| f.value) {
                Some(Value::Entity(to)) => Some(to),
                _ => None,
            };
        let mut bodies: Vec<(usize, EntityId)> = view
            .entities_with(WATER_KIND)
            .into_iter()
            .map(|body| {
                let (mut hops, mut at) = (0usize, body);
                while let (Some(next), true) = (downstream(at), hops < 64) {
                    (hops, at) = (hops + 1, next);
                }
                (hops, body)
            })
            .collect();
        bodies.sort_unstable_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
        let dt = self.step.dt_ms as i128;
        let mut received: BTreeMap<EntityId, i128> = BTreeMap::new();
        let now = self.clock.ms_at(ctx.tick());
        let mut flood: BTreeMap<EntityId, (i64, i128)> = BTreeMap::new();
        for &(_, body) in &bodies {
            let mut rng = ctx.rng(body.raw() ^ 0x3a7e);
            let kind = int(view, FactKey::new(body, WATER_KIND)).unwrap_or(0);
            let level = int(view, FactKey::new(body, WATER_LEVEL)).unwrap_or(0);
            let normal = int(view, FactKey::new(body, WATER_NORMAL)).unwrap_or(0);
            let drain_ms = int(view, FactKey::new(body, WATER_DRAIN_SECONDS))
                .unwrap_or(0)
                .max(1) as i128
                * 1000;
            let next = if kind == SEA {
                // The sea is the sea: what runs into it is lost in it, and it keeps the tide.
                normal + self.tide(now)
            } else {
                let area = area_cm2(view, body).max(1);
                let gained =
                    self.from_the_sky(view, body, area) + received.get(&body).copied().unwrap_or(0);
                // Solved in the step's own end: level' = level + gained/area − (level' −
                // normal)·dt/τ, above its normal; below it, nothing flows on.
                let filled = level as i128 * area + gained;
                let mut next = div_dither(filled, area, &mut rng) as i64;
                if next > normal {
                    next = div_dither(
                        filled * drain_ms + normal as i128 * area * dt,
                        area * (drain_ms + dt),
                        &mut rng,
                    ) as i64;
                    let flowed = (next - normal).max(0) as i128 * area * dt / drain_ms;
                    if let Some(to) = downstream(body) {
                        *received.entry(to).or_insert(0) += flowed;
                    }
                }
                next
            }
            .max(0);
            if next != level {
                push(
                    FactKey::new(body, WATER_LEVEL),
                    Change::Set(Value::Int(next)),
                    if next > level { "rising" } else { "falling" },
                );
            }
            // Over its banks, onto the low ground beside it.
            let bank = int(view, FactKey::new(body, WATER_BANK)).unwrap_or(i64::MAX);
            for f in view.read_all(FactKey::new(body, FLOODS)) {
                if let Value::Entity(ground) = f.value {
                    let over = (next - bank).max(0);
                    let entry = flood.entry(ground).or_insert((0, drain_ms));
                    if over >= entry.0 {
                        *entry = (over, drain_ms);
                    }
                }
            }
            // Ice, in frost; gone in a thaw.
            let air = crate::climate::ambient_temperature(view, body).unwrap_or(0);
            let ice = int(view, FactKey::new(body, ICE)).unwrap_or(0);
            let change = self.over(r.ice_mm_per_degree_hour * -air / 100, &mut rng);
            let next_ice = (ice + change).max(0);
            if next_ice != ice {
                push(
                    FactKey::new(body, ICE),
                    Change::Delta(next_ice - ice),
                    if next_ice > ice {
                        "freezing"
                    } else {
                        "thawing"
                    },
                );
            }
        }
        // 3. Flooded ground: the water over it rises toward what stands above the banks, and
        //    drains as the water falls, as fast as the water that floods it drains.
        let mut grounds: BTreeMap<EntityId, (i64, i128)> = flood;
        for ground in view.entities_with(FLOOD_DEPTH) {
            grounds.entry(ground).or_insert((0, 3_600_000));
        }
        for (ground, (over, drain_ms)) in grounds {
            let mut rng = ctx.rng(ground.raw() ^ 0xf100d);
            let held = int(view, FactKey::new(ground, FLOOD_DEPTH)).unwrap_or(0);
            let step =
                div_dither((over - held) as i128 * dt.min(drain_ms), drain_ms, &mut rng) as i64;
            if step != 0 {
                push(
                    FactKey::new(ground, FLOOD_DEPTH),
                    Change::Delta(step),
                    if step > 0 { "flooding" } else { "draining" },
                );
            }
        }
        out
    }
}
