//! Residents and the rhythm of a settlement (Vol. III Ch. 5; Appendix A, Ruling 19; Amendment
//! A-23): the city tier, where a person is a resident record and nothing else, and the day goes
//! by in a handful of segments — asleep, at home, walking, working, eating — each chosen when the
//! last one ends.
//!
//! The rhythm attends only to residents whose segment has ended: it finds them through the agenda
//! (Amendment A-22), so a resident with nothing ending costs nothing. Hunger and fatigue are
//! levels (Amendment A-20), written only when they change course.

use crate::schema::{
    AT_HOME, CONTAINED_IN, EATING, LARDER, POSITION, RESIDENT_AT, RESIDENT_DOING, RESIDENT_FATIGUE,
    RESIDENT_HOME, RESIDENT_HUNGER, RESIDENT_UNTIL, RESIDENT_WORK, SLEEPING, WALKING, WORKING,
};
use kernel::fact::{Cause, FactKey, FactType, SystemId};
use kernel::fixed::isqrt;
use kernel::identity::EntityId;
use kernel::level::Level;
use kernel::proposal::{Change, Proposal};
use kernel::system::{Cadence, CommittedView, System, TickContext};
use kernel::time::SimClock;
use kernel::value::Value;
use std::collections::BTreeMap;

/// The top of a level: 100%.
pub const FULL: i64 = 10_000;

/// The rules a settlement's rhythm follows, all from the world package (Vol. IV Ch. 2 §2.2).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct RhythmRules {
    /// The length of a day, in seconds.
    pub day_seconds: i64,
    /// When the night begins and ends, in seconds into the day (wrapping past midnight).
    pub sleep_from_seconds: i64,
    /// When the night ends.
    pub sleep_to_seconds: i64,
    /// When working hours begin and end, in seconds into the day.
    pub work_from_seconds: i64,
    /// When working hours end.
    pub work_to_seconds: i64,
    /// Hunger past which a resident with food at home eats.
    pub hungry_above: i64,
    /// How much a meal eases hunger.
    pub meal_nutrition: i64,
    /// How many meals a finished shift earns the household.
    pub meals_per_shift: i64,
    /// How long a meal takes, in seconds.
    pub meal_seconds: i64,
    /// How fast a resident walks, in centimetres a second.
    pub walk_speed_cm_s: i64,
    /// How fast hunger rises, per hour.
    pub hunger_per_hour: i64,
    /// How fast a resident tires awake, per hour.
    pub tire_per_hour: i64,
    /// How fast a resident recovers asleep, per hour.
    pub rest_per_hour: i64,
    /// How far, in seconds either side, a resident's own hours may lie from the settlement's.
    pub hours_spread_seconds: i64,
}

const RHYTHM_READS: &[FactType] = &[
    RESIDENT_HOME,
    RESIDENT_WORK,
    RESIDENT_DOING,
    RESIDENT_AT,
    RESIDENT_UNTIL,
    RESIDENT_HUNGER,
    RESIDENT_FATIGUE,
    LARDER,
    POSITION,
    CONTAINED_IN,
];
const RHYTHM_WRITES: &[FactType] = &[
    RESIDENT_DOING,
    RESIDENT_AT,
    RESIDENT_UNTIL,
    RESIDENT_HUNGER,
    RESIDENT_FATIGUE,
    LARDER,
];

/// The rhythm of a settlement (Amendment A-23): every tick, each resident whose segment has ended
/// — and only those — is given the next one, from the hour and their state:
///
/// - a finished shift first earns the household its meals;
/// - **at home:** hungry with food in the larder, eat; at night, sleep until morning; in working
///   hours, walk to work; otherwise stay, until the next of night, work, or hunger is due;
/// - **away:** at work in working hours, work until they end; otherwise walk home.
///
/// A walk takes the time its distance takes at the world's pace. Fatigue falls asleep and rises
/// awake; hunger rises always and falls by a meal's nutrition.
pub struct Rhythm {
    clock: SimClock,
    rules: RhythmRules,
}

impl Rhythm {
    /// A settlement's rhythm by `rules`, on `clock`.
    pub const fn new(clock: SimClock, rules: RhythmRules) -> Self {
        Self { clock, rules }
    }

    /// Seconds into the day at `tick`.
    fn time_of_day(&self, tick: u64) -> i64 {
        (self.clock.ms_at(tick) / 1000) as i64 % self.rules.day_seconds.max(1)
    }

