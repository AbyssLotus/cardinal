//! Facts about a pair through the real tick loop (Vol. V Ch. 2 §2.1; Amendment A-7): each
//! (holder, type, about) is its own fact — proposed, composed, owned, committed, chronicled,
//! and hashed like any other — and the read surface finds a holder's facts by what they are
//! about.

use kernel::domain::{Domain, ResolveError, Resolved};
use kernel::events::ChronicleEntry;
use kernel::fact::{Cause, FactKey, FactType, SystemId};
use kernel::identity::EntityId;
use kernel::proposal::{Change, Proposal};
use kernel::store::{MemoryStore, RealityStore};
use kernel::system::{Cadence, CommittedView, System, TickContext};
use kernel::tick::{run_tick, TickError};
use kernel::value::Value;

/// Where one believes something is: a fact a holder keeps about another entity.
const PLACE_OF: FactType = FactType::new("test.belief.place_of");
/// A fact type no domain owns.
const UNOWNED: FactType = FactType::new("test.unowned");

const ERIN: EntityId = EntityId::from_raw(1);
const BOB: EntityId = EntityId::from_raw(2);
const CAT: EntityId = EntityId::from_raw(3);
const KITCHEN: EntityId = EntityId::from_raw(10);
const YARD: EntityId = EntityId::from_raw(11);

/// Owns `PLACE_OF`; competing Sets on one key are a conflict, Deltas meaningless.
struct Beliefs;
impl Domain for Beliefs {
    fn name(&self) -> &'static str {
        "test"
    }
    fn owns(&self, ft: FactType) -> bool {
        ft == PLACE_OF
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
            _ => Err(ResolveError::new("one opinion per key")),
        }
    }
}

/// On tick 1, Erin sees Bob in the kitchen and the cat in the yard; on tick 2 she forgets Bob.
struct Glance;
impl System for Glance {
    fn id(&self) -> SystemId {
        SystemId::new("test.glance")
    }
    fn reads(&self) -> &'static [FactType] {
        &[PLACE_OF]
    }
    fn writes(&self) -> &'static [FactType] {
        &[PLACE_OF]
    }
    fn cadence(&self) -> Cadence {
        Cadence::EveryTick
    }
    fn evaluate(&self, _view: &dyn CommittedView, ctx: &TickContext) -> Vec<Proposal> {
        let propose = |about, change| {
            Proposal::new(
                self.id(),
                FactKey::pair(ERIN, PLACE_OF, about),
                ctx.basis_tick(),
                change,
                Cause::new("seen"),
            )
        };
        match ctx.tick() {
            1 => vec![
                propose(BOB, Change::Set(Value::Entity(KITCHEN))),
                propose(CAT, Change::Set(Value::Entity(YARD))),
            ],
            2 => vec![propose(BOB, Change::Tombstone)],
            _ => Vec::new(),
        }
    }
}

/// Bob also forms a belief about where the cat is — the same type, a different holder.
struct BobGlance;
impl System for BobGlance {
    fn id(&self) -> SystemId {
        SystemId::new("test.bob_glance")
    }
    fn reads(&self) -> &'static [FactType] {
        &[]
    }
    fn writes(&self) -> &'static [FactType] {
        &[PLACE_OF]
    }
    fn cadence(&self) -> Cadence {
        Cadence::EveryTick
    }
    fn evaluate(&self, _view: &dyn CommittedView, ctx: &TickContext) -> Vec<Proposal> {
        if ctx.tick() != 1 {
            return Vec::new();
        }
        vec![Proposal::new(
            self.id(),
            FactKey::pair(BOB, PLACE_OF, CAT),
            ctx.basis_tick(),
            Change::Set(Value::Entity(KITCHEN)),
            Cause::new("seen"),
        )]
    }
}

fn run(ticks: u64) -> (MemoryStore, Vec<ChronicleEntry>) {
    let domains: [&dyn Domain; 1] = [&Beliefs];
    let systems: Vec<Box<dyn System>> = vec![Box::new(Glance), Box::new(BobGlance)];
    let mut store = MemoryStore::new();
    let mut chronicle = Vec::new();
    for t in 1..=ticks {
        run_tick(&mut store, &domains, &systems, t, 9, &mut chronicle).expect("commits");
    }
    (store, chronicle)
}

