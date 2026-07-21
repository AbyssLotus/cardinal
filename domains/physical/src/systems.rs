//! Hermetic transformations owned by the Physical Reality domain (Vol. V Ch. 3 §3.1).
//!
//! Each system declares its read/write sets and cadence, reads committed reality, and emits
//! proposals — mutating nothing directly (Vol. V Ch. 3 §3.1-3.2). These drive the
//! environmental fields that vary across space and time (Vol. III Ch. 1 §1.10): temperature
//! (a deterministic day/night swing plus stochastic weather), illumination (the sun's
//! position), and humidity (weather). Implement causes, never outcomes (Vol. III Ch. 11).
//!
//! Every system here is **scope-generic**: one instance discovers its subjects from
//! committed reality each tick — regions by the [`TEMPERATURE`] fact every region carries,
//! portal hosts by [`HAS_PORTAL`] — rather than being pinned to an entity list at
//! construction (Vol. V Ch. 2 §2.1, clause 5, queries are the product). A region that comes
//! into being mid-simulation is simulated the moment its facts commit; systems hold no
//! state between ticks (Vol. II Ch. 3).

use crate::schema::{
    ADJACENT_TO, CONTAINED_IN, ELEVATION, EXPOSURE, HAS_PORTAL, HUMIDITY, ILLUMINATION, MAX_DANGER,
    PERCENT_FULL, PORTAL_DANGER, PORTAL_DANGER_OVERRIDE, POSITION_Z, PRESSURE, TEMPERATURE,
    WIND_SPEED, WIND_TOWARD,
};
use kernel::fact::{Cause, FactKey, FactType, SystemId};
use kernel::identity::EntityId;
use kernel::proposal::{Change, Proposal};
use kernel::system::{Cadence, CommittedView, System, TickContext};
use kernel::value::Value;

// Read sets. Every weather system also reads TEMPERATURE — not for its value, but to
// enumerate the regions to simulate: the loader guarantees each region carries a temperature
// fact, so `entities_with(TEMPERATURE)` is the region roster (Vol. V Ch. 2 §2.1, clause 5).
const DIURNAL_READS: &[FactType] = &[TEMPERATURE, EXPOSURE];
const TEMPERATURE_W: &[FactType] = &[TEMPERATURE];
const ILLUMINATION_READS: &[FactType] = &[TEMPERATURE, EXPOSURE];
const ILLUMINATION_W: &[FactType] = &[ILLUMINATION];
const HUMIDITY_READS: &[FactType] = &[TEMPERATURE, HUMIDITY, EXPOSURE];
const HUMIDITY_WRITES: &[FactType] = &[HUMIDITY];
const PRESSURE_READS: &[FactType] = &[TEMPERATURE, PRESSURE, ELEVATION, EXPOSURE];
const PRESSURE_WRITES: &[FactType] = &[PRESSURE];
const WIND_READS: &[FactType] = &[TEMPERATURE, ADJACENT_TO, PRESSURE];
const WIND_WRITES: &[FactType] = &[WIND_SPEED, WIND_TOWARD];
const DANGER_READS: &[FactType] = &[HAS_PORTAL, PORTAL_DANGER_OVERRIDE, CONTAINED_IN, POSITION_Z];
const DANGER_WRITES: &[FactType] = &[PORTAL_DANGER];

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

/// The regions to simulate this tick: every entity carrying a committed temperature fact.
///
/// Temperature is the one environmental fact the loader seeds for every region, so it is the
/// region roster (Vol. V Ch. 2 §2.1, clause 5). Reading it through the scoped view both
/// discovers the regions and declares the dependency the tick loop checks.
fn regions(view: &dyn CommittedView) -> Vec<EntityId> {
    view.entities_with(TEMPERATURE)
}

/// A deterministic day/night temperature swing (Vol. III Ch. 1 §1.10). Proposes the per-tick
/// delta of a triangle wave over the day, so the swing is bounded and reproducible, for
/// every region.
pub struct DiurnalCycle {
    ticks_per_day: u64,
    amplitude_centi_c: i64,
}

impl DiurnalCycle {
    /// Drive every region with a swing of `amplitude_centi_c` over `ticks_per_day` ticks.
    pub const fn new(ticks_per_day: u64, amplitude_centi_c: i64) -> Self {
        Self {
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
        DIURNAL_READS
    }
    fn writes(&self) -> &'static [FactType] {
        TEMPERATURE_W
    }
    fn cadence(&self) -> Cadence {
        Cadence::EveryTick
    }
    fn evaluate(&self, view: &dyn CommittedView, ctx: &TickContext) -> Vec<Proposal> {
        let period = self.ticks_per_day.max(1);
        let level = wave(ctx.tick(), period, self.amplitude_centi_c);
        let prev = wave(ctx.tick().wrapping_sub(1), period, self.amplitude_centi_c);
        regions(view)
            .into_iter()
            .map(|region| {
                let exposure = exposure_of(view, region);
                Proposal::new(
                    self.id(),
                    FactKey::new(region, TEMPERATURE),
                    ctx.basis_tick(),
                    Change::Delta(attenuate(level - prev, exposure)),
                    Cause::new("diurnal_shift"),
                )
            })
            .collect()
    }
}

