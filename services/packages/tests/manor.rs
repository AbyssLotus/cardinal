//! Navigational connectivity across a loaded world (Vol. III Ch. 1 §1.5): portals join
//! regions, and reachability walks them -- so the basement reaches the yard only through the
//! ground floor, and the sealed vault is cut off entirely.

use packages::{engine_version, load, parse_world, LoadedWorld};
use physical::space::{can_reach, destinations, reachable_regions};
use std::collections::BTreeSet;

const MANOR: &str = include_str!("../../../worlds/manor.world");

fn e(id: u64) -> kernel::identity::EntityId {
    kernel::identity::EntityId::from_raw(id)
}

fn load_manor() -> LoadedWorld {
    let pkg = parse_world(MANOR).unwrap();
    load(&pkg, engine_version()).unwrap()
}

#[test]
fn the_basement_reaches_the_yard_only_through_the_ground_floor() {
    let w = load_manor();
    let s = w.store();
    // Yard = 1, ground = 2, basement = 3, second = 4, vault = 5.
    assert_eq!(destinations(s, e(3)), vec![e(2)]); // one hop from basement: ground floor only
    assert!(can_reach(s, e(3), e(1))); // the yard is reachable overall
    let reachable: BTreeSet<u64> = reachable_regions(s, e(3)).iter().map(|x| x.raw()).collect();
    assert_eq!(reachable, BTreeSet::from([1, 2, 3, 4]));
}

#[test]
fn the_sealed_vault_is_cut_off() {
    let w = load_manor();
    let s = w.store();
    assert_eq!(reachable_regions(s, e(5)), BTreeSet::from([e(5)]));
    assert!(!can_reach(s, e(3), e(5))); // can't get into the vault
    assert!(!can_reach(s, e(5), e(1))); // ...or out of it
}
