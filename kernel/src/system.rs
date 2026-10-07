//! Systems and their hermetic evaluation context (Vol. V Ch. 3 §3.1-3.2).
//!
//! A system reads a committed-state view scoped to its declared read set, draws from an
//! RNG substream the kernel issues, sees the clock, applies its rules, and returns
//! proposals — it touches nothing else (Vol. V Ch. 3 §3.1, hermetic evaluation). That seal
//! is what makes evaluation parallelizable and each system testable in isolation.

use crate::fact::{Fact, FactKey, FactType, SystemId};
use crate::identity::EntityId;
use crate::proposal::Proposal;
use crate::rng::{Rng, SubstreamKey};
use crate::spatial::SpatialQuery;
use crate::value::Value;
use std::cell::Cell;

/// How often a system runs, in simulation time (Vol. V Ch. 3 §3.2, Cadence).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Cadence {
    /// Runs on every tick.
    EveryTick,
    /// Runs every `n` ticks (a zero period never runs).
    EveryNTicks(u64),
}

impl Cadence {
    /// Whether a system with this cadence is due on `tick` (tick 0 is the initial world).
    pub const fn is_due(&self, tick: u64) -> bool {
        match self {
            Cadence::EveryTick => true,
            Cadence::EveryNTicks(n) => *n != 0 && tick % *n == 0,
        }
    }
}

/// A read-only view of committed reality handed to a system for one evaluation.
///
/// Backed by the last committed tick; it never exposes in-flight proposals
/// (Vol. V Ch. 2 §2.1, clause 3). Implemented by the store; the tick loop wraps it in a
/// read-set-scoped view before handing it to a system.
pub trait CommittedView {
    /// Read one committed value at `key`, or `None` if absent. For a cardinality-many fact
    /// this returns the least value; use [`CommittedView::read_all`] for the whole set.
    fn read(&self, key: FactKey) -> Option<Fact>;

    /// Read every committed value at `key`, in deterministic order. A cardinality-one fact
    /// yields zero or one; a cardinality-many fact yields the whole set.
    fn read_all(&self, key: FactKey) -> Vec<Fact>;

    /// Every entity currently bearing at least one committed value of `fact_type`, in
    /// deterministic ascending order (Vol. V Ch. 2 §2.1, clause 5 — queries are the
    /// product). This is how a system discovers its subjects from committed reality
    /// instead of carrying an entity list of its own: reality is authoritative, and an
    /// entity created mid-simulation is simulated the tick its facts commit.
    fn entities_with(&self, fact_type: FactType) -> Vec<EntityId>;

    /// Every committed fact of `fact_type` that `holder` holds about a second entity
    /// (Amendment A-7), as `(about, fact)` in ascending order of what it is about — one entry per
    /// value. "Everything Erin believes about where things are" is one call.
    fn read_about(&self, holder: EntityId, fact_type: FactType) -> Vec<(EntityId, Fact)>;

    /// The values of a cardinality-many fact that lie between `lo` and `hi` (inclusive, in the
    /// values' total order), sorted — a slice of a large set without reading all of it. A
    /// heightfield stored as a set of `[column, row, height]` samples answers "the sample at
    /// column 3, row 7" this way. The default filters [`CommittedView::read_all`]; stores
    /// override it with a range walk.
    fn read_range(&self, key: FactKey, lo: &Value, hi: &Value) -> Vec<Fact> {
        self.read_all(key)
            .into_iter()
            .filter(|f| &f.value >= lo && &f.value <= hi)
            .collect()
    }

    /// The tick this view's committed state represents: 0 for the initial world, N once tick N
    /// has committed. A system evaluating tick N reads a view at N−1 (Vol. V Ch. 2 §2.1,
    /// clause 3). Quantities that change continuously between commits — a body in motion — are
    /// evaluated at this tick.
    fn tick(&self) -> u64;

    /// The store's derived spatial index, if one is installed (Amendment A-2). `None` means
    /// "no index": a caller answers by scanning committed facts instead, and must get the same
    /// answer (the conformance rule). Through a system's scoped view this also requires the
    /// system to have declared every fact type the index mirrors.
    fn spatial(&self) -> Option<&dyn SpatialQuery> {
        None
    }

