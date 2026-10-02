//! Simulated duration — Vol. II Ch. 2 (*Simulated Duration*, Amendment A-1); Vol. V Ch. 3 §3.2.
//!
//! A tick is a declared length of simulated time. The world package states it (its clock
//! rule); the kernel only does the arithmetic that turns "every minute" into "every N ticks"
//! and tells a system how much simulated time its step really covers. The kernel knows no
//! particular tick length, day length, or rate — those are world content (Vol. IV Ch. 2 §2.2).
//!
//! Durations are carried in **milliseconds** so a world can tick in tenths of a second (an
//! interactive 3D scene) or in hours (a slow chronicle of seasons) with the same arithmetic.
//! Every rate in the engine is declared per unit of simulated time and converted through a
//! [`Step`]; that is what makes tick length a resolution choice rather than a rule
//! (Vol. II Ch. 2, invariant 11).

use crate::system::Cadence;

/// The world's clock rule: how much simulated time one tick lasts (Vol. II Ch. 2).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct SimClock {
    tick_ms: u64,
}

impl SimClock {
    /// A clock whose ticks each last `tick_ms` milliseconds of simulated time. A zero length
    /// is meaningless (time would not pass); it is raised to one millisecond, and the package
    /// loader rejects it before it ever gets here.
    pub const fn new(tick_ms: u64) -> Self {
        Self {
            tick_ms: if tick_ms == 0 { 1 } else { tick_ms },
        }
    }

    /// The simulated length of one tick, in milliseconds.
    pub const fn tick_ms(&self) -> u64 {
        self.tick_ms
    }

    /// The simulated time at the end of `tick`, in milliseconds since the world began. Tick 0
    /// is the initial world, at time zero.
    pub const fn ms_at(&self, tick: u64) -> u64 {
        tick.saturating_mul(self.tick_ms)
    }

    /// The step a system that wants to run every `every_ms` of simulated time actually takes:
    /// the nearest whole number of ticks (never fewer than one), and the simulated duration
    /// that number of ticks covers.
    ///
    /// Rounding to whole ticks is unavoidable — reality changes only at tick boundaries
    /// (Vol. II Ch. 2, invariant 1) — so a system must advance by [`Step::dt_ms`], the time its
    /// step really spans, never by the `every_ms` it asked for. Then nothing drifts: a weather
    /// system asking for a 90-second step in a world of one-minute ticks steps every two ticks
    /// and advances two minutes of weather each time.
    pub const fn step(&self, every_ms: u64) -> Step {
        // Round to the nearest whole tick; ties round up.
        let ticks = (every_ms + self.tick_ms / 2) / self.tick_ms;
        let period_ticks = if ticks == 0 { 1 } else { ticks };
        Step {
            period_ticks,
            dt_ms: period_ticks * self.tick_ms,
        }
    }
}

/// A system's cadence and the simulated time each of its runs covers (Vol. V Ch. 3 §3.2,
/// cadence in simulated time). Built by [`SimClock::step`].
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Step {
    /// How many ticks apart the system runs (at least one).
    pub period_ticks: u64,
    /// The simulated time one run advances, in milliseconds: `period_ticks` × the tick length.
    pub dt_ms: u64,
}

impl Step {
    /// The cadence to register with the scheduler for this step.
    pub const fn cadence(&self) -> Cadence {
        if self.period_ticks <= 1 {
            Cadence::EveryTick
        } else {
            Cadence::EveryNTicks(self.period_ticks)
        }
    }

    /// The tick this run's step began at, given the tick being computed: `tick - period`.
    /// A system that integrates over its step (the day/night swing) evaluates its rule at
    /// both ends of the step; at the first run that is tick 0, the initial world.
    pub const fn start_of(&self, tick: u64) -> u64 {
        tick.saturating_sub(self.period_ticks)
    }
}

#[cfg(test)]
mod tests {
    use super::SimClock;
    use crate::system::Cadence;

    #[test]
    fn a_step_rounds_to_whole_ticks_and_reports_its_true_length() {
        let minute_ticks = SimClock::new(60_000);
        // Asked for 90 s in a world of 60 s ticks: two ticks, two minutes advanced.
        let s = minute_ticks.step(90_000);
        assert_eq!((s.period_ticks, s.dt_ms), (2, 120_000));
        assert_eq!(s.cadence(), Cadence::EveryNTicks(2));
        // Asked for less than a tick: once per tick, one tick advanced.
        let s = minute_ticks.step(1_000);
        assert_eq!((s.period_ticks, s.dt_ms), (1, 60_000));
        assert_eq!(s.cadence(), Cadence::EveryTick);
    }

    #[test]
    fn the_same_request_covers_the_same_time_at_any_tick_length() {
        // "Every hour" advances an hour per run whether ticks are seconds or hours.
        for tick_ms in [1_000, 60_000, 3_600_000] {
            let s = SimClock::new(tick_ms).step(3_600_000);
            assert_eq!(s.dt_ms, 3_600_000, "tick {tick_ms} ms");
        }
    }

    #[test]
    fn simulated_time_is_ticks_times_length() {
        let c = SimClock::new(100);
        assert_eq!(c.ms_at(0), 0);
        assert_eq!(c.ms_at(36_000), 3_600_000);
        assert_eq!(
            SimClock::new(0).tick_ms(),
            1,
            "a zero-length tick is refused"
        );
    }
}
