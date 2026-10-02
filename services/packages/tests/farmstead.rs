//! Overlapping regions loaded from a world file (Vol. III Ch. 1 §1.7): a farmstead whose one
//! containment hierarchy (county > farm > farmhouse, field; county > wood, mill) is overlaid by a
//! climate zone, a watershed, a fox's territory that straddles the farm's boundary, and a
//! discontiguous frost hollow. Each region answers a different class of question, and none of
//! them had to be forced into the hierarchy.

use kernel::fact::FactKey;
use kernel::identity::EntityId;
use kernel::system::CommittedView;
use kernel::value::Value;
use packages::{engine_version, load, parse_world, LoadedWorld};
use physical::regions::{is_within, members_of, overlaps, regions_of, shared_regions};
use physical::schema::{ADJACENT_TO, TEMPERATURE};
use std::collections::BTreeSet;

const FARMSTEAD: &str = include_str!("../../../worlds/farmstead.world");

// The hierarchy.
const COUNTY: u64 = 1;
const FARM: u64 = 2;
const FARMHOUSE: u64 = 3;
const FIELD: u64 = 4;
const WOOD: u64 = 5;
const MILL: u64 = 6;
const FARMER: u64 = 10;
const FOX: u64 = 11;
const MILLER: u64 = 12;
// The overlapping regions.
const CLIMATE: u64 = 900;
const WATERSHED: u64 = 901;
const TERRITORY: u64 = 902;
const HOLLOW: u64 = 903;

fn e(id: u64) -> EntityId {
    EntityId::from_raw(id)
}

fn ids(set: BTreeSet<EntityId>) -> Vec<u64> {
    set.into_iter().map(|e| e.raw()).collect()
}

fn load_farmstead() -> LoadedWorld {
    let pkg = parse_world(FARMSTEAD).unwrap();
    load(&pkg, engine_version()).unwrap()
}

#[test]
fn the_farmhouse_lies_in_its_hierarchy_and_its_classifications() {
    let w = load_farmstead();
    let s = w.store();
    // §1.7's farmhouse, in miniature: farm and county from the hierarchy, the watershed through
    // the farm, the climate zone through the county -- one answer.
    assert_eq!(
        ids(regions_of(s, e(FARMHOUSE))),
        vec![COUNTY, FARM, CLIMATE, WATERSHED]
    );
    // Not the fox's territory (it takes in the field, not the house) nor the frost hollow.
    assert!(!is_within(s, e(FARMHOUSE), e(TERRITORY)));
    assert!(!is_within(s, e(FARMHOUSE), e(HOLLOW)));
}

#[test]
fn whoever_is_inside_inherits_every_region_of_their_place() {
    let w = load_farmstead();
    let s = w.store();
    assert_eq!(
        ids(regions_of(s, e(FARMER))),
        vec![COUNTY, FARM, FARMHOUSE, CLIMATE, WATERSHED]
    );
    assert_eq!(
        ids(regions_of(s, e(FOX))),
        vec![COUNTY, WOOD, CLIMATE, TERRITORY]
    );
    assert_eq!(
        ids(regions_of(s, e(MILLER))),
        vec![COUNTY, MILL, CLIMATE, WATERSHED, HOLLOW]
    );
}

#[test]
fn the_territory_overlaps_the_farm_without_either_containing_the_other() {
    let w = load_farmstead();
    let s = w.store();
    // The case a single hierarchy cannot express: the fox ranges over the wood and the farm's
    // far field, so territory and farm overlap -- at the field -- yet neither lies within the
    // other.
    assert!(overlaps(s, e(TERRITORY), e(FARM)));
    assert!(!is_within(s, e(FARM), e(TERRITORY)));
    assert!(!is_within(s, e(TERRITORY), e(FARM)));
    // The farmhouse is outside the territory, so those two do not overlap.
    assert!(!overlaps(s, e(TERRITORY), e(FARMHOUSE)));
    // Through the field, the territory also overlaps the watershed and the frost hollow.
    assert!(overlaps(s, e(TERRITORY), e(WATERSHED)));
    assert!(overlaps(s, e(TERRITORY), e(HOLLOW)));
}

#[test]
fn a_discontiguous_region_is_the_places_that_name_it() {
    let w = load_farmstead();
    let s = w.store();
    // The frost hollow is the field and the mill (and the miller inside it) -- two places that
    // do not border each other.
    assert_eq!(ids(members_of(s, e(HOLLOW))), vec![FIELD, MILL, MILLER]);
    let field_borders: Vec<Value> = s
        .read_all(FactKey::new(e(FIELD), ADJACENT_TO))
        .into_iter()
        .map(|f| f.value)
        .collect();
    assert!(!field_borders.contains(&Value::Entity(e(MILL))));
    // It reaches into the farm (at the field) without taking in the farmhouse.
    assert!(overlaps(s, e(HOLLOW), e(FARM)));
    assert!(!overlaps(s, e(HOLLOW), e(FARMHOUSE)));
}

#[test]
fn different_lenses_find_different_common_ground() {
    let w = load_farmstead();
    let s = w.store();
    // The farmer and the fox share only the county and its climate.
    assert_eq!(
        ids(shared_regions(s, e(FARMER), e(FOX))),
        vec![COUNTY, CLIMATE]
    );
    // The farmer and the miller live on different holdings, but drink from the same creek.
    assert_eq!(
        ids(shared_regions(s, e(FARMER), e(MILLER))),
        vec![COUNTY, CLIMATE, WATERSHED]
    );
}

#[test]
fn classification_regions_carry_no_weather_and_survive_the_day() {
    let mut w = load_farmstead();
    let mut chronicle = Vec::new();
    for t in 1..=24 {
        w.tick(t, 7, &mut chronicle).expect("the farmstead ticks");
    }
    let s = w.store();
    // The weather runs on places, not on classifications: nothing gave 900-903 a temperature.
    for region in [CLIMATE, WATERSHED, TERRITORY, HOLLOW] {
        assert!(s.read(FactKey::new(e(region), TEMPERATURE)).is_none());
    }
    // And a day of weather leaves the memberships exactly as seeded.
    assert_eq!(
        ids(regions_of(s, e(MILLER))),
        vec![COUNTY, MILL, CLIMATE, WATERSHED, HOLLOW]
    );
}
