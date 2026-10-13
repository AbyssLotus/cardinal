//! Residents and the rhythm of a settlement (Vol. III Ch. 5; Appendix A, Ruling 19; Amendment
//! A-23): the city tier, where a person is a resident record and nothing else, and the day goes
//! by in a handful of segments — asleep, at home, walking, working, eating — each chosen when the
//! last one ends.
//!
//! The rhythm attends only to residents whose segment has ended: it finds them through the agenda
//! (Amendment A-22), so a resident with nothing ending costs nothing. Hunger and fatigue are
//! levels (Amendment A-20), written only when they change course.

use crate::schema::{
    AT_HOME, CONTAINED_IN, EATING, LARDER, PANTRY, PANTRY_CROSSING, PANTRY_IN_SCENE,
    PANTRY_LEAVING, POSITION, RESIDENCY, RESIDENT_AT, RESIDENT_CROSSING, RESIDENT_DOING,
    RESIDENT_FATIGUE, RESIDENT_HOME, RESIDENT_HUNGER, RESIDENT_IN_SCENE, RESIDENT_UNTIL,
    RESIDENT_WORK, RESTOCK, SCENE, SLEEPING, STOCK, UNIT, WALKING, WORKING,
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
    RESIDENCY,
    RESIDENT_CROSSING,
    PANTRY,
    PANTRY_CROSSING,
    PANTRY_IN_SCENE,
    PANTRY_LEAVING,
    STOCK,
    RESTOCK,
    RESIDENT_IN_SCENE,
];
const RHYTHM_WRITES: &[FactType] = &[
    RESIDENT_CROSSING,
    RESIDENT_DOING,
    RESIDENT_AT,
    RESIDENT_UNTIL,
    RESIDENT_HUNGER,
    RESIDENT_FATIGUE,
    LARDER,
    RESTOCK,
];

/// Where a household's larder is this tick (Amendment A-26): crossing into the scene as its
/// pantry, in the scene, or leaving it — each naming the pantry.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Pantry {
    Crossing(EntityId),
    InScene(EntityId),
    Leaving(EntityId),
}

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

    /// The record of a resident coming out of the scene at `now` into `place`, a place at the
    /// city tier, standing at `from` (Amendment A-25): what they are doing, where, and until when.
    /// Lying down at home in their night, they are asleep until their morning, as the rhythm
    /// would have had them. Otherwise, at their home or their work they have arrived, and the
    /// rhythm takes them up on the next tick. Anywhere else they walk on from where they stand —
    /// to work in their working hours, otherwise home — taking the time that takes.
    pub(crate) fn emerging(
        &self,
        view: &dyn CommittedView,
        who: EntityId,
        place: EntityId,
        from: [i64; 2],
        resting: bool,
        now: u64,
    ) -> (i64, EntityId, u64) {
        let r = &self.rules;
        let home = entity(view, FactKey::new(who, RESIDENT_HOME));
        let work = entity(view, FactKey::new(who, RESIDENT_WORK));
        let day = r.day_seconds.max(1);
        let own = |s: i64| (s + personal_offset(who, r.hours_spread_seconds)).rem_euclid(day);
        let night = in_window(
            self.time_of_day(now),
            own(r.sleep_from_seconds),
            own(r.sleep_to_seconds),
        );
        if Some(place) == home && resting && night {
            return (SLEEPING, place, self.next(now, own(r.sleep_to_seconds)));
        }
        if Some(place) == home || Some(place) == work {
            return (WALKING, place, now + 1);
        }
        let working_hours = in_window(
            self.time_of_day(now),
            own(r.work_from_seconds),
            own(r.work_to_seconds),
        );
        let to = match (working_hours, work) {
            (true, Some(work)) => work,
            _ => home.unwrap_or(place),
        };
        let [bx, by] = whereabouts(view, to);
        let (dx, dy) = ((from[0] - bx) as i128, (from[1] - by) as i128);
        let cm = isqrt((dx * dx + dy * dy) as u128) as i64;
        let until = now + self.ticks_for(cm * 1000 / r.walk_speed_cm_s.max(1));
        (WALKING, to, until)
    }
}

/// A resident's own offset from the settlement's hours, in seconds within `±spread`: fixed for
/// each resident, derived from who they are, so the same resident keeps the same hours in every
/// run — and at every tier (Amendment A-24).
fn personal_offset(who: EntityId, spread: i64) -> i64 {
    who.jitter(spread)
}

