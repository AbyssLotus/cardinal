//! Hermetic transformations owned by the Physical Reality domain (Vol. V Ch. 3 §3.1).
//!
//! Each system declares its read/write sets and cadence, reads committed reality, and emits
//! proposals — mutating nothing directly (Vol. V Ch. 3 §3.1-3.2). These drive the
//! environmental fields that vary across space and time (Vol. III Ch. 1 §1.10): temperature
//! (a deterministic day/night swing plus stochastic weather), illumination (the sun's
//! position), and humidity (weather). Implement causes, never outcomes (Vol. III Ch. 11).

use crate::schema::{
    ADJACENT_TO, ELEVATION, EXPOSURE, HUMIDITY, ILLUMINATION, PERCENT_FULL, PRESSURE, TEMPERATURE,
    WIND_SPEED, WIND_TOWARD,
};
use kernel::fact::{Cause, FactKey, FactType, SystemId};
use kernel::identity::EntityId;
use kernel::proposal::{Change, Proposal};
use kernel::system::{Cadence, CommittedView, System, TickContext};
use kernel::value::Value;

const TEMPERATURE_RW: &[FactType] = &[TEMPERATURE];
const ILLUMINATION_W: &[FactType] = &[ILLUMINATION];
const HUMIDITY_READS: &[FactType] = &[HUMIDITY, EXPOSURE];
const HUMIDITY_WRITES: &[FactType] = &[HUMIDITY];
const PRESSURE_READS: &[FactType] = &[PRESSURE, ELEVATION, EXPOSURE];
const PRESSURE_WRITES: &[FactType] = &[PRESSURE];
const WIND_READS: &[FactType] = &[ADJACENT_TO, PRESSURE];
const WIND_WRITES: &[FactType] = &[WIND_SPEED, WIND_TOWARD];
const EXPOSURE_READS: &[FactType] = &[EXPOSURE];

/// An integer triangle wave in `0..=amp` over `period`, peaking at mid-period.
///
/// Systems that drift a field propose the *difference* between consecutive levels so summed
/// deltas telescope and the field stays bounded; systems that set an absolute level (the
/// sun) use the level directly.
fn wave(phase: u64, period: u64, amp: i64) -> i64 {
    let period = period.max(2) as i64;
    let phase = (phase as i64).rem_euclid(period);
    let half = period / 2;
    let up = if phase <= half { phase } else { period - phase };
    if half == 0 {
        0
    } else {
        (amp * up) / half
    }
}

/// A region's exposure to the open sky, in hundredths of a percent (0..=10000). A region with
/// no committed exposure fact is treated as fully exposed -- open ground under open sky -- so
/// exposure only ever *attenuates* surface weather (Vol. III Ch. 1 §1.6, Enclosed / Exposed).
/// A sealed chamber reads 0; a cave mouth or forest floor is partial; a field is full.
fn exposure_of(view: &dyn CommittedView, region: EntityId) -> i64 {
    view.read(FactKey::new(region, EXPOSURE))
        .and_then(|f| f.value.as_int())
        .map(|e| e.clamp(0, PERCENT_FULL))
        .unwrap_or(PERCENT_FULL)
}

/// Scale a surface-weather magnitude by exposure: `value * exposure / 10000`. Full exposure
/// passes it through unchanged; zero exposure (a sealed space) removes it entirely; a partial
/// value dampens it in proportion.
fn attenuate(value: i64, exposure: i64) -> i64 {
    value.saturating_mul(exposure) / PERCENT_FULL
}

/// A deterministic day/night temperature swing (Vol. III Ch. 1 §1.10). Proposes the per-tick
/// delta of a triangle wave over the day, so the swing is bounded and reproducible.
pub struct DiurnalCycle {
    region: EntityId,
    ticks_per_day: u64,
    amplitude_centi_c: i64,
}

