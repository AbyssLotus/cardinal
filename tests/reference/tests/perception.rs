//! What Ashford's people perceive and believe (Vol. II Ch. 4; Amendments A-7, A-8): they see
//! what is in view and in light, believe what they saw, remember it after it is gone — rightly or
//! not — know where they stand and how warm it is, and know nothing of what they never saw or
//! were never told.

use information::schema::{BLOCKED, FELT_BODY_HEAT, GOING_TO, IN_SIGHT, OPEN, PLACE_OF, WARMTH_OF};
use kernel::fact::{Cause, Fact, FactKey, FactType, SystemId};
use kernel::proposal::{Change, Proposal};
use kernel::system::{Cadence, CommittedView, System, TickContext};
use kernel::value::Value;
use physical::schema::{OPAQUE, TEMPERATURE};
use reference::id::*;
use reference::{e, package, with_tick_seconds, City};

/// What `who` believes about `about`, of kind `fact`.
fn belief(city: &City, who: u64, fact: FactType, about: u64) -> Option<Fact> {
    city.store().read(FactKey::pair(e(who), fact, e(about)))
}

/// Where `who` believes `about` is.
fn thinks_is_in(city: &City, who: u64, about: u64) -> Option<u64> {
    match belief(city, who, PLACE_OF, about)?.value {
        Value::Entity(place) => Some(place.raw()),
        _ => None,
    }
}

/// What `who` is watching now.
fn watching(city: &City, who: u64) -> Vec<u64> {
    city.store()
        .read_all(FactKey::new(e(who), IN_SIGHT))
        .into_iter()
        .filter_map(|f| match f.value {
            Value::Entity(x) => Some(x.raw()),
            _ => None,
        })
        .collect()
}

/// Ashford at ten-minute ticks, run to just past noon (tick 72), so the noon light has been
/// seen by and perceived through: sight reads the light committed a tick before, perception the
/// view committed a tick before that.
fn at_noon() -> City {
    let mut city = City::from(with_tick_seconds(600));
    city.run(74);
    city
}

#[test]
fn in_daylight_dave_sees_erin_and_believes_she_is_in_the_yard() {
    let city = at_noon();
    assert!(watching(&city, DAVE).contains(&ERIN));
    assert_eq!(thinks_is_in(&city, DAVE, ERIN), Some(YARD));
    let how = belief(&city, DAVE, PLACE_OF, ERIN)
        .unwrap()
        .provenance
        .cause;
    assert_eq!(how, Cause::new("seen"));
    // He sees her through the bedroom window; he cannot see Bob through the floor.
    assert!(!watching(&city, DAVE).contains(&BOB));
}

#[test]
fn in_the_dark_nothing_is_seen_but_what_was_seen_is_remembered() {
    let mut city = at_noon();
    // Night: by midnight nothing in Ashford is lit, and nobody watches anything.
    city.run(144 - 74);
    assert!(
        watching(&city, DAVE).is_empty(),
        "{:?}",
        watching(&city, DAVE)
    );
    assert!(watching(&city, FINN).is_empty());
    // Dave still believes Erin is in the yard, as of the moment he lost sight of her at dusk.
    assert_eq!(thinks_is_in(&city, DAVE, ERIN), Some(YARD));
    let last = belief(&city, DAVE, PLACE_OF, ERIN).unwrap().provenance;
    assert_eq!(last.cause, Cause::new("lost_sight"));
    assert!(
        (100..144).contains(&last.tick),
        "lost sight of her at tick {}",
        last.tick
    );
}

/// Draws the curtains across the bedroom window at tick `at`.
struct Curtains {
    at: u64,
}
impl System for Curtains {
    fn id(&self) -> SystemId {
        SystemId::new("probe.curtains")
    }
    fn reads(&self) -> &'static [FactType] {
        &[]
    }
    fn writes(&self) -> &'static [FactType] {
        &[OPAQUE]
    }
    fn cadence(&self) -> Cadence {
        Cadence::EveryTick
    }
    fn evaluate(&self, _view: &dyn CommittedView, ctx: &TickContext) -> Vec<Proposal> {
        if ctx.tick() != self.at {
            return Vec::new();
        }
        vec![Proposal::new(
            self.id(),
            FactKey::new(e(BEDROOM_WINDOW), OPAQUE),
            ctx.basis_tick(),
            Change::Set(Value::Bool(true)),
            Cause::new("curtains_drawn"),
        )]
    }
}

#[test]
fn drawn_curtains_take_the_yard_out_of_sight_but_not_out_of_mind() {
    let mut city = at_noon();
    assert!(watching(&city, DAVE).contains(&ERIN));
    city.attach(Box::new(Curtains { at: 75 }));
    city.run(3);
    assert!(!watching(&city, DAVE).contains(&ERIN));
    assert_eq!(thinks_is_in(&city, DAVE, ERIN), Some(YARD), "he remembers");
    assert_eq!(
        belief(&city, DAVE, PLACE_OF, ERIN)
            .unwrap()
            .provenance
            .cause,
        Cause::new("lost_sight")
    );
}

