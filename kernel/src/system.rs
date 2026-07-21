//! Systems and their hermetic evaluation context (Vol. V Ch. 3 §3.1-3.2).
//!
//! A system reads a committed-state view scoped to its declared read set, draws from an
//! RNG substream the kernel issues, sees the clock, applies its rules, and returns
//! proposals — it touches nothing else (Vol. V Ch. 3 §3.1, hermetic evaluation). That seal
//! is what makes evaluation parallelizable and each system testable in isolation.

use crate::fact::{Fact, FactKey, FactType, SystemId};
use crate::proposal::Proposal;
use crate::rng::{Rng, SubstreamKey};

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
}

/// A committed view scoped to a system's declared read set (Vol. V Ch. 3 §3.1).
///
/// Wraps the full committed view but only reveals fact types the system declared it reads;
/// any other read returns `None`. This enforces the "scoped reads" half of hermetic
/// evaluation — a system cannot depend on a fact it never declared (Vol. V Ch. 3 §3.5,
/// invariant 2). Kernel-internal: the tick loop builds one per system per tick.
pub(crate) struct ScopedView<'a> {
    inner: &'a dyn CommittedView,
    allowed: &'a [FactType],
}

impl<'a> ScopedView<'a> {
    /// Scope `inner` to the `allowed` read set.
    pub(crate) fn new(inner: &'a dyn CommittedView, allowed: &'a [FactType]) -> Self {
        Self { inner, allowed }
    }
}

impl CommittedView for ScopedView<'_> {
    fn read(&self, key: FactKey) -> Option<Fact> {
        if self.allowed.contains(&key.fact_type) {
            self.inner.read(key)
        } else {
            None
        }
    }

    fn read_all(&self, key: FactKey) -> Vec<Fact> {
        if self.allowed.contains(&key.fact_type) {
            self.inner.read_all(key)
        } else {
            Vec::new()
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
}

impl TickContext {
    /// Create a context for a system (identified by `system_code`) at `tick` under world
    /// `seed`. Kernel-internal — only the tick loop builds contexts.
    pub(crate) fn new(tick: u64, seed: u64, system_code: u32) -> Self {
        Self {
            tick,
            seed,
            system_code,
        }
    }

    /// The current tick.
    pub const fn tick(&self) -> u64 {
        self.tick
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
    }

    #[test]
    fn scoped_view_hides_undeclared_fact_types() {
        let e = EntityId::from_raw(1);
        let fact = Fact::new(
            Value::Int(5),
            Provenance::new(SystemId::new("t"), 0, Cause::new("seed")),
        );
        let mut map = BTreeMap::new();
        map.insert(FactKey::new(e, A), fact);
        map.insert(FactKey::new(e, B), fact);
        let inner = MapView(map);

        // A system that only declared reads of A sees A but never B.
        let scoped = ScopedView::new(&inner, &[A]);
        assert!(scoped.read(FactKey::new(e, A)).is_some());
        assert!(scoped.read(FactKey::new(e, B)).is_none());
    }
}
