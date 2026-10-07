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
    ACT_ARRIVE, ACT_CLOSE, ACT_CONSUME, ACT_DROP, ACT_FACE, ACT_LEAVE, ACT_OPEN, ACT_PICK,
    ACT_REFUSED, ACT_TAKE, BODY_SIZE, CONSUMED, CONTAINED_IN, ENCLOSED, HEADING, LEADS_TO, MADE_OF,
    MATERIAL_DENSITY, MATERIAL_EDIBLE, MOBILE, MOTION_END, MOTION_START, MOTION_TARGET, PICKED,
    PORTAL_FAR_SIDE, PORTAL_OPEN, POSITION, SOLID,
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

/// How far in front of a fixture a thing arriving at it is set down, beyond both their edges, cm.
const ARRIVAL_GAP_CM: i64 = 5;

const ACT_READS: &[FactType] = &[
    SOLID,
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
    ACT_TAKE,
    ACT_DROP,
    ACT_CONSUME,
    ACT_ARRIVE,
    ACT_LEAVE,
    ACT_PICK,
    ENCLOSED,
    MADE_OF,
    MATERIAL_DENSITY,
    MATERIAL_EDIBLE,
];
const ACT_WRITES: &[FactType] = &[
    ACT_OPEN,
    ACT_CLOSE,
    ACT_FACE,
    ACT_REFUSED,
    PORTAL_OPEN,
    HEADING,
    ACT_TAKE,
    ACT_DROP,
    ACT_CONSUME,
    CONTAINED_IN,
    POSITION,
    CONSUMED,
    ACT_ARRIVE,
    ACT_LEAVE,
    ACT_PICK,
    PICKED,
];

/// Carries out open, shut, and face intents (Amendment A-5), and take, drop, and consume
/// (Amendment A-12; Appendix A, Ruling 16):
///
/// - **take** — a thing in the body's place, within its reach, not moving, not an opening or a
///   walled place, whose weight is known and no more than the world's carrying limit, and not
///   taken by anyone else this tick (the lower id first). It is then held: contained in the
///   body, at its centre;
/// - **drop** — a thing the body holds, put down where the body stands;
/// - **consume** — a thing the body holds, made entirely of edible materials. It leaves the
///   world (its place is cleared; its identity and composition remain), and the body's
///   [`CONSUMED`] report names it.
pub struct Act {
    reach_cm: i64,
    carry_limit_g: i64,
}

impl Act {
    /// Acts within `reach_cm` beyond a body's own extent, and carries no more than
    /// `carry_limit_kg` (world rules).
    pub const fn new(reach_cm: i64, carry_limit_kg: i64) -> Self {
        Self {
            reach_cm,
            carry_limit_g: carry_limit_kg.saturating_mul(1000),
        }
    }

