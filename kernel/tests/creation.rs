//! Things coming into being (Vol. V Ch. 2 §2.1, clause 4; Amendment A-15): a system asks the
//! kernel for new ids; they are above every authored id, never repeat — across ticks, across
//! systems — and are the same on every replay.

use kernel::domain::{Domain, ResolveError, Resolved};
use kernel::fact::{Cause, FactKey, FactType, SystemId};
use kernel::identity::EntityId;
use kernel::proposal::{Change, Proposal};
use kernel::store::{MemoryStore, RealityStore};
use kernel::system::{Cadence, CommittedView, System, TickContext, RUNTIME_ID_FLOOR};
use kernel::tick::run_tick;
use kernel::value::Value;
use std::collections::BTreeSet;

const MADE: FactType = FactType::new("test.made");

struct Owner;
impl Domain for Owner {
    fn name(&self) -> &'static str {
        "test"
    }
    fn owns(&self, ft: FactType) -> bool {
        ft == MADE
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
            _ => Err(ResolveError::new("one maker per thing")),
        }
    }
}

/// Brings `per_tick` new things into being every tick, each marked with the tick it was made.
struct Maker(&'static str, u64);
impl System for Maker {
    fn id(&self) -> SystemId {
        SystemId::new(self.0)
    }
    fn reads(&self) -> &'static [FactType] {
        &[]
    }
    fn writes(&self) -> &'static [FactType] {
        &[MADE]
    }
    fn cadence(&self) -> Cadence {
        Cadence::EveryTick
    }
    fn evaluate(&self, _view: &dyn CommittedView, ctx: &TickContext) -> Vec<Proposal> {
        (0..self.1)
            .map(|_| {
                Proposal::new(
                    self.id(),
                    FactKey::new(ctx.new_entity(), MADE),
                    ctx.basis_tick(),
                    Change::Set(Value::Int(ctx.tick() as i64)),
                    Cause::new("made"),
                )
            })
            .collect()
    }
}

fn run(ticks: u64) -> MemoryStore {
    let domains: [&dyn Domain; 1] = [&Owner];
    let systems: Vec<Box<dyn System>> =
        vec![Box::new(Maker("test.b", 3)), Box::new(Maker("test.a", 2))];
    let mut store = MemoryStore::new();
    for t in 1..=ticks {
        run_tick(&mut store, &domains, &systems, t, 5, &mut Vec::new()).unwrap();
    }
    store
}

#[test]
fn new_things_have_new_ids_never_repeated() {
    let store = run(4);
    let made: Vec<EntityId> = store.entities_with(MADE);
    // Five a tick for four ticks, every one distinct, every one above the authored range.
    assert_eq!(made.len(), 20);
    assert_eq!(made.iter().collect::<BTreeSet<_>>().len(), 20);
    assert!(made.iter().all(|e| e.raw() >= RUNTIME_ID_FLOOR));
}

#[test]
fn the_same_world_makes_the_same_things() {
    let (a, b) = (run(3), run(3));
    assert_eq!(a.entities_with(MADE), b.entities_with(MADE));
    assert_eq!(a.state_hash(), b.state_hash());
}

#[test]
fn a_runaway_creator_fails_the_tick_and_nothing_commits() {
    // One more than a system may take in a tick: the tick is refused, naming the system, and
    // reality is exactly as it was — none of the ids it was handed name anything.
    let domains: [&dyn Domain; 1] = [&Owner];
    let systems: Vec<Box<dyn System>> = vec![Box::new(Maker(
        "test.runaway",
        kernel::system::IDS_PER_SYSTEM_PER_TICK + 1,
    ))];
    let mut store = MemoryStore::new();
    let before = store.state_hash();
    let err =
        run_tick(&mut store, &domains, &systems, 1, 5, &mut Vec::new()).expect_err("a runaway");
    assert!(
        matches!(err, kernel::tick::TickError::IdsExhausted(s) if s == SystemId::new("test.runaway")),
        "{err:?}"
    );
    assert_eq!(store.state_hash(), before);
    assert_eq!(store.tick(), 0);
}
