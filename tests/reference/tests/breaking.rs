//! Things that break (Vol. III Ch. 1; Appendix A, Ruling 9; Amendment A-27): Erin's fists cannot
//! break the packing crate in the yard, Carol's can, and anyone can with the hammer. What is broken
//! is still what it was — flat timber, its apple rolled out — and a walled room broken is a ruin,
//! open to the yard. Damage is anyone's to propose; breaking is Physical Reality's.

use kernel::fact::{Cause, FactKey, FactType, SystemId};
use kernel::identity::EntityId;
use kernel::proposal::{Change, Proposal};
use kernel::store::RealityStore;
use kernel::system::{Cadence, CommittedView, System, TickContext};
use kernel::value::Value;
use physical::schema::{
    BODY_SIZE, BROKEN, ENCLOSED, INTEGRITY, MADE_OF, OPAQUE, PORTAL_OPEN, POSITION, SOLID,
};
use physical::space::local_position;
use reference::id::*;
use reference::{e, without_minds, City};

/// Walk `who` up to `thing` and wait till they are there.
fn walk_up(city: &mut City, who: u64, thing: u64) {
    city.go(who, thing, 140);
    city.run_until(60, |c| !c.travelling(who));
}

/// One blow by `who` at `thing`, landed (or refused).
fn blow(city: &mut City, who: u64, thing: u64) {
    city.strike(who, thing);
    city.run(2);
}

fn broken(city: &City, thing: u64) -> bool {
    city.read(thing, BROKEN) == Some(Value::Bool(true))
}

#[test]
fn erins_fists_cannot_break_the_crate() {
    // At 40% strength, a fist lands at 20%: no harder than timber, so it leaves no mark.
    let mut city = City::quiet();
    walk_up(&mut city, ERIN, CRATE);
    blow(&mut city, ERIN, CRATE);
    assert_eq!(
        city.refused(ERIN),
        Some(CRATE),
        "she can tell it did nothing"
    );
    assert_eq!(city.read(CRATE, INTEGRITY), None, "whole");
    assert!(!broken(&city, CRATE));
}

#[test]
fn carol_breaks_the_crate_and_the_apple_rolls_out() {
    let mut city = City::quiet();
    walk_up(&mut city, CAROL, CRATE);
    let at = local_position(city.store(), e(CRATE));
    let mut blows = 0;
    while city.int(CRATE, INTEGRITY) != Some(0) && blows < 20 {
        blow(&mut city, CAROL, CRATE);
        blows += 1;
    }
    city.run(1); // and it breaks
    assert!(broken(&city, CRATE), "after {blows} blows");
    assert!((3..=10).contains(&blows), "{blows} blows from a strong arm");
    // Its wreckage: flat, no longer solid, still timber, still where it stood.
    assert_eq!(city.read(CRATE, SOLID), Some(Value::Bool(false)));
    let height = city
        .read(CRATE, BODY_SIZE)
        .and_then(|v| v.as_vec3())
        .unwrap()[2];
    assert!(height <= 20, "{height} cm tall");
    assert_eq!(
        city.store()
            .read(FactKey::new(e(CRATE), MADE_OF))
            .map(|f| f.value),
        Some(Value::Entity(e(TIMBER)))
    );
    assert_eq!(city.room_of(CRATE), YARD);
    // The apple spilled where the crate stood, into the yard.
    assert_eq!(city.room_of(CRATE_APPLE), YARD);
    let apple = local_position(city.store(), e(CRATE_APPLE));
    assert_eq!([apple[0], apple[1]], [at[0], at[1]]);
    // Every blow is in the chronicle.
    let struck = city
        .chronicle
        .iter()
        .filter(|c| c.subject() == e(CRATE) && c.cause() == Cause::new("struck"))
        .count();
    assert_eq!(struck, blows);
}

#[test]
fn with_the_hammer_anyone_can() {
    let mut city = City::quiet();
    walk_up(&mut city, ERIN, HAMMER);
    city.take(ERIN, HAMMER);
    city.run(2);
    assert_eq!(city.room_of(HAMMER), ERIN);
    walk_up(&mut city, ERIN, CRATE);
    blow(&mut city, ERIN, CRATE);
    assert_ne!(city.refused(ERIN), Some(CRATE));
    blow(&mut city, ERIN, CRATE);
    city.run(1);
    assert!(broken(&city, CRATE), "two blows of a hammer");
}

