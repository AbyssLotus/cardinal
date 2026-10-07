//! Crossing the seam (Vol. V Ch. 5 §5.2; Appendix A, Ruling 19; Amendment A-24): a resident who
//! stands where the scene is becomes an organism, with a body and a mind, in a single tick.
//!
//! Society marks the crossing; on the next tick every domain reads the mark from the same
//! committed record and gives the person its own facts — Physical Reality a body in the place,
//! Living Systems their hunger and fatigue, the information layer what they know, decision systems
//! a mind — while Society retires the record's state. No tick holds both.
//!
//! A resident is in the scene when the place they are at is. One walking into it is marked by the
//! rhythm as their walk ends (the agenda, Amendment A-22); everyone standing where attention
//! arrives is marked here (the residents *at* each place, Amendment A-24). Either way the cost is
//! in proportion to who crosses, not to the city.

use crate::residents::rung;
use crate::schema::{
    BODY_SIZE, CITY, CONTAINED_IN, HEADING, MOTION_END, MOTION_START, MOTION_TARGET, POSITION,
    RESIDENCY, RESIDENT_AT, RESIDENT_CROSSING, RESIDENT_DOING, RESIDENT_FATIGUE, RESIDENT_HOME,
    RESIDENT_HUNGER, RESIDENT_UNTIL, SCENE, WALKING,
};
use kernel::fact::{Cause, FactKey, FactType, SystemId};
use kernel::identity::EntityId;
use kernel::proposal::{Change, Proposal};
use kernel::system::{Cadence, CommittedView, System, TickContext};
use kernel::value::Value;
use std::collections::{BTreeMap, BTreeSet};

const CROSSING_READS: &[FactType] = &[
    RESIDENT_HOME,
    RESIDENT_AT,
    RESIDENT_DOING,
    RESIDENT_UNTIL,
    RESIDENT_HUNGER,
    RESIDENT_FATIGUE,
    RESIDENT_CROSSING,
    RESIDENCY,
    CONTAINED_IN,
    POSITION,
    BODY_SIZE,
    HEADING,
    MOTION_TARGET,
    MOTION_START,
    MOTION_END,
];
const CROSSING_WRITES: &[FactType] = &[
    RESIDENT_DOING,
    RESIDENT_AT,
    RESIDENT_UNTIL,
    RESIDENT_HUNGER,
    RESIDENT_FATIGUE,
    RESIDENT_CROSSING,
];

/// What a resident record holds besides who they are: retired when they cross. Their home and
/// work stay, because they are who the person is at every tier.
const STATE: &[FactType] = &[
    RESIDENT_DOING,
    RESIDENT_AT,
    RESIDENT_UNTIL,
    RESIDENT_HUNGER,
    RESIDENT_FATIGUE,
    RESIDENT_CROSSING,
];

/// Society's side of the seam (Amendment A-24), every tick:
///
/// - **retire** the record's state for everyone marked crossing in the last tick — the same tick
///   every other domain gives them its facts;
/// - **mark** every resident not walking who is at a place attention has just brought into the
///   scene. (One walking there crosses on arriving: the rhythm marks them.)
pub struct Crossing;

fn entity(view: &dyn CommittedView, key: FactKey) -> Option<EntityId> {
    match view.read(key)?.value {
        Value::Entity(e) => Some(e),
        _ => None,
    }
}

impl Crossing {
    /// Every resident at a place in the scene who is not walking, by the place they are at —
    /// within `places` (each with what lies within it), or, when the store cannot say what
    /// changed, the whole city.
    fn standing_in_scene(
        view: &dyn CommittedView,
        places: Option<Vec<EntityId>>,
        rungs: &mut BTreeMap<EntityId, i64>,
        out: &mut BTreeMap<EntityId, EntityId>,
    ) {
        let walking = |who: EntityId| {
            view.read(FactKey::new(who, RESIDENT_DOING))
                .and_then(|f| f.value.as_int())
                == Some(WALKING)
        };
        // Without a spatial index nothing says what lies within a place: look at everyone.
        let index = view.spatial();
        let Some(places) = places.filter(|_| index.is_some()) else {
            for who in view.entities_with(RESIDENT_AT) {
                if let Some(at) = entity(view, FactKey::new(who, RESIDENT_AT)) {
                    if !walking(who) && rung(view, at, rungs) == SCENE {
                        out.insert(who, at);
                    }
                }
            }
            return;
        };
        let mut seen = BTreeSet::new();
        let mut stack: Vec<EntityId> = places
            .into_iter()
            .filter(|p| rung(view, *p, rungs) == SCENE)
            .collect();
        while let Some(place) = stack.pop() {
            if !seen.insert(place) {
                continue;
            }
            for who in view.holding(RESIDENT_AT, Value::Entity(place)) {
                if !walking(who) {
                    out.insert(who, place);
                }
            }
            // Down into what lies within it, but not into a place that keeps the city tier.
            let within = index.map(|ix| ix.children(place)).unwrap_or_default();
            for child in within {
                let declared = view
                    .read(FactKey::new(child, RESIDENCY))
                    .and_then(|f| f.value.as_int());
                if declared != Some(CITY) {
                    stack.push(child);
                }
            }
        }
    }
}

impl System for Crossing {
    fn id(&self) -> SystemId {
        SystemId::new("society.crossing")
    }
    fn reads(&self) -> &'static [FactType] {
        CROSSING_READS
    }
    fn writes(&self) -> &'static [FactType] {
        CROSSING_WRITES
    }
    fn cadence(&self) -> Cadence {
        Cadence::EveryTick
    }
    fn evaluate(&self, view: &dyn CommittedView, ctx: &TickContext) -> Vec<Proposal> {
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
        // Retire the records of those who crossed: every other domain takes them up this tick.
        let crossing = view.entities_with(RESIDENT_CROSSING);
        for &who in &crossing {
            for &fact in STATE {
                if view.read(FactKey::new(who, fact)).is_some() {
                    push(FactKey::new(who, fact), Change::Tombstone, "crossed");
                }
            }
        }

        // Attention arriving: the places whose rung was set in the last tick. Seeds are not
        // changes, so the world as loaded has none (its residents are validated to be at the
        // city tier).
        let mut rungs = BTreeMap::new();
        let mut entering: BTreeMap<EntityId, EntityId> = BTreeMap::new();
        let basis = ctx.basis_tick();
        if basis > 0 {
            let changed = view.changed_since(RESIDENCY, basis - 1);
            if changed.as_ref().map_or(true, |c| !c.is_empty()) {
                Self::standing_in_scene(view, changed, &mut rungs, &mut entering);
            }
        }
        for (who, place) in entering {
            if crossing.binary_search(&who).is_err() {
                push(
                    FactKey::new(who, RESIDENT_CROSSING),
                    Change::Set(Value::Entity(place)),
                    "attended",
                );
            }
        }
        out
    }
}
