//! Hermetic transformations owned by the Physical Reality domain (Vol. V Ch. 3 §3.1).
//!
//! Each system declares its read/write sets and cadence, reads committed reality, and emits
//! proposals — mutating nothing directly (Vol. V Ch. 3 §3.1-3.2). These drive the
//! environmental fields that vary across space and time (Vol. III Ch. 1 §1.10): temperature
//! (a deterministic day/night swing plus stochastic weather), illumination (the sun's
//! position), humidity and pressure (weather), and wind. Implement causes, never outcomes
//! (Vol. III Ch. 11).
//!
//! Every system here is **scope-generic**: one instance discovers its subjects from
//! committed reality each tick — regions by the [`TEMPERATURE`] fact every region carries,
//! portal hosts by [`HAS_PORTAL`] — rather than being pinned to an entity list at
//! construction (Vol. V Ch. 2 §2.1, clause 5, queries are the product). A region that comes
//! into being mid-simulation is simulated the moment its facts commit; systems hold no
//! state between ticks (Vol. II Ch. 3).
//!
//! **Time has units** (Vol. II Ch. 2, *Simulated Duration*, Amendment A-1). The environmental
//! systems step on a cadence declared in simulated time (the world's
//! `environment_step_seconds`), and every rule is a rate or a statistic over simulated time —
//! a day length, a spread and a memory for weather — never an amount per tick. Two techniques
//! keep fixed-point arithmetic from eroding at fine resolution:
//!
//! - **Telescoping absolute levels.** A system that adds to a shared field (the day/night
//!   swing, the weather's share of temperature) proposes the difference between its absolute
//!   contribution at the end of the step and at the start. The differences sum exactly to the
//!   current contribution, so no step's rounding is ever lost.
//! - **Fine-grained anomalies.** Weather's departure from normal is carried in its own fact at
//!   [`ANOMALY_SCALE`] times the field's unit, so a step's tiny change survives; only the
//!   rounded level reaches the field itself.

use crate::climate::{daylight_fraction, exposure_of, is_sheltered, outside_temperature};
use crate::materials::thermal_mass_of;
use crate::schema::{
    ADJACENT_TO, ANOMALY_SCALE, BODY_SIZE, CONTAINED_IN, ELEVATION, ENCLOSED, EXPOSURE, HAS_PORTAL,
    HEADING, HUMIDITY, HUMIDITY_ANOMALY, ILLUMINATION, LEADS_TO, MADE_OF, MATERIAL_DENSITY,
    MATERIAL_THERMAL_CAPACITY, MAX_DANGER, MAX_PRESSURE, MOTION_END, MOTION_START, MOTION_TARGET,
    OPAQUE, PERCENT_FULL, PORTAL_DANGER, PORTAL_DANGER_OVERRIDE, PORTAL_FAR_SIDE, PORTAL_OPEN,
    POSITION, PRESSURE, PRESSURE_ANOMALY, TEMPERATURE, TEMPERATURE_ANOMALY, TERRAIN_SAMPLE,
    TERRAIN_SPACING, WIND_SPEED, WIND_TOWARD,
};
use crate::space::{far_side, height_above_ground, portal_destination};
use kernel::fact::{Cause, FactKey, FactType, SystemId};
use kernel::fixed::{div_dither, div_round, isqrt};
use kernel::hierarchy::lowest_common_ancestor;
use kernel::identity::EntityId;
use kernel::proposal::{Change, Proposal};
use kernel::rng::Rng;
use kernel::system::{Cadence, CommittedView, System, TickContext};
use kernel::time::{SimClock, Step};
use kernel::value::Value;

// Read sets. Every weather system also reads TEMPERATURE — not for its value, but to
// enumerate the regions to simulate: the loader guarantees each region carries a temperature
// fact, so `entities_with(TEMPERATURE)` is the region roster (Vol. V Ch. 2 §2.1, clause 5).
// The temperature systems additionally read a region's composition (MADE_OF) and its
// materials' density and specific heat, to damp the swing by thermal mass (Vol. III Ch. 1 §1.9,
// §1.10; Amendment A-6).
const DIURNAL_READS: &[FactType] = &[
    TEMPERATURE,
    EXPOSURE,
    ENCLOSED,
    MADE_OF,
    MATERIAL_DENSITY,
    MATERIAL_THERMAL_CAPACITY,
];
const TEMPERATURE_W: &[FactType] = &[TEMPERATURE];
const WEATHER_READS: &[FactType] = &[
    TEMPERATURE,
    TEMPERATURE_ANOMALY,
    ADJACENT_TO,
    WIND_TOWARD,
    WIND_SPEED,
    EXPOSURE,
    ENCLOSED,
    MADE_OF,
    MATERIAL_DENSITY,
    MATERIAL_THERMAL_CAPACITY,
];
const WEATHER_WRITES: &[FactType] = &[TEMPERATURE, TEMPERATURE_ANOMALY];
const ILLUMINATION_READS: &[FactType] = &[
    TEMPERATURE,
    crate::schema::LATITUDE,
    crate::schema::CLOUD,
    CONTAINED_IN,
    EXPOSURE,
    ENCLOSED,
    HAS_PORTAL,
    LEADS_TO,
    PORTAL_OPEN,
    OPAQUE,
    BODY_SIZE,
];
const ILLUMINATION_W: &[FactType] = &[ILLUMINATION];
const PRESSURE_READS: &[FactType] = &[
    TEMPERATURE,
    PRESSURE_ANOMALY,
    ELEVATION,
    EXPOSURE,
    ENCLOSED,
    ADJACENT_TO,
    WIND_TOWARD,
    WIND_SPEED,
];
const PRESSURE_WRITES: &[FactType] = &[PRESSURE, PRESSURE_ANOMALY];
const WIND_READS: &[FactType] = &[
    TEMPERATURE,
    ADJACENT_TO,
    PRESSURE,
    ELEVATION,
    EXPOSURE,
    ENCLOSED,
    crate::schema::STORM,
    PRESSURE_ANOMALY,
];
const WIND_WRITES: &[FactType] = &[WIND_SPEED, WIND_TOWARD];
// Danger reads a portal's height above the ground, composed up its containers — their
// positions (live, so their motion facts too) and headings.
const SHELTER_READS: &[FactType] = &[
    ENCLOSED,
    EXPOSURE,
    TEMPERATURE,
    CONTAINED_IN,
    MADE_OF,
    MATERIAL_DENSITY,
    MATERIAL_THERMAL_CAPACITY,
];
const DANGER_READS: &[FactType] = &[
    PORTAL_DANGER,
    HAS_PORTAL,
    PORTAL_DANGER_OVERRIDE,
    CONTAINED_IN,
    POSITION,
    HEADING,
    MOTION_TARGET,
    MOTION_START,
    MOTION_END,
    LEADS_TO,
    PORTAL_FAR_SIDE,
    TERRAIN_SPACING,
    TERRAIN_SAMPLE,
];
const DANGER_WRITES: &[FactType] = &[PORTAL_DANGER];

