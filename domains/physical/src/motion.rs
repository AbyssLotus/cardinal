//! Motion as segments (Vol. III Ch. 1, *Bodies, Facing, and Motion*; Amendment A-3).
//!
//! A moving body carries one straight segment: where it set out from ([`POSITION`]), where it
//! is heading ([`MOTION_TARGET`]), and the ticks it left and will arrive ([`MOTION_START`],
//! [`MOTION_END`]). Everything else about its motion is *derived*, here, the same way for every
//! consumer:
//!
//! - where it is at any tick ([`position_at`]) — linear along the segment, exact at both ends;
//! - whether it is moving now ([`is_moving`]), and when it arrives ([`arrival`]);
//! - its velocity and speed in centimetres per simulated second ([`velocity`], [`speed`]) and the
//!   compass course it is holding ([`course`]).
//!
//! **Why segments.** Writing a mover's position every tick made a world of walking people
//! write every footstep into history — about 48 MB of chronicle per second for 50,000 movers at
//! ten ticks a second (audit §4.3). A segment is written once when a body sets out, changes
//! course, or arrives; between those moments nothing is written at all, and anyone may still ask
//! where it is *now* or where it *will* be.
//!
//! Interpolation is integer arithmetic on `i128` with round-half-away-from-zero, so every
//! consumer, on every platform, computes the same centimetre (Vol. V Ch. 4 §4.1).

use crate::schema::{MOTION_END, MOTION_START, MOTION_TARGET, POSITION, TRAVEL_TO};
use kernel::fact::{Cause, FactKey, FactType, SystemId};
use kernel::fixed::{angle_of, div_round, isqrt};
use kernel::identity::EntityId;
use kernel::proposal::{Change, Proposal};
use kernel::system::{Cadence, CommittedView, System, TickContext};
use kernel::time::SimClock;
use kernel::value::Value;

/// One straight-line leg of a body's motion, in its container's frame.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Segment {
    /// Where the body was at `start`.
    pub from: [i64; 3],
    /// Where the body will be at `end`, and stays after.
    pub to: [i64; 3],
    /// The tick the leg began.
    pub start: u64,
    /// The tick the leg ends. Never before `start`.
    pub end: u64,
}

impl Segment {
    /// The body's position on this leg at `tick`: `from` until it sets out, `to` once it has
    /// arrived, and the straight-line point between, rounded to the centimetre.
    pub fn at(&self, tick: u64) -> [i64; 3] {
        if tick <= self.start {
            return self.from;
        }
        if tick >= self.end {
            return self.to;
        }
        let done = (tick - self.start) as i128;
        let span = (self.end - self.start) as i128;
        let mut p = [0i64; 3];
        for i in 0..3 {
            let delta = self.to[i] as i128 - self.from[i] as i128;
            p[i] = (self.from[i] as i128 + div_round(delta * done, span)) as i64;
        }
        p
    }
}

/// A body's stored position (where its current segment began, or simply where it is), or the
/// container's origin if it has none.
pub fn anchor(view: &dyn CommittedView, entity: EntityId) -> [i64; 3] {
    view.read(FactKey::new(entity, POSITION))
        .and_then(|f| f.value.as_vec3())
        .unwrap_or([0; 3])
}

fn tick_fact(view: &dyn CommittedView, entity: EntityId, fact: FactType) -> Option<u64> {
    view.read(FactKey::new(entity, fact))?
        .value
        .as_int()
        .map(|t| t.max(0) as u64)
}

/// The body's current segment, if it has one. A segment whose end precedes its start is read as
/// ending when it starts — an instantaneous arrival — never as travel backwards in time.
pub fn segment(view: &dyn CommittedView, entity: EntityId) -> Option<Segment> {
    let to = view
        .read(FactKey::new(entity, MOTION_TARGET))?
        .value
        .as_vec3()?;
    let start = tick_fact(view, entity, MOTION_START)?;
    let end = tick_fact(view, entity, MOTION_END)?.max(start);
    Some(Segment {
        from: anchor(view, entity),
        to,
        start,
        end,
    })
}

/// Where `entity` is in its container's frame at `tick`.
pub fn position_at(view: &dyn CommittedView, entity: EntityId, tick: u64) -> [i64; 3] {
    match segment(view, entity) {
        Some(leg) => leg.at(tick),
        None => anchor(view, entity),
    }
}

