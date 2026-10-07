//! Making things (Vol. III Ch. 4 §4.4; Appendix A, Ruling 17; Amendment A-17): at the hearth, two
//! apples and a scoop of flour become a pie. The inputs leave the world as the making begins; an
//! hour later the pie is in the baker's hand, or set down in front of the hearth if the baker has
//! gone. Without the
//! flour, or away from the hearth, nothing is made, and the apples stay in hand.

use economy::schema::{MADE, MAKE_REFUSED, MAKING};
use kernel::fact::{Cause, FactKey};
use kernel::system::CommittedView;
use kernel::value::Value;
use physical::schema::{CONTAINED_IN, MADE_OF};
use reference::id::*;
use reference::{e, with_tick_seconds, without_minds, City};

const HOUR: u64 = 60; // one-minute ticks

/// Ashford without minds, at one-minute ticks: nobody acts unless told.
fn quiet() -> City {
    City::from(without_minds(with_tick_seconds(60)))
}

/// What `who` carries: each thing with what it is made of, in ascending id.
fn carried(city: &City, who: u64) -> Vec<(u64, u64)> {
    let store = city.store();
    store
        .entities_with(CONTAINED_IN)
        .into_iter()
        .filter(|t| {
            store.read(FactKey::new(*t, CONTAINED_IN)).map(|f| f.value)
                == Some(Value::Entity(e(who)))
        })
        .filter_map(
            |t| match store.read(FactKey::new(t, MADE_OF)).map(|f| f.value) {
                Some(Value::Entity(m)) => Some((t.raw(), m.raw())),
                _ => None,
            },
        )
        .collect()
}

fn materials(city: &City, who: u64) -> Vec<u64> {
    let mut m: Vec<u64> = carried(city, who).into_iter().map(|(_, m)| m).collect();
    m.sort_unstable();
    m
}

/// Send `who` to `thing` and wait until there.
fn walk_to(city: &mut City, who: u64, thing: u64) {
    city.go(who, thing, 140);
    city.run(2);
    city.run_until(60, |c| !c.travelling(who));
}

/// `who` picks from `deposit`, and the new thing is in hand: the order, the pick, its yield, and
/// the arrival take a tick each.
fn pick(city: &mut City, who: u64, deposit: u64) {
    city.pick(who, deposit);
    city.run(4);
}

/// Bob, from the kitchen, picks two apples from the tree in the yard and comes back in.
fn bob_with_two_apples() -> City {
    let mut city = quiet();
    walk_to(&mut city, BOB, TREE);
    pick(&mut city, BOB, TREE);
    pick(&mut city, BOB, TREE);
    walk_to(&mut city, BOB, HEARTH);
    assert_eq!(city.room_of(BOB), KITCHEN);
    assert_eq!(materials(&city, BOB), [APPLE_STUFF, APPLE_STUFF]);
    city
}

#[test]
fn apples_and_flour_become_a_pie_at_the_hearth() {
    let mut city = bob_with_two_apples();
    walk_to(&mut city, BOB, FLOUR_BIN);
    pick(&mut city, BOB, FLOUR_BIN);
    assert_eq!(materials(&city, BOB), [APPLE_STUFF, APPLE_STUFF, FLOUR]);
    walk_to(&mut city, BOB, HEARTH);
    city.make(BOB, APPLE_PIE);
    city.run(3);
    // The apples and the flour are gone into the making.
    assert!(carried(&city, BOB).is_empty(), "{:?}", carried(&city, BOB));
    assert_eq!(city.read(BOB, MAKING), Some(Value::Entity(e(APPLE_PIE))));
    // An hour on, a pie in his hand: a new thing, made of pie.
    city.run(HOUR);
    let held = carried(&city, BOB);
    assert_eq!(held.len(), 1);
    let (pie, made_of) = held[0];
    assert_eq!(made_of, PIE);
    assert_eq!(city.read(BOB, MADE), Some(Value::Entity(e(pie))));
    assert_eq!(city.read(BOB, MAKING), None);
}

#[test]
fn without_the_flour_nothing_is_made() {
    let mut city = bob_with_two_apples();
    city.make(BOB, APPLE_PIE);
    city.run(3);
    let refused = city
        .store()
        .read(FactKey::new(e(BOB), MAKE_REFUSED))
        .unwrap();
    assert_eq!(refused.value, Value::Entity(e(APPLE_PIE)));
    assert_eq!(refused.provenance.cause, Cause::new("lacks_inputs"));
    city.run(HOUR);
    assert_eq!(
        materials(&city, BOB),
        [APPLE_STUFF, APPLE_STUFF],
        "the apples stay in hand"
    );
    assert_eq!(city.read(BOB, MADE), None);
}

#[test]
fn away_from_the_hearth_nothing_is_made() {
    let mut city = bob_with_two_apples();
    walk_to(&mut city, BOB, FLOUR_BIN);
    pick(&mut city, BOB, FLOUR_BIN);
    walk_to(&mut city, BOB, TREE);
    city.make(BOB, APPLE_PIE);
    city.run(3);
    let refused = city
        .store()
        .read(FactKey::new(e(BOB), MAKE_REFUSED))
        .unwrap();
    assert_eq!(refused.provenance.cause, Cause::new("away_from_work"));
    assert_eq!(materials(&city, BOB), [APPLE_STUFF, APPLE_STUFF, FLOUR]);
}

#[test]
fn a_pie_left_baking_waits_on_the_hearth() {
    let mut city = bob_with_two_apples();
    walk_to(&mut city, BOB, FLOUR_BIN);
    pick(&mut city, BOB, FLOUR_BIN);
    walk_to(&mut city, BOB, HEARTH);
    city.make(BOB, APPLE_PIE);
    city.run(3);
    // He goes out to the yard while it bakes.
    walk_to(&mut city, BOB, TREE);
    city.run(HOUR);
    assert!(carried(&city, BOB).is_empty());
    let Some(Value::Entity(pie)) = city.read(BOB, MADE) else {
        panic!("nothing made");
    };
    assert_eq!(
        city.read(pie.raw(), CONTAINED_IN),
        Some(Value::Entity(e(KITCHEN)))
    );
    // Set down in front of the hearth, where he can walk up to it and take it.
    walk_to(&mut city, BOB, pie.raw());
    city.take(BOB, pie.raw());
    city.run(3);
    assert_eq!(materials(&city, BOB), [PIE]);
}
