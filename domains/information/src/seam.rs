//! The information layer's side of the seam (Amendment A-24): what a person crossing from the
//! city tier into the scene knows as they arrive.
//!
//! A resident knows where they are, where their home and their work are, and the ways in and out
//! of each — the openings of those places, and of every place holding them, between it and the
//! next out — as anyone knows the streets between home and work. They feel their hunger and
//! fatigue as their body has them: the levels the record held (Amendment A-20). Everything else
//! they learn by looking.
//!
//! A person leaving the scene (Amendment A-25) is retired from the information layer's keeping
//! the tick after: the city tier keeps no memory, so what they learned in the scene ends at the
//! seam. What others believe of them stays theirs.

use crate::schema::{
    FELT_FATIGUE, FELT_HUNGER, LEADS_TO, NUTRITION, OPEN, OWNER, PHYS_CONTAINED_IN, PHYS_LEADS_TO,
    PHYS_NUTRITION, PHYS_PORTAL_OPEN, PLACE_OF, RES_STOCK, STOCK, YIELDS,
};
use kernel::fact::{Cause, FactKey, FactType, SystemId};
use kernel::identity::EntityId;
use kernel::proposal::{Change, Proposal};
use kernel::system::{Cadence, CommittedView, System, TickContext};
use kernel::value::Value;
use std::collections::BTreeMap;

/// A resident crossing into the scene, and the place they cross into — **Society's**.
pub const CROSSING: FactType = FactType::new("society.resident.crossing");
/// A resident leaving the scene — **Society's**.
pub const LEAVING: FactType = FactType::new("society.resident.leaving");
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
/// A household's pantry, on its home — **Society's** (Amendment A-26).
pub const PANTRY: FactType = FactType::new("society.household.pantry");
/// How many meals a household's larder holds, on its home — **Society's**.
pub const LARDER: FactType = FactType::new("society.household.larder");
/// One whole unit of a deposit's stock, in its hundredths — **Resources'**.
const UNIT: i64 = 100;

const SEAM_READS: &[FactType] = &[
    CROSSING,
    LEAVING,
    RESIDENT_HOME,
    RESIDENT_WORK,
    RESIDENT_HUNGER,
    RESIDENT_FATIGUE,
    PHYS_CONTAINED_IN,
    PHYS_HAS_PORTAL,
    PHYS_LEADS_TO,
    PHYS_PORTAL_OPEN,
    PHYS_NUTRITION,
    PANTRY,
    LARDER,
    RES_STOCK,
];
const SEAM_WRITES: &[FactType] = &[
    PLACE_OF,
    LEADS_TO,
    OPEN,
    FELT_HUNGER,
    FELT_FATIGUE,
    YIELDS,
    STOCK,
    NUTRITION,
    OWNER,
];

/// Gives each person crossing into the scene what a resident knows (Amendment A-24) — and their
/// household's pantry, if it has one (Amendment A-26): where it stands, that it is theirs, what it
/// holds, and how filling a meal is.
pub struct Seam {
    meal_material: Option<EntityId>,
}

impl Seam {
    /// Knowledge for residents whose pantries hold meals of `meal_material`, if they have any.
    pub const fn new(meal_material: Option<EntityId>) -> Self {
        Self { meal_material }
    }
}

fn entity(view: &dyn CommittedView, key: FactKey) -> Option<EntityId> {
    match view.read(key)?.value {
        Value::Entity(e) => Some(e),
        _ => None,
    }
}

impl Seam {
    /// What `me`, crossing into `here`, knows: each belief once.
    fn knowledge(
        &self,
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
        let mut ways: Vec<EntityId> = Vec::new();
        for place in places {
            // Its own ways in and out, every one; then, up through every place that holds it,
            // the ways between each and the next — the street's door, the district's gate.
            ways.extend(openings(place));
            let mut inner = place;
            let mut hops = 0;
            while let (Some(outer), true) = (
                entity(view, FactKey::new(inner, PHYS_CONTAINED_IN)),
                hops < 16,
            ) {
                known.insert(FactKey::pair(me, PLACE_OF, inner), Value::Entity(outer));
                let leads = |o: &EntityId, to: EntityId| {
                    entity(view, FactKey::new(*o, PHYS_LEADS_TO)) == Some(to)
                };
                ways.extend(openings(inner).into_iter().filter(|o| leads(o, outer)));
                ways.extend(openings(outer).into_iter().filter(|o| leads(o, inner)));
                inner = outer;
                hops += 1;
            }
        }
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
        // The household's pantry: in the scene its stock, at the city tier the larder it will be.
        let home = entity(view, FactKey::new(me, RESIDENT_HOME));
        let pantry = home.and_then(|h| entity(view, FactKey::new(h, PANTRY)).map(|p| (h, p)));
        if let (Some((home, pantry)), Some(meal)) = (pantry, self.meal_material) {
            let meals = match view.read(FactKey::new(pantry, RES_STOCK)) {
                Some(stock) => stock.value.as_int().unwrap_or(0) / UNIT,
                None => view
                    .read(FactKey::new(home, LARDER))
                    .and_then(|f| f.value.as_int())
                    .unwrap_or(0),
            };
            known.insert(FactKey::pair(me, PLACE_OF, pantry), Value::Entity(home));
            known.insert(FactKey::pair(me, OWNER, pantry), Value::Entity(home));
            known.insert(FactKey::pair(me, YIELDS, pantry), Value::Entity(meal));
            known.insert(FactKey::pair(me, STOCK, pantry), Value::Int(meals));
            if let Some(fed) = view.read(FactKey::new(meal, PHYS_NUTRITION)) {
                known.insert(FactKey::pair(me, NUTRITION, meal), fed.value);
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
        let mut out: Vec<Proposal> = view
            .entities_with(LEAVING)
            .into_iter()
            .map(|who| {
                Proposal::new(
                    self.id(),
                    FactKey::new(who, PLACE_OF),
                    ctx.basis_tick(),
                    Change::Retire,
                    Cause::new("left_the_scene"),
                )
            })
            .collect();
        for who in view.entities_with(CROSSING) {
            let Some(here) = entity(view, FactKey::new(who, CROSSING)) else {
                continue;
            };
            for (key, value) in self.knowledge(view, who, here) {
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
