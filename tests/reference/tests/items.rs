//! Things in hand (Vol. III Ch. 1, *Handling things*; Appendix A, Ruling 16; Amendment A-12):
//! taking what is within reach and light enough, carrying it where one goes, putting it down, and
//! consuming what is edible — after which it is nowhere, though it is not forgotten.

use information::schema::MADE_OF as BELIEF_MADE_OF;
use kernel::fact::{Cause, FactKey};
use kernel::system::CommittedView;
use kernel::value::Value;
use physical::schema::{ACT_REFUSED, CONSUMED, CONTAINED_IN, MADE_OF, POSITION};
use physical::space::{local_position, position_in};
use reference::id::*;
use reference::{e, with_tick_seconds, without_minds, City};

#[test]
fn bob_eats_the_apple_and_it_is_gone() {
    let mut city = City::quiet();
    city.take(BOB, APPLE);
    city.run(2);
    assert_eq!(city.room_of(APPLE), BOB, "in his hand");
    city.consume(BOB, APPLE);
    city.run(2);
    // Nowhere now...
    assert_eq!(city.read(APPLE, CONTAINED_IN), None);
    assert_eq!(city.read(APPLE, POSITION), None);
    assert_eq!(city.read(BOB, CONSUMED), Some(Value::Entity(e(APPLE))));
    // ...but not forgotten: what it was, and that it was eaten.
    assert_eq!(
        city.store()
            .read(FactKey::new(e(APPLE), MADE_OF))
            .map(|f| f.value),
        Some(Value::Entity(e(APPLE_STUFF)))
    );
    assert!(city
        .chronicle
        .iter()
        .any(|c| c.subject() == e(APPLE) && c.cause() == Cause::new("consumed")));
}

#[test]
fn bob_carries_the_lamp_upstairs_and_puts_it_down() {
    let mut city = City::quiet();
    city.go(BOB, LAMP, 140);
    city.run_until(20, |c| !c.travelling(BOB));
    city.take(BOB, LAMP);
    city.run(2);
    assert_eq!(city.room_of(LAMP), BOB);
    // Up the stairs: the lamp goes where he goes, without a word written about it.
    city.go(BOB, BEDROOM, 140);
    city.run_until(30, |c| !c.travelling(BOB));
    assert_eq!(city.room_of(BOB), BEDROOM);
    assert_eq!(
        position_in(city.store(), e(LAMP), e(HOUSE)),
        position_in(city.store(), e(BOB), e(HOUSE))
    );
    // Put down, it stands in the bedroom where he stood.
    city.drop_(BOB, LAMP);
    city.run(2);
    assert_eq!(city.room_of(LAMP), BEDROOM);
    assert_eq!(
        local_position(city.store(), e(LAMP)),
        local_position(city.store(), e(BOB))
    );
}

#[test]
fn the_courier_cannot_lift_the_wardrobe() {
    // A tonne of timber, against a carrying limit of 25 kg.
    let mut city = City::quiet();
    city.go(COURIER, WARDROBE, 140);
    city.run_until(20, |c| !c.travelling(COURIER));
    city.take(COURIER, WARDROBE);
    city.run(2);
    assert_eq!(city.room_of(WARDROBE), YARD);
    assert_eq!(
        city.read(COURIER, ACT_REFUSED),
        Some(Value::Entity(e(WARDROBE)))
    );
}

#[test]
fn what_is_out_of_reach_or_inedible_stays_as_it_is() {
    let mut city = City::quiet();
    // Erin, out in the yard, cannot take the apple on the kitchen floor.
    city.take(ERIN, APPLE);
    city.run(2);
    assert_eq!(city.room_of(APPLE), KITCHEN);
    assert_eq!(city.read(ERIN, ACT_REFUSED), Some(Value::Entity(e(APPLE))));
    // Bob can take the lamp, but not eat it.
    city.go(BOB, LAMP, 140);
    city.run_until(20, |c| !c.travelling(BOB));
    city.take(BOB, LAMP);
    city.run(2);
    city.consume(BOB, LAMP);
    city.run(2);
    assert_eq!(city.room_of(LAMP), BOB, "still in his hand");
    assert_eq!(city.read(BOB, ACT_REFUSED), Some(Value::Entity(e(LAMP))));
}

#[test]
fn what_one_sees_one_knows_the_stuff_of() {
    // At noon in the kitchen, Bob can see what the apple is.
    let mut city = City::from(without_minds(with_tick_seconds(600)));
    city.run(74);
    let believed: Vec<Value> = city
        .store()
        .read_all(FactKey::pair(e(BOB), BELIEF_MADE_OF, e(APPLE)))
        .into_iter()
        .map(|f| f.value)
        .collect();
    assert_eq!(believed, vec![Value::Entity(e(APPLE_STUFF))]);
}
