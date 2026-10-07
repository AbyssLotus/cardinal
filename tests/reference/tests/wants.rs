//! Wants (Vol. III Ch. 4 §4.4; Appendix A, Ruling 18; Amendment A-19): the curious courier goes to
//! see the rooms past the door he can see, and stops once he has stood in each; Erin, who likes
//! glass, picks up the bottle no one owns, claims it, and keeps it in her bedroom — and leaves
//! Bob's glass lamp alone, as she would the bottle were it his. A claim needs the thing in hand
//! and no owner already.

use economy::schema::{CLAIM_REFUSED, OWNER};
use information::schema::WARMTH_OF;
use kernel::fact::{Cause, FactKey};
use kernel::system::CommittedView;
use kernel::value::Value;
use minds::schema::{GOAL, REASON, REASON_CURIOSITY};
use physical::schema::CONTAINED_IN;
use reference::id::*;
use reference::{e, well_fed, with_tick_seconds, without_minds, City};

const HOUR: u64 = 6; // ten-minute ticks

/// Ashford at ten-minute ticks on a still day, well fed, as `change` adjusts it.
fn ashford(change: impl FnOnce(&mut packages::WorldPackage)) -> City {
    let mut pkg = well_fed(with_tick_seconds(600));
    pkg.physical_rules.temperature_variability_centi_c = 0;
    change(&mut pkg);
    City::from(pkg)
}

fn has_stood_in(city: &City, who: u64, place: u64) -> bool {
    city.store()
        .read(FactKey::pair(e(who), WARMTH_OF, e(place)))
        .is_some()
}

#[test]
fn the_curious_courier_goes_to_see_the_house() {
    let mut city = ashford(|_| {});
    assert!(!has_stood_in(&city, COURIER, KITCHEN));
    let mut curious = false;
    for _ in 0..12 * HOUR {
        city.run(1);
        curious |= city.int(COURIER, REASON) == Some(REASON_CURIOSITY);
    }
    assert!(curious, "he went for curiosity's sake");
    for room in [KITCHEN, BEDROOM] {
        assert!(has_stood_in(&city, COURIER, room), "he has seen {room}");
    }
    // Having seen what he knows a way to, he is no longer drawn anywhere.
    assert_eq!(city.read(COURIER, GOAL), None);
}

#[test]
fn erin_keeps_the_bottle_in_her_bedroom_and_leaves_bobs_lamp() {
    let mut city = ashford(|_| {});
    city.run(12 * HOUR);
    assert_eq!(city.read(BOTTLE, OWNER), Some(Value::Entity(e(ERIN))));
    assert_eq!(
        city.read(BOTTLE, CONTAINED_IN),
        Some(Value::Entity(e(BEDROOM)))
    );
    // The lamp is glass too, and she passed it in the kitchen — but it is Bob's.
    assert_eq!(city.read(LAMP, OWNER), Some(Value::Entity(e(BOB))));
    assert_eq!(
        city.read(LAMP, CONTAINED_IN),
        Some(Value::Entity(e(KITCHEN)))
    );
}

#[test]
fn what_is_someone_elses_is_left_where_it_lies() {
    // The bottle is Bob's: Erin sees it, believes it his, and leaves it be.
    let mut city = ashford(|p| p.owners.push((BOTTLE, BOB)));
    city.run(12 * HOUR);
    assert_eq!(
        city.read(BOTTLE, CONTAINED_IN),
        Some(Value::Entity(e(YARD)))
    );
    assert_eq!(city.read(BOTTLE, OWNER), Some(Value::Entity(e(BOB))));
}

#[test]
fn a_claim_needs_the_thing_in_hand_and_no_owner() {
    let mut city = City::from(without_minds(with_tick_seconds(60)));
    let refusal = |city: &City| {
        let f = city
            .store()
            .read(FactKey::new(e(ERIN), CLAIM_REFUSED))
            .unwrap();
        f.provenance.cause
    };
    // Not in hand.
    city.claim(ERIN, BOTTLE);
    city.run(3);
    assert_eq!(refusal(&city), Cause::new("not_carried"));
    assert_eq!(city.read(BOTTLE, OWNER), None);
    // In hand, and no one's: hers.
    city.go(ERIN, BOTTLE, 140);
    city.run(2);
    city.run_until(60, |c| !c.travelling(ERIN));
    city.take(ERIN, BOTTLE);
    city.run(3);
    city.claim(ERIN, BOTTLE);
    city.run(3);
    assert_eq!(city.read(BOTTLE, OWNER), Some(Value::Entity(e(ERIN))));
    assert_eq!(
        city.read(ERIN, CLAIM_REFUSED),
        None,
        "the refusal is cleared"
    );
    // Bob, carrying it off, cannot make it his by claiming it.
    city.go(BOB, ERIN, 140);
    city.run(2);
    city.run_until(60, |c| !c.travelling(BOB));
    city.drop_(ERIN, BOTTLE);
    city.run(3);
    city.take(BOB, BOTTLE);
    city.run(3);
    assert_eq!(city.read(BOTTLE, CONTAINED_IN), Some(Value::Entity(e(BOB))));
    city.claim(BOB, BOTTLE);
    city.run(3);
    let refused = city
        .store()
        .read(FactKey::new(e(BOB), CLAIM_REFUSED))
        .unwrap();
    assert_eq!(refused.provenance.cause, Cause::new("owned_already"));
    assert_eq!(city.read(BOTTLE, OWNER), Some(Value::Entity(e(ERIN))));
}
