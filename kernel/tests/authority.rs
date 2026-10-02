//! Owners may refuse writers (Amendment A-5): a domain that restricts a fact to its own systems
//! sees every other proposer refused, by name, with reality untouched — and no two systems may
//! share an id, since an id is how an owner recognises its own.

use kernel::domain::{Domain, ResolveError, Resolved};
use kernel::fact::{Cause, FactKey, FactType, SystemId};
use kernel::identity::EntityId;
use kernel::proposal::{Change, Proposal};
use kernel::store::{MemoryStore, RealityStore};
use kernel::system::{Cadence, CommittedView, System, TickContext};
use kernel::tick::{run_tick, TickError};
use kernel::value::Value;

const WHERE: FactType = FactType::new("test.where"); // restricted: only the owner's mover
const WISH: FactType = FactType::new("test.wish"); // open: anyone may propose
const MOVER: SystemId = SystemId::new("test.mover");

struct Owner;
impl Domain for Owner {
    fn name(&self) -> &'static str {
        "test"
    }
    fn owns(&self, ft: FactType) -> bool {
        ft == WHERE || ft == WISH
    }
    fn systems(&self) -> Vec<Box<dyn System>> {
        vec![Box::new(Writer(MOVER, WHERE))]
    }
    fn compose(
        &self,
        _ft: FactType,
        _current: Option<Value>,
        changes: &[Change],
    ) -> Result<Resolved, ResolveError> {
        match changes.first() {
            Some(Change::Set(v)) => Ok(Resolved::Write(*v)),
            _ => Err(ResolveError::new("test owner takes one set")),
        }
    }
    fn accepts(&self, ft: FactType, system: SystemId) -> bool {
        ft != WHERE || system == MOVER
    }
}

/// Sets `fact` on entity 1 to 7, under id `.0`.
struct Writer(SystemId, FactType);
impl System for Writer {
    fn id(&self) -> SystemId {
        self.0
    }
    fn reads(&self) -> &'static [FactType] {
        &[]
    }
    fn writes(&self) -> &'static [FactType] {
        &[WHERE, WISH]
    }
    fn cadence(&self) -> Cadence {
        Cadence::EveryTick
    }
    fn evaluate(&self, _v: &dyn CommittedView, ctx: &TickContext) -> Vec<Proposal> {
        vec![Proposal::new(
            self.0,
            FactKey::new(EntityId::from_raw(1), self.1),
            ctx.basis_tick(),
            Change::Set(Value::Int(7)),
            Cause::new("write"),
        )]
    }
}

fn tick(systems: Vec<Box<dyn System>>) -> (Result<(), TickError>, MemoryStore) {
    let mut store = MemoryStore::new();
    let owner = Owner;
    let domains: [&dyn Domain; 1] = [&owner];
    let r = run_tick(&mut store, &domains, &systems, 1, 0, &mut Vec::new());
    (r, store)
}

#[test]
fn the_owners_own_system_may_write_a_restricted_fact() {
    let (r, store) = tick(Owner.systems());
    assert!(r.is_ok());
    assert!(store
        .read(FactKey::new(EntityId::from_raw(1), WHERE))
        .is_some());
}

#[test]
fn a_stranger_is_refused_by_name_and_nothing_commits() {
    let stranger = SystemId::new("other.teleporter");
    let (r, store) = tick(vec![Box::new(Writer(stranger, WHERE))]);
    assert_eq!(
        r,
        Err(TickError::Refused {
            system: stranger,
            fact_type: WHERE
        })
    );
    assert!(store.is_empty(), "a failed tick never happened");
}

#[test]
fn a_stranger_may_still_propose_an_open_fact() {
    let (r, _) = tick(vec![Box::new(Writer(SystemId::new("other.wisher"), WISH))]);
    assert!(r.is_ok());
}

#[test]
fn two_systems_may_not_share_an_id() {
    // An impostor wearing the owner's mover's name alongside the real one is caught before
    // anything runs — and so is any accidental duplicate, which would share random streams.
    let mut systems = Owner.systems();
    systems.push(Box::new(Writer(MOVER, WISH)));
    let (r, store) = tick(systems);
    assert_eq!(r, Err(TickError::DuplicateSystem(MOVER)));
    assert!(store.state_hash() == MemoryStore::new().state_hash());
}
