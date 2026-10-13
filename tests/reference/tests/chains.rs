//! The chains (Vol. III Ch. 4 §4.4; Amendment A-43): bread is baked only while the oven burns;
//! flour is ground only while the Brack turns the mill, and not once it freezes; planks are sawn
//! only with an edge to hand, four from a log; charcoal comes from a log in the burning kiln, and
//! an iron bar from ore and charcoal at the forge.

use kernel::fact::FactKey;
use kernel::system::CommittedView;
use kernel::value::Value;
use packages::model::{BodySpec, ContainmentSpec, DepositSpec, MadeOfSpec, PositionSpec};
use packages::WorldPackage;
use physical::schema::{CONTAINED_IN, MADE_OF};
use reference::id::*;
use reference::{e, with_tick_seconds, without_minds, City};

const MILL: u64 = 604;
const WOOD: u64 = 603;
const CABIN: u64 = 402;
const VALE: u64 = 1;
const FLOUR_BIN: u64 = 123;
const TIMBER: u64 = 3122;
const IRON_SEAM: u64 = 3120;
const OVEN: u64 = 621;
const WHEAT: u64 = 717;
const FLOUR: u64 = 706;
const BREAD: u64 = 709;
const TIMBER_M: u64 = 700;
const CHARCOAL: u64 = 719;
const IRON: u64 = 702;
const IRON_ORE: u64 = 715;
const GRIND: u64 = 951;
const BAKE: u64 = 952;
const SAW: u64 = 953;
const BURN_CHARCOAL: u64 = 954;
const SMELT_IRON: u64 = 955;

/// Ashford without minds at minute ticks, in steady weather.
fn ashford() -> WorldPackage {
    let mut pkg = without_minds(with_tick_seconds(60));
    pkg.physical_rules.temperature_variability_centi_c = 0;
    pkg.physical_rules.humidity_variability = 0;
    pkg.physical_rules.pressure_variability = 0;
    pkg
}

/// Put `who` in `place` at `at`, and the hammer in their hand if `hammer`.
fn stand(pkg: &mut WorldPackage, who: u64, place: u64, at: [i64; 2], hammer: bool) {
    for c in &mut pkg.containment {
        if c.child_id == who {
            c.parent_id = place;
        }
        if hammer && c.child_id == HAMMER {
            c.parent_id = who;
        }
    }
    for o in &mut pkg.organisms {
        if o.id == who {
            o.region_id = place;
        }
    }
    place_at(pkg, who, at);
}

fn place_at(pkg: &mut WorldPackage, thing: u64, at: [i64; 2]) {
    pkg.positions.retain(|p| p.entity_id != thing);
    pkg.positions.push(PositionSpec {
        entity_id: thing,
        x: at[0],
        y: at[1],
        z: Some(0),
    });
}

/// Move `thing` into `place` at `at`.
fn move_to(pkg: &mut WorldPackage, thing: u64, place: u64, at: [i64; 2]) {
    for c in &mut pkg.containment {
        if c.child_id == thing {
            c.parent_id = place;
        }
    }
    place_at(pkg, thing, at);
}

/// A heap of `material` in `place` at `at`: a deposit to pick from, as id `id`.
fn heap(pkg: &mut WorldPackage, id: u64, material: u64, place: u64, at: [i64; 2]) {
    pkg.containment.push(ContainmentSpec {
        child_id: id,
        parent_id: place,
    });
    place_at(pkg, id, at);
    pkg.bodies.push(BodySpec {
        entity_id: id,
        half_width: 30,
        half_depth: 30,
        height: 30,
    });
    pkg.made_of.push(MadeOfSpec {
        object_id: id,
        material_id: material,
    });
    pkg.deposits.push(DepositSpec {
        id,
        made_of: material,
        size: [10, 10, 10],
        per_day: 0,
        cap: 10,
        stock: 10,
        ..Default::default()
    });
}

/// What `who` holds that is made of `material`.
fn holding(city: &City, who: u64, material: u64) -> usize {
    let s = city.store();
    s.entities_with(CONTAINED_IN)
        .into_iter()
        .filter(|t| {
            s.read(FactKey::new(*t, CONTAINED_IN)).map(|f| f.value) == Some(Value::Entity(e(who)))
        })
        .filter(|t| {
            s.read_all(FactKey::new(*t, MADE_OF))
                .iter()
                .any(|m| m.value == Value::Entity(e(material)))
        })
        .count()
}

/// Take from `deposit` until `who` holds `count` of `material`, at most 60 goes.
fn gather(city: &mut City, who: u64, deposit: u64, material: u64, count: usize) {
    for _ in 0..60 {
        if holding(city, who, material) >= count {
            return;
        }
        city.pick(who, deposit);
        city.run(4);
    }
    panic!("{who} could not gather {count} of {material}");
}

/// Ask `who` to make `recipe`, and wait `minutes` for it.
fn make(city: &mut City, who: u64, recipe: u64, minutes: u64) {
    city.make(who, recipe);
    city.run(minutes + 4);
}