/// The day's temperature curve (Vol. III Ch. 1 §1.10; Amendments A-1, A-35): an integer
/// triangle in `0..=amplitude` over a day, coldest at `coldest_ms` into the day, rising straight
/// to its warmest at `warmest_ms`, and falling straight back — coldest about dawn and warmest in
/// the afternoon, as air is, if the world says so; at midnight and noon if it does not.
///
/// Its argument is simulated time, not a tick count, so it has the same shape at any tick
/// length (Amendment A-1). Systems that add to a field propose the *difference* between the
/// curve at the two ends of their step, so summed deltas telescope back to the curve itself.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct DayCurve {
    /// The length of a day, in milliseconds.
    pub day_ms: u64,
    /// The swing from coldest to warmest, in centidegrees.
    pub amplitude_centi_c: i64,
    /// When in the day it is coldest, in milliseconds after midnight.
    pub coldest_ms: u64,
    /// When in the day it is warmest.
    pub warmest_ms: u64,
}

impl DayCurve {
    /// Where the curve stands at `at_ms`, in `0..=amplitude`.
    pub fn level(&self, at_ms: u64) -> i64 {
        let day = self.day_ms.max(2) as i128;
        let phase = (at_ms as i128 - self.coldest_ms as i128).rem_euclid(day);
        let mut rise = (self.warmest_ms as i128 - self.coldest_ms as i128).rem_euclid(day);
        if rise == 0 {
            rise = day / 2;
        }
        let amp = self.amplitude_centi_c as i128;
        let up = if phase <= rise {
            amp * phase / rise
        } else {
            amp * (day - phase) / (day - rise)
        };
        up as i64
    }
}

/// Scale `value` by `exposure` (0..=10000): full exposure passes it unchanged, a sealed
/// chamber (0) blocks it entirely.
fn attenuate(value: i64, exposure: i64) -> i64 {
    value.saturating_mul(exposure) / PERCENT_FULL
}

/// A region's thermal-mass damping factor for temperature changes, in hundredths of a percent
/// (0..=10000), from the thermal mass of the materials it is built of — heat stored per unit
/// volume (Vol. III Ch. 1 §1.9, §1.10; Amendment A-6). Thermal mass is inertia: a heavy stone
/// hall resists the day/night swing a canvas tent cannot. The factor is
/// `reference / (reference + mass)` — 1.0 (no damping) when the region has no thermal-mass
/// material, falling toward 0 as mass grows past `reference`, the world-tuned thermal mass at
/// which the swing is halved. A region that declares
/// no composition is unaffected, so worlds without materials behave exactly as before.
fn thermal_damping(view: &dyn CommittedView, region: EntityId, reference: i64) -> i64 {
    let mass = thermal_mass_of(view, region).unwrap_or(0).max(0);
    let reference = reference.max(0);
    let denom = reference.saturating_add(mass);
    if denom == 0 {
        // No reference and no mass: nothing to damp.
        return PERCENT_FULL;
    }
    (reference.saturating_mul(PERCENT_FULL) / denom).clamp(0, PERCENT_FULL)
}

/// How far above its coldest the day's swing stands in `region` at `at_ms`, in centidegrees
/// (Amendment A-35): `curve`, damped by the region's exposure and by its thermal mass against
/// `thermal_mass_reference` — what [`DiurnalCycle`] adds to the region's declared coldest.
pub fn day_share(
    view: &dyn CommittedView,
    region: EntityId,
    curve: &DayCurve,
    thermal_mass_reference: i64,
    at_ms: u64,
) -> i64 {
    let exposure = exposure_of(view, region);
    let mass = thermal_damping(view, region, thermal_mass_reference);
    attenuate(attenuate(curve.level(at_ms), exposure), mass)
}

/// The regions the sky's weather runs on this tick: every climate region — an entity carrying a
/// committed temperature — that is not sheltered (Amendment A-5).
///
/// Temperature marks a climate (Vol. V Ch. 2 §2.1, clause 5), and places nested inside a climate
/// without one inherit it (`crate::climate`), so the weather is simulated once per climate, not
/// once per room. A sheltered room's temperature is driven by [`Shelter`] instead.
fn regions(view: &dyn CommittedView) -> Vec<EntityId> {
    view.entities_with(TEMPERATURE)
        .into_iter()
        .filter(|r| !is_sheltered(view, *r))
        .collect()
}

/// Every climate region, sheltered or not — the roster for daylight, which reaches indoors
/// through openings.
fn all_climates(view: &dyn CommittedView) -> Vec<EntityId> {
    view.entities_with(TEMPERATURE)
}

/// A mean-reverting random process declared by its statistics (Amendment A-1: "stochastic
/// rules are declared by their statistics, not their steps"): an anomaly that wanders about
/// zero with a stationary standard deviation of `spread` and a memory of `memory` — how long a
/// spell of weather tends to last.
///
/// Discretised as a first-order autoregression over the step's simulated duration `dt`:
///
/// ```text
/// next = keep · current + noise,   keep = memory / (memory + dt),
/// noise ~ uniform(−w, +w),         w = spread · √(3 · (1 − keep²))
/// ```
///
/// The width `w` is chosen so the stationary variance is exactly `spread²` whatever `dt` is
/// (`keep²·σ² + w²/3 = σ²`), which is what makes the weather's spread independent of tick
/// length. `keep` is the implicit-Euler decay factor: always in `[0, 1)`, so the process
/// reverts toward zero and can never wander without bound — the defect the shipped random walk
/// had (`docs/audits/3d-world-readiness.md` §4.1). For short steps it approximates the
/// continuous-time process `e^(−dt/memory)` closely; for steps much longer than the memory, each
/// step is nearly independent weather, which is the right limit.
///
/// The anomaly is carried at [`ANOMALY_SCALE`] times the field's unit, so the small per-step
/// change at a fine tick length is not lost to rounding.
#[derive(Clone, Copy, Debug)]
struct Weather {
    /// `keep`, scaled by `KEEP_ONE`.
    keep: i128,
    /// Noise half-width `w`, in anomaly sub-units.
    half_width: i64,
}

/// Fixed-point scale of [`Weather::keep`].
const KEEP_ONE: i128 = 1 << 30;

impl Weather {
    /// A process with stationary standard deviation `spread` (in the field's own unit) and
    /// memory `memory_ms`, stepped every `dt_ms`.
    fn new(spread: i64, memory_ms: u64, dt_ms: u64) -> Self {
        let keep = (memory_ms as i128 * KEEP_ONE) / (memory_ms as i128 + dt_ms.max(1) as i128);
        let unit_width = isqrt((3 * (KEEP_ONE * KEEP_ONE - keep * keep)) as u128) as i128;
        let half_width = (spread.max(0) as i128 * ANOMALY_SCALE as i128 * unit_width) / KEEP_ONE;
        Self {
            keep,
            half_width: half_width.min(i64::MAX as i128 / 2) as i64,
        }
    }

