//! Operating and turning: the intents besides travel (Appendix A, Ruling 13 as amended by A-5).
//!
//! A decider proposes that a body open a door ([`ACT_OPEN`]), shut one ([`ACT_CLOSE`]), or face a
//! new way ([`ACT_FACE`]). Door state and facing are restricted facts — only Physical Reality's
//! own systems write them — so these intents are the only way anything else can change them, and
//! [`Act`] carries them out only when the world allows:
//!
//! - **A door within reach.** The body must stand in the same region as one face of the opening,
//!   no farther from it than its own half-width, the opening's half-width, and the world's
//!   `reach_cm` together. Both faces of the opening then open or shut together.
//! - **A body that can turn.** Only a mobile body turns on request; a wall does not.
//!
//! Every intent is cleared the tick it is considered. One that could not be carried out is
//! recorded in [`ACT_REFUSED`] (naming the door, or the body itself for a turn), so the decider
//! can notice and reconsider; the body's next successful act clears it. When two bodies would
//! set one door both ways in the same tick, the lower id acts and the other is refused — door
//! state never receives contradictory writes.

use crate::index::{container_of, size_of};
use crate::schema::{
    ACT_CLOSE, ACT_FACE, ACT_OPEN, ACT_REFUSED, BODY_SIZE, CONTAINED_IN, HEADING, LEADS_TO, MOBILE,
    MOTION_END, MOTION_START, MOTION_TARGET, PORTAL_FAR_SIDE, PORTAL_OPEN, POSITION,
};
use crate::space::{far_side, local_position, portal_destination};
use crate::terrain::is_true;
use kernel::fact::{Cause, FactKey, FactType, SystemId};
use kernel::fixed::isqrt;
use kernel::identity::EntityId;
use kernel::proposal::{Change, Proposal};
use kernel::system::{Cadence, CommittedView, System, TickContext};
use kernel::value::Value;
use std::collections::BTreeMap;

const ACT_READS: &[FactType] = &[
    ACT_OPEN,
    ACT_CLOSE,
    ACT_FACE,
    ACT_REFUSED,
    MOBILE,
    CONTAINED_IN,
    POSITION,
    BODY_SIZE,
    HEADING,
    MOTION_TARGET,
    MOTION_START,
    MOTION_END,
    LEADS_TO,
    PORTAL_FAR_SIDE,
    PORTAL_OPEN,
];
const ACT_WRITES: &[FactType] = &[
    ACT_OPEN,
    ACT_CLOSE,
    ACT_FACE,
    ACT_REFUSED,
    PORTAL_OPEN,
    HEADING,
];

/// Carries out open, shut, and face intents (Amendment A-5).
pub struct Act {
    reach_cm: i64,
}

impl Act {
    /// Acts within `reach_cm` beyond a body's own extent (a world rule).
    pub const fn new(reach_cm: i64) -> Self {
        Self { reach_cm }
    }

    /// Whether `body` can reach door face `portal`: same region, and close enough.
    fn within_reach(&self, view: &dyn CommittedView, body: EntityId, portal: EntityId) -> bool {
        if container_of(view, body).is_none()
            || container_of(view, body) != container_of(view, portal)
        {
            return false;
        }
        let (a, b) = (local_position(view, body), local_position(view, portal));
        let (dx, dy) = ((a[0] - b[0]) as i128, (a[1] - b[1]) as i128);
        let gap = isqrt((dx * dx + dy * dy) as u128) as i64;
        let half = |e| size_of(view, e).map_or(0, |s| s[0].max(s[1]));
        gap <= half(body) + half(portal) + self.reach_cm.max(0)
    }

    /// The face of `door` the body can reach, if any: the door itself or its far side.
    fn reachable_face(
        &self,
        view: &dyn CommittedView,
        body: EntityId,
        door: EntityId,
    ) -> Option<EntityId> {
        if self.within_reach(view, body, door) {
            return Some(door);
        }
        let dest = portal_destination(view, door)?;
        far_side(view, door, dest).filter(|q| self.within_reach(view, body, *q))
    }
}

fn entity_at(view: &dyn CommittedView, e: EntityId, fact: FactType) -> Option<EntityId> {
    match view.read(FactKey::new(e, fact))?.value {
        Value::Entity(x) => Some(x),
        _ => None,
    }
}

impl System for Act {
    fn id(&self) -> SystemId {
        SystemId::new("physical.act")
    }
    fn reads(&self) -> &'static [FactType] {
        ACT_READS
    }
    fn writes(&self) -> &'static [FactType] {
        ACT_WRITES
    }
    fn cadence(&self) -> Cadence {
        Cadence::EveryTick
    }
    fn evaluate(&self, view: &dyn CommittedView, ctx: &TickContext) -> Vec<Proposal> {
        let mut out = Vec::new();
        let mut push = |e: EntityId, fact: FactType, change: Change, cause: &'static str| {
            out.push(Proposal::new(
                self.id(),
                FactKey::new(e, fact),
                ctx.basis_tick(),
                change,
                Cause::new(cause),
            ));
        };
        // Every body with any intent, in ascending id: the lower id acts first on a shared door.
        let mut actors: Vec<EntityId> = Vec::new();
        for fact in [ACT_OPEN, ACT_CLOSE, ACT_FACE] {
            actors.extend(view.entities_with(fact));
        }
        actors.sort_unstable();
        actors.dedup();
        // Door faces already set this tick, and to what.
        let mut doors: BTreeMap<EntityId, bool> = BTreeMap::new();
        for body in actors {
            let mut refused: Option<EntityId> = None;
            let mut succeeded = false;
            for (fact, open) in [(ACT_OPEN, true), (ACT_CLOSE, false)] {
                let Some(door) = entity_at(view, body, fact) else {
                    continue;
                };
                push(body, fact, Change::Tombstone, "considered");
                let faces = match self.reachable_face(view, body, door) {
                    Some(_) => {
                        let mut faces = vec![door];
                        if let Some(dest) = portal_destination(view, door) {
                            faces.extend(far_side(view, door, dest));
                        }
                        faces
                    }
                    None => {
                        refused = Some(door);
                        continue;
                    }
                };
                if faces
                    .iter()
                    .any(|f| doors.get(f).is_some_and(|v| *v != open))
                {
                    refused = Some(door); // someone else set it the other way this tick
                    continue;
                }
                for f in faces {
                    if doors.insert(f, open).is_none() {
                        push(
                            f,
                            PORTAL_OPEN,
                            Change::Set(Value::Bool(open)),
                            if open { "opened" } else { "shut" },
                        );
                    }
                }
                succeeded = true;
            }
            if let Some(Value::Int(bearing)) =
                view.read(FactKey::new(body, ACT_FACE)).map(|f| f.value)
            {
                push(body, ACT_FACE, Change::Tombstone, "considered");
                if is_true(view, body, MOBILE) {
                    push(
                        body,
                        HEADING,
                        Change::Set(Value::Int(bearing.rem_euclid(36_000))),
                        "turned",
                    );
                    succeeded = true;
                } else {
                    refused = Some(body);
                }
            }
            match refused {
                Some(what) => push(
                    body,
                    ACT_REFUSED,
                    Change::Set(Value::Entity(what)),
                    "refused",
                ),
                None if succeeded && view.read(FactKey::new(body, ACT_REFUSED)).is_some() => {
                    push(body, ACT_REFUSED, Change::Tombstone, "done")
                }
                None => {}
            }
        }
        out
    }
}