    /// The first tick after `tick` at which the day reaches `boundary` (seconds into the day).
    fn next(&self, tick: u64, boundary: i64) -> u64 {
        let day = self.rules.day_seconds.max(1);
        let mut wait = (boundary - self.time_of_day(tick)).rem_euclid(day);
        if wait == 0 {
            wait = day;
        }
        tick + self.ticks_for(wait * 1000)
    }

    /// How many whole ticks `ms` of simulated time takes: at least one.
    fn ticks_for(&self, ms: i64) -> u64 {
        let tick_ms = self.clock.tick_ms() as i64;
        ((ms.max(0) + tick_ms - 1) / tick_ms).max(1) as u64
    }
}

/// A resident's own offset from the settlement's hours, in seconds within `±spread`: fixed for
/// each resident, derived from who they are (SplitMix64 of their id), so the same resident keeps
/// the same hours in every run.
fn personal_offset(who: EntityId, spread: i64) -> i64 {
    if spread <= 0 {
        return 0;
    }
    let mut z = who.raw().wrapping_add(0x9e37_79b9_7f4a_7c15);
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^= z >> 31;
    (z % (2 * spread as u64 + 1)) as i64 - spread
}

/// Whether `t` (seconds into the day) lies in `[from, to)`, wrapping past midnight when
/// `from > to`.
fn in_window(t: i64, from: i64, to: i64) -> bool {
    if from <= to {
        from <= t && t < to
    } else {
        t >= from || t < to
    }
}

fn entity(view: &dyn CommittedView, key: FactKey) -> Option<EntityId> {
    match view.read(key)?.value {
        Value::Entity(e) => Some(e),
        _ => None,
    }
}

fn int(view: &dyn CommittedView, key: FactKey) -> Option<i64> {
    view.read(key).and_then(|f| f.value.as_int())
}

fn level(view: &dyn CommittedView, key: FactKey) -> Option<Level> {
    view.read(key).and_then(|f| Level::from_value(f.value))
}

/// Where `place` stands, as the crow flies, through every frame that holds it: its position in its
/// container, plus its container's, and so on up. Frames are not turned here, so a walk measured
/// between places in turned frames is an estimate — good enough for how long a walk takes.
fn whereabouts(view: &dyn CommittedView, place: EntityId) -> [i64; 2] {
    let mut at = [0i64; 2];
    let mut here = Some(place);
    let mut hops = 0;
    while let Some(p) = here.filter(|_| hops < 64) {
        if let Some([x, y, _]) = view
            .read(FactKey::new(p, POSITION))
            .and_then(|f| f.value.as_vec3())
        {
            at = [at[0] + x, at[1] + y];
        }
        here = entity(view, FactKey::new(p, CONTAINED_IN));
        hops += 1;
    }
    at
}

