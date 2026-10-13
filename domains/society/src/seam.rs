//! Crossing the seam (Vol. V Ch. 5 §5.2; Appendix A, Ruling 19; Amendments A-24, A-25): a
//! resident who stands where the scene is becomes an organism, with a body and a mind, in a
//! single tick; an organism who was a resident and stands where the city tier is becomes a
//! record again, in a single tick.
//!
//! **Into the scene.** Society marks the crossing; on the next tick every domain reads the mark
//! from the same committed record and gives the person its own facts — Physical Reality a body in
//! the place, Living Systems their hunger and fatigue, the information layer what they know,
//! decision systems a mind — while Society retires the record's state. One walking into the scene
//! is marked by the rhythm as their walk ends (the agenda, Amendment A-22); everyone standing where
//! attention arrives is marked here (the residents *at* each place, Amendment A-24).
//!
//! **Out of it.** A resident in the scene standing in a place at the city tier, alive and carrying
//! nothing, is marked leaving. On the next tick every other domain retires them from its keeping
//! (Amendment A-25) — whatever its systems were about to write — and Society takes up their record
//! again: their hunger and fatigue exactly as their body had them, and the walk on from where they
//! stand.
//!
//! Either way no tick holds both, and the cost is in proportion to who crosses, not to the city.

use crate::residents::{rung, whereabouts, Rhythm, RhythmRules, FULL};
use crate::schema::{
    BODY_SIZE, CITY, CONTAINED_IN, HEADING, LARDER, LIVING_FATIGUE, LIVING_HEALTH, LIVING_HUNGER,
    LIVING_REST, MOTION_END, MOTION_START, MOTION_TARGET, PANTRY, PANTRY_CROSSING, PANTRY_IN_SCENE,
    PANTRY_LEAVING, POSITION, RESIDENCY, RESIDENT_AT, RESIDENT_CROSSING, RESIDENT_DOING,
    RESIDENT_FATIGUE, RESIDENT_HOME, RESIDENT_HUNGER, RESIDENT_IN_SCENE, RESIDENT_LEAVING,
    RESIDENT_UNTIL, RESIDENT_WORK, SCENE, WALKING,
};
use kernel::fact::{Cause, FactKey, FactType, SystemId};
use kernel::identity::EntityId;
use kernel::level::Level;
use kernel::proposal::{Change, Proposal};
use kernel::system::{Cadence, CommittedView, System, TickContext};
use kernel::time::SimClock;
use kernel::value::Value;
use std::collections::{BTreeMap, BTreeSet};

const CROSSING_READS: &[FactType] = &[
    RESIDENT_HOME,
    RESIDENT_WORK,
    RESIDENT_AT,
    RESIDENT_DOING,
    RESIDENT_UNTIL,
    RESIDENT_HUNGER,
    RESIDENT_FATIGUE,
    RESIDENT_CROSSING,
    RESIDENT_IN_SCENE,
    RESIDENT_LEAVING,
    RESIDENCY,
    CONTAINED_IN,
    POSITION,
    BODY_SIZE,
    HEADING,
    MOTION_TARGET,
    MOTION_START,
    MOTION_END,
    LIVING_HUNGER,
    LIVING_FATIGUE,
    LIVING_HEALTH,
    LIVING_REST,
    LARDER,
    PANTRY,
    PANTRY_CROSSING,
    PANTRY_IN_SCENE,
    PANTRY_LEAVING,
];
const CROSSING_WRITES: &[FactType] = &[
    RESIDENT_DOING,
    RESIDENT_AT,
    RESIDENT_UNTIL,
    RESIDENT_HUNGER,
    RESIDENT_FATIGUE,
    RESIDENT_CROSSING,
    RESIDENT_IN_SCENE,
    RESIDENT_LEAVING,
    PANTRY_CROSSING,
    PANTRY_IN_SCENE,
    PANTRY_LEAVING,
];

/// What a resident record holds besides who they are: retired when they cross into the scene.
/// Their home and work stay, because they are who the person is at every tier.
const STATE: &[FactType] = &[
    RESIDENT_DOING,
    RESIDENT_AT,
    RESIDENT_UNTIL,
    RESIDENT_HUNGER,
    RESIDENT_FATIGUE,
    RESIDENT_CROSSING,
];