    /// One step of the process from `anomaly` (sub-units), drawing from `rng`.
    fn step(&self, anomaly: i64, rng: &mut Rng) -> i64 {
        let kept = div_round(anomaly as i128 * self.keep, KEEP_ONE) as i64;
        let w = self.half_width;
        let noise = rng.below((w as u64) * 2 + 1) as i64 - w;
        kept.saturating_add(noise)
    }
}

/// An anomaly in sub-units, rounded to the field's own unit.
fn anomaly_level(anomaly: i64) -> i64 {
    div_round(anomaly as i128, ANOMALY_SCALE as i128) as i64
}

/// A region's committed anomaly for `fact`, in sub-units; zero (normal) if none yet.
fn anomaly_of(view: &dyn CommittedView, region: EntityId, fact: FactType) -> i64 {
    view.read(FactKey::new(region, fact))
        .and_then(|f| f.value.as_int())
        .unwrap_or(0)
}

/// How weather passes between neighbouring climates (Amendment A-36), world rules: each
/// climate's weather — its departures from normal temperature, humidity, and pressure — mixes
/// with its neighbours' with a time constant of `mixing_ms`, and the wind blowing into it from a
/// neighbour carries that neighbour's weather across `distance_cm`, the distance between their
/// middles. Zero turns either off.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Fronts {
    /// The time constant of mixing between neighbours, in milliseconds.
    pub mixing_ms: u64,
    /// How far the wind carries weather from a neighbour's middle to this climate's, in
    /// centimetres.
    pub distance_cm: i64,
}

/// Fixed-point scale of [`Fronts::mix`]'s weights.
const MIX_ONE: i128 = 1 << 30;

impl Fronts {
    /// `region`'s committed anomaly of `fact`, mixed over a step of `dt_ms` with its neighbours'
    /// (Amendment A-36): a weighted mean, its own at weight one, each neighbouring climate's at
    /// the share of the step's mixing and of the wind it blows this way. Implicit in its own value
    /// — a mean always lies between what it averages — so stable at any step length.
    fn mix(&self, view: &dyn CommittedView, region: EntityId, fact: FactType, dt_ms: u64) -> i64 {
        let own = anomaly_of(view, region, fact);
        if self.mixing_ms == 0 && self.distance_cm <= 0 {
            return own;
        }
        let dt = dt_ms as i128;
        let mut sum = own as i128 * MIX_ONE;
        let mut weight = MIX_ONE;
        let mut neighbours: Vec<EntityId> = view
            .read_all(FactKey::new(region, ADJACENT_TO))
            .into_iter()
            .filter_map(|f| match f.value {
                Value::Entity(n) => Some(n),
                _ => None,
            })
            .filter(|n| {
                view.read(FactKey::new(*n, TEMPERATURE)).is_some() && !is_sheltered(view, *n)
            })
            .collect();
        neighbours.sort_unstable();
        neighbours.dedup();
        for n in neighbours {
            let mut w: i128 = 0;
            if self.mixing_ms > 0 {
                w += dt * MIX_ONE / self.mixing_ms as i128;
            }
            let toward = view.read(FactKey::new(n, WIND_TOWARD)).map(|f| f.value);
            if self.distance_cm > 0 && toward == Some(Value::Entity(region)) {
                let wind = view
                    .read(FactKey::new(n, WIND_SPEED))
                    .and_then(|f| f.value.as_int())
                    .unwrap_or(0)
                    .max(0) as i128;
                w += wind * dt * MIX_ONE / (1000 * self.distance_cm as i128);
            }
            sum += w * anomaly_of(view, n, fact) as i128;
            weight += w;
        }
        div_round(sum, weight) as i64
    }
}

/// A deterministic day/night temperature swing (Vol. III Ch. 1 §1.10) for every region —
/// damped by the region's exposure and its thermal mass (§1.9).
///
/// The swing is a triangle wave over the world's day length in simulated time. Each run
/// proposes the change in the region's *damped* swing between the start and end of its step:
/// damping the absolute level and then differencing (rather than damping each step's small
/// difference) means the differences telescope to exactly the damped wave, so a sheltered
/// region keeps its full damped swing at any tick length. Damping the difference instead is
/// what made sheltered regions lose their day entirely at one-minute ticks (audit §4.1).
pub struct DiurnalCycle {
    clock: SimClock,
    step: Step,
    curve: DayCurve,
    thermal_mass_reference: i64,
}

impl DiurnalCycle {
    /// Swing every region along `curve`, stepping as `step` says. `thermal_mass_reference` is the
    /// thermal mass (kJ/(m³·K)) at which a region's swing is halved (Vol. III Ch. 1 §1.9;
    /// Amendment A-6); larger means thermal mass matters less.
    pub const fn new(
        clock: SimClock,
        step: Step,
        curve: DayCurve,
        thermal_mass_reference: i64,
    ) -> Self {
        Self {
            clock,
            step,
            curve,
            thermal_mass_reference,
        }
    }
}

impl System for DiurnalCycle {
    fn id(&self) -> SystemId {
        SystemId::new("physical.diurnal_cycle")
    }
    fn reads(&self) -> &'static [FactType] {
        DIURNAL_READS
    }
    fn writes(&self) -> &'static [FactType] {
        TEMPERATURE_W
    }
    fn cadence(&self) -> Cadence {
        self.step.cadence()
    }
    fn evaluate(&self, view: &dyn CommittedView, ctx: &TickContext) -> Vec<Proposal> {
        let now = self.curve.level(self.clock.ms_at(ctx.tick()));
        let then = self
            .curve
            .level(self.clock.ms_at(self.step.start_of(ctx.tick())));
        regions(view)
            .into_iter()
            .map(|region| {
                let exposure = exposure_of(view, region);
                let mass = thermal_damping(view, region, self.thermal_mass_reference);
                // Both dampers apply to the absolute level: an open field of heavy stone still
                // swings less than an open field of nothing (§1.9, §1.10).
                let damped = |level| attenuate(attenuate(level, exposure), mass);
                Proposal::new(
                    self.id(),
                    FactKey::new(region, TEMPERATURE),
                    ctx.basis_tick(),
                    Change::Delta(damped(now) - damped(then)),
                    Cause::new("diurnal_shift"),
                )
            })
            .collect()
    }
}

/// Weather's share of temperature: a spell-by-spell departure from the region's normal, for
/// every region (Vol. III Ch. 1 §1.10) — the mean-reverting weather process (`Weather`,
/// above), damped like the day/night swing by exposure and thermal mass.
///
/// The anomaly itself lives in [`TEMPERATURE_ANOMALY`] at fine resolution; temperature
/// receives the change in the anomaly's damped, rounded level, so temperature always equals
/// the region's normal plus the diurnal swing plus exactly the current weather — nothing
/// accumulates. Each region draws from its own substream (scope = region id), so its weather is
/// independent and replays identically (Vol. V Ch. 4 §4.1).
pub struct TemperatureWeather {
    step: Step,
    weather: Weather,
    thermal_mass_reference: i64,
    fronts: Fronts,
}