impl System for Rhythm {
    fn id(&self) -> SystemId {
        SystemId::new("society.rhythm")
    }
    fn reads(&self) -> &'static [FactType] {
        RHYTHM_READS
    }
    fn writes(&self) -> &'static [FactType] {
        RHYTHM_WRITES
    }
    fn cadence(&self) -> Cadence {
        Cadence::EveryTick
    }
    fn evaluate(&self, view: &dyn CommittedView, ctx: &TickContext) -> Vec<Proposal> {
        let r = &self.rules;
        let now = ctx.tick();
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
        // What each household's larder holds as this tick's residents take from it and add to it,
        // in ascending order of resident.
        let mut larders: BTreeMap<EntityId, i64> = BTreeMap::new();
        let mut stocked: BTreeMap<EntityId, i64> = BTreeMap::new();
        let time = self.time_of_day(now);
        let day = r.day_seconds.max(1);

        for who in view.due(RESIDENT_UNTIL, now) {
            // This resident's own hours: the settlement's, shifted by their offset.
            let own = |s: i64| (s + personal_offset(who, r.hours_spread_seconds)).rem_euclid(day);
            let (sleep_from, sleep_to) = (own(r.sleep_from_seconds), own(r.sleep_to_seconds));
            let (work_from, work_to) = (own(r.work_from_seconds), own(r.work_to_seconds));
            let night = in_window(time, sleep_from, sleep_to);
            let working_hours = in_window(time, work_from, work_to);
            let Some(home) = entity(view, FactKey::new(who, RESIDENT_HOME)) else {
                continue;
            };
            let work = entity(view, FactKey::new(who, RESIDENT_WORK));
            let doing = int(view, FactKey::new(who, RESIDENT_DOING)).unwrap_or(AT_HOME);
            let at = entity(view, FactKey::new(who, RESIDENT_AT)).unwrap_or(home);
            let hunger = level(view, FactKey::new(who, RESIDENT_HUNGER)).unwrap_or(Level::new(
                0,
                r.hunger_per_hour,
                now,
            ));
            let fatigue = level(view, FactKey::new(who, RESIDENT_FATIGUE)).unwrap_or(Level::new(
                0,
                r.tire_per_hour,
                now,
            ));

            // A finished shift earns the household its keep: a boundary process standing in for
            // the economy the city tier does not simulate (Vol. I, Conservation).
            let held = *larders
                .entry(home)
                .or_insert_with(|| int(view, FactKey::new(home, LARDER)).unwrap_or(0));
            let mut in_larder = held + stocked.get(&home).copied().unwrap_or(0);
            if doing == WORKING {
                *stocked.entry(home).or_insert(0) += r.meals_per_shift;
                in_larder += r.meals_per_shift;
            }
            let walk = |from: EntityId, to: EntityId| {
                let ([ax, ay], [bx, by]) = (whereabouts(view, from), whereabouts(view, to));
                let (dx, dy) = ((ax - bx) as i128, (ay - by) as i128);
                let cm = isqrt((dx * dx + dy * dy) as u128) as i64;
                now + self.ticks_for(cm * 1000 / r.walk_speed_cm_s.max(1))
            };
            let hungry = hunger.at(self.clock, now, 0, FULL);
            let mut ate = false;
            let (next, place, until, why) = if at == home {
                if hungry > r.hungry_above && in_larder > 0 {
                    ate = true;
                    let end = now + self.ticks_for(r.meal_seconds * 1000);
                    (EATING, home, end, "eating")
                } else if night {
                    (SLEEPING, home, self.next(now, sleep_to), "sleeping")
                } else if let (true, Some(work)) = (working_hours, work) {
                    (WALKING, work, walk(home, work), "walking_to_work")
                } else {
                    // Nothing pressing: stay until the next of night, work, or hunger is due.
                    let mut until = self.next(now, sleep_from);
                    if work.is_some() {
                        until = until.min(self.next(now, work_from));
                    }
                    if in_larder > 0 {
                        let lo = hunger.reaches(self.clock, r.hungry_above + 1, now, 0, FULL);
                        if let Some(t) = lo.filter(|t| *t > now) {
                            until = until.min(t);
                        }
                    }
                    (AT_HOME, home, until, "at_home")
                }
            } else if Some(at) == work && working_hours {
                (WORKING, at, self.next(now, work_to), "working")
            } else {
                (WALKING, home, walk(at, home), "walking_home")
            };

            if next != doing {
                push(
                    FactKey::new(who, RESIDENT_DOING),
                    Change::Set(Value::Int(next)),
                    why,
                );
            }
            if place != at {
                push(
                    FactKey::new(who, RESIDENT_AT),
                    Change::Set(Value::Entity(place)),
                    why,
                );
            }
            push(
                FactKey::new(who, RESIDENT_UNTIL),
                Change::Set(Value::Int(until.max(now + 1) as i64)),
                why,
            );
            if ate {
                *larders.entry(home).or_insert(0) -= 1;
                let eased = Level::new((hungry - r.meal_nutrition).max(0), r.hunger_per_hour, now);
                push(
                    FactKey::new(who, RESIDENT_HUNGER),
                    Change::Set(eased.to_value()),
                    "ate",
                );
            } else if hunger.per_hour != r.hunger_per_hour {
                let rising = hunger.from_tick(self.clock, now, 0, FULL, r.hunger_per_hour);
                push(
                    FactKey::new(who, RESIDENT_HUNGER),
                    Change::Set(rising.to_value()),
                    "hungry_at_the_worlds_rate",
                );
            }
            let rate = if next == SLEEPING {
                -r.rest_per_hour
            } else {
                r.tire_per_hour
            };
            if fatigue.per_hour != rate {
                let course = fatigue.from_tick(self.clock, now, 0, FULL, rate);
                push(
                    FactKey::new(who, RESIDENT_FATIGUE),
                    Change::Set(course.to_value()),
                    why,
                );
            }
        }
        // One write per household whose larder changed.
        for (home, taken) in &larders {
            let earned = stocked.get(home).copied().unwrap_or(0);
            let held = int(view, FactKey::new(*home, LARDER)).unwrap_or(0);
            let now_held = taken + earned;
            if now_held != held {
                push(
                    FactKey::new(*home, LARDER),
                    Change::Set(Value::Int(now_held.max(0))),
                    "larder",
                );
            }
        }
        out
    }
}