/// Society's side of the seam (Amendments A-24, A-25), every tick:
///
/// - **retire** the record's state for everyone marked crossing in the last tick, and note them
///   in the scene — the same tick every other domain gives them its facts;
/// - **take up** the record again for everyone marked leaving in the last tick — the same tick
///   every other domain retires them;
/// - **mark leaving** every resident in the scene who stands at the city tier, alive and carrying
///   nothing;
/// - **mark crossing** every resident not walking who is at a place attention has just brought
///   into the scene. (One walking there crosses on arriving: the rhythm marks them.)
pub struct Crossing {
    clock: SimClock,
    rhythm: Rhythm,
    rules: RhythmRules,
}

impl Crossing {
    /// The seam of a settlement living by `rules`, on `clock`.
    pub const fn new(clock: SimClock, rules: RhythmRules) -> Self {
        Self {
            clock,
            rhythm: Rhythm::new(clock, rules),
            rules,
        }
    }

    /// Every resident at a place in the scene who is not walking, by the place they are at —
    /// within `places` (each with what lies within it), or, when the store cannot say what
    /// changed, the whole city.
    fn standing_in_scene(
        view: &dyn CommittedView,
        places: Option<Vec<EntityId>>,
        rungs: &mut BTreeMap<EntityId, i64>,
        out: &mut BTreeMap<EntityId, EntityId>,
        larders: &mut BTreeMap<EntityId, EntityId>,
    ) {
        // A larder kept as a count, at a home now in the scene, comes into it as its pantry.
        let mut larder_at = |home: EntityId| {
            if let (Some(pantry), true) = (
                entity(view, FactKey::new(home, PANTRY)),
                view.read(FactKey::new(home, LARDER)).is_some(),
            ) {
                larders.insert(pantry, home);
            }
        };
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
            for home in view.entities_with(PANTRY) {
                if rung(view, home, rungs) == SCENE {
                    larder_at(home);
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
            larder_at(place);
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

    /// Where `who` stands, as the crow flies: their place's whereabouts plus their position in it.
    fn standing_at(view: &dyn CommittedView, who: EntityId, place: EntityId) -> [i64; 2] {
        let [px, py] = whereabouts(view, place);
        let [x, y, _] = view
            .read(FactKey::new(who, POSITION))
            .and_then(|f| f.value.as_vec3())
            .unwrap_or([0, 0, 0]);
        [px + x, py + y]
    }

    /// A level as the body had it — exactly, rate and anchor and all — or, for one walking on,
    /// set on that walk's course from `now` if it was on another.
    fn carried(
        &self,
        view: &dyn CommittedView,
        key: FactKey,
        walking_rate: Option<i64>,
        now: u64,
    ) -> Value {
        let level = view
            .read(key)
            .and_then(|f| Level::from_value(f.value))
            .unwrap_or(Level::new(0, walking_rate.unwrap_or(0), now));
        match walking_rate {
            Some(rate) if level.per_hour != rate => {
                level.from_tick(self.clock, now, 0, FULL, rate).to_value()
            }
            _ => level.to_value(),
        }
    }
}

fn entity(view: &dyn CommittedView, key: FactKey) -> Option<EntityId> {
    match view.read(key)?.value {
        Value::Entity(e) => Some(e),
        _ => None,
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

        // Into the scene: retire the records of those who crossed; every other domain takes them
        // up this tick.
        let crossing = view.entities_with(RESIDENT_CROSSING);
        for &who in &crossing {
            for &fact in STATE {
                if view.read(FactKey::new(who, fact)).is_some() {
                    push(FactKey::new(who, fact), Change::Tombstone, "crossed");
                }
            }
            push(
                FactKey::new(who, RESIDENT_IN_SCENE),
                Change::Set(Value::Bool(true)),
                "crossed",
            );
        }

        // Out of it: take up the records of those leaving; every other domain lets them go this
        // tick. They emerge walking — arrived, at home or work, or on their way there.
        let leaving = view.entities_with(RESIDENT_LEAVING);
        for &who in &leaving {
            let Some(place) = entity(view, FactKey::new(who, RESIDENT_LEAVING)) else {
                continue;
            };
            let from = Self::standing_at(view, who, place);
            let resting = view.read(FactKey::new(who, LIVING_REST)).map(|f| f.value)
                == Some(Value::Bool(true));
            let (doing, at, until) = self.rhythm.emerging(view, who, place, from, resting, now);
            // Arrived, the rhythm sets their course next tick as it would anyone's; walking on,
            // they hunger and tire as walkers do.
            let walking_on = at != place;
            let hunger = self.carried(
                view,
                FactKey::new(who, LIVING_HUNGER),
                walking_on.then_some(self.rules.hunger_per_hour),
                now,
            );
            let fatigue = self.carried(
                view,
                FactKey::new(who, LIVING_FATIGUE),
                walking_on.then_some(self.rules.tire_per_hour),
                now,
            );
            for (fact, value) in [
                (RESIDENT_DOING, Value::Int(doing)),
                (RESIDENT_AT, Value::Entity(at)),
                (RESIDENT_UNTIL, Value::Int(until as i64)),
                (RESIDENT_HUNGER, hunger),
                (RESIDENT_FATIGUE, fatigue),
            ] {
                push(
                    FactKey::new(who, fact),
                    Change::Set(value),
                    "left_the_scene",
                );
            }
            for fact in [RESIDENT_LEAVING, RESIDENT_IN_SCENE] {
                push(FactKey::new(who, fact), Change::Tombstone, "left_the_scene");
            }
        }

        // Larders (Amendment A-26): a pantry that crossed is in the scene; one that left is not.
        // (The rhythm keeps the larder's count: it lets it go, and counts it back.)
        for pantry in view.entities_with(PANTRY_CROSSING) {
            if let Some(home) = entity(view, FactKey::new(pantry, PANTRY_CROSSING)) {
                push(
                    FactKey::new(pantry, PANTRY_CROSSING),
                    Change::Tombstone,
                    "crossed",
                );
                push(
                    FactKey::new(pantry, PANTRY_IN_SCENE),
                    Change::Set(Value::Entity(home)),
                    "crossed",
                );
            }
        }
        let pantries_leaving = view.entities_with(PANTRY_LEAVING);
        for &pantry in &pantries_leaving {
            for fact in [PANTRY_LEAVING, PANTRY_IN_SCENE] {
                push(
                    FactKey::new(pantry, fact),
                    Change::Tombstone,
                    "left_the_scene",
                );
            }
        }

        let mut rungs = BTreeMap::new();
        // A pantry whose home is back at the city tier leaves the scene.
        for pantry in view.entities_with(PANTRY_IN_SCENE) {
            if pantries_leaving.binary_search(&pantry).is_ok() {
                continue;
            }
            if let Some(home) = entity(view, FactKey::new(pantry, PANTRY_IN_SCENE)) {
                if rung(view, home, &mut rungs) == CITY {
                    push(
                        FactKey::new(pantry, PANTRY_LEAVING),
                        Change::Set(Value::Entity(home)),
                        "leaving_the_scene",
                    );
                }
            }
        }
        // Mark leaving: in the scene, standing where the city tier is, alive, hands empty.
        // (Carrying things across is not yet built: one carrying something stays until they set
        // it down.)
        let index = view.spatial();
        for who in view.entities_with(RESIDENT_IN_SCENE) {
            if leaving.binary_search(&who).is_ok() {
                continue;
            }
            let Some(place) = entity(view, FactKey::new(who, CONTAINED_IN)) else {
                continue;
            };
            let alive = view
                .read(FactKey::new(who, LIVING_HEALTH))
                .and_then(|f| f.value.as_int())
                != Some(0);
            let carrying = index.is_some_and(|ix| !ix.children(who).is_empty());
            if alive && !carrying && rung(view, place, &mut rungs) == CITY {
                push(
                    FactKey::new(who, RESIDENT_LEAVING),
                    Change::Set(Value::Entity(place)),
                    "leaving_the_scene",
                );
            }
        }

        // Mark crossing: attention arriving at the places whose rung was set in the last tick.
        // Seeds are not changes, so the world as loaded has none (its residents are validated to
        // be at the city tier).
        let mut entering: BTreeMap<EntityId, EntityId> = BTreeMap::new();
        let mut larders: BTreeMap<EntityId, EntityId> = BTreeMap::new();
        let basis = ctx.basis_tick();
        if basis > 0 {
            let changed = view.changed_since(RESIDENCY, basis - 1);
            if changed.as_ref().map_or(true, |c| !c.is_empty()) {
                Self::standing_in_scene(view, changed, &mut rungs, &mut entering, &mut larders);
            }
        }
        for (pantry, home) in larders {
            let set_out = view.read(FactKey::new(pantry, PANTRY_CROSSING)).is_some()
                || view.read(FactKey::new(pantry, PANTRY_IN_SCENE)).is_some();
            if !set_out {
                push(
                    FactKey::new(pantry, PANTRY_CROSSING),
                    Change::Set(Value::Entity(home)),
                    "attended",
                );
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
