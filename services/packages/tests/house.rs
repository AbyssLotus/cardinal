//! Relative position across a nested containment hierarchy, loaded from a world file
//! (Vol. III Ch. 1 §1.8, §1.12): people in rooms, rooms in a house, a house and a shed in a
//! city -- and the relative position of any two, in the frame of whichever region holds both.

use packages::{engine_version, load, parse_world, LoadedWorld};
use physical::space::{distance, relative_position};

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
