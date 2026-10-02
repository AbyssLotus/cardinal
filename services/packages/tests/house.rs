//! Relative position across a nested containment hierarchy, loaded from a world file
//! (Vol. III Ch. 1 §1.8, §1.12): people in rooms, rooms in a house, a house and a shed in a
//! city -- and the relative position of any two, in the frame of whichever region holds both.

use packages::{engine_version, load, parse_world, LoadedWorld};
use physical::materials::{flammability_of, is_flammable, materials_of, structural_hardness};
use physical::nearby::within;
use physical::space::{distance, relative_bearing, relative_position};

const HOUSE: &str = include_str!("../../../worlds/house.world");

fn e(id: u64) -> kernel::identity::EntityId {
    kernel::identity::EntityId::from_raw(id)
}

fn load_house() -> LoadedWorld {
    let pkg = parse_world(HOUSE).unwrap();
    load(&pkg, engine_version()).unwrap()
}

#[test]
fn people_have_relative_positions_from_the_world_file() {
    let w = load_house();
    let s = w.store();

    // Same room (bedroom): alice -> dave, 2 m apart.
    assert_eq!(relative_position(s, e(10), e(40)), Some([0, 200, 0]));

    // Across rooms via the house frame: alice (bedroom) -> bob (kitchen).
    assert_eq!(relative_position(s, e(10), e(20)), Some([450, -50, 0]));
    assert_eq!(distance(s, e(10), e(20)), Some(452));

    // Zoomed out via the city frame: alice (in the house) -> carol (in the shed).
    assert_eq!(relative_position(s, e(10), e(30)), Some([1910, -90, 0]));
    assert_eq!(distance(s, e(10), e(30)), Some(1912));
}

#[test]
fn materials_load_and_compose_from_the_world_file() {
    let w = load_house();
    let s = w.store();

    // The bedroom door (50) is a composite of timber (700) and iron (702), seeded from
    // [made_of]; the materials are listed in deterministic id order.
    assert_eq!(
        materials_of(s, e(50))
            .iter()
            .map(|m| m.raw())
            .collect::<Vec<_>>(),
        vec![700, 702]
    );

    // A structure is only as strong as its weakest material: the timber (3000), not the iron
    // band (9000), governs how the door fails (Vol. III Ch. 1 §1.9).
    assert_eq!(structural_hardness(s, e(50)), Some(3000));

    // ...and it burns as readily as its most flammable part: the timber (7000).
    assert_eq!(flammability_of(s, e(50)), Some(7000));
    assert!(is_flammable(s, e(50)));

    // The window (51) is glass only -- inert, so it cannot burn, and its hardness is the
    // glass's own.
    assert!(!is_flammable(s, e(51)));
    assert_eq!(structural_hardness(s, e(51)), Some(5500));
}

#[test]
fn people_face_somewhere_and_know_what_is_beside_them() {
    let w = load_house();
    let s = w.store();
    // Dave stands 2 m north of Alice, facing south: she is straight ahead of him...
    assert_eq!(relative_bearing(s, e(40), e(10)), Some(0));
    // ...and Alice, facing east, has Dave on her left.
    assert_eq!(relative_bearing(s, e(10), e(40)), Some(-9_000));
    // Who and what is within 2.5 m of Alice, nearest first? The door (1.1 m), Dave (2 m), and
    // the window (2.2 m) — not the bedroom or the house, which she is inside.
    let near: Vec<(u64, i64)> = within(s, e(10), 250)
        .iter()
        .map(|(x, d)| (x.raw(), *d))
        .collect();
    assert_eq!(near, vec![(50, 111), (40, 200), (51, 223)]);
}