#[test]
fn a_tool_strikes_no_harder_than_its_own_stuff() {
    // A glass bottle swung at an iron hammer: however hard she swings, glass does not dent iron.
    let mut city = City::quiet();
    walk_up(&mut city, ERIN, BOTTLE);
    city.take(ERIN, BOTTLE);
    city.run(2);
    walk_up(&mut city, ERIN, HAMMER);
    blow(&mut city, ERIN, HAMMER);
    assert_eq!(city.refused(ERIN), Some(HAMMER));
    assert_eq!(city.read(HAMMER, INTEGRITY), None);
    // Out of reach, nothing lands at all.
    blow(&mut city, ERIN, SHED);
    assert_eq!(city.refused(ERIN), Some(SHED));
}

/// A stand-in for anything that damages — a battle, a falling tree — proposing integrity lost
/// from `targets`, once, on the first tick (Appendix A, Ruling 9).
struct Wrecker {
    targets: &'static [(u64, i64)],
}

const WRECKER_WRITES: &[FactType] = &[INTEGRITY];

impl System for Wrecker {
    fn id(&self) -> SystemId {
        SystemId::new("test.wrecker")
    }
    fn reads(&self) -> &'static [FactType] {
        &[]
    }
    fn writes(&self) -> &'static [FactType] {
        WRECKER_WRITES
    }
    fn cadence(&self) -> Cadence {
        Cadence::EveryTick
    }
    fn evaluate(&self, _view: &dyn CommittedView, ctx: &TickContext) -> Vec<Proposal> {
        if ctx.tick() != 1 {
            return Vec::new();
        }
        self.targets
            .iter()
            .map(|&(target, loss)| {
                Proposal::new(
                    self.id(),
                    FactKey::new(EntityId::from_raw(target), INTEGRITY),
                    ctx.basis_tick(),
                    Change::Delta(-loss),
                    Cause::new("wrecked"),
                )
            })
            .collect()
    }
}

#[test]
fn damage_is_anyones_to_propose_and_adds_up() {
    // Two blows from outside in one tick, and a repair: they all apply (Vol. III Ch. 12).
    let mut city = City::quiet();
    city.attach(Box::new(Wrecker {
        targets: &[(SHED, 3_000)],
    }));
    city.attach(Box::new(Wrecker2));
    city.run(1);
    assert_eq!(
        city.int(SHED, INTEGRITY),
        Some(10_000 - 3_000 - 2_000 + 1_000)
    );
    assert!(!broken(&city, SHED));
}

/// A second proposer in the same tick: more damage, and a repair.
struct Wrecker2;

impl System for Wrecker2 {
    fn id(&self) -> SystemId {
        SystemId::new("test.mender")
    }
    fn reads(&self) -> &'static [FactType] {
        &[]
    }
    fn writes(&self) -> &'static [FactType] {
        WRECKER_WRITES
    }
    fn cadence(&self) -> Cadence {
        Cadence::EveryTick
    }
    fn evaluate(&self, _view: &dyn CommittedView, ctx: &TickContext) -> Vec<Proposal> {
        if ctx.tick() != 1 {
            return Vec::new();
        }
        [(-2_000, "wrecked"), (1_000, "mended")]
            .into_iter()
            .map(|(d, why)| {
                Proposal::new(
                    self.id(),
                    FactKey::new(e(SHED), INTEGRITY),
                    ctx.basis_tick(),
                    Change::Delta(d),
                    Cause::new(why),
                )
            })
            .collect()
    }
}

#[test]
fn a_room_broken_is_a_ruin_open_to_the_yard() {
    // The kitchen's walls brought down: no longer walled, its front door broken open and no
    // longer hiding anything — and Carol in the yard can see Bob in it.
    let mut city = City::quiet();
    city.attach(Box::new(Wrecker {
        targets: &[(KITCHEN, 10_000)],
    }));
    city.run(3);
    assert!(broken(&city, KITCHEN));
    assert_eq!(city.read(KITCHEN, ENCLOSED), Some(Value::Bool(false)));
    for face in [1001, 1002] {
        assert!(broken(&city, face), "{face}");
        assert_eq!(city.read(face, PORTAL_OPEN), Some(Value::Bool(true)));
        assert_ne!(city.read(face, OPAQUE), Some(Value::Bool(true)));
    }
    // The kitchen is still a place, and Bob is still in it.
    assert_eq!(city.room_of(BOB), KITCHEN);
    assert_eq!(city.room_of(KITCHEN), HOUSE);
}

