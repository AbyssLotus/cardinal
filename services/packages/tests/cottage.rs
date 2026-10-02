//! Space as a text world needs it, loaded from a world file (Amendments A-1 to A-4): the
//! cottage world declares a hillside, walled rooms, a door, a ladder, a table, and a villager
//! with somewhere to be — and the engine, given nothing but that intent, walks the villager up
//! the hill, through the door, around the table, and up the ladder to the loft.

use kernel::fact::FactKey;
use kernel::identity::EntityId;
use kernel::system::CommittedView;
use kernel::value::Value;
use packages::{engine_version, load, parse_world, LoadedWorld};
use physical::regions::is_within;
use physical::schema::{CONTAINED_IN, TRAVEL_BLOCKED, TRAVEL_TO};
use physical::shape::body_box;
use physical::sight::line_of_sight;
use physical::space::{height_above_ground, local_position};
use physical::terrain::ground;

const COTTAGE_WORLD: &str = include_str!("../../../worlds/cottage.world");

const HILLSIDE: u64 = 1;
const COTTAGE: u64 = 2;
const KITCHEN: u64 = 3;
const LOFT: u64 = 4;
const VILLAGER: u64 = 10;
const TABLE: u64 = 20;

fn e(id: u64) -> EntityId {
    EntityId::from_raw(id)
}

fn load_cottage() -> LoadedWorld {
    load(&parse_world(COTTAGE_WORLD).unwrap(), engine_version()).unwrap()
}

fn room_of(w: &LoadedWorld, id: u64) -> u64 {
    match w
        .store()
        .read(FactKey::new(e(id), CONTAINED_IN))
        .map(|f| f.value)
    {
        Some(Value::Entity(r)) => r.raw(),
        other => panic!("{id} is nowhere: {other:?}"),
    }
}

#[test]
fn the_world_file_declares_ground_rooms_and_intent() {
    let w = load_cottage();
    let s = w.store();
    // The hillside's terrain is loaded: 1 m up at x = 10 m, 3 m from x = 20 m.
    assert_eq!(ground(s, e(HILLSIDE), 1_000, 0), 100);
    assert_eq!(ground(s, e(HILLSIDE), 2_500, 1_000), 300);
    // The villager is outside, meaning to go to the loft.
    assert_eq!(room_of(&w, VILLAGER), HILLSIDE);
    assert_eq!(
        s.read(FactKey::new(e(VILLAGER), TRAVEL_TO)).unwrap().value,
        Value::Entity(e(LOFT))
    );
    // From the foot of the hill, the kitchen's table is out of sight: walls and a shut-able door.
    assert!(!line_of_sight(s, e(VILLAGER), e(TABLE)));
}

#[test]
fn the_villager_walks_home_up_the_hill_and_the_ladder() {
    let mut w = load_cottage();
    let table = body_box(w.store(), e(TABLE)).unwrap();
    let mut chronicle = Vec::new();
    let mut entered_kitchen = false;
    for t in 1..=120 {
        w.tick(t, 1, &mut chronicle).expect("the cottage ticks");
        let s = w.store();
        // While on the hillside the villager walks on the ground (or settles onto it).
        if room_of(&w, VILLAGER) == HILLSIDE {
            let at = local_position(s, e(VILLAGER));
            let floor = ground(s, e(HILLSIDE), at[0], at[1]);
            assert!(at[2] >= floor - 1, "tick {t}: below the hillside at {at:?}");
        }
        if room_of(&w, VILLAGER) == KITCHEN {
            entered_kitchen = true;
            let at = local_position(s, e(VILLAGER));
            assert!(
                !table.footprint_contains(at, 24),
                "tick {t}: walked through the table at {at:?}"
            );
        }
        if s.read(FactKey::new(e(VILLAGER), TRAVEL_TO)).is_none() {
            break;
        }
    }
    let s = w.store();
    assert!(entered_kitchen, "came in through the kitchen");
    assert_eq!(room_of(&w, VILLAGER), LOFT, "home");
    assert!(s.read(FactKey::new(e(VILLAGER), TRAVEL_TO)).is_none());
    assert!(s.read(FactKey::new(e(VILLAGER), TRAVEL_BLOCKED)).is_none());
    assert!(is_within(s, e(VILLAGER), e(COTTAGE)));
    // The cottage stands 3 m up the hill, the loft 2.6 m above its floor.
    assert_eq!(height_above_ground(s, e(VILLAGER)), 560);
}