/// Whether `entity` is travelling at the view's tick: it has set out and not yet arrived.
pub fn is_moving(view: &dyn CommittedView, entity: EntityId) -> bool {
    let now = view.tick();
    segment(view, entity).is_some_and(|leg| leg.start <= now && now < leg.end)
}

/// The tick `entity` arrives at the end of its current segment, if it has one.
pub fn arrival(view: &dyn CommittedView, entity: EntityId) -> Option<u64> {
    segment(view, entity).map(|leg| leg.end)
}

/// `entity`'s velocity at the view's tick, in centimetres per simulated second along each axis
/// of its container's frame; zero when it is not moving. `clock` converts ticks to time.
pub fn velocity(view: &dyn CommittedView, entity: EntityId, clock: &SimClock) -> [i64; 3] {
    let now = view.tick();
    match segment(view, entity) {
        Some(leg) if leg.start <= now && now < leg.end => {
            let ms = (leg.end - leg.start) as i128 * clock.tick_ms() as i128;
            let mut v = [0i64; 3];
            for i in 0..3 {
                let delta = leg.to[i] as i128 - leg.from[i] as i128;
                v[i] = div_round(delta * 1000, ms) as i64;
            }
            v
        }
        _ => [0; 3],
    }
}

/// `entity`'s speed at the view's tick, in centimetres per simulated second; zero at rest.
pub fn speed(view: &dyn CommittedView, entity: EntityId, clock: &SimClock) -> i64 {
    let v = velocity(view, entity, clock);
    let sq: i128 = v.iter().map(|c| (*c as i128).pow(2)).sum();
    isqrt(sq as u128) as i64
}

/// The compass course `entity` is holding (hundredths of a degree clockwise from its frame's
/// north), or `None` at rest or when moving straight up or down.
pub fn course(view: &dyn CommittedView, entity: EntityId) -> Option<i64> {
    let now = view.tick();
    let leg = segment(view, entity).filter(|leg| leg.start <= now && now < leg.end)?;
    compass(leg.to[0] - leg.from[0], leg.to[1] - leg.from[1])
}

/// The compass bearing of the horizontal vector `(dx, dy)` — clockwise from `+y` — or `None`
/// for the zero vector. Compass bearings and the mathematical angle differ by a quarter turn
/// and a reflection: bearing = 90° − angle.
pub fn compass(dx: i64, dy: i64) -> Option<i64> {
    angle_of(dx, dy).map(|a| (9_000 - a).rem_euclid(36_000))
}

/// The changes that send `entity` from wherever it is at the view's tick toward `target` (its
/// container's frame) at `speed_cm_s`, departing on the committed tick the view shows: the new
/// segment's start is set to where it is now, so a change of course mid-leg is seamless.
/// Arrival is rounded up to a whole tick, so a body never outruns its speed. Empty if the body
/// is already at `target` or `speed_cm_s` is not positive.
///
/// The caller wraps these in proposals under its own system id and cause — it is a builder, not
/// a system, so that whatever decides to move (Appendix A, Ruling 4) can use it.
pub fn depart(
    view: &dyn CommittedView,
    entity: EntityId,
    target: [i64; 3],
    speed_cm_s: i64,
    clock: &SimClock,
) -> Vec<(FactKey, Change)> {
    let now = view.tick();
    let here = position_at(view, entity, now);
    let dist_sq: i128 = (0..3)
        .map(|i| (target[i] as i128 - here[i] as i128).pow(2))
        .sum();
    if dist_sq == 0 || speed_cm_s <= 0 {
        return Vec::new();
    }
    let dist = isqrt(dist_sq as u128) as i128;
    // ticks = ceil(dist cm / (speed cm/s × tick s)) = ceil(dist·1000 / (speed · tick_ms)).
    let per_tick = speed_cm_s as i128 * clock.tick_ms() as i128;
    let ticks = ((dist * 1000 + per_tick - 1) / per_tick).max(1) as u64;
    vec![
        (
            FactKey::new(entity, POSITION),
            Change::Set(Value::Vec3(here)),
        ),
        (
            FactKey::new(entity, MOTION_TARGET),
            Change::Set(Value::Vec3(target)),
        ),
        (
            FactKey::new(entity, MOTION_START),
            Change::Set(Value::Int(now as i64)),
        ),
        (
            FactKey::new(entity, MOTION_END),
            Change::Set(Value::Int((now + ticks) as i64)),
        ),
    ]
}

