//! Kernel hermeticity: a system may only propose to facts it declared it writes
//! (Vol. V Ch. 3 §3.5, invariant 2). The scoped-read half is unit-tested in `system.rs`.

use kernel::domain::Domain;
use kernel::events::ChronicleEntry;
use kernel::fact::{Cause, FactKey, FactType, SystemId};
use kernel::identity::EntityId;
use kernel::proposal::{Change, Proposal};
use kernel::store::{MemoryStore, RealityStore};
use kernel::system::{Cadence, CommittedView, System, TickContext};
use kernel::tick::{run_tick, TickError};
use kernel::value::Value;

const DECLARED: FactType = FactType::new("test.declared");
const SNEAKY: FactType = FactType::new("test.sneaky");
const UNDECLARED: FactType = FactType::new("test.undeclared");

/// Declares it writes only `DECLARED`, but proposes to `SNEAKY` — a hermeticity violation.
struct LiarSystem;
impl System for LiarSystem {
    fn id(&self) -> SystemId {
        SystemId::new("test.liar")
    }
    fn reads(&self) -> &'static [FactType] {
        &[]
    }
    fn writes(&self) -> &'static [FactType] {
        &[DECLARED]
    }
    fn cadence(&self) -> Cadence {
        Cadence::EveryTick
    }
    fn evaluate(&self, _view: &dyn CommittedView, ctx: &TickContext) -> Vec<Proposal> {
        vec![Proposal::new(
            self.id(),
            FactKey::new(EntityId::from_raw(1), SNEAKY),
            ctx.basis_tick(),
            Change::Set(Value::Int(1)),
            Cause::new("sneak"),
        )]
    }
}

#[test]
fn undeclared_write_is_rejected_and_reality_untouched() {
    let mut store = MemoryStore::new();
    let no_domains: [&dyn Domain; 0] = [];
    let systems: Vec<Box<dyn System>> = vec![Box::new(LiarSystem)];
    let mut chronicle: Vec<ChronicleEntry> = Vec::new();

    let before = store.state_hash();
    let result = run_tick(&mut store, &no_domains, &systems, 1, 7, &mut chronicle);

    match result {
        Err(TickError::UndeclaredWrite { system, fact_type }) => {
            assert_eq!(system, SystemId::new("test.liar"));
            assert_eq!(fact_type, SNEAKY);
        }
        other => panic!("expected UndeclaredWrite, got {other:?}"),
    }
    // The violation is caught before commit: reality is untouched, nothing chronicled.
    assert_eq!(store.state_hash(), before);
    assert!(chronicle.is_empty());
}

/// Declares it reads only `DECLARED`, but reaches for `UNDECLARED` — the read half of
/// hermeticity. It writes `DECLARED` so the write check cannot fire first.
struct PeekingSystem;
impl System for PeekingSystem {
    fn id(&self) -> SystemId {
        SystemId::new("test.peeker")
    }
    fn reads(&self) -> &'static [FactType] {
        &[DECLARED]
    }
    fn writes(&self) -> &'static [FactType] {
        &[DECLARED]
    }
    fn cadence(&self) -> Cadence {
        Cadence::EveryTick
    }
    fn evaluate(&self, view: &dyn CommittedView, ctx: &TickContext) -> Vec<Proposal> {
        // Read a fact type never declared: the scoped view returns empty and records it, and
        // the tick loop must fail rather than let the system act on a silently-empty world.
        let _ = view.read(FactKey::new(EntityId::from_raw(1), UNDECLARED));
        vec![Proposal::new(
            self.id(),
            FactKey::new(EntityId::from_raw(1), DECLARED),
            ctx.basis_tick(),
            Change::Set(Value::Int(1)),
            Cause::new("peek"),
        )]
    }
}

#[test]
fn undeclared_read_is_rejected_and_reality_untouched() {
    let mut store = MemoryStore::new();
    let no_domains: [&dyn Domain; 0] = [];
    let systems: Vec<Box<dyn System>> = vec![Box::new(PeekingSystem)];
    let mut chronicle: Vec<ChronicleEntry> = Vec::new();

    let before = store.state_hash();
    let result = run_tick(&mut store, &no_domains, &systems, 1, 7, &mut chronicle);

    match result {
        Err(TickError::UndeclaredRead { system, fact_type }) => {
            assert_eq!(system, SystemId::new("test.peeker"));
            assert_eq!(fact_type, UNDECLARED);
        }
        other => panic!("expected UndeclaredRead, got {other:?}"),
    }
    assert_eq!(store.state_hash(), before);
    assert!(chronicle.is_empty());
}

/// Declares a basis tick that is not the committed tick it must have read — a proposal
/// computed against state it could not have seen (Vol. V Ch. 3 §3.1, conflict detection).
struct TimeTravellerSystem;
impl System for TimeTravellerSystem {
    fn id(&self) -> SystemId {
        SystemId::new("test.time_traveller")
    }
    fn reads(&self) -> &'static [FactType] {
        &[]
    }
    fn writes(&self) -> &'static [FactType] {
        &[DECLARED]
    }
    fn cadence(&self) -> Cadence {
        Cadence::EveryTick
    }
    fn evaluate(&self, _view: &dyn CommittedView, _ctx: &TickContext) -> Vec<Proposal> {
        // A future basis (999) instead of the committed tick the loop expects.
        vec![Proposal::new(
            self.id(),
            FactKey::new(EntityId::from_raw(1), DECLARED),
            999,
            Change::Set(Value::Int(1)),
            Cause::new("time_travel"),
        )]
    }
}

#[test]
fn stale_basis_is_rejected_and_reality_untouched() {
    let mut store = MemoryStore::new();
    let no_domains: [&dyn Domain; 0] = [];
    let systems: Vec<Box<dyn System>> = vec![Box::new(TimeTravellerSystem)];
    let mut chronicle: Vec<ChronicleEntry> = Vec::new();

    let before = store.state_hash();
    // Compute tick 5: every proposal must declare basis 4, the committed tick it read.
    let result = run_tick(&mut store, &no_domains, &systems, 5, 7, &mut chronicle);

    match result {
        Err(TickError::StaleBasis {
            system,
            expected,
            found,
        }) => {
            assert_eq!(system, SystemId::new("test.time_traveller"));
            assert_eq!(expected, 4);
            assert_eq!(found, 999);
        }
        other => panic!("expected StaleBasis, got {other:?}"),
    }
    assert_eq!(store.state_hash(), before);
    assert!(chronicle.is_empty());
}
