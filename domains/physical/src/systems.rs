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
    EXPOSURE,
    ENCLOSED,
    MADE_OF,
    MATERIAL_DENSITY,
    MATERIAL_THERMAL_CAPACITY,
];
const WEATHER_WRITES: &[FactType] = &[TEMPERATURE, TEMPERATURE_ANOMALY];
const ILLUMINATION_READS: &[FactType] = &[
    TEMPERATURE,
    EXPOSURE,
    ENCLOSED,
    HAS_PORTAL,
    LEADS_TO,
    PORTAL_OPEN,
    OPAQUE,
    BODY_SIZE,
];
const ILLUMINATION_W: &[FactType] = &[ILLUMINATION];
const HUMIDITY_READS: &[FactType] = &[TEMPERATURE, HUMIDITY_ANOMALY, EXPOSURE, ENCLOSED];
const HUMIDITY_WRITES: &[FactType] = &[HUMIDITY, HUMIDITY_ANOMALY];
const PRESSURE_READS: &[FactType] = &[TEMPERATURE, PRESSURE_ANOMALY, ELEVATION, EXPOSURE, ENCLOSED];
const PRESSURE_WRITES: &[FactType] = &[PRESSURE, PRESSURE_ANOMALY];
const WIND_READS: &[FactType] = &[TEMPERATURE, ADJACENT_TO, PRESSURE, EXPOSURE, ENCLOSED];
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

/// An integer triangle wave in `0..=amp` over a period of `period_ms`, evaluated at simulated
/// time `at_ms`: zero at the start of the period, peaking at its middle.
///
/// The argument is simulated time, not a tick count, so the wave has the same shape at any
/// tick length (Amendment A-1). Systems that add to a field propose the *difference* between
/// the wave at the two ends of their step, so summed deltas telescope back to the wave itself;
/// systems that set an absolute level (the sun) use the level directly.
fn wave(at_ms: u64, period_ms: u64, amp: i64) -> i64 {
    let period = period_ms.max(2) as i128;
    let phase = (at_ms as i128).rem_euclid(period);
    let half = period / 2;
    let up = if phase <= half { phase } else { period - phase };
    ((amp as i128 * up) / half) as i64
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
    day_ms: u64,
    amplitude_centi_c: i64,
    thermal_mass_reference: i64,
}