/// The changes that end `entity`'s segment where it stands at the view's tick: its position
/// becomes the point reached, and the segment is cleared — a halt mid-leg, or the tidy close of
/// a leg already finished.
pub fn halt(view: &dyn CommittedView, entity: EntityId) -> Vec<(FactKey, Change)> {
    let here = position_at(view, entity, view.tick());
    vec![
        (
            FactKey::new(entity, POSITION),
            Change::Set(Value::Vec3(here)),
        ),
        (FactKey::new(entity, MOTION_TARGET), Change::Tombstone),
        (FactKey::new(entity, MOTION_START), Change::Tombstone),
        (FactKey::new(entity, MOTION_END), Change::Tombstone),
    ]
}

const SETTLE_READS: &[FactType] = &[POSITION, MOTION_TARGET, MOTION_START, MOTION_END, TRAVEL_TO];
const SETTLE_WRITES: &[FactType] = &[POSITION, MOTION_TARGET, MOTION_START, MOTION_END];

/// Closes finished segments: once a body has arrived, its position becomes the target and the
/// segment is cleared, so the stored position is again simply where it is (Amendment A-3).
///
/// One write per arrival, never per tick of travel. Nothing about where the body *is* changes —
/// [`position_at`] already answered the target from the arrival tick on — but a settled body is
/// cheaper to ask about (the spatial index files it at a point again, rather than along its whole
/// path) and its history records the arrival as an event.
pub struct Settle;

impl System for Settle {
    fn id(&self) -> SystemId {
        SystemId::new("physical.settle")
    }
    fn reads(&self) -> &'static [FactType] {
        SETTLE_READS
    }
    fn writes(&self) -> &'static [FactType] {
        SETTLE_WRITES
    }
    fn cadence(&self) -> Cadence {
        Cadence::EveryTick
    }
    fn evaluate(&self, view: &dyn CommittedView, ctx: &TickContext) -> Vec<Proposal> {
        let now = view.tick();
        // Read only each segment's end first: in a world of walkers almost every segment is
        // still under way, and those need nothing more read at all.
        view.entities_with(MOTION_END)
            .into_iter()
            .filter(|e| tick_fact(view, *e, MOTION_END).is_some_and(|end| end <= now))
            // A traveller's legs are closed by travel itself as it plans the next one or passes
            // through an opening (Amendment A-4); settling them here would race it.
            .filter(|e| view.read(FactKey::new(*e, TRAVEL_TO)).is_none())
            .filter(|e| segment(view, *e).is_some_and(|leg| leg.end <= now))
            .flat_map(|e| halt(view, e))
            .map(|(key, change)| {
                Proposal::new(
                    self.id(),
                    key,
                    ctx.basis_tick(),
                    change,
                    Cause::new("arrived"),
                )
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::{compass, Segment};

    #[test]
    fn a_segment_interpolates_and_is_exact_at_its_ends() {
        let leg = Segment {
            from: [0, 0, 0],
            to: [1_000, -300, 0],
            start: 10,
            end: 20,
        };
        assert_eq!(leg.at(0), [0, 0, 0], "before setting out");
        assert_eq!(leg.at(10), [0, 0, 0]);
        assert_eq!(leg.at(15), [500, -150, 0], "halfway");
        assert_eq!(leg.at(13), [300, -90, 0]);
        assert_eq!(leg.at(20), [1_000, -300, 0], "arrived");
        assert_eq!(leg.at(99), [1_000, -300, 0], "and stays");
    }

    #[test]
    fn compass_bearings_run_clockwise_from_north() {
        assert_eq!(compass(0, 10), Some(0), "north");
        assert_eq!(compass(10, 0), Some(9_000), "east");
        assert_eq!(compass(0, -10), Some(18_000), "south");
        assert_eq!(compass(-10, 0), Some(27_000), "west");
        assert_eq!(compass(10, 10), Some(4_500), "north-east");
        assert_eq!(compass(0, 0), None);
    }
}
