//! Things that come into being (Vol. V Ch. 2 §2.1; Vol. III Ch. 3; Appendix A, Ruling 16;
//! Amendment A-15): the apple tree ripens fruit at its rate, up to what it can hold; picking it
//! brings a new apple — a new thing, with a new id — into the picker's hand; a bare tree yields
//! nothing; and the same world makes the same things.

use kernel::fact::FactKey;
use kernel::system::{CommittedView, RUNTIME_ID_FLOOR};
use kernel::value::Value;
use physical::schema::{CONTAINED_IN, MADE_OF};
use reference::id::*;
use reference::{e, with_tick_seconds, without_minds, City};
use resources::schema::{STOCK, UNIT};

const HOUR: u64 = 6; // ten-minute ticks

fn stock(city: &City) -> i64 {
    city.int(TREE, STOCK).unwrap()
}

/// The things in `who`'s hand that did not exist when the world began.
fn new_things_held(city: &City, who: u64) -> Vec<u64> {
    city.store()
        .entities_with(CONTAINED_IN)
        .into_iter()
        .filter(|t| t.raw() >= RUNTIME_ID_FLOOR)
        .filter(|t| {
            city.store()
                .read(FactKey::new(*t, CONTAINED_IN))
                .map(|f| f.value)
                == Some(Value::Entity(e(who)))
        })
        .map(|t| t.raw())
        .collect()
}

#[test]
fn the_tree_ripens_up_to_what_it_can_hold() {
    // Six apples at dawn of the world, twenty more a day, never more than thirty.
    let mut city = City::from(without_minds(with_tick_seconds(600)));
    assert_eq!(stock(&city), 6 * UNIT);
    city.run(24 * HOUR);
    let a_day_on = stock(&city);
    assert!((25 * UNIT..=27 * UNIT).contains(&a_day_on), "{a_day_on}");
    city.run(24 * HOUR);
    assert_eq!(stock(&city), 30 * UNIT, "and no more");
}

#[test]
fn picking_brings_a_new_apple_into_the_hand() {
    let mut city = City::quiet();
    city.go(CAROL, TREE, 140);
    city.run_until(60, |c| !c.travelling(CAROL));
    city.pick(CAROL, TREE);
    city.run(4);
    let held = new_things_held(&city, CAROL);
    assert_eq!(held.len(), 1, "one new apple in her hand");
    let apple = held[0];
    assert_eq!(
        city.store()
            .read(FactKey::new(e(apple), MADE_OF))
            .map(|f| f.value),
        Some(Value::Entity(e(APPLE_STUFF)))
    );
    assert_eq!(stock(&city), 5 * UNIT, "one fewer on the tree");
    // And it is a thing like any other: she can eat it.
    city.consume(CAROL, apple);
    city.run(3);
    assert_eq!(city.read(apple, CONTAINED_IN), None);
}

#[test]
fn a_bare_tree_yields_nothing() {
    let mut pkg = without_minds(reference::package());
    pkg.deposits[0].stock = 0;
    pkg.deposits[0].per_day = 0;
    let mut city = City::from(pkg);
    city.go(CAROL, TREE, 140);
    city.run_until(60, |c| !c.travelling(CAROL));
    city.pick(CAROL, TREE);
    city.run(4);
    assert!(
        new_things_held(&city, CAROL).is_empty(),
        "the hand comes away empty"
    );
    assert_eq!(stock(&city), 0);
}

#[test]
fn the_same_world_makes_the_same_things() {
    let run = || {
        let mut city = City::quiet();
        city.go(CAROL, TREE, 140);
        city.run_until(60, |c| !c.travelling(CAROL));
        city.pick(CAROL, TREE);
        city.run(4);
        new_things_held(&city, CAROL)
    };
    let (a, b) = (run(), run());
    assert_eq!(a, b);
    assert!(a[0] >= RUNTIME_ID_FLOOR);
}