impl TemperatureWeather {
    /// Weather whose temperature departures have a typical size (standard deviation) of
    /// `spread_centi_c` and persist for about `memory_ms` of simulated time, stepping as `step`
    /// says.
    pub fn new(
        step: Step,
        spread_centi_c: i64,
        memory_ms: u64,
        thermal_mass_reference: i64,
        fronts: Fronts,
    ) -> Self {
        Self {
            step,
            weather: Weather::new(spread_centi_c, memory_ms, step.dt_ms),
            thermal_mass_reference,
            fronts,
        }
    }
}

impl System for TemperatureWeather {
    fn id(&self) -> SystemId {
        SystemId::new("physical.temperature_weather")
    }
    fn reads(&self) -> &'static [FactType] {
        WEATHER_READS
    }
    fn writes(&self) -> &'static [FactType] {
        WEATHER_WRITES
    }
    fn cadence(&self) -> Cadence {
        self.step.cadence()
    }
    fn evaluate(&self, view: &dyn CommittedView, ctx: &TickContext) -> Vec<Proposal> {
        let mut out = Vec::new();
        for region in regions(view) {
            let before = anomaly_of(view, region, TEMPERATURE_ANOMALY);
            // Mixed with the neighbours' weather first (Amendment A-36).
            let mixed = self
                .fronts
                .mix(view, region, TEMPERATURE_ANOMALY, self.step.dt_ms);
            let after = self.weather.step(mixed, &mut ctx.rng(region.raw()));
            let exposure = exposure_of(view, region);
            let mass = thermal_damping(view, region, self.thermal_mass_reference);
            let felt = |anomaly| attenuate(attenuate(anomaly_level(anomaly), exposure), mass);
            out.push(Proposal::new(
                self.id(),
                FactKey::new(region, TEMPERATURE_ANOMALY),
                ctx.basis_tick(),
                Change::Set(Value::Int(after)),
                Cause::new("weather"),
            ));
            out.push(Proposal::new(
                self.id(),
                FactKey::new(region, TEMPERATURE),
                ctx.basis_tick(),
                Change::Delta(felt(after) - felt(before)),
                Cause::new("weather"),
            ));
        }
        out
    }
}

/// The seasons' share of temperature (Vol. III Ch. 1 §1.10, *Time and Change*; Amendment A-34):
/// every climate open to the sky warms toward midsummer and cools toward midwinter, a lag
/// behind the sun, by the world's declared swing — damped, like the day's, by exposure and
/// thermal mass. Each step proposes the change in the damped seasonal level between its start
/// and end, so the steps telescope to exactly the season's level at any tick length (as for
/// [`DiurnalCycle`]). A region's declared temperature is its air on the world's first day; the
/// seasons move it from there. A world with no year has no seasons, and this proposes nothing.
pub struct Seasons {
    clock: SimClock,
    step: Step,
    calendar: crate::calendar::Calendar,
    thermal_mass_reference: i64,
}

impl Seasons {
    /// The seasons `calendar` gives, stepping as `step` says; `thermal_mass_reference` as for the
    /// day/night swing.
    pub const fn new(
        clock: SimClock,
        step: Step,
        calendar: crate::calendar::Calendar,
        thermal_mass_reference: i64,
    ) -> Self {
        Self {
            clock,
            step,
            calendar,
            thermal_mass_reference,
        }
    }
}

const SEASONS_READS: &[FactType] = &[
    TEMPERATURE,
    EXPOSURE,
    ENCLOSED,
    MADE_OF,
    MATERIAL_DENSITY,
    MATERIAL_THERMAL_CAPACITY,
    crate::schema::LATITUDE,
    CONTAINED_IN,
];

impl System for Seasons {
    fn id(&self) -> SystemId {
        SystemId::new("physical.seasons")
    }
    fn reads(&self) -> &'static [FactType] {
        SEASONS_READS
    }
    fn writes(&self) -> &'static [FactType] {
        TEMPERATURE_W
    }
    fn cadence(&self) -> Cadence {
        self.step.cadence()
    }
    fn evaluate(&self, view: &dyn CommittedView, ctx: &TickContext) -> Vec<Proposal> {
        if self.calendar.year_days == 0 {
            return Vec::new();
        }
        let now = self.clock.ms_at(ctx.tick());
        let then = self.clock.ms_at(self.step.start_of(ctx.tick()));
        let mut out = Vec::new();
        for region in regions(view) {
            let latitude = crate::climate::latitude_of(view, region, self.calendar.latitude);
            let exposure = exposure_of(view, region);
            let mass = thermal_damping(view, region, self.thermal_mass_reference);
            let damped = |at| {
                attenuate(
                    attenuate(self.calendar.seasonal(at, latitude), exposure),
                    mass,
                )
            };
            let delta = damped(now) - damped(then);
            if delta != 0 {
                out.push(Proposal::new(
                    self.id(),
                    FactKey::new(region, TEMPERATURE),
                    ctx.basis_tick(),
                    Change::Delta(delta),
                    Cause::new("season"),
                ));
            }
        }
        out
    }
}

/// The sun crossing the sky: illumination set to its absolute level for the moment, following
/// the sun's height — highest at noon, falling to nothing at sunset, and dark until sunrise but
/// for the moon (Vol. III Ch. 1 §1.10, Time and Change; Amendments A-8, A-34), for every region.
/// How high the sun stands, and how long it is up, follow the season and the region's latitude
/// ([`crate::calendar`]). The sky is a pure function of simulated time; exposure scales it so a
/// sealed cave stays dark even at noon.
pub struct DayNightCycle {
    clock: SimClock,
    step: Step,
    calendar: crate::calendar::Calendar,
    peak_illumination: i64,
    cloud_shade: i64,
}

impl DayNightCycle {
    /// Light each region by the sky `calendar` gives, up to `peak_illumination` with the sun
    /// overhead, less `cloud_shade` of it under a full overcast (Amendment A-35).
    pub const fn new(
        clock: SimClock,
        step: Step,
        calendar: crate::calendar::Calendar,
        peak_illumination: i64,
        cloud_shade: i64,
    ) -> Self {
        Self {
            clock,
            step,
            calendar,
            peak_illumination,
            cloud_shade,
        }
    }
}

impl System for DayNightCycle {
    fn id(&self) -> SystemId {
        SystemId::new("physical.day_night_cycle")
    }
    fn reads(&self) -> &'static [FactType] {
        ILLUMINATION_READS
    }
    fn writes(&self) -> &'static [FactType] {
        ILLUMINATION_W
    }
    fn cadence(&self) -> Cadence {
        self.step.cadence()
    }
    fn evaluate(&self, view: &dyn CommittedView, ctx: &TickContext) -> Vec<Proposal> {
        let at_ms = self.clock.ms_at(ctx.tick());
        light_by_region(
            view,
            at_ms,
            &self.calendar,
            self.peak_illumination,
            self.cloud_shade,
        )
        .into_iter()
        .map(|(region, light)| {
            Proposal::new(
                self.id(),
                FactKey::new(region, ILLUMINATION),
                ctx.basis_tick(),
                Change::Set(Value::Int(light)),
                Cause::new("solar_position"),
            )
        })
        .collect()
    }
}