    /// Why `body` cannot take `thing` now, or `None` if it can.
    fn cannot_take(&self, view: &dyn CommittedView, body: EntityId, thing: EntityId) -> Option<()> {
        let unfit = thing == body
            || view.read(FactKey::new(thing, LEADS_TO)).is_some()
            || is_true(view, thing, ENCLOSED)
            || view.read(FactKey::new(thing, MOTION_TARGET)).is_some()
            || !self.within_reach(view, body, thing);
        let bearable =
            crate::materials::weight_g(view, thing).is_some_and(|g| g <= self.carry_limit_g);
        (unfit || !bearable).then_some(())
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
        for fact in [
            ACT_OPEN,
            ACT_CLOSE,
            ACT_FACE,
            ACT_TAKE,
            ACT_DROP,
            ACT_CONSUME,
            ACT_PICK,
        ] {
            actors.extend(view.entities_with(fact));
        }
        actors.sort_unstable();
        actors.dedup();
        // Door faces already set this tick, and to what; things already handled this tick.
        let mut doors: BTreeMap<EntityId, bool> = BTreeMap::new();
        let mut handled: std::collections::BTreeSet<EntityId> = Default::default();
        // Things coming into the world, and leaving it (Amendment A-15): into a place, or a hand;
        // or, asked to arrive at a fixture — a hearth — set down just in front of it, in its
        // place, where one can walk up to it (Amendment A-18).
        for thing in view.entities_with(ACT_ARRIVE) {
            let Some(at) = entity_at(view, thing, ACT_ARRIVE) else {
                continue;
            };
            push(thing, ACT_ARRIVE, Change::Tombstone, "considered");
            let (into, position) = match container_of(view, at) {
                Some(place) if is_true(view, at, SOLID) => {
                    let [x, y, z] = local_position(view, at);
                    let depth = size_of(view, at).map_or(0, |s| s[1]);
                    let own = size_of(view, thing).map_or(0, |s| s[1]);
                    (place, [x, y - depth - own - ARRIVAL_GAP_CM, z])
                }
                _ => (at, [0, 0, 0]),
            };
            if handled.insert(thing) {
                push(
                    thing,
                    CONTAINED_IN,
                    Change::Set(Value::Entity(into)),
                    "arrived",
                );
                push(
                    thing,
                    POSITION,
                    Change::Set(Value::Vec3(position)),
                    "arrived",
                );
            }
        }
        for thing in view.entities_with(ACT_LEAVE) {
            push(thing, ACT_LEAVE, Change::Tombstone, "considered");
            if handled.insert(thing) && container_of(view, thing).is_some() {
                push(thing, CONTAINED_IN, Change::Tombstone, "used_up");
                push(thing, POSITION, Change::Tombstone, "used_up");
            }
        }
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
            // Handling things (Amendment A-12).
            if let Some(thing) = entity_at(view, body, ACT_TAKE) {
                push(body, ACT_TAKE, Change::Tombstone, "considered");
                if self.cannot_take(view, body, thing).is_some() || !handled.insert(thing) {
                    refused = Some(thing);
                } else {
                    push(
                        thing,
                        CONTAINED_IN,
                        Change::Set(Value::Entity(body)),
                        "taken",
                    );
                    push(
                        thing,
                        POSITION,
                        Change::Set(Value::Vec3([0, 0, 0])),
                        "taken",
                    );
                    succeeded = true;
                }
            }
            if let Some(thing) = entity_at(view, body, ACT_DROP) {
                push(body, ACT_DROP, Change::Tombstone, "considered");
                let held = container_of(view, thing) == Some(body);
                match container_of(view, body).filter(|_| held && handled.insert(thing)) {
                    Some(place) => {
                        let at = local_position(view, body);
                        push(
                            thing,
                            CONTAINED_IN,
                            Change::Set(Value::Entity(place)),
                            "dropped",
                        );
                        push(thing, POSITION, Change::Set(Value::Vec3(at)), "dropped");
                        succeeded = true;
                    }
                    None => refused = Some(thing),
                }
            }
            if let Some(thing) = entity_at(view, body, ACT_CONSUME) {
                push(body, ACT_CONSUME, Change::Tombstone, "considered");
                let held = container_of(view, thing) == Some(body);
                if held && crate::materials::is_edible(view, thing) && handled.insert(thing) {
                    push(thing, CONTAINED_IN, Change::Tombstone, "consumed");
                    push(thing, POSITION, Change::Tombstone, "consumed");
                    push(
                        body,
                        CONSUMED,
                        Change::Set(Value::Entity(thing)),
                        "consumed",
                    );
                    succeeded = true;
                } else {
                    refused = Some(thing);
                }
            }
            if let Some(deposit) = entity_at(view, body, ACT_PICK) {
                push(body, ACT_PICK, Change::Tombstone, "considered");
                if self.within_reach(view, body, deposit) {
                    push(body, PICKED, Change::Set(Value::Entity(deposit)), "picked");
                    succeeded = true;
                } else {
                    refused = Some(deposit);
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