impl DiurnalCycle {
    /// Swing every region by `amplitude_centi_c` over a day of `day_ms` simulated time, stepping
    /// as `step` says. `thermal_mass_reference` is the thermal mass (kJ/(m³·K)) at which a
    /// region's swing is halved (Vol. III Ch. 1 §1.9; Amendment A-6); larger means thermal mass matters less.
    pub const fn new(
        clock: SimClock,
        step: Step,
        day_ms: u64,
        amplitude_centi_c: i64,
        thermal_mass_reference: i64,
    ) -> Self {
        Self {
            clock,
            step,
            day_ms,
            amplitude_centi_c,
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
        let now = wave(
            self.clock.ms_at(ctx.tick()),
            self.day_ms,
            self.amplitude_centi_c,
        );
        let then = wave(
            self.clock.ms_at(self.step.start_of(ctx.tick())),
            self.day_ms,
            self.amplitude_centi_c,
        );
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
    ) -> Self {
        Self {
            step,
            weather: Weather::new(spread_centi_c, memory_ms, step.dt_ms),
            thermal_mass_reference,
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
            let after = self.weather.step(before, &mut ctx.rng(region.raw()));
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

/// The sun crossing the sky: illumination set to its absolute level for the moment, peaking
/// at midday and dark at midnight (Vol. III Ch. 1 §1.10, Time and Change), for every region.
/// The sun's position is a pure function of simulated time; exposure scales it so a sealed cave
/// stays dark even at noon.
pub struct DayNightCycle {
    clock: SimClock,
    step: Step,
    day_ms: u64,
    peak_illumination: i64,
}

impl DayNightCycle {
    /// Light each region up to `peak_illumination` at midday, over a day of `day_ms`.
    pub const fn new(clock: SimClock, step: Step, day_ms: u64, peak_illumination: i64) -> Self {
        Self {
            clock,
            step,
            day_ms,
            peak_illumination,
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
        let sun = wave(
            self.clock.ms_at(ctx.tick()),
            self.day_ms,
            self.peak_illumination,
        );
        all_climates(view)
            .into_iter()
            .map(|region| {
                // Open ground takes the sun as its exposure allows; a sheltered room takes only
                // what its openings let in (Amendment A-5).
                let share = if is_sheltered(view, region) {
                    daylight_fraction(view, region)
                } else {
                    exposure_of(view, region)
                };
                Proposal::new(
                    self.id(),
                    FactKey::new(region, ILLUMINATION),
                    ctx.basis_tick(),
                    Change::Set(Value::Int(attenuate(sun, share))),
                    Cause::new("solar_position"),
                )
            })
            .collect()
    }
}

/// Weather driving humidity: each region's humidity is its baseline plus a mean-reverting
/// weather anomaly (`Weather`), the anomaly's effect scaled by exposure (a sealed chamber
/// keeps its baseline air), for every region. Sets the absolute level each step, so humidity
/// can never drift from its rule.
pub struct Precipitation {
    step: Step,
    baseline: i64,
    weather: Weather,
}

impl Precipitation {
    /// Humidity about `baseline` (hundredths of a percent) with weather departures of typical
    /// size `spread` lasting about `memory_ms`, stepping as `step` says.
    pub fn new(step: Step, baseline: i64, spread: i64, memory_ms: u64) -> Self {
        Self {
            step,
            baseline,
            weather: Weather::new(spread, memory_ms, step.dt_ms),
        }
    }
}

impl System for Precipitation {
    fn id(&self) -> SystemId {
        SystemId::new("physical.precipitation")
    }
    fn reads(&self) -> &'static [FactType] {
        HUMIDITY_READS
    }
    fn writes(&self) -> &'static [FactType] {
        HUMIDITY_WRITES
    }
    fn cadence(&self) -> Cadence {
        self.step.cadence()
    }
    fn evaluate(&self, view: &dyn CommittedView, ctx: &TickContext) -> Vec<Proposal> {
        let mut out = Vec::new();
        for region in regions(view) {
            let before = anomaly_of(view, region, HUMIDITY_ANOMALY);
            let after = self.weather.step(before, &mut ctx.rng(region.raw()));
            let level = self.baseline + attenuate(anomaly_level(after), exposure_of(view, region));
            out.push(Proposal::new(
                self.id(),
                FactKey::new(region, HUMIDITY_ANOMALY),
                ctx.basis_tick(),
                Change::Set(Value::Int(after)),
                Cause::new("precipitation"),
            ));
            out.push(Proposal::new(
                self.id(),
                FactKey::new(region, HUMIDITY),
                ctx.basis_tick(),
                Change::Set(Value::Int(level.clamp(0, PERCENT_FULL))),
                Cause::new("precipitation"),
            ));
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
    ) -> Self {
        Self {
            step,
            sea_level,
            elevation_factor,
            weather: Weather::new(spread, memory_ms, step.dt_ms),
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
            let before = anomaly_of(view, region, PRESSURE_ANOMALY);
            let after = self.weather.step(before, &mut ctx.rng(region.raw()));
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
    step: Step,
    gradient_divisor: i64,
    max_wind: i64,
}

impl WindSystem {
    /// Configure wind. `gradient_divisor` scales speed per unit pressure difference (larger =
    /// gentler); `max_wind` clamps the speed (world-package rules). Wind is a function of the
    /// current pressure field, not a rate, so it simply steps with the rest of the environment.
    pub const fn new(step: Step, gradient_divisor: i64, max_wind: i64) -> Self {
        Self {
            step,
            gradient_divisor,
            max_wind,
        }
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
            let my_pressure = match view
                .read(FactKey::new(region, PRESSURE))
                .and_then(|f| f.value.as_int())
            {
                Some(p) => p,
                // No committed pressure yet (e.g. the first tick): calm, and nothing to write.
                None => continue,
            };

            // Scan neighbours (a cardinality-many read) for the lowest pressure; deterministic
            // tie-break by smallest entity id.
            let mut best: Option<(EntityId, i64)> = None;
            for f in view.read_all(FactKey::new(region, ADJACENT_TO)) {
                if let Value::Entity(neighbour) = f.value {
                    if let Some(np) = view
                        .read(FactKey::new(neighbour, PRESSURE))
                        .and_then(|nf| nf.value.as_int())
                    {
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
            match best {
                Some((neighbour, np)) if np < my_pressure => {
                    let speed =
                        ((my_pressure - np) / self.gradient_divisor.max(1)).clamp(0, self.max_wind);
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
                    out.push(Proposal::new(
                        self.id(),
                        speed_key,
                        ctx.basis_tick(),
                        Change::Set(Value::Int(0)),
                        Cause::new("calm"),
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
