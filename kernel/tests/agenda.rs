//! The agenda (Amendment A-22): who is due by a tick is the same with the store's index as by
//! reading every holder, through seeds, commits, and clears.

use kernel::fact::{Cause, Fact, FactKey, FactType, Provenance, SystemId};
use kernel::identity::EntityId;
use kernel::store::{CommitBatch, MemoryStore, RealityStore, Resolution};
use kernel::system::{due_by_scan, CommittedView};
use kernel::value::Value;

const DUE: FactType = FactType::new("test.due");

fn fact(t: i64, tick: u64) -> Fact {
    Fact::new(
        Value::Int(t),
        Provenance::new(SystemId::new("test"), tick, Cause::new("due")),
    )
}

#[test]
fn the_index_answers_what_a_scan_would() {
    let mut store = MemoryStore::new();
    for e in 1..=50u64 {
        store.seed(
            FactKey::new(EntityId::from_raw(e), DUE),
            fact((e * 7 % 40) as i64, 0),
        );
    }
    store.install_agenda(&[DUE]);
    let check = |s: &MemoryStore| {
        for upto in [0, 5, 13, 20, 39, 100] {
            assert_eq!(s.due(DUE, upto), due_by_scan(s, DUE, upto), "by {upto}");
        }
    };
    check(&store);
    // Commits move some entities' due ticks and clear others; the index keeps up.
    let mut resolutions = Vec::new();
    for e in (1..=50u64).step_by(3) {
        resolutions.push(Resolution::One {
            key: FactKey::new(EntityId::from_raw(e), DUE),
            fact: fact(((e * 11) % 60) as i64, 1),
        });
    }
    for e in (2..=50u64).step_by(5) {
        resolutions.push(Resolution::Clear {
            key: FactKey::new(EntityId::from_raw(e), DUE),
        });
    }
    store.apply(CommitBatch {
        tick: 1,
        resolutions,
    });
    check(&store);
    assert!(
        !store.due(DUE, 100).contains(&EntityId::from_raw(2)),
        "cleared"
    );
}