/// The light every region has at `at_ms` of simulated time under the sky `calendar` gives,
/// `peak` with the sun overhead: what [`DayNightCycle`] sets each step, and what a world begins
/// with, so the first tick of a night is dark (Amendment A-16). Open ground takes the sky as its
/// exposure allows; a sheltered room takes only what its openings let in (Amendment A-5). Each
/// region sees the sky from its own latitude (Amendment A-34), through its cloud (A-35).
pub fn light_by_region(
    view: &dyn CommittedView,
    at_ms: u64,
    calendar: &crate::calendar::Calendar,
    peak: i64,
    cloud_shade: i64,
) -> Vec<(EntityId, i64)> {
    all_climates(view)
        .into_iter()
        .map(|region| {
            let latitude = crate::climate::latitude_of(view, region, calendar.latitude);
            // Cloud keeps out its share of the light (Amendment A-35): the cloud over the region,
            // or over the climate around it.
            let cloud = crate::climate::ambient(view, region, crate::schema::CLOUD)
                .map_or(0, |(_, c)| c.clamp(0, PERCENT_FULL));
            let sky = calendar.sky_light(at_ms, latitude, peak);
            let sky = sky - attenuate(attenuate(sky, cloud), cloud_shade);
            let share = if is_sheltered(view, region) {
                daylight_fraction(view, region)
            } else {
                exposure_of(view, region)
            };
            (region, attenuate(sky, share))
        })
        .collect()
}

/// The sky's rules (Amendment A-35), all from the world package.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct SkyRules {
    /// The air's humidity, in hundredths of a percent, past which cloud begins to form.
    pub cloud_above: i64,
    /// The air's humidity at and past which rain falls (or snow): the sky is overcast by then.
    pub rain_above: i64,
    /// How much of the day's light a full overcast keeps out, in hundredths of a percent.
    pub cloud_shade: i64,
    /// How much of the day/night swing a full overcast holds back, in hundredths of a percent.
    pub cloud_damp: i64,
    /// How hard it rains with the air saturated, in hundredths of a millimetre an hour; less,
    /// in proportion, between the rain line and saturation.
    pub rain_per_hour: i64,
    /// The air temperature, in centidegrees, at and below which what falls is snow.
    pub snow_below_centi_c: i64,
    /// The relative humidity near the ground past which fog forms.
    pub fog_above: i64,
    /// The wind, in cm/s, at and above which fog cannot settle; it thins toward it.
    pub fog_wind_below_cm_s: i64,
    /// How much the relative humidity near the ground rises for each degree the air cools below
    /// its day's mean, and falls for each degree it warms above, in hundredths of a percent.
    pub humidity_per_degree: i64,
}

/// The sky over every climate open to it (Vol. III Ch. 1 §1.10; Amendments A-1, A-35):
///
/// - **The air's humidity** is the climate's normal — its own, or the world's baseline — plus a
///   mean-reverting weather anomaly (`Weather`), scaled by exposure.
/// - **Cloud** forms as that air grows humid past the world's cloud line, and the sky is overcast
///   by the rain line. It holds back the day's swing — cooler afternoons, warmer nights — by the
///   world's `cloud_damp`: the system proposes the change in that share of the temperature
///   between its step's start and end, read from the cloud it committed last, so the shares
///   telescope exactly as the day's swing does. (Its dimming of the light is the sun's system's.)
/// - **Rain** falls from air at the rain line or past it, harder toward saturation; below the
///   world's snow line, it falls as **snow**.
/// - **The humidity near the ground** — the fact [`HUMIDITY`] — is the air's, raised as the air
///   cools below its day's mean and lowered as it warms above it: damp dawns, dry afternoons.
/// - **Fog** forms when that humidity passes the world's fog line in still air: thicker toward
///   saturation, thinner as the wind rises toward the world's fog wind.
///
/// Humidity is set to its level each step, so it can never drift from its rule. Cloud, rain,
/// snow, and fog are written only when they change, and are absent when there is none.
pub struct Precipitation {
    clock: SimClock,
    step: Step,
    baseline: i64,
    weather: Weather,
    sky: SkyRules,
    curve: DayCurve,
    thermal_mass_reference: i64,
    fronts: Fronts,
}

impl Precipitation {
    /// Humidity about `baseline` (hundredths of a percent) with weather departures of typical
    /// size `spread` lasting about `memory_ms`, stepping as `step` says, under the `sky`'s rules.
    /// The day's swing — `curve`, damped by thermal mass against `thermal_mass_reference` — is
    /// the one [`DiurnalCycle`] makes, which cloud holds back and the ground's humidity answers.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        clock: SimClock,
        step: Step,
        baseline: i64,
        spread: i64,
        memory_ms: u64,
        sky: SkyRules,
        curve: DayCurve,
        thermal_mass_reference: i64,
        fronts: Fronts,
    ) -> Self {
        Self {
            clock,
            step,
            baseline,
            weather: Weather::new(spread, memory_ms, step.dt_ms),
            sky,
            curve,
            thermal_mass_reference,
            fronts,
        }
    }

    /// Write `now` at `key` if it differs from what is held: absent when zero.
    fn mirror(
        &self,
        view: &dyn CommittedView,
        ctx: &TickContext,
        out: &mut Vec<Proposal>,
        key: FactKey,
        now: i64,
        why: &'static str,
    ) {
        let held = view.read(key).and_then(|f| f.value.as_int()).unwrap_or(0);
        if held == now {
            return;
        }
        let change = if now == 0 {
            Change::Tombstone
        } else {
            Change::Set(Value::Int(now))
        };
        out.push(Proposal::new(
            self.id(),
            key,
            ctx.basis_tick(),
            change,
            Cause::new(why),
        ));
    }
}

const SKY_READS: &[FactType] = &[
    TEMPERATURE,
    HUMIDITY_ANOMALY,
    ADJACENT_TO,
    WIND_TOWARD,
    EXPOSURE,
    ENCLOSED,
    MADE_OF,
    MATERIAL_DENSITY,
    MATERIAL_THERMAL_CAPACITY,
    WIND_SPEED,
    crate::schema::HUMIDITY_NORMAL,
    crate::schema::CLOUD,
    crate::schema::RAIN,
    crate::schema::SNOW,
    crate::schema::FOG,
];
const SKY_WRITES: &[FactType] = &[
    HUMIDITY,
    HUMIDITY_ANOMALY,
    TEMPERATURE,
    crate::schema::CLOUD,
    crate::schema::RAIN,
    crate::schema::SNOW,
    crate::schema::FOG,
];

