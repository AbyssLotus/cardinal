//! Space in Ashford (Vol. III Ch. 1 §1.4–1.8; Amendments A-2, A-3): where everyone is, nested
//! up through rooms, houses, districts, and the Vale; which way they face and what is on their
//! left; what fills space; who is near whom; and Millside's overlapping regions.

use kernel::spatial::Aabb;
use kernel::value::Value;
use physical::materials::{
    flammability_of, is_flammable, materials_of, structural_hardness, thermal_mass_of,
};
use physical::nearby::{contents, in_box, nearest, within};
use physical::regions::{is_within, members_of, overlaps, regions_of, shared_regions};
use physical::schema::{CONTAINED_IN, MOTION_END, MOTION_START, MOTION_TARGET, POSITION};
use physical::space::{
    distance, heading_in, height_above_ground, local_position, position_in, relative_bearing,
    relative_position,
};
use reference::id::*;
use reference::{e, without_minds, City};
use std::collections::BTreeSet;

fn ids<I: IntoIterator<Item = kernel::identity::EntityId>>(set: I) -> Vec<u64> {
    set.into_iter().map(|x| x.raw()).collect()
}

#[test]
fn everyone_has_a_place_relative_to_everyone_else() {
    let city = City::quiet();
    let s = city.store();
    // Alice stands 1 m in and 1 m north in her bedroom, which is a storey up her house, which
    // is 10 m east in Old Town: in the city's terms she is at (11 m, 1 m, 3 m).
    assert_eq!(
        position_in(s, e(ALICE), e(OLD_TOWN)),
        Some([1_100, 100, 300])
    );
    assert_eq!(position_in(s, e(ALICE), e(REACH)), Some([1_100, 100, 300]));
    // Dave, at the window across the room; Bob, in the kitchen below; Erin, out in the yard.
    assert_eq!(
        relative_position(s, e(ALICE), e(DAVE)),
        Some([-530, -100, 0])
    );
    assert_eq!(distance(s, e(ALICE), e(DAVE)), Some(539));
    assert_eq!(
        distance(s, e(ALICE), e(BOB)),
        Some(449),
        "through the floor"
    );
    assert_eq!(distance(s, e(ALICE), e(ERIN)), Some(1_048));
    // And Hal, 5 km off on Highmoor and 400 m up, in the Reach's terms.
    assert_eq!(
        relative_position(s, e(ALICE), e(HAL)),
        Some([498_900, -100, 39_700])
    );
    assert_eq!(height_above_ground(s, e(ALICE)), 300, "a storey up");
}

#[test]
fn facing_says_what_is_ahead_and_what_is_behind() {
    let mut city = City::quiet();
    // Dave faces west, out of the bedroom window: it is dead ahead, and Alice is behind him.
    let s = city.store();
    assert_eq!(relative_bearing(s, e(DAVE), e(BEDROOM_WINDOW)), Some(0));
    let alice = relative_bearing(s, e(DAVE), e(ALICE)).unwrap();
    assert!(alice.abs() > 16_000, "Alice is behind Dave ({alice})");
    // The harbour guard faces north: the well ahead, the gate to the right, the stables behind.
    assert_eq!(relative_bearing(s, e(GUARD), e(WELL)), Some(0));
    assert_eq!(relative_bearing(s, e(GUARD), e(GATE)), Some(9_000));
    assert_eq!(relative_bearing(s, e(GUARD), e(STABLES)), Some(18_000));
    // Asked to face east, the guard turns: now the gate is ahead and the well on the left.
    city.face(GUARD, 90);
    city.run(2);
    let s = city.store();
    assert_eq!(relative_bearing(s, e(GUARD), e(GATE)), Some(0));
    assert_eq!(relative_bearing(s, e(GUARD), e(WELL)), Some(-9_000));
}

#[test]
fn when_the_heron_comes_about_her_deck_turns_with_her() {
    let mut city = City::quiet();
    let s = city.store();
    // Bow north: the captain, 5 m forward, is 5 m north of the mast in harbour terms.
    assert_eq!(
        position_in(s, e(CAPTAIN), e(HARBOUR)),
        Some([10_000, 500, 0])
    );
    city.face(HERON, 90);
    city.run(2);
    let s = city.store();
    // Nothing about the crew was written; "forward" now points east, starboard south.
    assert_eq!(local_position(s, e(CAPTAIN)), [0, 500, 0]);
    assert_eq!(position_in(s, e(CAPTAIN), e(HARBOUR)), Some([10_500, 0, 0]));
    assert_eq!(
        position_in(s, e(BOSUN), e(HARBOUR)),
        Some([10_000, -200, 0])
    );
    assert_eq!(
        distance(s, e(CAPTAIN), e(BOSUN)),
        Some(538),
        "same spacing on deck"
    );
    assert_eq!(
        heading_in(s, e(CAPTAIN), e(HARBOUR)),
        Some(9_000),
        "facing the bow, east"
    );
}

