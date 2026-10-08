//! Collapse and mending (Vol. III Ch. 1; Amendments A-31, A-44): walls that fall strike whoever is
//! inside; a broken thing is mended with a tool no softer than it and with timber to hand, used up
//! as it goes, a little at a time, and once whole it is what it was — a crate standing as tall and
//! as solid, a ruin walled again with its doors back.

use kernel::fact::{Cause, FactKey, FactType, SystemId};
use kernel::proposal::{Change, Proposal};
use kernel::system::{Cadence, CommittedView, System, TickContext};
use kernel::value::Value;
use packages::model::{BodySpec, ContainmentSpec, MadeOfSpec, PositionSpec};
use packages::WorldPackage;
use physical::schema::{
    BODY_SIZE, BROKEN, ENCLOSED, IMPACT, INTEGRITY, OPAQUE, PORTAL_OPEN, SOLID, WHOLE_SHAPE,
};
use reference::id::*;
use reference::{e, without_minds, City};

fn ashford() -> WorldPackage {
    without_minds(reference::package())
}

/// `n` pieces of timber, each `size`, ids from `first`, lying in `place` at `at`.
fn timber(pkg: &mut WorldPackage, first: u64, n: u64, place: u64, at: [i64; 2], size: [i64; 3]) {
    for id in first..first + n {
        pkg.containment.push(ContainmentSpec {
            child_id: id,
            parent_id: place,
        });
        pkg.positions.push(PositionSpec {
            entity_id: id,
            x: at[0],
            y: at[1],
            z: Some(0),
        });
        pkg.bodies.push(BodySpec {
            entity_id: id,
            half_width: size[0],
            half_depth: size[1],
            height: size[2],
        });
        pkg.made_of.push(MadeOfSpec {
            object_id: id,
            material_id: 700,
        });
    }
}

/// How many of the pieces with ids `first..first + n` are still in the world.
fn left(city: &City, first: u64, n: u64) -> u64 {
    (first..first + n)
        .filter(|id| city.read(*id, physical::schema::CONTAINED_IN).is_some())
        .count() as u64
}

fn broken(city: &City, thing: u64) -> bool {
    city.read(thing, BROKEN) == Some(Value::Bool(true))
}

fn walk_up(city: &mut City, who: u64, thing: u64) {
    city.go(who, thing, 140);
    city.run_until(60, |c| !c.travelling(who));
}