impl System for Precipitation {
    fn id(&self) -> SystemId {
        SystemId::new("physical.precipitation")
    }
    fn reads(&self) -> &'static [FactType] {
        SKY_READS
    }
    fn writes(&self) -> &'static [FactType] {
        SKY_WRITES
    }
    fn cadence(&self) -> Cadence {
        self.step.cadence()
    }
    fn evaluate(&self, view: &dyn CommittedView, ctx: &TickContext) -> Vec<Proposal> {
        use crate::schema::{CLOUD, FOG, HUMIDITY_NORMAL, RAIN, SNOW};
        let sky = &self.sky;
        let int = |region: EntityId, fact: FactType| {
            view.read(FactKey::new(region, fact))
                .and_then(|f| f.value.as_int())
        };
        let now = self.clock.ms_at(ctx.tick());
        let then = self.clock.ms_at(self.step.start_of(ctx.tick()));
        let mut out = Vec::new();
        for region in regions(view) {
            let exposure = exposure_of(view, region);
            // Mixed with the neighbours' weather first (Amendment A-36).
            let mixed = self
                .fronts
                .mix(view, region, HUMIDITY_ANOMALY, self.step.dt_ms);
            let after = self.weather.step(mixed, &mut ctx.rng(region.raw()));
            out.push(Proposal::new(
                self.id(),
                FactKey::new(region, HUMIDITY_ANOMALY),
                ctx.basis_tick(),
                Change::Set(Value::Int(after)),
                Cause::new("precipitation"),
            ));
            // The air's humidity, and the cloud it makes.
            let normal = int(region, HUMIDITY_NORMAL).unwrap_or(self.baseline);
            let air = (normal + attenuate(anomaly_level(after), exposure)).clamp(0, PERCENT_FULL);
            let cloud = if air <= sky.cloud_above {
                0
            } else {
                ((air - sky.cloud_above) * PERCENT_FULL / (sky.rain_above - sky.cloud_above).max(1))
                    .min(PERCENT_FULL)
            };
            let cloud = attenuate(cloud, exposure);
            // The day's swing about its mean, as the day/night cycle gives it, and the share of
            // it the cloud holds back — at the step's end under this cloud, at its start under
            // the cloud committed then.
            let mass = thermal_damping(view, region, self.thermal_mass_reference);
            let swing = |at| {
                let level = attenuate(attenuate(self.curve.level(at), exposure), mass);
                let mean = attenuate(attenuate(self.curve.amplitude_centi_c / 2, exposure), mass);
                level - mean
            };
            let held = |c: i64, at| -swing(at) * c / PERCENT_FULL * sky.cloud_damp / PERCENT_FULL;
            let cloud_before = int(region, CLOUD).unwrap_or(0);
            let shift = held(cloud, now) - held(cloud_before, then);
            if shift != 0 {
                out.push(Proposal::new(
                    self.id(),
                    FactKey::new(region, TEMPERATURE),
                    ctx.basis_tick(),
                    Change::Delta(shift),
                    Cause::new("cloud"),
                ));
            }
            self.mirror(
                view,
                ctx,
                &mut out,
                FactKey::new(region, CLOUD),
                cloud,
                "cloud",
            );
            // Rain, or snow below the snow line.
            let falling = if air < sky.rain_above {
                0
            } else {
                (sky.rain_per_hour * (air - sky.rain_above + 1)
                    / (PERCENT_FULL - sky.rain_above + 1))
                    .max(1)
            };
            let falling = attenuate(falling, exposure);
            let cold = int(region, TEMPERATURE).is_some_and(|t| t <= sky.snow_below_centi_c);
            let (rain, snow) = if cold { (0, falling) } else { (falling, 0) };
            self.mirror(
                view,
                ctx,
                &mut out,
                FactKey::new(region, RAIN),
                rain,
                "rain",
            );
            self.mirror(
                view,
                ctx,
                &mut out,
                FactKey::new(region, SNOW),
                snow,
                "snow",
            );
            // Near the ground: damp as the air cools below its day's mean, dry as it warms.
            let cooled = -(swing(now) + held(cloud, now));
            let ground = (air + sky.humidity_per_degree * cooled / 100).clamp(0, PERCENT_FULL);
            out.push(Proposal::new(
                self.id(),
                FactKey::new(region, HUMIDITY),
                ctx.basis_tick(),
                Change::Set(Value::Int(ground)),
                Cause::new("precipitation"),
            ));
            // Fog, in still air near saturation.
            let wind = int(region, WIND_SPEED).unwrap_or(0).max(0);
            let fog = if ground <= sky.fog_above || wind >= sky.fog_wind_below_cm_s {
                0
            } else {
                let thick =
                    (ground - sky.fog_above) * PERCENT_FULL / (PERCENT_FULL - sky.fog_above).max(1);
                thick * (sky.fog_wind_below_cm_s - wind) / sky.fog_wind_below_cm_s.max(1)
            };
            self.mirror(
                view,
                ctx,
                &mut out,
                FactKey::new(region, FOG),
                attenuate(fog, exposure),
                "fog",
            );
        }
        out
    }
}

/// Atmospheric pressure (Vol. III Ch. 1 §1.10): each region's pressure is a baseline that falls
/// with elevation, plus a mean-reverting weather anomaly (`Weather`) scaled by exposure, for
/// every region. Sets the absolute level each step.
pub struct PressureSystem {
    step: Step,
    sea_level: i64,
    elevation_factor: i64,
    weather: Weather,
    fronts: Fronts,
}

impl PressureSystem {
    /// Configure pressure. `sea_level` is baseline pressure at the datum; `elevation_factor`
    /// is decapascals dropped per metre of elevation; weather departs from that baseline by a
    /// typical `spread` (decapascals) for about `memory_ms` (world-package rules, Vol. IV Ch. 2).
    pub fn new(
        step: Step,
        sea_level: i64,
        elevation_factor: i64,
        spread: i64,
        memory_ms: u64,
        fronts: Fronts,
    ) -> Self {
        Self {
            step,
            sea_level,
            elevation_factor,
            weather: Weather::new(spread, memory_ms, step.dt_ms),
            fronts,
        }
    }

    fn baseline(&self, elevation_cm: i64) -> i64 {
        // Elevation stored in centimetres; drop pressure per metre climbed.
        (self.sea_level - (elevation_cm / 100) * self.elevation_factor).max(0)
    }
}

