//! Levels (Amendment A-20; Vol. V Ch. 2 §2.2, *What Is Never Stored Per Tick*): a quantity that
//! moves at a steady rate between events — hunger between meals, fatigue between lying down and
//! getting up — stored as where it stood, how fast it moves, and since when.
//!
//! A level is one three-component fact, `[value, rate per hour, since tick]`. Its value at any
//! later tick is derived from the simulated time elapsed, exactly, in whole milliseconds and
//! integer arithmetic: the same at every tick length and on every platform, with no rounding
//! carried from one tick to the next. This module is the one definition every domain reads a
//! level by, so every reader agrees on its value at every tick.

use crate::time::SimClock;
use crate::value::Value;

/// Milliseconds in an hour: a level's rate is per hour of simulated time.
const MS_PER_HOUR: i128 = 3_600_000;

/// A quantity moving at a steady rate: its `value` at tick `since`, and how much it changes per
/// hour of simulated time from then on (negative to fall).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Level {
    /// Where it stood at `since`.
    pub value: i64,
    /// How much it changes per hour, in its own units.
    pub per_hour: i64,
    /// The tick it stood at `value`.
    pub since: u64,
}

impl Level {
    /// A level standing at `value` at tick `since`, moving `per_hour` from then on.
    pub const fn new(value: i64, per_hour: i64, since: u64) -> Self {
        Self {
            value,
            per_hour,
            since,
        }
    }

    /// Read a level from its fact. A plain number is a level that does not move.
    pub fn from_value(value: Value) -> Option<Self> {
        match value {
            Value::Vec3([v, rate, since]) => Some(Self::new(v, rate, since.max(0) as u64)),
            Value::Int(v) => Some(Self::new(v, 0, 0)),
            _ => None,
        }
    }

    /// The level as its fact's value.
    pub const fn to_value(self) -> Value {
        Value::Vec3([self.value, self.per_hour, self.since as i64])
    }

    /// Its value at `tick` on `clock`, held within `lo..=hi`. Before `since` it is where it
    /// stood. The change is rounded toward negative infinity, the same way every time.
    pub fn at(&self, clock: SimClock, tick: u64, lo: i64, hi: i64) -> i64 {
        let elapsed = clock.ms_at(tick).saturating_sub(clock.ms_at(self.since)) as i128;
        let moved = (self.per_hour as i128 * elapsed).div_euclid(MS_PER_HOUR);
        (self.value as i128 + moved).clamp(lo as i128, hi as i128) as i64
    }

    /// The same level carried to `tick` — standing where it had reached, held within `lo..=hi`
    /// — moving `per_hour` from then on. How a level changes course.
    pub fn from_tick(&self, clock: SimClock, tick: u64, lo: i64, hi: i64, per_hour: i64) -> Self {
        Self::new(self.at(clock, tick, lo, hi), per_hour, tick)
    }

    /// The first tick at or after `from` at which the level, held within `lo..=hi`, has reached
    /// `line` — risen to it if it rises, fallen to it if it falls — or `None` if it never will.
    pub fn reaches(&self, clock: SimClock, line: i64, from: u64, lo: i64, hi: i64) -> Option<u64> {
        let now = self.at(clock, from, lo, hi);
        let reached = |v: i64| {
            if self.per_hour >= 0 {
                v >= line
            } else {
                v <= line
            }
        };
        if reached(now) {
            return Some(from);
        }
        if self.per_hour == 0 || !reached(line.clamp(lo, hi)) {
            return None; // still, or bounded short of the line
        }
        // The fewest milliseconds after `since` at which the change covers the gap, then the
        // first tick at or after that moment.
        let gap = (line as i128 - self.value as i128).abs();
        let rate = (self.per_hour as i128).abs();
        let ms = (gap * MS_PER_HOUR + rate - 1) / rate;
        let at_ms = clock.ms_at(self.since) as i128 + ms;
        let tick_ms = clock.tick_ms() as i128;
        let tick = ((at_ms + tick_ms - 1) / tick_ms).max(from as i128);
        // Rounding toward negative infinity can leave a falling level a tick short; step on.
        (tick..tick + 2)
            .map(|t| t as u64)
            .find(|t| reached(self.at(clock, *t, lo, hi)))
    }
}