    /// The entities that had facts of `fact_type` written in the ticks after `since`, up to this
    /// view's tick, in ascending order — or `None` if this view cannot say (Amendment A-21): it
    /// keeps no memory of changes, or its memory does not reach back that far. A system given
    /// `None` must do all the work it would do without the answer, so that nothing committed can
    /// depend on how much a store remembers. "Written" includes a value set again unchanged.
    fn changed_since(&self, fact_type: FactType, since: u64) -> Option<Vec<EntityId>> {
        let _ = (fact_type, since);
        None
    }

    /// The entities due by `fact_type` — an agenda fact (Amendment A-22), whose value is the tick
    /// an entity is next due — at or before tick `upto`, in ascending order of id. A store with an
    /// agenda index answers in proportion to the answer; without one, by scanning; the same
    /// either way.
    fn due(&self, fact_type: FactType, upto: u64) -> Vec<EntityId> {
        due_by_scan(self, fact_type, upto)
    }
}

/// Who is due by `fact_type` at or before `upto`, found by reading every holder (Amendment A-22):
/// the answer every agenda index must agree with.
pub fn due_by_scan<V: CommittedView + ?Sized>(
    view: &V,
    fact_type: FactType,
    upto: u64,
) -> Vec<EntityId> {
    view.entities_with(fact_type)
        .into_iter()
        .filter(|e| {
            view.read_all(FactKey::new(*e, fact_type))
                .iter()
                .any(|f| matches!(f.value, Value::Int(t) if t <= upto.min(i64::MAX as u64) as i64))
        })
        .collect()
}

/// A committed view scoped to a system's declared read set (Vol. V Ch. 3 §3.1).
///
/// Wraps the full committed view but only serves fact types the system declared it reads.
/// A read outside the declared set is *recorded*, not silently defaulted — the tick loop
/// checks [`ScopedView::violation`] after evaluation and fails the tick, because an
/// undeclared read is a hermeticity failure, never a default (Vol. V Ch. 3 §3.5,
/// invariant 2; Vol. IV Ch. 2, missing-is-failure). Kernel-internal: the tick loop builds
/// one per system per tick.
pub(crate) struct ScopedView<'a> {
    inner: &'a dyn CommittedView,
    allowed: &'a [FactType],
    // The first undeclared fact type read, if any. A Cell because the read surface is &self
    // while the violation must outlive the call — evaluation itself stays read-only.
    violation: Cell<Option<FactType>>,
}

impl<'a> ScopedView<'a> {
    /// Scope `inner` to the `allowed` read set.
    pub(crate) fn new(inner: &'a dyn CommittedView, allowed: &'a [FactType]) -> Self {
        Self {
            inner,
            allowed,
            violation: Cell::new(None),
        }
    }

    /// The first fact type read outside the declared set during evaluation, if any.
    pub(crate) fn violation(&self) -> Option<FactType> {
        self.violation.get()
    }

    /// Whether `fact_type` is declared; records the first violation otherwise.
    fn check(&self, fact_type: FactType) -> bool {
        let ok = self.allowed.contains(&fact_type);
        if !ok && self.violation.get().is_none() {
            self.violation.set(Some(fact_type));
        }
        ok
    }
}

impl CommittedView for ScopedView<'_> {
    fn read(&self, key: FactKey) -> Option<Fact> {
        if self.check(key.fact_type) {
            self.inner.read(key)
        } else {
            None
        }
    }

    fn read_all(&self, key: FactKey) -> Vec<Fact> {
        if self.check(key.fact_type) {
            self.inner.read_all(key)
        } else {
            Vec::new()
        }
    }

    fn entities_with(&self, fact_type: FactType) -> Vec<EntityId> {
        if self.check(fact_type) {
            self.inner.entities_with(fact_type)
        } else {
            Vec::new()
        }
    }

    fn read_range(&self, key: FactKey, lo: &Value, hi: &Value) -> Vec<Fact> {
        if self.check(key.fact_type) {
            self.inner.read_range(key, lo, hi)
        } else {
            Vec::new()
        }
    }

    fn read_about(&self, holder: EntityId, fact_type: FactType) -> Vec<(EntityId, Fact)> {
        if self.check(fact_type) {
            self.inner.read_about(holder, fact_type)
        } else {
            Vec::new()
        }
    }

    fn tick(&self) -> u64 {
        self.inner.tick()
    }

    fn spatial(&self) -> Option<&dyn SpatialQuery> {
        // The index mirrors its watched facts, so reading it is reading them: every one must be
        // in the declared read set, or the read is undeclared like any other (Vol. V Ch. 3
        // §3.5). Check them all (recording the first violation) before handing it out.
        let index = self.inner.spatial()?;
        let mut declared = true;
        for fact_type in index.watches() {
            declared &= self.check(*fact_type);
        }
        declared.then_some(index)
    }

    fn due(&self, fact_type: FactType, upto: u64) -> Vec<EntityId> {
        if self.check(fact_type) {
            self.inner.due(fact_type, upto)
        } else {
            Vec::new()
        }
    }

    fn changed_since(&self, fact_type: FactType, since: u64) -> Option<Vec<EntityId>> {
        if self.check(fact_type) {
            self.inner.changed_since(fact_type, since)
        } else {
            None
        }
    }
}