impl DiurnalCycle {
    /// Drive `region` with a swing of `amplitude_centi_c` over `ticks_per_day` ticks.
    pub const fn new(region: EntityId, ticks_per_day: u64, amplitude_centi_c: i64) -> Self {
        Self {
            region,
            ticks_per_day,
            amplitude_centi_c,
        }
    }
}

impl System for DiurnalCycle {
    fn id(&self) -> SystemId {
        SystemId::new("physical.diurnal_cycle")
    }
    fn reads(&self) -> &'static [FactType] {
        EXPOSURE_READS
    }
    fn writes(&self) -> &'static [FactType] {
        TEMPERATURE_RW
    }
    fn cadence(&self) -> Cadence {
        Cadence::EveryTick
    }
    fn evaluate(&self, view: &dyn CommittedView, ctx: &TickContext) -> Vec<Proposal> {
        let period = self.ticks_per_day.max(1);
        let level = wave(ctx.tick(), period, self.amplitude_centi_c);
        let prev = wave(ctx.tick().wrapping_sub(1), period, self.amplitude_centi_c);
        let exposure = exposure_of(view, self.region);
        vec![Proposal::new(
            self.id(),
            FactKey::new(self.region, TEMPERATURE),
            ctx.tick(),
            Change::Delta(attenuate(level - prev, exposure)),
            Cause::new("diurnal_shift"),
        )]
    }
}

/// A small stochastic weather perturbation of temperature, drawn from the kernel-issued
/// substream (Vol. V Ch. 4 §4.1).
pub struct WeatherNoise {
    region: EntityId,
    max_swing_centi_c: i64,
}

impl WeatherNoise {
    /// Perturb `region`'s temperature by up to +/- `max_swing_centi_c` each tick.
    pub const fn new(region: EntityId, max_swing_centi_c: i64) -> Self {
        Self {
            region,
            max_swing_centi_c,
        }
    }
}

impl System for WeatherNoise {
    fn id(&self) -> SystemId {
        SystemId::new("physical.weather_noise")
    }
    fn reads(&self) -> &'static [FactType] {
        EXPOSURE_READS
    }
    fn writes(&self) -> &'static [FactType] {
        TEMPERATURE_RW
    }
    fn cadence(&self) -> Cadence {
        Cadence::EveryTick
    }
    fn evaluate(&self, view: &dyn CommittedView, ctx: &TickContext) -> Vec<Proposal> {
        let swing = self.max_swing_centi_c.max(0);
        let span = (swing as u64) * 2 + 1;
        let mut rng = ctx.rng(self.region.raw());
        let raw = rng.below(span) as i64 - swing;
        let exposure = exposure_of(view, self.region);
        vec![Proposal::new(
            self.id(),
            FactKey::new(self.region, TEMPERATURE),
            ctx.tick(),
            Change::Delta(attenuate(raw, exposure)),
            Cause::new("weather_perturbation"),
        )]
    }
}

/// The sun crossing the sky: illumination set to its absolute level for the tick, peaking at
/// midday and dark at midnight (Vol. III Ch. 1 §1.10, Time and Change). Reads nothing — the
/// sun's position is a pure function of the clock — and creates the fact on first tick.
pub struct DayNightCycle {
    region: EntityId,
    ticks_per_day: u64,
    peak_illumination: i64,
}

impl DayNightCycle {
    /// Light `region` up to `peak_illumination` at midday over `ticks_per_day` ticks.
    pub const fn new(region: EntityId, ticks_per_day: u64, peak_illumination: i64) -> Self {
        Self {
            region,
            ticks_per_day,
            peak_illumination,
        }
    }
}