impl System for PressureSystem {
    fn id(&self) -> SystemId {
        SystemId::new("physical.pressure")
    }
    fn reads(&self) -> &'static [FactType] {
        PRESSURE_READS
    }
    fn writes(&self) -> &'static [FactType] {
        PRESSURE_WRITES
    }
    fn cadence(&self) -> Cadence {
        self.step.cadence()
    }
    fn evaluate(&self, view: &dyn CommittedView, ctx: &TickContext) -> Vec<Proposal> {
        let mut out = Vec::new();
        for region in regions(view) {
            let elevation = view
                .read(FactKey::new(region, ELEVATION))
                .and_then(|f| f.value.as_int())
                .unwrap_or(0);
            // Mixed with the neighbours' weather first (Amendment A-36).
            let mixed = self
                .fronts
                .mix(view, region, PRESSURE_ANOMALY, self.step.dt_ms);
            let after = self.weather.step(mixed, &mut ctx.rng(region.raw()));
            let level = self.baseline(elevation)
                + attenuate(anomaly_level(after), exposure_of(view, region));
            out.push(Proposal::new(
                self.id(),
                FactKey::new(region, PRESSURE_ANOMALY),
                ctx.basis_tick(),
                Change::Set(Value::Int(after)),
                Cause::new("pressure_weather"),
            ));
            out.push(Proposal::new(
                self.id(),
                FactKey::new(region, PRESSURE),
                ctx.basis_tick(),
                Change::Set(Value::Int(level.clamp(0, MAX_PRESSURE))),
                Cause::new("pressure_weather"),
            ));
        }
        out
    }
}

/// Wind as the consequence of pressure gradients across the topology (Vol. III Ch. 1 §1.10,
/// "Wind flows"), for every region. Reads each region's pressure and every neighbour's
/// pressure (via adjacency), then blows toward the lowest-pressure neighbour with a speed
/// proportional to the gradient — a genuinely multi-fact, topology-aware system that writes
/// both wind facts. Calm (speed zero, no direction) when no neighbour is lower. Wind
/// therefore lags pressure by one environment step, as effects chain across commits
/// (Vol. III Ch. 12 §12.2).
pub struct WindSystem {
    clock: SimClock,
    step: Step,
    per_decapascal: i64,
    elevation_factor: i64,
    max_wind: i64,
    gale: Gale,
}

/// How a deep low brews a gale (Amendment A-33), world rules: below `below` decapascals under its
/// normal pressure, a region's wind rises by `per_decapascal` cm/s for every decapascal deeper.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Gale {
    /// How far under normal, in decapascals, the pressure must fall before a gale brews.
    pub below: i64,
    /// How much wind, in cm/s, each decapascal deeper brings.
    pub per_decapascal: i64,
}

impl WindSystem {
    /// Configure wind: `per_decapascal` cm/s for each decapascal of difference between
    /// neighbours' pressure brought to the datum — `elevation_factor` decapascals for each metre
    /// of height, as pressure falls (Amendment A-36) — clamped at `max_wind` (world-package rules).
    /// Wind is a function of the current pressure field, not a rate, so it simply steps with the
    /// rest of the environment.
    pub const fn new(
        clock: SimClock,
        step: Step,
        per_decapascal: i64,
        elevation_factor: i64,
        max_wind: i64,
        gale: Gale,
    ) -> Self {
        Self {
            clock,
            step,
            per_decapascal,
            elevation_factor,
            max_wind,
            gale,
        }
    }

    /// `region`'s pressure brought down to the datum: what it would be at sea level. Height alone
    /// lowers pressure without moving air; only a difference of this drives the wind
    /// (Amendment A-36).
    fn at_datum(&self, view: &dyn CommittedView, region: EntityId) -> Option<i64> {
        let pressure = view
            .read(FactKey::new(region, PRESSURE))
            .and_then(|f| f.value.as_int())?;
        let elevation = view
            .read(FactKey::new(region, ELEVATION))
            .and_then(|f| f.value.as_int())
            .unwrap_or(0);
        Some(pressure + elevation / 100 * self.elevation_factor)
    }
}

impl System for WindSystem {
    fn id(&self) -> SystemId {
        SystemId::new("physical.wind")
    }
    fn reads(&self) -> &'static [FactType] {
        WIND_READS
    }
    fn writes(&self) -> &'static [FactType] {
        WIND_WRITES
    }
    fn cadence(&self) -> Cadence {
        self.step.cadence()
    }
    fn evaluate(&self, view: &dyn CommittedView, ctx: &TickContext) -> Vec<Proposal> {
        let mut out = Vec::new();
        for region in regions(view) {
            let my_pressure = match self.at_datum(view, region) {
                Some(p) => p,
                // No committed pressure yet (e.g. the first tick): calm, and nothing to write.
                None => continue,
            };

            // Scan neighbours (a cardinality-many read) for the lowest pressure; deterministic
            // tie-break by smallest entity id.
            let mut best: Option<(EntityId, i64)> = None;
            for f in view.read_all(FactKey::new(region, ADJACENT_TO)) {
                if let Value::Entity(neighbour) = f.value {
                    if let Some(np) = self.at_datum(view, neighbour) {
                        let better = match best {
                            None => true,
                            Some((bn, bp)) => np < bp || (np == bp && neighbour.raw() < bn.raw()),
                        };
                        if better {
                            best = Some((neighbour, np));
                        }
                    }
                }
            }

            let speed_key = FactKey::new(region, WIND_SPEED);
            let toward_key = FactKey::new(region, WIND_TOWARD);
            // A storm the world declares blows at least its own wind while it lasts (Amendment
            // A-29).
            let now_s = (self.clock.ms_at(ctx.tick()) / 1000) as i64;
            let storm = view
                .read_all(FactKey::new(region, crate::schema::STORM))
                .into_iter()
                .filter_map(|f| f.value.as_vec3())
                .filter(|[from, to, _]| *from <= now_s && now_s < *to)
                .map(|[_, _, wind]| wind)
                .max()
                .unwrap_or(0)
                .clamp(0, self.max_wind);
            // A deep low brews its own gale (Amendment A-33).
            let low = -anomaly_level(anomaly_of(view, region, PRESSURE_ANOMALY));
            let gale =
                ((low - self.gale.below).max(0) * self.gale.per_decapascal).clamp(0, self.max_wind);
            let storm = storm.max(gale);
            match best {
                Some((neighbour, np)) if np < my_pressure => {
                    let speed = ((my_pressure - np).saturating_mul(self.per_decapascal))
                        .clamp(0, self.max_wind)
                        .max(storm);
                    out.push(Proposal::new(
                        self.id(),
                        speed_key,
                        ctx.basis_tick(),
                        Change::Set(Value::Int(speed)),
                        Cause::new("pressure_gradient"),
                    ));
                    out.push(Proposal::new(
                        self.id(),
                        toward_key,
                        ctx.basis_tick(),
                        Change::Set(Value::Entity(neighbour)),
                        Cause::new("pressure_gradient"),
                    ));
                }
                _ => {
                    let (speed, why) = if storm > 0 {
                        (storm, "storm")
                    } else {
                        (0, "calm")
                    };
                    out.push(Proposal::new(
                        self.id(),
                        speed_key,
                        ctx.basis_tick(),
                        Change::Set(Value::Int(speed)),
                        Cause::new(why),
                    ));
                    out.push(Proposal::new(
                        self.id(),
                        toward_key,
                        ctx.basis_tick(),
                        Change::Tombstone,
                        Cause::new("calm"),
                    ));
                }
            }
        }
        out
    }
}