#[test]
fn bodies_fill_their_size() {
    let city = City::quiet();
    let s = city.store();
    // A box over the kitchen table's north end finds the table, though its base is elsewhere.
    let north_end = Aabb::new([-10, 120, 0], [10, 140, 10]);
    assert_eq!(ids(in_box(s, e(KITCHEN), &north_end)), vec![TABLE]);
    // Above the 80 cm tabletop there is nothing.
    let above = Aabb::new([-10, -10, 90], [10, 10, 200]);
    assert!(in_box(s, e(KITCHEN), &above).is_empty());
}

#[test]
fn who_is_near_alice() {
    let city = City::quiet();
    let s = city.store();
    // Within 6 m, nearest first — measured straight through floors and walls, because
    // nearness is geometry (whether she can *see* or *reach* them are other questions): the
    // kitchen below and its table, the lamp, the apple on the floor, Bob, the cat out in the
    // yard, both faces of the front door and the cellar bulkhead's outer face, Finn and Gwen on
    // the doorstep, Dave, the flour bin by the hearth, and the stairs down. Not her own bedroom or house — she is in those, not near them.
    let near: Vec<(u64, i64)> = within(s, e(ALICE), 600)
        .into_iter()
        .map(|(x, d)| (x.raw(), d))
        .collect();
    assert_eq!(
        near,
        vec![
            (KITCHEN, 331),
            (TABLE, 331),
            (LAMP, 374),
            (APPLE, 422),
            (BOB, 449),
            (CAT, 500),
            (DOOR_OUT, 509),
            (DOOR_IN, 509),
            (BULKHEAD_OUT, 509),
            (FINN, 509),
            (GWEN, 509),
            (DAVE, 539),
            (FLOUR_BIN, 583),
            (STAIRS_DOWN, 583),
        ]
    );
    assert_eq!(
        ids(nearest(s, e(ALICE), 3, 10_000).into_iter().map(|(x, _)| x)),
        vec![KITCHEN, TABLE, LAMP]
    );
    // What the bedroom holds — "you see here…".
    assert_eq!(
        ids(contents(s, e(BEDROOM))),
        vec![STAIRS_DOWN, BEDROOM_WINDOW, ALICE, DAVE]
    );
}

#[test]
fn the_nesting_answers_where_and_within_what() {
    let city = City::quiet();
    let s = city.store();
    let alice: BTreeSet<u64> = ids(regions_of(s, e(ALICE))).into_iter().collect();
    for place in [BEDROOM, HOUSE, OLD_TOWN, ASHFORD, VALE, REACH, TEMPERATE] {
        assert!(alice.contains(&place), "Alice is within {place}");
    }
    assert!(is_within(s, e(ALICE), e(ASHFORD)));
    assert!(!is_within(s, e(HAL), e(ASHFORD)), "Hal is out on the moor");
    assert_eq!(ids(shared_regions(s, e(ALICE), e(HAL))), vec![REACH]);
}

#[test]
fn millside_lies_in_regions_that_overlap_without_nesting() {
    let city = City::quiet();
    let s = city.store();
    // The fox runs the wood and the farm's far field: the run and the farm overlap at the
    // field, though neither lies within the other.
    assert!(overlaps(s, e(FOX_RUN), e(FARM)));
    assert!(!is_within(s, e(FARM), e(FOX_RUN)));
    assert!(!overlaps(s, e(FOX_RUN), e(FARMHOUSE)));
    // The frost hollow is the field and the mill, which do not touch — and what is in the mill.
    assert_eq!(
        ids(members_of(s, e(FROST_HOLLOW))),
        vec![FAR_FIELD, MILL, MILLER, MILLERS_LOAVES]
    );
    // The farmer and the miller live on different holdings but drink from one river.
    let shared: Vec<u64> = ids(shared_regions(s, e(FARMER), e(MILLER)));
    assert!(shared.contains(&WATERSHED) && shared.contains(&MILLSIDE));
    assert!(!ids(shared_regions(s, e(FARMER), e(FOX))).contains(&WATERSHED));
}