/// The per-evaluation context: the clock and the system's issued RNG substream.
///
/// The kernel constructs this; a system cannot fabricate a stream or read a clock of its
/// own (Vol. V Ch. 3 §3.3; Vol. V Ch. 4 §4.1).
pub struct TickContext {
    tick: u64,
    seed: u64,
    system_code: u32,
    // The system's place among all registered systems, sorted by id: its slot for new ids.
    slot: u64,
    // How many new ids it has taken this tick.
    created: Cell<u64>,
    // Whether it asked for more than one system may take in a tick.
    exhausted: Cell<bool>,
}

/// The first id the kernel ever issues at run time (Amendment A-15). Authored worlds number their
/// entities below it; everything created while the world runs is numbered above it.
pub const RUNTIME_ID_FLOOR: u64 = 1 << 62;

/// How many new ids one system may take in one tick.
pub const IDS_PER_SYSTEM_PER_TICK: u64 = 1 << 14;

impl TickContext {
    /// Create a context for a system (identified by `system_code`, in `slot` among all systems
    /// sorted by id) at `tick` under world `seed`. Kernel-internal — only the tick loop builds
    /// contexts.
    pub(crate) fn new(tick: u64, seed: u64, system_code: u32, slot: u64) -> Self {
        Self {
            tick,
            seed,
            system_code,
            slot,
            created: Cell::new(0),
            exhausted: Cell::new(false),
        }
    }

    /// A new entity id, never issued before and never to be issued again (Amendment A-15; Vol. V
    /// Ch. 2 §2.1, clause 4): `floor + tick·2²⁴ + slot·2¹⁴ + n`, the system's `n`th this tick.
    /// Deterministic — the same system in the same world creates the same ids on every replay —
    /// and disjoint between systems and ticks, so no two systems can ever collide.
    ///
    /// A system that asks for more than [`IDS_PER_SYSTEM_PER_TICK`] in a tick — a runaway, not a
    /// world — fails the tick (`TickError::IdsExhausted`): nothing it proposed commits, so the
    /// placeholder it is handed past the limit never names anything.
    pub fn new_entity(&self) -> EntityId {
        let n = self.created.get();
        if n >= IDS_PER_SYSTEM_PER_TICK {
            self.exhausted.set(true);
            return EntityId::from_raw(u64::MAX);
        }
        self.created.set(n + 1);
        EntityId::from_raw(RUNTIME_ID_FLOOR + (self.tick << 24) + (self.slot << 14) + n)
    }

    /// Whether this system asked for more new ids than it may take this tick.
    pub(crate) fn exhausted(&self) -> bool {
        self.exhausted.get()
    }

    /// The current tick — the one being computed.
    pub const fn tick(&self) -> u64 {
        self.tick
    }

    /// The committed tick a system reads as it evaluates this tick: `tick - 1`, the state
    /// its proposals are based on (Vol. V Ch. 3 §3.1, Proposals). Pass this as a proposal's
    /// basis so conflict detection can tell a fresh proposal from a stale one.
    pub const fn basis_tick(&self) -> u64 {
        self.tick.saturating_sub(1)
    }

