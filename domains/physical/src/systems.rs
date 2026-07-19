//! Hermetic transformations owned by the Physical Reality domain (Vol. V Ch. 3 §3.1).
//!
//! Each system declares its read/write sets and cadence, reads committed reality, and emits
//! proposals — mutating nothing directly (Vol. V Ch. 3 §3.1-3.2). These drive the
//! environmental fields that vary across space and time (Vol. III Ch. 1 §1.10): temperature
//! (a deterministic day/night swing plus stochastic weather), illumination (the sun's
//! position), and humidity (weather). Implement causes, never outcomes (Vol. III Ch. 11).

use crate::schema::{HUMIDITY, ILLUMINATION, TEMPERATURE};
use kernel::fact::{Cause, FactKey, FactType, SystemId};
use kernel::identity::EntityId;
use kernel::proposal::{Change, Proposal};
use kernel::system::{Cadence, CommittedView, System, TickContext};

const TEMPERATURE_RW: &[FactType] = &[TEMPERATURE];
const ILLUMINATION_W: &[FactType] = &[ILLUMINATION];
const HUMIDITY_RW: &[FactType] = &[HUMIDITY];
const NOTHING: &[FactType] = &[];

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
        NOTHING
    }
    fn writes(&self) -> &'static [FactType] {
        TEMPERATURE_RW
    }
    fn cadence(&self) -> Cadence {
        Cadence::EveryTick
    }
    fn evaluate(&self, _view: &dyn CommittedView, ctx: &TickContext) -> Vec<Proposal> {
        let period = self.ticks_per_day.max(1);
        let level = wave(ctx.tick(), period, self.amplitude_centi_c);
        let prev = wave(ctx.tick().wrapping_sub(1), period, self.amplitude_centi_c);
        vec![Proposal::new(
            self.id(),
            FactKey::new(self.region, TEMPERATURE),
            ctx.tick(),
            Change::Delta(level - prev),
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
        NOTHING
    }
    fn writes(&self) -> &'static [FactType] {
        TEMPERATURE_RW
    }
    fn cadence(&self) -> Cadence {
        Cadence::EveryTick
    }
    fn evaluate(&self, _view: &dyn CommittedView, ctx: &TickContext) -> Vec<Proposal> {
        let swing = self.max_swing_centi_c.max(0);
        let span = (swing as u64) * 2 + 1;
        let mut rng = ctx.rng(self.region.raw());
        let delta = rng.below(span) as i64 - swing;
        vec![Proposal::new(
            self.id(),
            FactKey::new(self.region, TEMPERATURE),
            ctx.tick(),
            Change::Delta(delta),
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
        NOTHING
    }
    fn writes(&self) -> &'static [FactType] {
        ILLUMINATION_W
    }
    fn cadence(&self) -> Cadence {
        Cadence::EveryTick
    }
    fn evaluate(&self, _view: &dyn CommittedView, ctx: &TickContext) -> Vec<Proposal> {
        let sun = wave(
            ctx.tick(),
            self.ticks_per_day.max(2),
            self.peak_illumination,
        );
        vec![Proposal::new(
            self.id(),
            FactKey::new(self.region, ILLUMINATION),
            ctx.tick(),
            // Absolute level: the sun's position doesn't accumulate, it *is* wherever it is.
            Change::Set(kernel::value::Value::Int(sun)),
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
        HUMIDITY_RW
    }
    fn writes(&self) -> &'static [FactType] {
        HUMIDITY_RW
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
        vec![Proposal::new(
            self.id(),
            key,
            ctx.tick(),
            Change::Delta(drift + noise),
            Cause::new("precipitation"),
        )]
    }
}
