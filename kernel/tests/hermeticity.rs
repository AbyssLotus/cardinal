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
            ctx.tick(),
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