/// A small stochastic weather perturbation of temperature, drawn from the kernel-issued
/// substream (Vol. V Ch. 4 §4.1), for every region.
pub struct WeatherNoise {
    max_swing_centi_c: i64,
}

impl WeatherNoise {
    /// Perturb each region's temperature by up to +/- `max_swing_centi_c` each tick.
    pub const fn new(max_swing_centi_c: i64) -> Self {
        Self { max_swing_centi_c }
    }
}

impl System for WeatherNoise {
    fn id(&self) -> SystemId {
        SystemId::new("physical.weather_noise")
    }
    fn reads(&self) -> &'static [FactType] {
        DIURNAL_READS
    }
    fn writes(&self) -> &'static [FactType] {
        TEMPERATURE_W
    }
    fn cadence(&self) -> Cadence {
        Cadence::EveryTick
    }
    fn evaluate(&self, view: &dyn CommittedView, ctx: &TickContext) -> Vec<Proposal> {
        let swing = self.max_swing_centi_c.max(0);
        let span = (swing as u64) * 2 + 1;
        regions(view)
            .into_iter()
            .map(|region| {
                // Scope the substream by region id: content-keyed, so each region's weather is
                // independent and replays identically (Vol. V Ch. 4 §4.1).
                let mut rng = ctx.rng(region.raw());
                let raw = rng.below(span) as i64 - swing;
                let exposure = exposure_of(view, region);
                Proposal::new(
                    self.id(),
                    FactKey::new(region, TEMPERATURE),
                    ctx.basis_tick(),
                    Change::Delta(attenuate(raw, exposure)),
                    Cause::new("weather_perturbation"),
                )
            })
            .collect()
    }
}

/// The sun crossing the sky: illumination set to its absolute level for the tick, peaking at
/// midday and dark at midnight (Vol. III Ch. 1 §1.10, Time and Change), for every region.
/// The sun's position is a pure function of the clock; exposure scales it so a sealed cave
/// stays dark even at noon.
pub struct DayNightCycle {
    ticks_per_day: u64,
    peak_illumination: i64,
}

