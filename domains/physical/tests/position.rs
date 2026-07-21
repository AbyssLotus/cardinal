//! Relative position between loaded entities across a nested containment hierarchy
//! (Vol. III Ch. 1 §1.8, §1.12). The house example: people located within rooms, rooms
//! within a house, houses within a city -- relative position is found by composing local
//! positions up to whichever region contains both.

use kernel::fact::{Cause, Fact, FactKey, FactType, Provenance, SystemId};
use kernel::identity::EntityId;
use kernel::store::MemoryStore;
use kernel::value::Value;
use physical::schema::{CONTAINED_IN, POSITION_X, POSITION_Y, POSITION_Z};
use physical::space::{distance, position_in, relative_position};

fn e(id: u64) -> EntityId {
    EntityId::from_raw(id)
}

fn prov() -> Provenance {
    Provenance::new(SystemId::new("worldgen"), 0, Cause::new("seed"))
}

fn seed(store: &mut MemoryStore, entity: u64, fact_type: FactType, v: Value) {
    store.seed(FactKey::new(e(entity), fact_type), Fact::new(v, prov()));
}

/// Place `entity` inside `container` at local position (x, y, z) centimetres.
fn place(store: &mut MemoryStore, entity: u64, container: u64, x: i64, y: i64, z: i64) {
    seed(store, entity, CONTAINED_IN, Value::Entity(e(container)));
    seed(store, entity, POSITION_X, Value::Int(x));
    seed(store, entity, POSITION_Y, Value::Int(y));
    seed(store, entity, POSITION_Z, Value::Int(z));
}

/// A small world: a city holding a house and a shed; the house holding a bedroom and a
/// kitchen; people in the rooms. Distances in centimetres.
fn world() -> MemoryStore {
    let mut s = MemoryStore::new();
    // city (100) is the root. house (1) and shed (4) sit within it.
    place(&mut s, 1, 100, 0, 0, 0); // house at the city origin
    place(&mut s, 4, 100, 2000, 0, 0); // shed 20 m east of the house
                                       // rooms within the house.
    place(&mut s, 2, 1, 0, 0, 0); // bedroom at the house origin
    place(&mut s, 3, 1, 500, 0, 0); // kitchen 5 m east within the house
                                    // people within the rooms.
    place(&mut s, 10, 2, 100, 100, 0); // alice in the bedroom
    place(&mut s, 40, 2, 100, 300, 0); // dave in the bedroom, 2 m north of alice
    place(&mut s, 20, 3, 50, 50, 0); // bob in the kitchen
    place(&mut s, 30, 4, 10, 10, 0); // carol in the shed
    s
}

#[test]
fn same_room_relative_position() {
    let s = world();
    // Alice and Dave share the bedroom: 2 m apart on the Y axis.
    assert_eq!(relative_position(&s, e(10), e(40)), Some([0, 200, 0]));
    assert_eq!(distance(&s, e(10), e(40)), Some(200));
}

#[test]
fn across_rooms_via_the_house_frame() {
    let s = world();
    // Alice (bedroom) to Bob (kitchen): composed in the house's frame.
    // Alice in house = (0,0,0)+(100,100,0); Bob = (500,0,0)+(50,50,0).
    assert_eq!(relative_position(&s, e(10), e(20)), Some([450, -50, 0]));
    assert_eq!(distance(&s, e(10), e(20)), Some(452)); // floor(sqrt(205000))
}

#[test]
fn zoomed_out_via_the_city_frame() {
    let s = world();
    // Alice (bedroom, in house) to Carol (shed): their lowest common region is the city.
    // Alice in city = (0,0,0)+(0,0,0)+(100,100,0); Carol = (2000,0,0)+(10,10,0).
    assert_eq!(relative_position(&s, e(10), e(30)), Some([1910, -90, 0]));
    assert_eq!(distance(&s, e(10), e(30)), Some(1912));
}

#[test]
fn position_in_an_ancestor_frame() {
    let s = world();
    // Alice's position within the house, and within the city (house sits at the city origin).
    assert_eq!(position_in(&s, e(10), e(1)), Some([100, 100, 0]));
    assert_eq!(position_in(&s, e(10), e(100)), Some([100, 100, 0]));
    // The kitchen's own position within the house.
    assert_eq!(position_in(&s, e(3), e(1)), Some([500, 0, 0]));
}

#[test]
fn relative_position_is_antisymmetric() {
    let s = world();
    let ab = relative_position(&s, e(10), e(20)).unwrap();
    let ba = relative_position(&s, e(20), e(10)).unwrap();
    assert_eq!(ba, [-ab[0], -ab[1], -ab[2]]);
}

#[test]
fn entities_in_unconnected_hierarchies_have_no_relative_position() {
    let mut s = world();
    // An astronaut on the moon shares no container with anyone in the city.
    place(&mut s, 50, 500, 0, 0, 0); // moon (500) is a separate root
    assert_eq!(relative_position(&s, e(10), e(50)), None);
    assert_eq!(distance(&s, e(10), e(50)), None);
}