/// A place's rung on the residency ladder (Amendment A-24): its own, or the nearest declared by a
/// place holding it; with none, the scene. `memo` remembers places already answered.
pub fn rung(view: &dyn CommittedView, place: EntityId, memo: &mut BTreeMap<EntityId, i64>) -> i64 {
    let mut path = Vec::new();
    let mut here = Some(place);
    let mut found = SCENE;
    while let Some(p) = here.filter(|_| path.len() < 64) {
        if let Some(r) = memo.get(&p) {
            found = *r;
            break;
        }
        path.push(p);
        if let Some(r) = int(view, FactKey::new(p, RESIDENCY)) {
            found = r;
            break;
        }
        here = entity(view, FactKey::new(p, CONTAINED_IN));
    }
    for p in path {
        memo.insert(p, found);
    }
    found
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
pub(crate) fn whereabouts(view: &dyn CommittedView, place: EntityId) -> [i64; 2] {
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
        // Larders in the scene as pantries, by home (Amendment A-26). The rhythm keeps every
        // larder's books across the seam: it lets the count go as the pantry crosses in, sends
        // what is earned meanwhile to the pantry, and counts the pantry back in as it leaves.
        let mut pantries: BTreeMap<EntityId, Pantry> = BTreeMap::new();
        for (fact, state) in [
            (PANTRY_IN_SCENE, Pantry::InScene as fn(EntityId) -> Pantry),
            (PANTRY_CROSSING, Pantry::Crossing),
            (PANTRY_LEAVING, Pantry::Leaving),
        ] {
            for pantry in view.entities_with(fact) {
                if let Some(home) = entity(view, FactKey::new(pantry, fact)) {
                    pantries.insert(home, state(pantry));
                }
            }
        }
        // What a larder holds now: its count, or, as its pantry leaves, what the pantry holds and
        // what was delivered to it and not yet taken in. In the scene it holds nothing here: its
        // meals are the pantry's.
        let held_in = |home: EntityId| match pantries.get(&home) {
            None => int(view, FactKey::new(home, LARDER)).unwrap_or(0),
            Some(Pantry::Leaving(p)) => {
                int(view, FactKey::new(*p, STOCK)).unwrap_or(0) / UNIT
                    + int(view, FactKey::new(*p, RESTOCK)).unwrap_or(0)
            }
            Some(_) => 0,
        };
        let time = self.time_of_day(now);
        let day = r.day_seconds.max(1);
        // The scene (Amendment A-24). Those crossing are the seam's. A resident's place changes
        // only by walking, so only one arriving can find themselves somewhere the scene is —
        // unless attention moved in the last tick, when the seam marks everyone standing where it
        // arrived, and the rhythm leaves them be. A store that cannot say what changed is taken
        // to say that anything may have.
        let crossing = view.entities_with(RESIDENT_CROSSING);
        let basis = ctx.basis_tick();
        let attention_moved = basis > 0
            && view
                .changed_since(RESIDENCY, basis - 1)
                .map_or(true, |c| !c.is_empty());
        let mut rungs = BTreeMap::new();

        for who in view.due(RESIDENT_UNTIL, now) {
            if crossing.binary_search(&who).is_ok() {
                continue;
            }
            let doing = int(view, FactKey::new(who, RESIDENT_DOING)).unwrap_or(AT_HOME);
            let at = entity(view, FactKey::new(who, RESIDENT_AT));
            if let (Some(at), true) = (at, doing == WALKING || attention_moved) {
                if rung(view, at, &mut rungs) == SCENE {
                    if doing == WALKING {
                        push(
                            FactKey::new(who, RESIDENT_CROSSING),
                            Change::Set(Value::Entity(at)),
                            "arrived_in_the_scene",
                        );
                    }
                    continue;
                }
            }
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
            let at = at.unwrap_or(home);
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
            let held = *larders.entry(home).or_insert_with(|| held_in(home));
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
            } else if let (true, Some(work)) = (working_hours && Some(at) != work, work) {
                // Neither home nor at work — just come out of the scene (Amendment A-25) — in
                // working hours: on to work.
                (WALKING, work, walk(at, work), "walking_to_work")
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
        // A shift worked in the scene earns its keep as one at the city tier does (Amendment
        // A-26): a resident in the scene standing at their work as their own hours end.
        if now > 0 {
            let before = self.time_of_day(now - 1);
            for who in view.entities_with(RESIDENT_IN_SCENE) {
                let (Some(home), Some(work)) = (
                    entity(view, FactKey::new(who, RESIDENT_HOME)),
                    entity(view, FactKey::new(who, RESIDENT_WORK)),
                ) else {
                    continue;
                };
                let end = (r.work_to_seconds + personal_offset(who, r.hours_spread_seconds))
                    .rem_euclid(day);
                let ended = if before <= time {
                    before < end && end <= time
                } else {
                    end > before || end <= time
                };
                if ended && entity(view, FactKey::new(who, CONTAINED_IN)) == Some(work) {
                    larders.entry(home).or_insert_with(|| held_in(home));
                    *stocked.entry(home).or_insert(0) += r.meals_per_shift;
                }
            }
        }
        // One write per household whose larder changed — and every larder crossing the seam.
        for (&home, state) in &pantries {
            if !matches!(state, Pantry::InScene(_)) {
                larders.entry(home).or_insert_with(|| held_in(home));
            }
        }
        for (home, taken) in &larders {
            let earned = stocked.get(home).copied().unwrap_or(0);
            match pantries.get(home) {
                None => {
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
                Some(Pantry::Leaving(_)) => push(
                    FactKey::new(*home, LARDER),
                    Change::Set(Value::Int((taken + earned).max(0))),
                    "larder_counted_back",
                ),
                Some(Pantry::Crossing(pantry) | Pantry::InScene(pantry)) => {
                    if matches!(pantries.get(home), Some(Pantry::Crossing(_)))
                        && view.read(FactKey::new(*home, LARDER)).is_some()
                    {
                        push(
                            FactKey::new(*home, LARDER),
                            Change::Tombstone,
                            "larder_set_out",
                        );
                    }
                    if earned > 0 {
                        push(
                            FactKey::new(*pantry, RESTOCK),
                            Change::Delta(earned),
                            "larder",
                        );
                    }
                }
            }
        }
        out
    }
}