impl DayNightCycle {
    /// Light each region up to `peak_illumination` at midday over `ticks_per_day` ticks.
    pub const fn new(ticks_per_day: u64, peak_illumination: i64) -> Self {
        Self {
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
        ILLUMINATION_READS
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
        regions(view)
            .into_iter()
            .map(|region| {
                let exposure = exposure_of(view, region);
                Proposal::new(
                    self.id(),
                    FactKey::new(region, ILLUMINATION),
                    ctx.basis_tick(),
                    Change::Set(Value::Int(attenuate(sun, exposure))),
                    Cause::new("solar_position"),
                )
            })
            .collect()
    }
}

/// Weather driving humidity: it drifts toward a baseline while stochastic weather perturbs
/// it, drawn from the kernel-issued substream (Vol. V Ch. 4 §4.1), for every region. Reads
/// each region's own committed value to compute the drift, and creates the fact at the
/// baseline on first exposure.
pub struct Precipitation {
    baseline: i64,
    swing: i64,
    drying_divisor: i64,
}

impl Precipitation {
    /// Pull each region's humidity toward `baseline` (over `drying_divisor` ticks) with a
    /// per-tick weather perturbation of up to +/- `swing`.
    pub const fn new(baseline: i64, swing: i64, drying_divisor: i64) -> Self {
        Self {
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
        let mut out = Vec::new();
        for region in regions(view) {
            let key = FactKey::new(region, HUMIDITY);
            let current = match view.read(key) {
                None => {
                    // Establish humidity at its baseline the first time this runs.
                    out.push(Proposal::new(
                        self.id(),
                        key,
                        ctx.basis_tick(),
                        Change::Create(Value::Int(self.baseline)),
                        Cause::new("precipitation"),
                    ));
                    continue;
                }
                Some(fact) => match fact.value.as_int() {
                    Some(h) => h,
                    None => continue,
                },
            };
            let drift = (self.baseline - current) / self.drying_divisor.max(1);
            let swing = self.swing.max(0);
            let mut rng = ctx.rng(region.raw());
            let noise = rng.below((swing as u64) * 2 + 1) as i64 - swing;
            let exposure = exposure_of(view, region);
            out.push(Proposal::new(
                self.id(),
                key,
                ctx.basis_tick(),
                Change::Delta(drift + attenuate(noise, exposure)),
                Cause::new("precipitation"),
            ));
        }
        out
    }
}

/// Atmospheric pressure dynamics (Vol. III Ch. 1 §1.10): pressure settles toward a baseline
/// that falls with elevation, perturbed by stochastic weather, for every region. Reads each
/// region's elevation and its own pressure; creates the fact at the elevation baseline on
/// the first tick.
pub struct PressureSystem {
    sea_level: i64,
    elevation_factor: i64,
    weather_swing: i64,
    settle_divisor: i64,
}

impl PressureSystem {
    /// Configure pressure. `sea_level` is baseline pressure at the datum; `elevation_factor`
    /// is decapascals dropped per metre of elevation; `weather_swing` and `settle_divisor`
    /// govern the stochastic perturbation and the drift back to baseline (world-package
    /// rules, Vol. IV Ch. 2).
    pub const fn new(
        sea_level: i64,
        elevation_factor: i64,
        weather_swing: i64,
        settle_divisor: i64,
    ) -> Self {
        Self {
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
        let mut out = Vec::new();
        for region in regions(view) {
            let elevation = view
                .read(FactKey::new(region, ELEVATION))
                .and_then(|f| f.value.as_int())
                .unwrap_or(0);
            let baseline = self.baseline(elevation);
            let key = FactKey::new(region, PRESSURE);
            match view.read(key) {
                None => out.push(Proposal::new(
                    self.id(),
                    key,
                    ctx.basis_tick(),
                    Change::Create(Value::Int(baseline)),
                    Cause::new("pressure_baseline"),
                )),
                Some(fact) => {
                    if let Some(current) = fact.value.as_int() {
                        let drift = (baseline - current) / self.settle_divisor.max(1);
                        let swing = self.weather_swing.max(0);
                        let mut rng = ctx.rng(region.raw());
                        let noise = rng.below((swing as u64) * 2 + 1) as i64 - swing;
                        let exposure = exposure_of(view, region);
                        out.push(Proposal::new(
                            self.id(),
                            key,
                            ctx.basis_tick(),
                            Change::Delta(drift + attenuate(noise, exposure)),
                            Cause::new("pressure_weather"),
                        ));
                    }
                }
            }
        }
        out
    }
}

/// Wind as the consequence of pressure gradients across the topology (Vol. III Ch. 1 §1.10,
/// "Wind flows"), for every region. Reads each region's pressure and every neighbour's
/// pressure (via adjacency), then blows toward the lowest-pressure neighbour with a speed
/// proportional to the gradient — a genuinely multi-fact, topology-aware system that writes
/// both wind facts. Calm (speed zero, no direction) when no neighbour is lower. Wind
/// therefore lags pressure by one tick, as effects chain across commits (Vol. III Ch. 12
/// §12.2).
pub struct WindSystem {
    gradient_divisor: i64,
    max_wind: i64,
}

impl WindSystem {
    /// Configure wind. `gradient_divisor` scales speed per unit pressure difference (larger =
    /// gentler); `max_wind` clamps the speed (world-package rules).
    pub const fn new(gradient_divisor: i64, max_wind: i64) -> Self {
        Self {
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

/// The height of `entity` above the ground, in centimetres: the sum of local Z from the
/// entity up through its containers (excluding the root frame, whose origin is the ground
/// datum). A window on a stacked upper floor is high; a ground-floor door is at zero.
fn absolute_height(view: &dyn CommittedView, entity: EntityId) -> i64 {
    let mut total = 0i64;
    let mut here = entity;
    let mut guard = 0u32;
    loop {
        match view.read(FactKey::new(here, CONTAINED_IN)).map(|f| f.value) {
            Some(Value::Entity(parent)) => {
                total = total.saturating_add(
                    view.read(FactKey::new(here, POSITION_Z))
                        .and_then(|f| f.value.as_int())
                        .unwrap_or(0),
                );
                here = parent;
            }
            _ => break,
        }
        guard += 1;
        if guard > 1024 {
            break;
        }
    }
    total
}

/// Writes each portal's effective danger (Vol. III Ch. 1 §1.11). If the world pinned a fixed
/// danger the system echoes it; otherwise it derives danger from the portal's height above
/// the ground -- a fall -- leaving a slot for weather to raise it later. Enumerates every
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
                    // Derived: danger of the fall from this portal's height.
                    None => {
                        let height = absolute_height(view, portal).max(0);
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
