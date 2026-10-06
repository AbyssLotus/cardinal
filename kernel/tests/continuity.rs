//! Reality advances one tick at a time (Vol. V Ch. 3: tick N reads N − 1). A tick that is not
//! the next one — skipped ahead, repeated, or rewound — is refused, and nothing commits.

use kernel::domain::{Domain, ResolveError, Resolved};
use kernel::fact::{Cause, FactKey, FactType, SystemId};
use kernel::identity::EntityId;
use kernel::proposal::{Change, Proposal};
use kernel::store::{MemoryStore, RealityStore};
use kernel::system::{Cadence, CommittedView, System, TickContext};
use kernel::tick::{run_tick, TickError};
use kernel::value::Value;

const COUNT: FactType = FactType::new("test.count");

struct Owner;
impl Domain for Owner {
    fn name(&self) -> &'static str {
        "test"
    }
    fn owns(&self, ft: FactType) -> bool {
        ft == COUNT
    }
    fn systems(&self) -> Vec<Box<dyn System>> {
        Vec::new()
    }
    fn compose(
        &self,
        _ft: FactType,
        _current: Option<Value>,
        changes: &[Change],
    ) -> Result<Resolved, ResolveError> {
        match changes {
            [Change::Set(v)] => Ok(Resolved::Write(*v)),
            _ => Err(ResolveError::new("one counter")),
        }
    }
}

/// Counts the ticks on entity 1.
struct Counter;
impl System for Counter {
    fn id(&self) -> SystemId {
        SystemId::new("test.counter")
    }
    fn reads(&self) -> &'static [FactType] {
        &[COUNT]
    }
    fn writes(&self) -> &'static [FactType] {
        &[COUNT]
    }
    fn cadence(&self) -> Cadence {
        Cadence::EveryTick
    }
    fn evaluate(&self, view: &dyn CommittedView, ctx: &TickContext) -> Vec<Proposal> {
        let key = FactKey::new(EntityId::from_raw(1), COUNT);
        let n = view.read(key).and_then(|f| f.value.as_int()).unwrap_or(0);
        vec![Proposal::new(
            self.id(),
            key,
            ctx.basis_tick(),
            Change::Set(Value::Int(n + 1)),
            Cause::new("counted"),
        )]
    }
}

/// A store advanced through ticks 1..=3, and a tick runner for it.
fn three_ticks() -> MemoryStore {
    let mut store = MemoryStore::new();
    for t in 1..=3 {
        tick(&mut store, t).expect("in order");
    }
    store
}

fn tick(store: &mut MemoryStore, t: u64) -> Result<(), TickError> {
    let domains: [&dyn Domain; 1] = [&Owner];
    let systems: Vec<Box<dyn System>> = vec![Box::new(Counter)];
    run_tick(store, &domains, &systems, t, 7, &mut Vec::new())
}

fn refused(requested: u64) {
    let mut store = three_ticks();
    let before = store.state_hash();
    let err = tick(&mut store, requested).expect_err("not the next tick");
    assert!(
        matches!(err, TickError::NotNext { committed: 3, requested: r } if r == requested),
        "{err:?}"
    );
    assert_eq!(store.state_hash(), before, "nothing committed");
    assert_eq!(store.tick(), 3);
}

#[test]
fn the_next_tick_is_the_only_one_that_runs() {
    let mut store = three_ticks();
    tick(&mut store, 4).expect("the next tick");
    assert_eq!(store.tick(), 4);
}

#[test]
fn a_skipped_tick_is_refused() {
    refused(5);
}

#[test]
fn a_repeated_tick_is_refused() {
    refused(3);
}

#[test]
fn a_rewound_tick_is_refused() {
    refused(1);
}

#[test]
fn tick_zero_never_runs() {
    let mut store = MemoryStore::new();
    let err = tick(&mut store, 0).expect_err("zero is the world as made");
    assert!(matches!(
        err,
        TickError::NotNext {
            committed: 0,
            requested: 0
        }
    ));
}
