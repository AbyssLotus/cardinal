//! Retirement (Amendment A-25): an entity retired from an owner's keeping loses every fact that
//! owner keeps for it — single and paired, whatever else was proposed to them that tick — and
//! keeps every fact other owners keep. A fact the owner restricts to its own systems is retired
//! only by one of them.

use kernel::domain::{Domain, ResolveError, Resolved};
use kernel::fact::{Cause, Fact, FactKey, FactType, Provenance, SystemId};
use kernel::identity::EntityId;
use kernel::proposal::{Change, Proposal};
use kernel::store::{MemoryStore, RealityStore};
use kernel::system::{Cadence, CommittedView, System, TickContext};
use kernel::tick::{run_tick, TickError};
use kernel::value::Value;

const BODY: FactType = FactType::new("a.body"); // A's, restricted to A's own system
const MOOD: FactType = FactType::new("a.mood"); // A's, open
const KNOWS: FactType = FactType::new("a.knows"); // A's, about a pair
const NAME: FactType = FactType::new("b.name"); // B's
const KEEPER: SystemId = SystemId::new("a.keeper");

/// A domain owning `facts`, refusing strangers on `restricted`.
struct Owner {
    name: &'static str,
    facts: &'static [FactType],
    restricted: Option<FactType>,
}
impl Domain for Owner {
    fn name(&self) -> &'static str {
        self.name
    }
    fn owns(&self, ft: FactType) -> bool {
        self.facts.contains(&ft)
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
            [Change::Tombstone] => Ok(Resolved::Tombstone),
            _ => Err(ResolveError::new("one change at a time")),
        }
    }
    fn accepts(&self, ft: FactType, system: SystemId) -> bool {
        Some(ft) != self.restricted || system == KEEPER
    }
}

/// Proposes `change` to `key`, under id `.0`.
struct Proposer(SystemId, FactKey, Change);
impl System for Proposer {
    fn id(&self) -> SystemId {
        self.0
    }
    fn reads(&self) -> &'static [FactType] {
        &[]
    }
    fn writes(&self) -> &'static [FactType] {
        &[BODY, MOOD, KNOWS, NAME]
    }
    fn cadence(&self) -> Cadence {
        Cadence::EveryTick
    }
    fn evaluate(&self, _v: &dyn CommittedView, ctx: &TickContext) -> Vec<Proposal> {
        vec![Proposal::new(
            self.0,
            self.1,
            ctx.basis_tick(),
            self.2,
            Cause::new("left"),
        )]
    }
}

fn e(raw: u64) -> EntityId {
    EntityId::from_raw(raw)
}

fn seeded() -> MemoryStore {
    let mut store = MemoryStore::new();
    let seed = |v| {
        Fact::new(
            v,
            Provenance::new(SystemId::new("seed"), 0, Cause::new("seed")),
        )
    };
    store.seed(FactKey::new(e(1), BODY), seed(Value::Int(1)));
    store.seed(FactKey::new(e(1), MOOD), seed(Value::Int(2)));
    store.seed(FactKey::pair(e(1), KNOWS, e(7)), seed(Value::Int(3)));
    store.seed(FactKey::pair(e(1), KNOWS, e(8)), seed(Value::Int(4)));
    store.seed(FactKey::new(e(1), NAME), seed(Value::Int(5)));
    // Another entity's facts, and a fact *about* entity 1 held by someone else, stay.
    store.seed(FactKey::new(e(2), MOOD), seed(Value::Int(6)));
    store.seed(FactKey::pair(e(2), KNOWS, e(1)), seed(Value::Int(7)));
    store
}

fn run(
    store: &mut MemoryStore,
    systems: Vec<Box<dyn System>>,
) -> (Result<(), TickError>, Vec<kernel::events::ChronicleEntry>) {
    let a = Owner {
        name: "a",
        facts: &[BODY, MOOD, KNOWS],
        restricted: Some(BODY),
    };
    let b = Owner {
        name: "b",
        facts: &[NAME],
        restricted: None,
    };
    let domains: [&dyn Domain; 2] = [&a, &b];
    let mut chronicle = Vec::new();
    let r = run_tick(store, &domains, &systems, 1, 0, &mut chronicle);
    (r, chronicle)
}

#[test]
fn a_retired_entity_leaves_its_owners_keeping_and_no_one_elses() {
    let mut store = seeded();
    let (r, chronicle) = run(
        &mut store,
        vec![
            Box::new(Proposer(KEEPER, FactKey::new(e(1), BODY), Change::Retire)),
            // Moot: entity 1 is leaving A's keeping this tick.
            Box::new(Proposer(
                SystemId::new("a.moods"),
                FactKey::new(e(1), MOOD),
                Change::Set(Value::Int(99)),
            )),
            Box::new(Proposer(
                SystemId::new("a.learning"),
                FactKey::pair(e(1), KNOWS, e(9)),
                Change::Set(Value::Int(99)),
            )),
            // Not moot: B's fact of entity 1, and A's fact of entity 2.
            Box::new(Proposer(
                SystemId::new("b.names"),
                FactKey::new(e(1), NAME),
                Change::Set(Value::Int(50)),
            )),
            Box::new(Proposer(
                SystemId::new("a.others"),
                FactKey::new(e(2), MOOD),
                Change::Set(Value::Int(60)),
            )),
        ],
    );
    assert_eq!(r, Ok(()));
    let facts: Vec<(FactKey, i64)> = store
        .facts_of(e(1))
        .into_iter()
        .map(|(k, f)| (k, f.value.as_int().unwrap()))
        .collect();
    assert_eq!(facts, vec![(FactKey::new(e(1), NAME), 50)]);
    assert_eq!(
        store.read(FactKey::new(e(2), MOOD)).map(|f| f.value),
        Some(Value::Int(60))
    );
    assert!(store.read(FactKey::pair(e(2), KNOWS, e(1))).is_some());
    // Every fact retired is chronicled, under the retirement's cause.
    let retired = chronicle
        .iter()
        .filter(|c| c.cause().event() == "left" && c.subject() == e(1))
        .count();
    assert_eq!(retired, 4 + 1, "four of A's facts, and B's name set");
}

#[test]
fn a_restricted_fact_is_retired_only_by_its_owners_own() {
    let mut store = seeded();
    let stranger = SystemId::new("other.eraser");
    let (r, _) = run(
        &mut store,
        vec![Box::new(Proposer(
            stranger,
            FactKey::new(e(1), MOOD),
            Change::Retire,
        ))],
    );
    assert_eq!(
        r,
        Err(TickError::Refused {
            system: stranger,
            fact_type: BODY
        })
    );
    assert!(store.read(FactKey::new(e(1), BODY)).is_some(), "untouched");
}