fn refused(city: &City, who: u64) -> bool {
    city.read(who, economy::schema::MAKE_REFUSED).is_some()
}

#[test]
fn bread_is_baked_only_while_the_oven_burns() {
    // The miller, with the flour bin beside him, takes a scoop and bakes a loaf in half an hour.
    // With the oven cold, he cannot.
    let bake = |lit: bool| {
        let mut pkg = ashford();
        move_to(&mut pkg, FLOUR_BIN, MILL, [0, 80]);
        if !lit {
            pkg.fires.retain(|f| f.0 != OVEN);
        }
        let mut city = City::from(pkg);
        gather(&mut city, MILLER, FLOUR_BIN, FLOUR, 1);
        make(&mut city, MILLER, BAKE, 30);
        (holding(&city, MILLER, BREAD), refused(&city, MILLER))
    };
    assert_eq!(bake(true), (1, false), "a loaf");
    assert_eq!(bake(false), (0, true), "no fire, no bread");
}

#[test]
fn the_mill_grinds_while_the_brack_runs_and_not_once_it_freezes() {
    // A bin of wheat in the mill: two sheaves make a scoop of flour in ten minutes. In a hard
    // frost, once the Brack has iced over, the wheel stands still.
    let grind = |frost: bool| {
        let mut pkg = ashford();
        move_to(&mut pkg, FLOUR_BIN, MILL, [0, 80]);
        pkg.deposits
            .iter_mut()
            .find(|d| d.id == FLOUR_BIN)
            .unwrap()
            .made_of = WHEAT;
        if frost {
            pkg.regions
                .iter_mut()
                .find(|r| r.id == VALE)
                .unwrap()
                .temperature_centi_c = -1_500;
        }
        let mut city = City::from(pkg);
        city.run(60);
        gather(&mut city, MILLER, FLOUR_BIN, WHEAT, 2);
        make(&mut city, MILLER, GRIND, 10);
        (holding(&city, MILLER, FLOUR), refused(&city, MILLER))
    };
    assert_eq!(grind(false), (1, false), "flour");
    assert_eq!(grind(true), (0, true), "the Brack is frozen");
}

#[test]
fn planks_are_sawn_only_with_an_edge_to_hand() {
    // Carol fells a log from the stand and saws it at the sawpit: four planks. Without the hammer
    // in hand — the only edge she has — the log stays a log.
    let mut pkg = ashford();
    stand(&mut pkg, CAROL, WOOD, [500, 640], true);
    let mut city = City::from(pkg);
    gather(&mut city, CAROL, TIMBER, TIMBER_M, 1);
    city.drop_(CAROL, HAMMER);
    city.run(3);
    make(&mut city, CAROL, SAW, 15);
    assert!(refused(&city, CAROL), "no edge");
    assert_eq!(holding(&city, CAROL, TIMBER_M), 1, "still a log");
    city.take(CAROL, HAMMER);
    city.run(3);
    make(&mut city, CAROL, SAW, 15);
    assert_eq!(holding(&city, CAROL, TIMBER_M), 4, "four planks");
}

#[test]
fn charcoal_comes_from_the_burning_kiln() {
    // A log in the kiln, an hour: two lumps of charcoal. (Two logs are more than anyone carries:
    // Amendment A-48.)
    let mut pkg = ashford();
    stand(&mut pkg, CAROL, WOOD, [500, 640], true);
    let mut city = City::from(pkg);
    gather(&mut city, CAROL, TIMBER, TIMBER_M, 1);
    make(&mut city, CAROL, BURN_CHARCOAL, 60);
    assert_eq!(holding(&city, CAROL, CHARCOAL), 2);
    assert_eq!(holding(&city, CAROL, TIMBER_M), 0, "the logs went into it");
}

#[test]
fn iron_is_smelted_from_ore_and_charcoal_at_the_forge() {
    // In the miners' cabin, by the forge: the iron seam brought near, and a heap of charcoal.
    // Two lumps of ore and one of charcoal make an iron bar in an hour.
    let mut pkg = ashford();
    stand(&mut pkg, CAROL, CABIN, [200, 0], true);
    move_to(&mut pkg, IRON_SEAM, CABIN, [200, 150]);
    heap(&mut pkg, 9_001, CHARCOAL, CABIN, [300, -50]);
    let mut city = City::from(pkg);
    gather(&mut city, CAROL, IRON_SEAM, IRON_ORE, 2);
    gather(&mut city, CAROL, 9_001, CHARCOAL, 1);
    let iron_before = holding(&city, CAROL, IRON);
    make(&mut city, CAROL, SMELT_IRON, 60);
    assert_eq!(
        holding(&city, CAROL, IRON),
        iron_before + 1,
        "a bar, beside the hammer"
    );
    assert_eq!(holding(&city, CAROL, IRON_ORE), 0);
}
