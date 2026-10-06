//! Jobs (Vol. III Ch. 5 §5.4; Vol. V Ch. 9 §9.3; Amendment A-18): Carol works the orchard round —
//! she picks the apple tree and carries the apples in to the kitchen — and Bob bakes, making pies
//! from what he can find and leaving them in the kitchen. Each works only in their hours, and the
//! hungry eat what they made.

use economy::schema::MADE;
use kernel::fact::FactKey;
use kernel::store::RealityStore;
use kernel::system::{CommittedView, RUNTIME_ID_FLOOR};
use kernel::value::Value;
use minds::schema::{REASON, REASON_WORK};
use physical::schema::{CONSUMED, CONTAINED_IN, MADE_OF};
use reference::id::*;
use reference::{e, well_fed, with_tick_seconds, City};
use resources::schema::{STOCK, UNIT};

const HOUR: u64 = 6; // ten-minute ticks

/// Ashford at ten-minute ticks on a still day, as `change` adjusts it.
fn ashford(change: impl FnOnce(packages::WorldPackage) -> packages::WorldPackage) -> City {
    let mut pkg = with_tick_seconds(600);
    pkg.physical_rules.temperature_variability_centi_c = 0;
    City::from(change(pkg))
}

/// The things in `place` made of `material`.
fn things_in(city: &City, place: u64, material: u64) -> Vec<u64> {
    let store = city.store();
    store
        .entities_with(CONTAINED_IN)
        .into_iter()
        .filter(|t| {
            store.read(FactKey::new(*t, CONTAINED_IN)).map(|f| f.value)
                == Some(Value::Entity(e(place)))
        })
        .filter(|t| {
            store
                .read_all(FactKey::new(*t, MADE_OF))
                .iter()
                .any(|m| m.value == Value::Entity(e(material)))
        })
        .map(|t| t.raw())
        .collect()
}

/// Run `ticks`, noting whether `who` ever worked.
fn watch_work(city: &mut City, who: u64, ticks: u64) -> bool {
    let mut worked = false;
    for _ in 0..ticks {
        city.run(1);
        worked |= city.int(who, REASON) == Some(REASON_WORK);
    }
    worked
}

#[test]
fn before_their_hours_nobody_works() {
    let mut city = ashford(well_fed);
    for _ in 0..6 * HOUR - 1 {
        city.run(1);
        for who in [CAROL, BOB] {
            assert_ne!(
                city.int(who, REASON),
                Some(REASON_WORK),
                "{who} at {}",
                city.tick
            );
        }
    }
}

#[test]
fn carol_brings_the_harvest_in() {
    let mut city = ashford(well_fed);
    city.run(6 * HOUR);
    assert!(watch_work(&mut city, CAROL, 10 * HOUR), "she worked");
    // By four in the afternoon the kitchen floor holds apples the tree grew that morning.
    let grown: Vec<u64> = things_in(&city, KITCHEN, APPLE_STUFF)
        .into_iter()
        .filter(|a| *a >= RUNTIME_ID_FLOOR)
        .collect();
    assert!(grown.len() >= 4, "{} apples brought in", grown.len());
}

#[test]
fn bob_bakes_pies_for_the_kitchen() {
    let mut city = ashford(well_fed);
    city.run(8 * HOUR);
    assert!(watch_work(&mut city, BOB, 12 * HOUR), "he worked");
    let pies = things_in(&city, KITCHEN, PIE);
    assert!(!pies.is_empty(), "a pie in the kitchen by eight");
    let Some(Value::Entity(made)) = city.read(BOB, MADE) else {
        panic!("he made nothing");
    };
    assert!(made.raw() >= RUNTIME_ID_FLOOR);
    // The pies took flour from the bin, which the mill tops up more slowly than he scoops.
    let flour = city.int(FLOUR_BIN, STOCK).unwrap();
    assert!(flour < 20 * UNIT, "{flour}");
}

#[test]
fn the_hungry_eat_the_pies() {
    let mut city = ashford(|p| p);
    let mut eaten = Vec::new();
    for _ in 0..30 * HOUR {
        city.run(1);
        for who in city.store().entities_with(CONSUMED) {
            let f = city.store().read(FactKey::new(who, CONSUMED)).unwrap();
            let Value::Entity(thing) = f.value else {
                continue;
            };
            let pie = city
                .store()
                .read_all(FactKey::new(thing, MADE_OF))
                .iter()
                .any(|m| m.value == Value::Entity(e(PIE)));
            if pie && f.provenance.tick == city.tick {
                eaten.push((who.raw(), thing.raw()));
            }
        }
    }
    assert!(!eaten.is_empty(), "nobody ate a pie");
}

#[test]
fn the_same_seed_works_the_same_day() {
    let day = || {
        let mut city = ashford(well_fed);
        city.run(20 * HOUR);
        city.store().state_hash()
    };
    assert_eq!(day(), day());
}