impl System for DayNightCycle {
    fn id(&self) -> SystemId {
        SystemId::new("physical.day_night_cycle")
    }
    fn reads(&self) -> &'static [FactType] {
        EXPOSURE_READS
    }
    fn writes(&self) -> &'static [FactType] {
        ILLUMINATION_W
    }
    fn cadence(&self) -> Cadence {
        Cadence::EveryTick
    }
    fn evaluate(&self, view: &dyn CommittedView, ctx: &TickContext) -> Vec<Proposal> {
        let sun = wave(
            ctx.tick(),
            self.ticks_per_day.max(2),
            self.peak_illumination,
        );
        let exposure = exposure_of(view, self.region);
        vec![Proposal::new(
            self.id(),
            FactKey::new(self.region, ILLUMINATION),
            ctx.tick(),
            // Absolute level scaled by how open the location is to the sky: a sealed cave
            // stays dark even at noon.
            Change::Set(Value::Int(attenuate(sun, exposure))),
            Cause::new("solar_position"),
        )]
    }
}

/// Weather driving humidity: it drifts toward a baseline while stochastic weather perturbs
/// it, drawn from the kernel-issued substream (Vol. V Ch. 4 §4.1). Reads its own committed
/// value to compute the drift, and creates the fact at the baseline on first tick.
pub struct Precipitation {
    region: EntityId,
    baseline: i64,
    swing: i64,
    drying_divisor: i64,
}

