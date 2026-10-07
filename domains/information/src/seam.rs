//! The information layer's side of the seam (Amendment A-24): what a person crossing from the
//! city tier into the scene knows as they arrive.
//!
//! A resident knows where they are, where their home and their work are, and the ways in and out
//! of each — the openings of those places, and the openings into them from where they lie — as
//! anyone knows the street they live on. They feel their hunger and fatigue as their body has
//! them: the levels the record held (Amendment A-20). Everything else they learn by looking.

use crate::schema::{
    FELT_FATIGUE, FELT_HUNGER, LEADS_TO, OPEN, PHYS_CONTAINED_IN, PHYS_LEADS_TO, PHYS_PORTAL_OPEN,
    PLACE_OF,
};
use kernel::fact::{Cause, FactKey, FactType, SystemId};
use kernel::identity::EntityId;
use kernel::proposal::{Change, Proposal};
use kernel::system::{Cadence, CommittedView, System, TickContext};
use kernel::value::Value;
use std::collections::BTreeMap;

/// A resident crossing into the scene, and the place they cross into — **Society's**.
pub const CROSSING: FactType = FactType::new("society.resident.crossing");
/// A resident's home — **Society's**.
pub const RESIDENT_HOME: FactType = FactType::new("society.resident.home");
/// Where a resident works — **Society's**.
pub const RESIDENT_WORK: FactType = FactType::new("society.resident.work");
/// A resident's hunger, a level — **Society's**.
pub const RESIDENT_HUNGER: FactType = FactType::new("society.resident.hunger");
/// A resident's fatigue, a level — **Society's**.
pub const RESIDENT_FATIGUE: FactType = FactType::new("society.resident.fatigue");
/// The openings a place hosts — **Physical Reality's** (a set).
pub const PHYS_HAS_PORTAL: FactType = FactType::new("physical.space.has_portal");

const SEAM_READS: &[FactType] = &[
    CROSSING,
    RESIDENT_HOME,
    RESIDENT_WORK,
    RESIDENT_HUNGER,
    RESIDENT_FATIGUE,
    PHYS_CONTAINED_IN,
    PHYS_HAS_PORTAL,
    PHYS_LEADS_TO,
    PHYS_PORTAL_OPEN,
];
const SEAM_WRITES: &[FactType] = &[PLACE_OF, LEADS_TO, OPEN, FELT_HUNGER, FELT_FATIGUE];

/// Gives each person crossing into the scene what a resident knows (Amendment A-24).
pub struct Seam;

fn entity(view: &dyn CommittedView, key: FactKey) -> Option<EntityId> {
    match view.read(key)?.value {
        Value::Entity(e) => Some(e),
        _ => None,
    }
}

impl Seam {
    /// What `me`, crossing into `here`, knows: each belief once.
    fn knowledge(
        view: &dyn CommittedView,
        me: EntityId,
        here: EntityId,
    ) -> BTreeMap<FactKey, Value> {
        let mut known = BTreeMap::new();
        known.insert(FactKey::pair(me, PLACE_OF, me), Value::Entity(here));
        let openings = |place: EntityId| -> Vec<EntityId> {
            view.read_all(FactKey::new(place, PHYS_HAS_PORTAL))
                .into_iter()
                .filter_map(|f| match f.value {
                    Value::Entity(o) => Some(o),
                    _ => None,
                })
                .collect()
        };
        let mut places = vec![here];
        places.extend(entity(view, FactKey::new(me, RESIDENT_HOME)));
        places.extend(entity(view, FactKey::new(me, RESIDENT_WORK)));
        for place in places {
            let around = entity(view, FactKey::new(place, PHYS_CONTAINED_IN));
            if let Some(around) = around {
                known.insert(FactKey::pair(me, PLACE_OF, place), Value::Entity(around));
            }
            // Its own ways out, and the ways into it from where it lies.
            let mut ways = openings(place);
            ways.extend(
                around
                    .into_iter()
                    .flat_map(openings)
                    .filter(|o| entity(view, FactKey::new(*o, PHYS_LEADS_TO)) == Some(place)),
            );
            for o in ways {
                let (Some(host), Some(to)) = (
                    entity(view, FactKey::new(o, PHYS_CONTAINED_IN)),
                    entity(view, FactKey::new(o, PHYS_LEADS_TO)),
                ) else {
                    continue;
                };
                let open = view
                    .read(FactKey::new(o, PHYS_PORTAL_OPEN))
                    .map(|f| f.value)
                    != Some(Value::Bool(false));
                known.insert(FactKey::pair(me, PLACE_OF, o), Value::Entity(host));
                known.insert(FactKey::pair(me, LEADS_TO, o), Value::Entity(to));
                known.insert(FactKey::pair(me, OPEN, o), Value::Bool(open));
            }
        }
        for (record, felt) in [
            (RESIDENT_HUNGER, FELT_HUNGER),
            (RESIDENT_FATIGUE, FELT_FATIGUE),
        ] {
            if let Some(level) = view.read(FactKey::new(me, record)) {
                known.insert(FactKey::new(me, felt), level.value);
            }
        }
        known
    }
}

impl System for Seam {
    fn id(&self) -> SystemId {
        SystemId::new("information.seam")
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
        let mut out = Vec::new();
        for who in view.entities_with(CROSSING) {
            let Some(here) = entity(view, FactKey::new(who, CROSSING)) else {
                continue;
            };
            for (key, value) in Self::knowledge(view, who, here) {
                out.push(Proposal::new(
                    self.id(),
                    key,
                    ctx.basis_tick(),
                    Change::Set(value),
                    Cause::new("crossed"),
                ));
            }
        }
        out
    }
}
