//! Decision systems' side of the seam (Amendment A-24; Appendix A, Ruling 14 as amended): a person
//! crossing from the city tier into the scene is given a mind.
//!
//! A mind decides from beliefs alone. This is not deciding: it is making the mind, and a mind is
//! made from who the person was — the record they cross from, read by its published ids. They walk
//! at the pace residents walk; they keep their things at home; one who was asleep is lying down;
//! and one with work keeps their own hours, the same hours the city tier gave them (each
//! resident's, a fixed offset from the settlement's, Amendment A-23), going to work for them and
//! home after.

use crate::schema::{HOME, RESTING, ROUTINE, WALK_SPEED};
use kernel::fact::{Cause, FactKey, FactType, SystemId};
use kernel::identity::EntityId;
use kernel::proposal::{Change, Proposal};
use kernel::system::{Cadence, CommittedView, System, TickContext};
use kernel::value::Value;

/// A resident crossing into the scene — **Society's**.
pub const CROSSING: FactType = FactType::new("society.resident.crossing");
/// A resident's home — **Society's**.
pub const RESIDENT_HOME: FactType = FactType::new("society.resident.home");
/// Where a resident works — **Society's**.
pub const RESIDENT_WORK: FactType = FactType::new("society.resident.work");
/// What a resident is doing — **Society's**.
pub const RESIDENT_DOING: FactType = FactType::new("society.resident.doing");
/// Asleep, in Society's record.
pub const SLEEPING: i64 = 1;

/// The record the seam reads: Society's, and nothing else.
pub const SEAM_READS: &[FactType] = &[CROSSING, RESIDENT_HOME, RESIDENT_WORK, RESIDENT_DOING];
const SEAM_WRITES: &[FactType] = &[WALK_SPEED, HOME, RESTING, ROUTINE];

/// How a resident's mind is made, all from the world package (Vol. IV Ch. 2 §2.2).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ResidentMind {
    /// How fast residents walk, in centimetres a second.
    pub walk_speed_cm_s: i64,
    /// When the settlement's working hours begin and end, in seconds into the day.
    pub work_from_seconds: i64,
    /// When they end.
    pub work_to_seconds: i64,
    /// How far, in seconds either side, a resident's own hours may lie from the settlement's.
    pub hours_spread_seconds: i64,
}

/// Gives each person crossing into the scene a mind (Amendment A-24).
pub struct Seam {
    day_seconds: i64,
    rules: ResidentMind,
}

impl Seam {
    /// Minds by `rules`, in a world whose day is `day_seconds` long.
    pub const fn new(day_seconds: i64, rules: ResidentMind) -> Self {
        Self { day_seconds, rules }
    }
}

fn entity(view: &dyn CommittedView, key: FactKey) -> Option<EntityId> {
    match view.read(key)?.value {
        Value::Entity(e) => Some(e),
        _ => None,
    }
}

impl System for Seam {
    fn id(&self) -> SystemId {
        SystemId::new("minds.seam")
    }
    fn reads(&self) -> &'static [FactType] {
        SEAM_READS
    }
    fn writes(&self) -> &'static [FactType] {
        SEAM_WRITES
    }
    fn cadence(&self) -> Cadence {
        Cadence::EveryTick
    }
    fn evaluate(&self, view: &dyn CommittedView, ctx: &TickContext) -> Vec<Proposal> {
        let r = &self.rules;
        let day = self.day_seconds.max(1);
        let mut out = Vec::new();
        for who in view.entities_with(CROSSING) {
            let mut push = |key: FactKey, value: Value| {
                out.push(Proposal::new(
                    self.id(),
                    key,
                    ctx.basis_tick(),
                    Change::Set(value),
                    Cause::new("crossed"),
                ));
            };
            push(FactKey::new(who, WALK_SPEED), Value::Int(r.walk_speed_cm_s));
            let home = entity(view, FactKey::new(who, RESIDENT_HOME));
            if let Some(home) = home {
                push(FactKey::new(who, HOME), Value::Entity(home));
            }
            let doing = view
                .read(FactKey::new(who, RESIDENT_DOING))
                .and_then(|f| f.value.as_int());
            if doing == Some(SLEEPING) {
                push(FactKey::new(who, RESTING), Value::Bool(true));
            }
            if let (Some(work), Some(home)) = (entity(view, FactKey::new(who, RESIDENT_WORK)), home)
            {
                let own = |s: i64| (s + who.jitter(r.hours_spread_seconds)).rem_euclid(day);
                let (from, to) = (own(r.work_from_seconds), own(r.work_to_seconds));
                push(
                    FactKey::pair(who, ROUTINE, work),
                    Value::Vec3([from, to, 0]),
                );
                push(
                    FactKey::pair(who, ROUTINE, home),
                    Value::Vec3([to, from, 0]),
                );
            }
        }
        out
    }
}
