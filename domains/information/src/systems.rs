//! Perception (Vol. II Ch. 4 *Observation*; Vol. I Ch. 4 §10.1; Amendment A-8): reality, as
//! Physical Reality and Living Systems publish it, turned into what one mind observes, believes,
//! and remembers.

use crate::schema::{
    BLOCKED, FELT_BODY_HEAT, GOING_TO, IN_SIGHT, LEADS_TO, LIVING_BODY_HEAT, LIVING_SIGHT_RANGE,
    OPEN, PHYS_ACT_REFUSED, PHYS_CONTAINED_IN, PHYS_IN_VIEW, PHYS_LEADS_TO, PHYS_PORTAL_OPEN,
    PHYS_TEMPERATURE, PHYS_TRAVEL_BLOCKED, PHYS_TRAVEL_TO, PLACE_OF, REFUSED, WARMTH_OF,
};
use kernel::fact::{Cause, FactKey, FactType, SystemId};
use kernel::hierarchy::ancestry;
use kernel::identity::EntityId;
use kernel::proposal::{Change, Proposal};
use kernel::system::{Cadence, CommittedView, System, TickContext};
use kernel::time::Step;
use kernel::value::Value;
use std::collections::BTreeSet;

// Perception reads what is in view and the facts it reports about what is seen, the air and
// the body it feels, the body's own intent and reports — and its own beliefs, to write only
// what changed.
const PERCEPTION_READS: &[FactType] = &[
    LIVING_SIGHT_RANGE,
    PHYS_IN_VIEW,
    PHYS_CONTAINED_IN,
    PHYS_LEADS_TO,
    PHYS_PORTAL_OPEN,
    PHYS_TEMPERATURE,
    PHYS_TRAVEL_TO,
    PHYS_TRAVEL_BLOCKED,
    PHYS_ACT_REFUSED,
    LIVING_BODY_HEAT,
    IN_SIGHT,
    PLACE_OF,
    LEADS_TO,
    OPEN,
    WARMTH_OF,
    FELT_BODY_HEAT,
    GOING_TO,
    BLOCKED,
    REFUSED,
];
const PERCEPTION_WRITES: &[FactType] = &[
    IN_SIGHT,
    PLACE_OF,
    LEADS_TO,
    OPEN,
    WARMTH_OF,
    FELT_BODY_HEAT,
    GOING_TO,
    BLOCKED,
    REFUSED,
];

/// Every mind with senses perceives, each step:
///
/// - **itself** — where it stands, how warm the air is there, how warm its body feels, where it
///   is trying to go, whether its way is blocked, and what it last could not do;
/// - **what is in view** — where each thing it sees is (stamped *seen* as it comes into sight),
///   and for an opening, where it leads and whether it is open;
/// - **what it lost sight of** — the belief about it is stamped with this moment, the last time
///   it was seen, and then left alone: memory.
///
/// A belief is written only when it changes (or, losing sight, once to stamp it), so a still
/// world writes nothing. Warmth, felt or remembered, is only noticed in steps of the world's
/// declared resolution.
pub struct Perception {
    step: Step,
    warmth_resolution: i64,
}

impl Perception {
    /// Perceive as `step` says, noticing changes of warmth of at least `warmth_resolution`
    /// centidegrees.
    pub const fn new(step: Step, warmth_resolution: i64) -> Self {
        Self {
            step,
            warmth_resolution,
        }
    }
}

fn int(view: &dyn CommittedView, key: FactKey) -> Option<i64> {
    view.read(key).and_then(|f| f.value.as_int())
}

fn value(view: &dyn CommittedView, key: FactKey) -> Option<Value> {
    view.read(key).map(|f| f.value)
}

fn entities(view: &dyn CommittedView, key: FactKey) -> BTreeSet<EntityId> {
    view.read_all(key)
        .into_iter()
        .filter_map(|f| match f.value {
            Value::Entity(e) => Some(e),
            _ => None,
        })
        .collect()
}

/// The air temperature where `entity` stands: its place's, or the nearest enclosing place's.
fn air_around(view: &dyn CommittedView, entity: EntityId) -> Option<i64> {
    ancestry(view, entity, PHYS_CONTAINED_IN)
        .into_iter()
        .skip(1)
        .find_map(|place| int(view, FactKey::new(place, PHYS_TEMPERATURE)))
}

impl Perception {
    fn propose(
        &self,
        ctx: &TickContext,
        key: FactKey,
        change: Change,
        why: &'static str,
    ) -> Proposal {
        Proposal::new(
            SystemId::new("information.perception"),
            key,
            ctx.basis_tick(),
            change,
            Cause::new(why),
        )
    }

    /// Write `now` at `key` if it differs from what is held (removing it when `now` is `None`).
    fn mirror(
        &self,
        view: &dyn CommittedView,
        ctx: &TickContext,
        out: &mut Vec<Proposal>,
        key: FactKey,
        now: Option<Value>,
        why: &'static str,
    ) {
        match (value(view, key), now) {
            (held, Some(v)) if held != Some(v) => {
                out.push(self.propose(ctx, key, Change::Set(v), why));
            }
            (Some(_), None) => out.push(self.propose(ctx, key, Change::Tombstone, why)),
            _ => {}
        }
    }