#[cfg(test)]
mod tests {
    use super::Level;
    use crate::time::SimClock;
    use crate::value::Value;

    const MINUTE: SimClock = SimClock::new(60_000);
    const TEN: SimClock = SimClock::new(600_000);

    #[test]
    fn a_level_moves_at_its_rate_from_where_it_stood() {
        // 200 an hour from 1,000 at tick 6 (ten-minute ticks): an hour later, 1,200.
        let hunger = Level::new(1_000, 200, 6);
        assert_eq!(hunger.at(TEN, 6, 0, 10_000), 1_000);
        assert_eq!(hunger.at(TEN, 12, 0, 10_000), 1_200);
        assert_eq!(
            hunger.at(TEN, 3, 0, 10_000),
            1_000,
            "before it began, where it stood"
        );
    }

    #[test]
    fn the_same_moment_has_the_same_value_at_any_tick_length() {
        // Ten hours at 200 an hour is 2,000, by the minute or by ten minutes.
        let by_minute = Level::new(0, 200, 0).at(MINUTE, 600, 0, 10_000);
        let by_ten = Level::new(0, 200, 0).at(TEN, 60, 0, 10_000);
        assert_eq!((by_minute, by_ten), (2_000, 2_000));
        // A rate that does not divide the tick: 450 an hour for 7 minutes is 52.5, rounded down.
        assert_eq!(Level::new(0, 450, 0).at(MINUTE, 7, 0, 10_000), 52);
    }

    #[test]
    fn a_level_is_held_within_its_bounds() {
        let resting = Level::new(500, -1_250, 0);
        assert_eq!(resting.at(TEN, 60, 0, 10_000), 0);
        let starving = Level::new(9_900, 200, 0);
        assert_eq!(starving.at(TEN, 60, 0, 10_000), 10_000);
    }

    #[test]
    fn changing_course_starts_from_where_it_had_reached() {
        let awake = Level::new(1_000, 450, 0);
        let lying_down = awake.from_tick(TEN, 12, 0, 10_000, -1_250);
        assert_eq!(lying_down, Level::new(1_900, -1_250, 12));
        assert_eq!(lying_down.at(TEN, 18, 0, 10_000), 650);
    }

    #[test]
    fn it_knows_when_it_will_reach_a_line() {
        // From 3,000 at 200 an hour, 4,000 is five hours off: tick 30 at ten-minute ticks.
        let hunger = Level::new(3_000, 200, 0);
        assert_eq!(hunger.reaches(TEN, 4_000, 0, 0, 10_000), Some(30));
        assert_eq!(hunger.at(TEN, 29, 0, 10_000), 3_966);
        // Already past it, it has reached it now; held below it, never; still, never.
        assert_eq!(hunger.reaches(TEN, 2_000, 5, 0, 10_000), Some(5));
        assert_eq!(hunger.reaches(TEN, 12_000, 0, 0, 10_000), None);
        assert_eq!(
            Level::new(3_000, 0, 0).reaches(TEN, 4_000, 0, 0, 10_000),
            None
        );
        // Falling: from 2,000 at −1,250 an hour, 1,500 is 24 minutes off: tick 3.
        let resting = Level::new(2_000, -1_250, 0);
        assert_eq!(resting.reaches(TEN, 1_500, 0, 0, 10_000), Some(3));
        assert!(resting.at(TEN, 3, 0, 10_000) <= 1_500 && resting.at(TEN, 2, 0, 10_000) > 1_500);
    }

    #[test]
    fn a_level_round_trips_through_its_fact_and_a_number_stands_still() {
        let l = Level::new(42, -7, 9);
        assert_eq!(Level::from_value(l.to_value()), Some(l));
        assert_eq!(Level::from_value(Value::Int(5)), Some(Level::new(5, 0, 0)));
        assert_eq!(Level::from_value(Value::Bool(true)), None);
    }
}
