//! Generic graph queries over the fact store — walking parent-link hierarchies
//! (Vol. V Ch. 2 §2.1, relationships are facts; Vol. III Ch. 1 §1.8, containment).
//!
//! These are content-free: a caller passes the fact type that encodes a parent link (an
//! entity-reference fact such as containment) and gets ancestry and lowest-common-ancestor
//! over whatever hierarchy that link defines. Any domain can use them through a published
//! fact-type id, without the kernel knowing what the hierarchy means. The spatial layer
//! builds relative position on top of these (`physical::space`).

use crate::fact::{FactKey, FactType};
use crate::identity::EntityId;
use crate::system::CommittedView;
use crate::value::Value;
use std::collections::BTreeSet;

/// The chain of entities from `entity` up to its root, following `link` (each entity's parent
/// is the entity its `link` fact references). Index 0 is `entity` itself; the last is the
/// root — an entity with no `link` fact. A malformed cycle is broken at the first repeat, so
/// the result is always finite.
pub fn ancestry(view: &dyn CommittedView, entity: EntityId, link: FactType) -> Vec<EntityId> {
    let mut path = vec![entity];
    // A membership set alongside the ordered path: the cycle guard is then O(log n) per step
    // instead of a linear scan of everything walked so far (O(n^2) over a deep hierarchy).
    let mut seen = BTreeSet::from([entity]);
    let mut here = entity;
    while let Some(Value::Entity(parent)) = view.read(FactKey::new(here, link)).map(|f| f.value) {
        if !seen.insert(parent) {
            break;
        }
        path.push(parent);
        here = parent;
    }
    path
}

/// The deepest entity that is an ancestor of both `a` and `b` under `link` (each entity is
/// its own ancestor). `None` if they share no ancestor — e.g. they sit in different,
/// unconnected hierarchies.
pub fn lowest_common_ancestor(
    view: &dyn CommittedView,
    a: EntityId,
    b: EntityId,
    link: FactType,
) -> Option<EntityId> {
    let ancestors_of_b: BTreeSet<EntityId> = ancestry(view, b, link).into_iter().collect();
    ancestry(view, a, link)
        .into_iter()
        .find(|e| ancestors_of_b.contains(e))
}

#[cfg(test)]
mod tests {
    use super::{ancestry, lowest_common_ancestor};
    use crate::fact::{Cause, Fact, FactKey, FactType, Provenance, SystemId};
    use crate::identity::EntityId;
    use crate::store::MemoryStore;
    use crate::value::Value;

    const LINK: FactType = FactType::new("test.parent");

    fn parent(store: &mut MemoryStore, child: u64, parent: u64) {
        store.seed(
            FactKey::new(EntityId::from_raw(child), LINK),
            Fact::new(
                Value::Entity(EntityId::from_raw(parent)),
                Provenance::new(SystemId::new("t"), 0, Cause::new("seed")),
            ),
        );
    }

    #[test]
    fn ancestry_walks_to_the_root() {
        // 3 -> 2 -> 1 (root)
        let mut store = MemoryStore::new();
        parent(&mut store, 3, 2);
        parent(&mut store, 2, 1);
        let path: Vec<u64> = ancestry(&store, EntityId::from_raw(3), LINK)
            .iter()
            .map(|e| e.raw())
            .collect();
        assert_eq!(path, vec![3, 2, 1]);
    }

    #[test]
    fn lca_finds_the_deepest_shared_container() {
        // 10 -> 2 -> 1 ; 20 -> 3 -> 1 : the LCA of 10 and 20 is 1.
        // 11 -> 2 -> 1 : the LCA of 10 and 11 is 2 (deeper).
        let mut store = MemoryStore::new();
        parent(&mut store, 10, 2);
        parent(&mut store, 11, 2);
        parent(&mut store, 2, 1);
        parent(&mut store, 20, 3);
        parent(&mut store, 3, 1);
        let e = EntityId::from_raw;
        assert_eq!(
            lowest_common_ancestor(&store, e(10), e(20), LINK),
            Some(e(1))
        );
        assert_eq!(
            lowest_common_ancestor(&store, e(10), e(11), LINK),
            Some(e(2))
        );
        // Disconnected: 99 has no shared ancestor with 10.
        assert_eq!(lowest_common_ancestor(&store, e(10), e(99), LINK), None);
    }
}