    /// Write a warmth at `key` if none is held or it has moved by the world's resolution.
    fn feel(
        &self,
        view: &dyn CommittedView,
        ctx: &TickContext,
        out: &mut Vec<Proposal>,
        key: FactKey,
        now: i64,
    ) {
        let changed =
            int(view, key).map_or(true, |held| (now - held).abs() >= self.warmth_resolution);
        if changed {
            out.push(self.propose(ctx, key, Change::Set(Value::Int(now)), "felt"));
        }
    }

    fn perceive(
        &self,
        view: &dyn CommittedView,
        ctx: &TickContext,
        mind: EntityId,
        out: &mut Vec<Proposal>,
    ) {
        // Oneself: where one stands, the air there, one's body, one's own intent and reports.
        let here = value(view, FactKey::new(mind, PHYS_CONTAINED_IN));
        self.mirror(
            view,
            ctx,
            out,
            FactKey::pair(mind, PLACE_OF, mind),
            here,
            "felt",
        );
        if let (Some(Value::Entity(place)), Some(air)) = (here, air_around(view, mind)) {
            self.feel(view, ctx, out, FactKey::pair(mind, WARMTH_OF, place), air);
        }
        if let Some(body) = int(view, FactKey::new(mind, LIVING_BODY_HEAT)) {
            self.feel(view, ctx, out, FactKey::new(mind, FELT_BODY_HEAT), body);
        }
        for (theirs, mine) in [
            (PHYS_TRAVEL_TO, GOING_TO),
            (PHYS_TRAVEL_BLOCKED, BLOCKED),
            (PHYS_ACT_REFUSED, REFUSED),
        ] {
            let now = value(view, FactKey::new(mind, theirs));
            self.mirror(view, ctx, out, FactKey::new(mind, mine), now, "felt");
        }

        // What is in view now, against what the mind was watching.
        let now = entities(view, FactKey::new(mind, PHYS_IN_VIEW));
        let before = entities(view, FactKey::new(mind, IN_SIGHT));
        for &seen in now.iter().filter(|e| **e != mind) {
            let place = value(view, FactKey::new(seen, PHYS_CONTAINED_IN));
            let key = FactKey::pair(mind, PLACE_OF, seen);
            match place {
                // Come into sight: seen now, whether or not it is where it was last seen.
                Some(place) if !before.contains(&seen) => {
                    out.push(self.propose(ctx, key, Change::Set(place), "seen"));
                }
                Some(_) => self.mirror(view, ctx, out, key, place, "seen"),
                None => {}
            }
            if let Some(leads) = value(view, FactKey::new(seen, PHYS_LEADS_TO)) {
                self.mirror(
                    view,
                    ctx,
                    out,
                    FactKey::pair(mind, LEADS_TO, seen),
                    Some(leads),
                    "seen",
                );
                let open = !matches!(
                    value(view, FactKey::new(seen, PHYS_PORTAL_OPEN)),
                    Some(Value::Bool(false))
                );
                self.mirror(
                    view,
                    ctx,
                    out,
                    FactKey::pair(mind, OPEN, seen),
                    Some(Value::Bool(open)),
                    "seen",
                );
            }
        }
        // Lost sight of: stamp the belief with the last moment it was seen — then leave it be.
        for &gone in before.difference(&now) {
            let key = FactKey::pair(mind, PLACE_OF, gone);
            if let Some(last) = value(view, key) {
                out.push(self.propose(ctx, key, Change::Set(last), "lost_sight"));
            }
        }
        let sight = FactKey::new(mind, IN_SIGHT);
        for &came in now.difference(&before) {
            out.push(self.propose(ctx, sight, Change::Add(Value::Entity(came)), "seen"));
        }
        for &went in before.difference(&now) {
            out.push(self.propose(
                ctx,
                sight,
                Change::Remove(Value::Entity(went)),
                "lost_sight",
            ));
        }
    }
}

impl System for Perception {
    fn id(&self) -> SystemId {
        SystemId::new("information.perception")
    }
    fn reads(&self) -> &'static [FactType] {
        PERCEPTION_READS
    }
    fn writes(&self) -> &'static [FactType] {
        PERCEPTION_WRITES
    }
    fn cadence(&self) -> Cadence {
        self.step.cadence()
    }
    fn evaluate(&self, view: &dyn CommittedView, ctx: &TickContext) -> Vec<Proposal> {
        // The minds: everything with senses (Living Systems' sight range) — and anything still
        // watching something, so a mind that loses its senses also loses its view.
        let mut minds: BTreeSet<EntityId> =
            view.entities_with(LIVING_SIGHT_RANGE).into_iter().collect();
        minds.extend(view.entities_with(IN_SIGHT));
        let mut out = Vec::new();
        for mind in minds {
            self.perceive(view, ctx, mind, &mut out);
        }
        out
    }
}