#[test]
fn the_rolling_cart_carries_its_rider_without_writing_them() {
    let mut city = City::quiet();
    // The cart was already rolling west at the start: 30 m in 30 s. Ten seconds on, it and
    // its rider are 10 m along, and nothing was written to move the rider.
    city.run(10);
    let s = city.store();
    assert_eq!(position_in(s, e(RIDER), e(OLD_TOWN)), Some([-1_500, 0, 0]));
    assert_eq!(
        local_position(s, e(RIDER)),
        [0, 0, 0],
        "still on the cart's bed"
    );
    // Nothing about where the rider is was written; they did watch the town go by.
    let placed = [
        POSITION,
        CONTAINED_IN,
        MOTION_START,
        MOTION_END,
        MOTION_TARGET,
    ];
    assert!(city
        .chronicle
        .iter()
        .all(|c| c.subject() != e(RIDER) || !placed.contains(&c.fact_type())));
}

#[test]
fn where_one_is_from_the_other_is_where_the_other_is_from_the_one_reversed() {
    let city = City::quiet();
    let s = city.store();
    for (a, b) in [(ALICE, HAL), (BOB, CAPTAIN), (RACCOON, NELL), (FOX, LENA)] {
        let there = relative_position(s, e(a), e(b)).unwrap();
        let back = relative_position(s, e(b), e(a)).unwrap();
        assert_eq!(there.map(|c| -c), back, "{a} and {b}");
    }
    // Something not in the world is nowhere relative to anyone.
    assert_eq!(relative_position(s, e(ALICE), e(99_999)), None);
}

#[test]
fn what_things_are_made_of() {
    let city = City::quiet();
    let s = city.store();
    // The front door is timber banded with iron: as strong as its timber, as flammable as its
    // timber.
    assert_eq!(ids(materials_of(s, e(DOOR_OUT))), vec![TIMBER, IRON]);
    assert_eq!(structural_hardness(s, e(DOOR_OUT)), Some(3_000));
    assert_eq!(flammability_of(s, e(DOOR_OUT)), Some(7_000));
    assert!(is_flammable(s, e(DOOR_OUT)));
    assert!(!is_flammable(s, e(WINDOW_OUT)), "glass does not burn");
    // Heat stored per cubic metre (Amendment A-6): the granite hall holds more than the timber
    // kitchen, though timber stores more per kilogram.
    assert_eq!(thermal_mass_of(s, e(KITCHEN)), Some(1_190));
    assert_eq!(thermal_mass_of(s, e(HALL)), Some(2_133));
    assert_eq!(
        thermal_mass_of(s, e(YARD)),
        None,
        "the yard is built of nothing"
    );
}

#[test]
fn regions_that_lie_in_each_other_do_not_trap_the_question() {
    // Ashford, with the fox's run declared to lie in the frost hollow and the hollow in the run.
    let text =
        reference::ASHFORD.replacen("[in_region]\n", "[in_region]\n902 = 903\n903 = 902\n", 1);
    let city = City::from(without_minds(packages::parse_world(&text).unwrap()));
    let s = city.store();
    let fox = ids(regions_of(s, e(FOX)));
    assert!(fox.contains(&FOX_RUN) && fox.contains(&FROST_HOLLOW));
    assert!(!fox.contains(&FOX), "nothing is within itself");
    assert!(!ids(regions_of(s, e(FOX_RUN))).contains(&FOX_RUN));
}

#[test]
fn the_cart_writes_its_motion_when_it_sets_off_and_when_it_stops_and_never_between() {
    let mut city = City::quiet();
    let cart_writes = |city: &City| {
        city.chronicle
            .iter()
            .filter(|c| c.subject() == e(CART))
            .map(|c| c.fact_type())
            .collect::<Vec<_>>()
    };
    // Rolling, for twenty-nine seconds: nothing about the cart is written.
    city.run(29);
    assert!(cart_writes(&city).is_empty(), "{:?}", cart_writes(&city));
    // On arrival its position is written, once, and its motion is cleared.
    city.run(2);
    let written = cart_writes(&city);
    assert_eq!(
        written.iter().filter(|f| **f == POSITION).count(),
        1,
        "{written:?}"
    );
    for f in [MOTION_START, MOTION_END, MOTION_TARGET] {
        assert!(written.contains(&f), "{f:?} cleared");
        assert_eq!(city.read(CART, f), None);
    }
    assert_eq!(city.read(CART, POSITION), Some(Value::Vec3([-3_500, 0, 0])));
    assert_eq!(
        position_in(city.store(), e(RIDER), e(OLD_TOWN)),
        Some([-3_500, 0, 0])
    );
}
