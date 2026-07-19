//! Region adjacency: a cardinality-many spatial relationship (Vol. III Ch. 1 §1.5).
//!
//! Neighbours are read as a set and walked to answer reachability -- the topology other
//! domains stand on. Adjacency is seeded state here; distinct topologies (roads, rivers)
//! would be distinct fact types over the same regions (§1.5, No Single Topology).

use kernel::fact::{Cause, Fact, FactKey, Provenance, SystemId};
use kernel::identity::EntityId;
use kernel::store::MemoryStore;
use kernel::system::CommittedView;
use kernel::value::Value;
use physical::schema::ADJACENT_TO;
use std::collections::{BTreeSet, VecDeque};

fn edge(store: &mut MemoryStore, a: u64, b: u64) {
    let prov = Provenance::new(SystemId::new("worldgen"), 0, Cause::new("seed"));
    // Undirected: seed both directions of the relationship.
    store.seed(
        FactKey::new(EntityId::from_raw(a), ADJACENT_TO),
        Fact::new(Value::Entity(EntityId::from_raw(b)), prov),
    );
    store.seed(
        FactKey::new(EntityId::from_raw(b), ADJACENT_TO),
        Fact::new(Value::Entity(EntityId::from_raw(a)), prov),
    );
}

fn neighbours(store: &MemoryStore, region: u64) -> Vec<u64> {
    let mut v: Vec<u64> = store
        .read_all(FactKey::new(EntityId::from_raw(region), ADJACENT_TO))
        .into_iter()
        .filter_map(|f| match f.value {
            Value::Entity(e) => Some(e.raw()),
            _ => None,
        })
        .collect();
    v.sort_unstable();
    v
}

fn reachable(store: &MemoryStore, start: u64) -> BTreeSet<u64> {
    let mut seen = BTreeSet::new();
    let mut queue = VecDeque::new();
    seen.insert(start);
    queue.push_back(start);
    while let Some(r) = queue.pop_front() {
        for n in neighbours(store, r) {
            if seen.insert(n) {
                queue.push_back(n);
            }
        }
    }
    seen
}

#[test]
fn a_region_has_several_neighbours() {
    // 1 -- 2 -- 3, and 2 -- 4: region 2 borders three others (a set-valued fact).
    let mut store = MemoryStore::new();
    edge(&mut store, 1, 2);
    edge(&mut store, 2, 3);
    edge(&mut store, 2, 4);
    assert_eq!(neighbours(&store, 2), vec![1, 3, 4]);
    assert_eq!(neighbours(&store, 1), vec![2]);
}

#[test]
fn reachability_walks_the_topology() {
    // Two disconnected components: {1,2,3} and {10,11}.
    let mut store = MemoryStore::new();
    edge(&mut store, 1, 2);
    edge(&mut store, 2, 3);
    edge(&mut store, 10, 11);
    assert_eq!(reachable(&store, 1), BTreeSet::from([1, 2, 3]));
    assert!(!reachable(&store, 1).contains(&10));
}
