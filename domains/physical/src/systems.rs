//! Hermetic transformations owned by the Physical Reality domain (Vol. V Ch. 3 §3.1).
//!
//! Each system declares its read/write sets and cadence, reads committed reality, and emits
//! proposals — mutating nothing directly (Vol. V Ch. 3 §3.1-3.2). Implement causes, never
//! outcomes: no phenomenon systems (Vol. III Ch. 11 §11.3).

use crate::schema::TEMPERATURE;
use kernel::fact::{Cause, FactKey, FactType, SystemId};
use kernel::identity::EntityId;
use kernel::proposal::{Change, Proposal};
use kernel::system::{Cadence, CommittedView, System, TickContext};

const READS: &[FactType] = &[TEMPERATURE];
const WRITES: &[FactType] = &[TEMPERATURE];

/// The temperature fact addressed on a given region.
fn region_key(region: EntityId) -> FactKey {
    FactKey::new(region, TEMPERATURE)
}

/// An integer triangle wave in `0..=amp` over `period`, peaking at mid-period.
///
/// Systems propose the *difference* between consecutive levels, so summed deltas telescope
/// and temperature stays bounded rather than drifting.
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

/// A deterministic day/night temperature swing (Vol. III Ch. 1, environmental state).
///
/// Purely a function of the clock — no randomness. Proposes the per-tick delta of a
/// triangle wave over the day, so the swing is bounded and reproducible.
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
        READS
    }

    fn writes(&self) -> &'static [FactType] {
        WRITES
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
            region_key(self.region),
            ctx.tick(),
            Change::Delta(level - prev),
            Cause::new("diurnal_shift"),
        )]
    }
}

/// A small stochastic weather perturbation, drawn from the kernel-issued substream
/// (Vol. V Ch. 4 §4.1).
///
/// Demonstrates deterministic randomness through the real pipeline: the same seed yields
/// the same perturbations, a different seed diverges.
pub struct WeatherNoise {
    region: EntityId,
    max_swing_centi_c: i64,
}

impl WeatherNoise {
    /// Perturb `region` by up to +/- `max_swing_centi_c` each tick.
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
        READS
    }

    fn writes(&self) -> &'static [FactType] {
        WRITES
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
            region_key(self.region),
            ctx.tick(),
            Change::Delta(delta),
            Cause::new("weather_perturbation"),
        )]
    }
}