fn place(v: Value) -> EntityId {
    match v {
        Value::Entity(e) => e,
        other => panic!("not a place: {other:?}"),
    }
}

#[test]
fn each_pair_is_its_own_fact_through_a_tick() {
    let (store, chronicle) = run(1);
    // Erin's two beliefs, by what they are about; Bob's is his own, though about the same cat.
    let erin: Vec<(EntityId, EntityId)> = store
        .read_about(ERIN, PLACE_OF)
        .into_iter()
        .map(|(about, f)| (about, place(f.value)))
        .collect();
    assert_eq!(erin, vec![(BOB, KITCHEN), (CAT, YARD)]);
    assert_eq!(
        store
            .read(FactKey::pair(BOB, PLACE_OF, CAT))
            .map(|f| place(f.value)),
        Some(KITCHEN),
        "Bob and Erin disagree about the cat"
    );
    assert_eq!(store.entities_with(PLACE_OF), vec![ERIN, BOB]);
    // The provenance is the belief's: who formed it, when, and how.
    let f = store.read(FactKey::pair(ERIN, PLACE_OF, BOB)).unwrap();
    assert_eq!(f.provenance.tick, 1);
    // The chronicle says what each belief was about.
    let mut about: Vec<(EntityId, Option<EntityId>)> =
        chronicle.iter().map(|c| (c.subject(), c.about())).collect();
    about.sort();
    assert_eq!(
        about,
        vec![(ERIN, Some(BOB)), (ERIN, Some(CAT)), (BOB, Some(CAT))]
    );
}

#[test]
fn a_pair_is_cleared_alone() {
    let (store, chronicle) = run(2);
    // Erin forgot where Bob is; she still believes the cat is in the yard.
    let erin: Vec<EntityId> = store
        .read_about(ERIN, PLACE_OF)
        .into_iter()
        .map(|(about, _)| about)
        .collect();
    assert_eq!(erin, vec![CAT]);
    assert_eq!(store.entities_with(PLACE_OF), vec![ERIN, BOB]);
    let last = chronicle.last().unwrap();
    assert_eq!(
        (last.tick(), last.subject(), last.about()),
        (2, ERIN, Some(BOB))
    );
}

#[test]
fn pairs_replay_and_hash_exactly() {
    let (a, ca) = run(3);
    let (b, cb) = run(3);
    assert_eq!(a.state_hash(), b.state_hash());
    assert!(ca == cb);
    // And the digest knows the difference between believing it of Bob and of the cat.
    let (one, _) = run(1);
    assert_ne!(one.state_hash(), a.state_hash());
}

/// Writes a pair fact it does not own — refused like any unowned write.
struct Stranger;
impl System for Stranger {
    fn id(&self) -> SystemId {
        SystemId::new("test.stranger")
    }
    fn reads(&self) -> &'static [FactType] {
        &[]
    }
    fn writes(&self) -> &'static [FactType] {
        &[UNOWNED]
    }
    fn cadence(&self) -> Cadence {
        Cadence::EveryTick
    }
    fn evaluate(&self, _view: &dyn CommittedView, ctx: &TickContext) -> Vec<Proposal> {
        vec![Proposal::new(
            self.id(),
            FactKey::pair(ERIN, UNOWNED, BOB),
            ctx.basis_tick(),
            Change::Set(Value::Int(1)),
            Cause::new("stray"),
        )]
    }
}

#[test]
fn a_pair_has_an_owner_like_any_fact() {
    let domains: [&dyn Domain; 1] = [&Beliefs];
    let systems: Vec<Box<dyn System>> = vec![Box::new(Stranger)];
    let mut store = MemoryStore::new();
    let err = run_tick(&mut store, &domains, &systems, 1, 0, &mut Vec::new()).unwrap_err();
    assert_eq!(err, TickError::Unowned(UNOWNED));
}