/// Indoor air (Amendment A-5): every walled region's temperature follows the air outside it —
/// the nearest enclosing climate — with the lag the world declares (`indoor_coupling_seconds`),
/// lengthened by the thermal mass of what the room is built of, so a stone cottage holds the
/// day's warmth longer than a timber shed. A walled region with no temperature yet is given the
/// outside air's on its first step, and from then on is an indoor climate of its own — what
/// stands in it inherits *its* temperature, not the sky's.
///
/// Solved implicitly over the step, so it is stable at any step length, and rounded without
/// bias so a slow lag at fine ticks still converges (Amendment A-1).
pub struct Shelter {
    step: Step,
    coupling_ms: u64,
    thermal_mass_reference: i64,
}

impl Shelter {
    /// Indoor air following outdoor air with time constant `coupling_ms`, stepping as `step`
    /// says; `thermal_mass_reference` as for the day/night swing.
    pub const fn new(step: Step, coupling_ms: u64, thermal_mass_reference: i64) -> Self {
        Self {
            step,
            coupling_ms,
            thermal_mass_reference,
        }
    }
}

impl System for Shelter {
    fn id(&self) -> SystemId {
        SystemId::new("physical.shelter")
    }
    fn reads(&self) -> &'static [FactType] {
        SHELTER_READS
    }
    fn writes(&self) -> &'static [FactType] {
        TEMPERATURE_W
    }
    fn cadence(&self) -> Cadence {
        self.step.cadence()
    }
    fn evaluate(&self, view: &dyn CommittedView, ctx: &TickContext) -> Vec<Proposal> {
        let mut out = Vec::new();
        for room in view.entities_with(ENCLOSED) {
            if !crate::terrain::is_true(view, room, ENCLOSED) {
                continue;
            }
            let Some(outside) = outside_temperature(view, room) else {
                continue;
            };
            let key = FactKey::new(room, TEMPERATURE);
            let Some(inside) = view.read(key).and_then(|f| f.value.as_int()) else {
                out.push(Proposal::new(
                    self.id(),
                    key,
                    ctx.basis_tick(),
                    Change::Set(Value::Int(outside)),
                    Cause::new("sheltered"),
                ));
                continue;
            };
            // Heavier rooms answer more slowly: the lag stretches by the inverse of the same
            // damping factor that shrinks their day/night swing (§1.9).
            let damping = thermal_damping(view, room, self.thermal_mass_reference).max(1) as i128;
            let tau = self.coupling_ms.max(1) as i128 * PERCENT_FULL as i128 / damping;
            let dt = self.step.dt_ms as i128;
            let next = div_dither(
                inside as i128 * tau + outside as i128 * dt,
                tau + dt,
                &mut ctx.rng(room.raw()),
            ) as i64;
            if next != inside {
                out.push(Proposal::new(
                    self.id(),
                    key,
                    ctx.basis_tick(),
                    Change::Delta(next - inside),
                    Cause::new("indoor_air"),
                ));
            }
        }
        out
    }
}

/// How far one would drop on passing through `portal`: the height of the landing spot above the
/// floor or ground of the destination. The landing spot is the opening's far face where it has
/// one, else the opening itself as seen from the destination; an opening that leads nowhere is
/// judged by its height above the ground.
fn drop_beyond(view: &dyn CommittedView, portal: EntityId) -> i64 {
    let Some(dest) = portal_destination(view, portal) else {
        return height_above_ground(view, portal);
    };
    let landing = match far_side(view, portal, dest) {
        Some(face) => Some(crate::space::local_position(view, face)),
        None => lowest_common_ancestor(view, portal, dest, CONTAINED_IN).and_then(|common| {
            let at = crate::space::position_in(view, portal, common)?;
            crate::space::lower(view, at, dest, common)
        }),
    };
    match landing {
        Some(p) => p[2] - crate::terrain::ground(view, dest, p[0], p[1]),
        None => height_above_ground(view, portal),
    }
}

/// Writes each portal's effective danger (Vol. III Ch. 1 §1.11). If the world pinned a fixed
/// danger the system echoes it; otherwise it derives danger from the drop on the far side — how
/// high the face one emerges from stands above the ground beneath it — leaving a slot for
/// weather to raise it later. Enumerates every
/// region that hosts portals through `has_portal`, so it needs no separate portal list.
pub struct PortalDanger {
    fall_danger_per_meter: i64,
}

impl PortalDanger {
    /// Configure with the world's fall-danger rate (danger points per metre of height;
    /// world-package rule, Vol. IV Ch. 2).
    pub const fn new(fall_danger_per_meter: i64) -> Self {
        Self {
            fall_danger_per_meter,
        }
    }
}

impl System for PortalDanger {
    fn id(&self) -> SystemId {
        SystemId::new("physical.portal_danger")
    }
    fn reads(&self) -> &'static [FactType] {
        DANGER_READS
    }
    fn writes(&self) -> &'static [FactType] {
        DANGER_WRITES
    }
    fn cadence(&self) -> Cadence {
        Cadence::EveryTick
    }
    fn evaluate(&self, view: &dyn CommittedView, ctx: &TickContext) -> Vec<Proposal> {
        let mut out = Vec::new();
        // Every region that hosts at least one portal, discovered from committed reality.
        for region in view.entities_with(HAS_PORTAL) {
            for f in view.read_all(FactKey::new(region, HAS_PORTAL)) {
                let portal = match f.value {
                    Value::Entity(p) => p,
                    _ => continue,
                };
                let danger = match view
                    .read(FactKey::new(portal, PORTAL_DANGER_OVERRIDE))
                    .and_then(|f| f.value.as_int())
                {
                    // World-defined: pinned regardless of height or weather.
                    Some(pinned) => pinned.clamp(0, MAX_DANGER),
                    // Derived: the drop on the far side — how far the spot one emerges at
                    // stands above the floor or ground of the place one emerges into (Amendment
                    // A-5). Stairs land you on a floor; a ground-floor door on a hillside drops
                    // you nowhere; a bedroom window's yard face hangs 3.8 m above the yard.
                    None => {
                        let height = drop_beyond(view, portal).max(0);
                        let fall = height.saturating_mul(self.fall_danger_per_meter) / 100;
                        // TODO(weather): add a term from the host region's wind/precipitation.
                        fall.clamp(0, MAX_DANGER)
                    }
                };
                // Written when it changes, so a still world chronicles nothing here.
                let held = view
                    .read(FactKey::new(portal, PORTAL_DANGER))
                    .and_then(|f| f.value.as_int());
                if held == Some(danger) {
                    continue;
                }
                out.push(Proposal::new(
                    self.id(),
                    FactKey::new(portal, PORTAL_DANGER),
                    ctx.basis_tick(),
                    Change::Set(Value::Int(danger)),
                    Cause::new("portal_danger"),
                ));
            }
        }
        out
    }
}