/// `target` brought down at tick `at` (Appendix A, Ruling 9).
struct Wrecker {
    at: u64,
    target: u64,
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
fn when_the_walls_fall_those_inside_are_struck() {
    // The kitchen comes down on Bob: struck as by a fall from as high as its walls stood.
    let mut city = City::from(ashford());
    city.attach(Box::new(Wrecker {
        at: 1,
        target: KITCHEN,
    }));
    city.run(3);
    assert_eq!(city.int(BOB, IMPACT), Some(280));
    city.run(120); // Living judges harm at its metabolism step
    let health = city.int(BOB, living::schema::HEALTH).unwrap();
    assert!(health < 10_000, "Bob unhurt at {health}");
    // Dave, upstairs, is not in the kitchen: untouched.
    assert_eq!(city.read(DAVE, IMPACT), None);
}

#[test]
fn the_crate_is_mended_whole_and_stands_again() {
    // A stack of eight boards beside the crate, seven kilos of timber each.
    let mut pkg = ashford();
    timber(&mut pkg, 9_100, 8, YARD, [0, -600], [25, 10, 10]);
    let mut city = City::from(pkg);
    walk_up(&mut city, CAROL, CRATE);
    while city.int(CRATE, INTEGRITY) != Some(0) {
        city.strike(CAROL, CRATE);
        city.run(2);
    }
    city.run(1);
    assert!(broken(&city, CRATE));
    assert!(
        city.read(CRATE, WHOLE_SHAPE).is_some(),
        "what it was is kept"
    );
    // Bare hands mend nothing.
    city.mend(CAROL, CRATE);
    city.run(2);
    assert_eq!(city.refused(CAROL), Some(CRATE));
    // With the hammer — iron, no softer than timber — a few goes make it whole.
    walk_up(&mut city, CAROL, HAMMER);
    city.take(CAROL, HAMMER);
    city.run(2);
    walk_up(&mut city, CAROL, CRATE);
    let mut goes = 0;
    while broken(&city, CRATE) && goes < 20 {
        city.mend(CAROL, CRATE);
        city.run(2);
        goes += 1;
    }
    assert!(!broken(&city, CRATE), "whole after {goes} goes");
    // The boards went into it: the 45 kilos of timber its wreckage weighs, in seven-kilo boards
    // (Amendment A-44), with a little laid by.
    let used = 8 - left(&city, 9_100, 8);
    assert!((6..=7).contains(&used), "{used} boards used");
    // What it was: as tall, as solid; and nothing more to mend.
    assert_eq!(city.read(CRATE, BODY_SIZE), Some(Value::Vec3([30, 25, 40])));
    assert_eq!(city.read(CRATE, SOLID), Some(Value::Bool(true)));
    assert_eq!(city.read(CRATE, INTEGRITY), None);
    assert_eq!(city.read(CRATE, WHOLE_SHAPE), None);
}

#[test]
fn a_ruin_rebuilt_is_walled_again_with_its_doors() {
    // Bob, in the kitchen ruin with the hammer, rebuilds it (here a quicker builder than most),
    // from eighty half-tonne beams piled by him: the kitchen's walls, floor, and roof are 44
    // tonnes of timber (Amendment A-44).
    let mut pkg = ashford();
    pkg.physical_rules.mend_scale = 30_000;
    timber(&mut pkg, 9_200, 80, KITCHEN, [0, 420], [100, 50, 40]);
    // The hammer is in Bob's hand.
    for c in &mut pkg.containment {
        if c.child_id == HAMMER {
            c.parent_id = BOB;
        }
    }
    for p in &mut pkg.positions {
        if p.entity_id == HAMMER {
            (p.x, p.y, p.z) = (0, 0, Some(0));
        }
    }
    let mut city = City::from(pkg);
    city.attach(Box::new(Wrecker {
        at: 1,
        target: KITCHEN,
    }));
    city.run(3);
    assert!(broken(&city, KITCHEN));
    assert!(broken(&city, 1001));
    // With only a hammer and nothing to build with, nothing comes of it.
    let mut bare = City::from({
        let mut pkg = ashford();
        for c in &mut pkg.containment {
            if c.child_id == HAMMER {
                c.parent_id = BOB;
            }
        }
        pkg
    });
    bare.attach(Box::new(Wrecker {
        at: 1,
        target: KITCHEN,
    }));
    bare.run(3);
    bare.mend(BOB, KITCHEN);
    bare.run(3);
    assert_eq!(bare.refused(BOB), Some(KITCHEN), "no timber to hand");
    let mut goes = 0;
    while broken(&city, KITCHEN) && goes < 200 {
        city.mend(BOB, KITCHEN);
        city.run(1);
        goes += 1;
    }
    city.run(2);
    assert!(!broken(&city, KITCHEN), "rebuilt after {goes} goes");
    let used = 80 - left(&city, 9_200, 80);
    assert!((75..=80).contains(&used), "{used} beams went into it");
    assert_eq!(city.read(KITCHEN, ENCLOSED), Some(Value::Bool(true)));
    // Its front door is whole again — opaque, and it can be shut.
    for face in [1001, 1002] {
        assert!(!broken(&city, face), "{face}");
        assert_eq!(city.read(face, OPAQUE), Some(Value::Bool(true)));
    }
    city.close(BOB, 1002);
    city.run(2);
    assert_eq!(city.read(1002, PORTAL_OPEN), Some(Value::Bool(false)));
}