impl Precipitation {
    /// Pull `region`'s humidity toward `baseline` (over `drying_divisor` ticks) with a
    /// per-tick weather perturbation of up to +/- `swing`.
    pub const fn new(region: EntityId, baseline: i64, swing: i64, drying_divisor: i64) -> Self {
        Self {
            region,
            baseline,
            swing,
            drying_divisor,
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
        Cadence::EveryTick
    }
    fn evaluate(&self, view: &dyn CommittedView, ctx: &TickContext) -> Vec<Proposal> {
        let key = FactKey::new(self.region, HUMIDITY);
        let current = match view.read(key) {
            None => {
                // Establish humidity at its baseline the first time this runs.
                return vec![Proposal::new(
                    self.id(),
                    key,
                    ctx.tick(),
                    Change::Create(kernel::value::Value::Int(self.baseline)),
                    Cause::new("precipitation"),
                )];
            }
            Some(fact) => match fact.value.as_int() {
                Some(h) => h,
                None => return Vec::new(),
            },
        };
        let drift = (self.baseline - current) / self.drying_divisor.max(1);
        let swing = self.swing.max(0);
        let mut rng = ctx.rng(self.region.raw());
        let noise = rng.below((swing as u64) * 2 + 1) as i64 - swing;
        let exposure = exposure_of(view, self.region);
        vec![Proposal::new(
            self.id(),
            key,
            ctx.tick(),
            Change::Delta(drift + attenuate(noise, exposure)),
            Cause::new("precipitation"),
        )]
    }
}

/// Atmospheric pressure dynamics (Vol. III Ch. 1 §1.10): pressure settles toward a baseline
/// that falls with elevation, perturbed by stochastic weather. Reads the region's elevation
/// and its own pressure; creates the fact at the elevation baseline on the first tick.
pub struct PressureSystem {
    region: EntityId,
    sea_level: i64,
    elevation_factor: i64,
    weather_swing: i64,
    settle_divisor: i64,
}

impl PressureSystem {
    /// Configure pressure for `region`. `sea_level` is baseline pressure at the datum;
    /// `elevation_factor` is decapascals dropped per metre of elevation; `weather_swing` and
    /// `settle_divisor` govern the stochastic perturbation and the drift back to baseline
    /// (world-package rules, Vol. IV Ch. 2).
    pub const fn new(
        region: EntityId,
        sea_level: i64,
        elevation_factor: i64,
        weather_swing: i64,
        settle_divisor: i64,
    ) -> Self {
        Self {
            region,
            sea_level,
            elevation_factor,
            weather_swing,
            settle_divisor,
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
        Cadence::EveryTick
    }
    fn evaluate(&self, view: &dyn CommittedView, ctx: &TickContext) -> Vec<Proposal> {
        let elevation = view
            .read(FactKey::new(self.region, ELEVATION))
            .and_then(|f| f.value.as_int())
            .unwrap_or(0);
        let baseline = self.baseline(elevation);
        let key = FactKey::new(self.region, PRESSURE);
        match view.read(key) {
            None => vec![Proposal::new(
                self.id(),
                key,
                ctx.tick(),
                Change::Create(Value::Int(baseline)),
                Cause::new("pressure_baseline"),
            )],
            Some(fact) => match fact.value.as_int() {
                None => Vec::new(),
                Some(current) => {
                    let drift = (baseline - current) / self.settle_divisor.max(1);
                    let swing = self.weather_swing.max(0);
                    let mut rng = ctx.rng(self.region.raw());
                    let noise = rng.below((swing as u64) * 2 + 1) as i64 - swing;
                    let exposure = exposure_of(view, self.region);
                    vec![Proposal::new(
                        self.id(),
                        key,
                        ctx.tick(),
                        Change::Delta(drift + attenuate(noise, exposure)),
                        Cause::new("pressure_weather"),
                    )]
                }
            },
        }
    }
}

/// Wind as the consequence of pressure gradients across the topology (Vol. III Ch. 1 §1.10,
/// "Wind flows"). Reads the region's pressure and every neighbour's pressure (via adjacency),
/// then blows toward the lowest-pressure neighbour with a speed proportional to the gradient
/// — a genuinely multi-fact, topology-aware system that writes both wind facts. Calm (speed
/// zero, no direction) when no neighbour is lower. Wind therefore lags pressure by one tick,
/// as effects chain across commits (Vol. III Ch. 12 §12.2).
pub struct WindSystem {
    region: EntityId,
    gradient_divisor: i64,
    max_wind: i64,
}

impl WindSystem {
    /// Configure wind for `region`. `gradient_divisor` scales speed per unit pressure
    /// difference (larger = gentler); `max_wind` clamps the speed (world-package rules).
    pub const fn new(region: EntityId, gradient_divisor: i64, max_wind: i64) -> Self {
        Self {
            region,
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
        Cadence::EveryTick
    }
    fn evaluate(&self, view: &dyn CommittedView, ctx: &TickContext) -> Vec<Proposal> {
        let my_pressure = match view
            .read(FactKey::new(self.region, PRESSURE))
            .and_then(|f| f.value.as_int())
        {
            Some(p) => p,
            None => return Vec::new(),
        };

        // Scan neighbours (a cardinality-many read) for the lowest pressure; deterministic
        // tie-break by smallest entity id.
        let mut best: Option<(EntityId, i64)> = None;
        for f in view.read_all(FactKey::new(self.region, ADJACENT_TO)) {
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

        let speed_key = FactKey::new(self.region, WIND_SPEED);
        let toward_key = FactKey::new(self.region, WIND_TOWARD);
        match best {
            Some((neighbour, np)) if np < my_pressure => {
                let speed =
                    ((my_pressure - np) / self.gradient_divisor.max(1)).clamp(0, self.max_wind);
                vec![
                    Proposal::new(
                        self.id(),
                        speed_key,
                        ctx.tick(),
                        Change::Set(Value::Int(speed)),
                        Cause::new("pressure_gradient"),
                    ),
                    Proposal::new(
                        self.id(),
                        toward_key,
                        ctx.tick(),
                        Change::Set(Value::Entity(neighbour)),
                        Cause::new("pressure_gradient"),
                    ),
                ]
            }
            _ => vec![
                Proposal::new(
                    self.id(),
                    speed_key,
                    ctx.tick(),
                    Change::Set(Value::Int(0)),
                    Cause::new("calm"),
                ),
                Proposal::new(
                    self.id(),
                    toward_key,
                    ctx.tick(),
                    Change::Tombstone,
                    Cause::new("calm"),
                ),
            ],
        }
    }
}