#[test]
fn dave_still_believes_bob_is_in_the_kitchen_after_he_has_gone() {
    // Dave knows from the start that Bob is in the kitchen. Bob goes down to the cellar; Dave,
    // upstairs, cannot see it happen.
    let mut city = City::new();
    city.go(BOB, CELLAR, 140);
    city.run_until(40, |c| !c.travelling(BOB));
    assert_eq!(city.room_of(BOB), CELLAR);
    assert_eq!(
        thinks_is_in(&city, DAVE, BOB),
        Some(KITCHEN),
        "wrong, and he can't know"
    );
    let known = belief(&city, DAVE, PLACE_OF, BOB).unwrap().provenance;
    assert_eq!((known.tick, known.cause), (0, Cause::new("known")));
    // Bob knows where he is.
    assert_eq!(thinks_is_in(&city, BOB, BOB), Some(CELLAR));
}

#[test]
fn hal_on_the_moor_knows_nothing_of_ashford() {
    let city = at_noon();
    let about: Vec<u64> = city
        .store()
        .read_about(e(HAL), PLACE_OF)
        .into_iter()
        .map(|(x, _)| x.raw())
        .collect();
    assert_eq!(about, vec![HAL], "only where he himself stands");
    // And the courier, who knows only the yard, has no idea how warm the kitchen is.
    assert!(belief(&city, COURIER, WARMTH_OF, KITCHEN).is_none());
}

#[test]
fn everyone_knows_where_they_stand_and_how_warm_it_is() {
    let mut city = City::new();
    city.run(2);
    let s = city.store();
    let vale = s
        .read(FactKey::new(e(VALE), TEMPERATURE))
        .unwrap()
        .value
        .as_int()
        .unwrap();
    // Erin, in the yard, feels the Vale's air and her own body.
    assert_eq!(thinks_is_in(&city, ERIN, ERIN), Some(YARD));
    let felt = belief(&city, ERIN, WARMTH_OF, YARD)
        .unwrap()
        .value
        .as_int()
        .unwrap();
    assert!((felt - vale).abs() < 50, "felt {felt}, air {vale}");
    let body = s
        .read(FactKey::new(e(ERIN), FELT_BODY_HEAT))
        .unwrap()
        .value
        .as_int()
        .unwrap();
    assert!((body - 3_700).abs() < 50);
    // What she knows from the start of the kitchen: its air as the world began, still warm from
    // last night's stove.
    let kitchen = belief(&city, ERIN, WARMTH_OF, KITCHEN).unwrap();
    assert_eq!(kitchen.provenance.cause, Cause::new("known"));
    assert_eq!(kitchen.value, Value::Int(2_000));
}

#[test]
fn the_courier_knows_when_his_way_is_shut() {
    let mut city = City::new();
    city.close(BOB, DOOR_IN);
    city.run(2);
    city.go(COURIER, KITCHEN, 140);
    city.run(15);
    let s = city.store();
    assert_eq!(
        s.read(FactKey::new(e(COURIER), GOING_TO)).map(|f| f.value),
        Some(Value::Entity(e(KITCHEN)))
    );
    assert_eq!(
        s.read(FactKey::new(e(COURIER), BLOCKED)).map(|f| f.value),
        Some(Value::Bool(true))
    );
}

#[test]
fn finn_sees_the_door_shut() {
    let mut city = at_noon();
    assert_eq!(
        belief(&city, FINN, OPEN, DOOR_OUT).map(|f| f.value),
        Some(Value::Bool(true))
    );
    city.close(BOB, DOOR_IN);
    city.run(3);
    assert_eq!(
        belief(&city, FINN, OPEN, DOOR_OUT).map(|f| f.value),
        Some(Value::Bool(false)),
        "he saw it swing to"
    );
}

#[test]
fn perceiving_never_disturbs_reality() {
    // The information layer reads and never writes reality: Ashford with and without it moves
    // and warms identically.
    fn trajectory(pkg: &packages::WorldPackage) -> Vec<Vec<Option<Value>>> {
        let mut city = City::from(pkg.clone());
        city.go(BOB, BEDROOM, 140).go(CAT, KITCHEN, 300);
        (0..60)
            .map(|_| {
                city.run(1);
                [BOB, CAT, CART]
                    .iter()
                    .map(|id| city.read(*id, physical::schema::POSITION))
                    .chain([VALE, KITCHEN].iter().map(|id| city.read(*id, TEMPERATURE)))
                    .collect()
            })
            .collect()
    }
    let with = package();
    let mut without = with.clone();
    without.manifest.domains.retain(|d| d != "information");
    without.information_rules = None;
    without.knows = Vec::new();
    assert_eq!(trajectory(&with), trajectory(&without));
}