#[test]
fn a_door_broken_open_stays_open() {
    let mut city = City::quiet();
    city.attach(Box::new(Wrecker {
        targets: &[(1011, 10_000)], // the shut bedroom window
    }));
    city.run(3);
    for face in [1011, 1012] {
        assert!(broken(&city, face), "{face}");
        assert_eq!(city.read(face, PORTAL_OPEN), Some(Value::Bool(true)));
    }
    // Shutting it again is refused: there is nothing left to shut.
    city.close(DAVE, 1011);
    city.run(2);
    assert_eq!(city.read(1011, PORTAL_OPEN), Some(Value::Bool(true)));
}

#[test]
fn what_stood_on_a_broken_table_falls() {
    // The green bottle set on the kitchen table, and the table wrecked out from under it.
    let mut pkg = without_minds(reference::package());
    for c in &mut pkg.containment {
        if c.child_id == BOTTLE {
            c.parent_id = KITCHEN;
        }
    }
    for p in &mut pkg.positions {
        if p.entity_id == BOTTLE {
            (p.x, p.y, p.z) = (0, 0, Some(80));
        }
    }
    let mut city = City::from(pkg);
    city.run(5);
    let standing = city
        .read(BOTTLE, POSITION)
        .and_then(|v| v.as_vec3())
        .unwrap();
    assert_eq!(standing[2], 80, "on the table");
    city.attach(Box::new(Late {
        at: city.tick + 1,
        target: TABLE,
    }));
    city.run(10);
    assert!(broken(&city, TABLE));
    let fallen = city
        .read(BOTTLE, POSITION)
        .and_then(|v| v.as_vec3())
        .unwrap();
    assert_eq!(fallen[2], 0, "on the floor");
}

/// A wrecker that strikes `target` down at tick `at`.
struct Late {
    at: u64,
    target: u64,
}

impl System for Late {
    fn id(&self) -> SystemId {
        SystemId::new("test.late_wrecker")
    }
    fn reads(&self) -> &'static [FactType] {
        &[]
    }
    fn writes(&self) -> &'static [FactType] {
        WRECKER_WRITES
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
            FactKey::new(e(self.target), INTEGRITY),
            ctx.basis_tick(),
            Change::Delta(-10_000),
            Cause::new("wrecked"),
        )]
    }
}

#[test]
fn the_same_blows_break_the_same_way() {
    let run = || {
        let mut city = City::quiet();
        walk_up(&mut city, CAROL, CRATE);
        for _ in 0..8 {
            blow(&mut city, CAROL, CRATE);
        }
        city.run(3);
        city.store().state_hash()
    };
    assert_eq!(run(), run());
}

#[test]
fn with_its_walls_down_the_kitchen_is_in_sight_of_the_yard() {
    // Noon, minute ticks, the front door shut: from the yard nobody sees through the kitchen's
    // walls — until they fall.
    let mut pkg = without_minds(reference::with_tick_seconds(60));
    pkg.physical_rules.temperature_variability_centi_c = 0;
    for f in &mut pkg.flags {
        if f.entity_id == 1001 || f.entity_id == 1002 {
            f.flags.push(packages::model::Flag::Closed);
        }
    }
    let mut city = City::from(pkg);
    city.run(12 * 60);
    let sees = |city: &City, who: u64, what: u64| {
        city.store()
            .read_all(FactKey::new(e(who), physical::schema::IN_VIEW))
            .iter()
            .any(|f| f.value == Value::Entity(e(what)))
    };
    let watchers: Vec<u64> = [CAROL, ERIN, FINN, GWEN]
        .into_iter()
        .filter(|w| city.room_of(*w) == YARD)
        .collect();
    assert!(!watchers.is_empty());
    for &w in &watchers {
        assert!(!sees(&city, w, BOB), "{w} sees through a wall");
    }
    city.attach(Box::new(Late {
        at: city.tick + 1,
        target: KITCHEN,
    }));
    // The walls fall the tick after the blow, daylight floods in the tick after that, and then it
    // is seen.
    city.run(4);
    assert!(
        watchers.iter().any(|&w| sees(&city, w, BOB)),
        "nobody in the yard sees Bob in the ruin"
    );
}
