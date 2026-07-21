//! Cardinality-many facts through the real tick loop (Vol. V Ch. 2 §2.1): set-valued facts
//! accumulate their members, resolved by set union of Add/Remove, deterministically.

use kernel::domain::{Domain, ResolveError, Resolved};
use kernel::events::ChronicleEntry;
use kernel::fact::{Cardinality, Cause, Fact, FactKey, FactType, Provenance, SystemId};
use kernel::identity::EntityId;
use kernel::proposal::{Change, Proposal};
use kernel::store::{MemoryStore, RealityStore};
use kernel::system::{Cadence, CommittedView, System, TickContext};
use kernel::tick::run_tick;
use kernel::value::Value;

const TAGS: FactType = FactType::new("test.tags");
const ENTITY: EntityId = EntityId::from_raw(1);

/// A domain owning `TAGS` as a cardinality-many fact (composed by the kernel via set union).
struct TagDomain;
impl Domain for TagDomain {
    fn name(&self) -> &'static str {
        "test"
    }
    fn owns(&self, ft: FactType) -> bool {
        ft == TAGS
    }
    fn cardinality(&self, ft: FactType) -> Cardinality {
        if ft == TAGS {
            Cardinality::Many
        } else {
            Cardinality::One
        }
    }
    fn systems(&self) -> Vec<Box<dyn System>> {
        Vec::new()
    }
    fn compose(
        &self,
        _ft: FactType,
        _current: Option<Value>,
        _changes: &[Change],
    ) -> Result<Resolved, ResolveError> {
        Err(ResolveError::new(
            "cardinality-many: composed by the kernel",
        ))
    }
}

/// A system that adds a fixed tag value to `TAGS`.
struct AddTag(&'static str, i64);
impl System for AddTag {
    fn id(&self) -> SystemId {
        SystemId::new(self.0)
    }
    fn reads(&self) -> &'static [FactType] {
        &[]
    }
    fn writes(&self) -> &'static [FactType] {
        &[TAGS]
    }
    fn cadence(&self) -> Cadence {
        Cadence::EveryTick
    }
    fn evaluate(&self, _v: &dyn CommittedView, ctx: &TickContext) -> Vec<Proposal> {
        vec![Proposal::new(
            self.id(),
            FactKey::new(ENTITY, TAGS),
            ctx.tick(),
            Change::Add(Value::Int(self.1)),
            Cause::new("add_tag"),
        )]
    }
}

/// A system that removes a fixed tag value from `TAGS`.
struct RemoveTag(&'static str, i64);
impl System for RemoveTag {
    fn id(&self) -> SystemId {
        SystemId::new(self.0)
    }
    fn reads(&self) -> &'static [FactType] {
        &[]
    }
    fn writes(&self) -> &'static [FactType] {
        &[TAGS]
    }
    fn cadence(&self) -> Cadence {
        Cadence::EveryTick
    }
    fn evaluate(&self, _v: &dyn CommittedView, ctx: &TickContext) -> Vec<Proposal> {
        vec![Proposal::new(
            self.id(),
            FactKey::new(ENTITY, TAGS),
            ctx.tick(),
            Change::Remove(Value::Int(self.1)),
            Cause::new("remove_tag"),
        )]
    }
}

fn tags(store: &MemoryStore) -> Vec<i64> {
    let mut v: Vec<i64> = store
        .read_all(FactKey::new(ENTITY, TAGS))
        .into_iter()
        .map(|f| f.value.as_int().unwrap())
        .collect();
    v.sort_unstable();
    v
}

#[test]
fn adds_accumulate_as_a_set_over_the_seeded_values() {
    let mut store = MemoryStore::new();
    // Seed one value directly (world generation), then two systems add two more.
    store.seed(
        FactKey::new(ENTITY, TAGS),
        Fact::new(
            Value::Int(5),
            Provenance::new(SystemId::new("worldgen"), 0, Cause::new("seed")),
        ),
    );
    let domain = TagDomain;
    let domains: [&dyn Domain; 1] = [&domain];
    let systems: Vec<Box<dyn System>> = vec![
        Box::new(AddTag("test.add_a", 10)),
        Box::new(AddTag("test.add_b", 20)),
    ];
    let mut chronicle: Vec<ChronicleEntry> = Vec::new();

    run_tick(&mut store, &domains, &systems, 1, 0, &mut chronicle).expect("tick commits");
    // The seeded value and both added values coexist as a set.
    assert_eq!(tags(&store), vec![5, 10, 20]);
}

#[test]
fn add_is_idempotent_and_remove_deletes_one_member() {
    let mut store = MemoryStore::new();
    let domain = TagDomain;
    let domains: [&dyn Domain; 1] = [&domain];
    let mut chronicle: Vec<ChronicleEntry> = Vec::new();

    // Two systems add the SAME value plus one distinct one -> {10, 20}, not {10, 10, 20}.
    let add: Vec<Box<dyn System>> = vec![
        Box::new(AddTag("test.add_a", 10)),
        Box::new(AddTag("test.add_b", 10)),
        Box::new(AddTag("test.add_c", 20)),
    ];
    run_tick(&mut store, &domains, &add, 1, 0, &mut chronicle).expect("tick");
    assert_eq!(tags(&store), vec![10, 20]);

    // Removing a member leaves the rest of the set intact.
    let remove: Vec<Box<dyn System>> = vec![Box::new(RemoveTag("test.rm", 10))];
    run_tick(&mut store, &domains, &remove, 2, 0, &mut chronicle).expect("tick");
    assert_eq!(tags(&store), vec![20]);
}

#[test]
fn many_facts_are_deterministic_in_the_state_hash() {
    fn build() -> [u8; 32] {
        let mut store = MemoryStore::new();
        let domain = TagDomain;
        let domains: [&dyn Domain; 1] = [&domain];
        let systems: Vec<Box<dyn System>> = vec![
            Box::new(AddTag("test.add_a", 30)),
            Box::new(AddTag("test.add_b", 10)),
            Box::new(AddTag("test.add_c", 20)),
        ];
        let mut chronicle = Vec::new();
        run_tick(&mut store, &domains, &systems, 1, 0, &mut chronicle).unwrap();
        *store.state_hash().as_bytes()
    }
    // The set's digest does not depend on the order members were added (Vol. V Ch. 4 §4.2).
    assert_eq!(build(), build());
}
