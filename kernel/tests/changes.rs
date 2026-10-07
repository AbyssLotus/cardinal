//! What changed (Amendment A-21): the committed view says which entities had facts of a type
//! written after a tick — or that it cannot say, when its memory does not reach back that far,
//! or it keeps none. It never guesses.

use kernel::domain::{Domain, ResolveError, Resolved};
use kernel::fact::{Cause, FactKey, FactType, SystemId};
use kernel::identity::EntityId;
use kernel::proposal::{Change, Proposal};
use kernel::store::MemoryStore;
use kernel::system::{Cadence, CommittedView, System, TickContext};
use kernel::tick::run_tick;
use kernel::value::Value;

const MARK: FactType = FactType::new("test.mark");
const OTHER: FactType = FactType::new("test.other");

struct Owner;
impl Domain for Owner {
    fn name(&self) -> &'static str {
        "test"
    }
    fn owns(&self, ft: FactType) -> bool {
        ft == MARK || ft == OTHER
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
            _ => Err(ResolveError::new("one writer")),
        }
    }
}

/// Marks entity `tick % 3 + 1` every tick, and entity 9's other fact on even ticks.
struct Marker;
impl System for Marker {
    fn id(&self) -> SystemId {
        SystemId::new("test.marker")
    }
    fn reads(&self) -> &'static [FactType] {
        &[]
    }
    fn writes(&self) -> &'static [FactType] {
        &[MARK, OTHER]
    }
    fn cadence(&self) -> Cadence {
        Cadence::EveryTick
    }
    fn evaluate(&self, _view: &dyn CommittedView, ctx: &TickContext) -> Vec<Proposal> {
        let mut out = vec![Proposal::new(
            self.id(),
            FactKey::new(EntityId::from_raw(ctx.tick() % 3 + 1), MARK),
            ctx.basis_tick(),
            Change::Set(Value::Int(ctx.tick() as i64)),
            Cause::new("marked"),
        )];
        if ctx.tick() % 2 == 0 {
            out.push(Proposal::new(
                self.id(),
                FactKey::new(EntityId::from_raw(9), OTHER),
                ctx.basis_tick(),
                Change::Set(Value::Int(1)),
                Cause::new("marked"),
            ));
        }
        out
    }
}

fn world(ticks: u64, window: usize) -> MemoryStore {
    let domains: [&dyn Domain; 1] = [&Owner];
    let systems: Vec<Box<dyn System>> = vec![Box::new(Marker)];
    let mut store = MemoryStore::new();
    store.set_change_window(window);
    for t in 1..=ticks {
        run_tick(&mut store, &domains, &systems, t, 3, &mut Vec::new()).unwrap();
    }
    store
}

fn ids(v: Option<Vec<EntityId>>) -> Option<Vec<u64>> {
    v.map(|v| v.into_iter().map(|e| e.raw()).collect())
}

#[test]
fn what_was_written_after_a_tick_is_named() {
    let store = world(10, 64);
    // Tick 10 marked entity 2; ticks 9 and 10 marked 1 and 2; ticks 8–10 all three.
    assert_eq!(ids(store.changed_since(MARK, 9)), Some(vec![2]));
    assert_eq!(ids(store.changed_since(MARK, 8)), Some(vec![1, 2]));
    assert_eq!(ids(store.changed_since(MARK, 7)), Some(vec![1, 2, 3]));
    // Per fact type: entity 9's other fact was written on tick 10, not on tick 9.
    assert_eq!(ids(store.changed_since(OTHER, 9)), Some(vec![9]));
    assert_eq!(ids(store.changed_since(OTHER, 10)), Some(vec![]));
    // Nothing has happened after now.
    assert_eq!(ids(store.changed_since(MARK, 10)), Some(vec![]));
}

#[test]
fn what_is_beyond_memory_is_unknown_never_guessed() {
    // Remembering four ticks, it can speak of ticks 7–10 and no further back.
    let store = world(10, 4);
    assert_eq!(ids(store.changed_since(MARK, 6)), Some(vec![1, 2, 3]));
    assert_eq!(ids(store.changed_since(MARK, 5)), None);
    // Remembering nothing, it can say nothing — except that nothing happens after now.
    let store = world(10, 0);
    assert_eq!(ids(store.changed_since(MARK, 9)), None);
    assert_eq!(ids(store.changed_since(MARK, 10)), Some(vec![]));
}

#[test]
fn forgetting_changes_nothing_that_was_committed() {
    use kernel::store::RealityStore;
    assert_eq!(world(20, 64).state_hash(), world(20, 0).state_hash());
}

/// Asks what changed in a fact type it never declared reading.
struct Snoop;
impl System for Snoop {
    fn id(&self) -> SystemId {
        SystemId::new("test.snoop")
    }
    fn reads(&self) -> &'static [FactType] {
        &[OTHER]
    }
    fn writes(&self) -> &'static [FactType] {
        &[]
    }
    fn cadence(&self) -> Cadence {
        Cadence::EveryTick
    }
    fn evaluate(&self, view: &dyn CommittedView, ctx: &TickContext) -> Vec<Proposal> {
        let _ = view.changed_since(MARK, ctx.basis_tick().saturating_sub(1));
        Vec::new()
    }
}

#[test]
fn asking_what_changed_is_a_read_like_any_other() {
    let domains: [&dyn Domain; 1] = [&Owner];
    let systems: Vec<Box<dyn System>> = vec![Box::new(Marker), Box::new(Snoop)];
    let mut store = MemoryStore::new();
    let err = run_tick(&mut store, &domains, &systems, 1, 3, &mut Vec::new())
        .expect_err("an undeclared read");
    assert!(
        matches!(err, kernel::tick::TickError::UndeclaredRead { fact_type, .. } if fact_type == MARK),
        "{err:?}"
    );
}