    /// Issue this system's deterministic RNG substream for `scope`
    /// (per `(system, tick, scope)` — Vol. V Ch. 4 §4.1).
    pub fn rng(&self, scope: u64) -> Rng {
        Rng::for_substream(
            self.seed,
            SubstreamKey::new(self.system_code, self.tick, scope),
        )
    }
}

/// A hermetic transformation of committed reality into proposals (Vol. V Ch. 3 §3.1).
pub trait System {
    /// The system's stable identity.
    fn id(&self) -> SystemId;

    /// The fact types this system may read — its declared read set (Vol. V Ch. 3 §3.2).
    fn reads(&self) -> &'static [FactType];

    /// The fact types this system may write — its declared write set (Vol. V Ch. 3 §3.2).
    fn writes(&self) -> &'static [FactType];

    /// How often the system runs (Vol. V Ch. 3 §3.2).
    fn cadence(&self) -> Cadence;

    /// Evaluate hermetically: read committed state via `view`, draw from `ctx`'s stream,
    /// and return proposals. Mutates no shared state (Vol. V Ch. 3 §3.1).
    fn evaluate(&self, view: &dyn CommittedView, ctx: &TickContext) -> Vec<Proposal>;
}

#[cfg(test)]
mod tests {
    use super::{CommittedView, ScopedView};
    use crate::fact::{Cause, Fact, FactKey, FactType, Provenance, SystemId};
    use crate::identity::EntityId;
    use crate::value::Value;
    use std::collections::BTreeMap;

    const A: FactType = FactType::new("test.a");
    const B: FactType = FactType::new("test.b");

    struct MapView(BTreeMap<FactKey, Fact>);
    impl CommittedView for MapView {
        fn read(&self, key: FactKey) -> Option<Fact> {
            self.0.get(&key).copied()
        }
        fn read_all(&self, key: FactKey) -> Vec<Fact> {
            self.0.get(&key).copied().into_iter().collect()
        }
        fn entities_with(&self, fact_type: FactType) -> Vec<EntityId> {
            self.0
                .keys()
                .filter(|k| k.fact_type == fact_type)
                .map(|k| k.entity)
                .collect()
        }
        fn read_about(&self, holder: EntityId, fact_type: FactType) -> Vec<(EntityId, Fact)> {
            self.0
                .iter()
                .filter(|(k, _)| k.entity == holder && k.fact_type == fact_type)
                .filter_map(|(k, f)| Some((k.about?, *f)))
                .collect()
        }
        fn tick(&self) -> u64 {
            0
        }
    }

    fn map_view() -> MapView {
        let e = EntityId::from_raw(1);
        let fact = Fact::new(
            Value::Int(5),
            Provenance::new(SystemId::new("t"), 0, Cause::new("seed")),
        );
        let mut map = BTreeMap::new();
        map.insert(FactKey::new(e, A), fact);
        map.insert(FactKey::new(e, B), fact);
        MapView(map)
    }

    #[test]
    fn scoped_view_hides_undeclared_fact_types() {
        let e = EntityId::from_raw(1);
        let inner = map_view();

        // A system that only declared reads of A sees A but never B.
        let scoped = ScopedView::new(&inner, &[A]);
        assert!(scoped.read(FactKey::new(e, A)).is_some());
        assert!(scoped.violation().is_none());
        assert!(scoped.read(FactKey::new(e, B)).is_none());
        // The undeclared read is recorded — the tick loop turns it into a failed tick
        // (Vol. V Ch. 3 §3.5, invariant 2).
        assert_eq!(scoped.violation(), Some(B));
    }

    #[test]
    fn scoped_view_records_undeclared_enumeration() {
        let inner = map_view();
        let scoped = ScopedView::new(&inner, &[A]);
        assert_eq!(scoped.entities_with(A).len(), 1);
        assert!(scoped.violation().is_none());
        assert!(scoped.entities_with(B).is_empty());
        assert_eq!(scoped.violation(), Some(B));
    }

    #[test]
    fn scoped_view_records_undeclared_pair_reads() {
        let inner = map_view();
        let scoped = ScopedView::new(&inner, &[A]);
        assert!(scoped.read_about(EntityId::from_raw(1), A).is_empty());
        assert!(scoped.violation().is_none());
        assert!(scoped.read_about(EntityId::from_raw(1), B).is_empty());
        assert_eq!(scoped.violation(), Some(B));
    }
}
