//! The Reality Store contract and a reference in-memory implementation — Vol. V Ch. 2 §2.1.
//!
//! The store is the single source of committed truth. Its defining law: `apply` is the
//! only mutation path in the engine (Vol. V Ch. 2 §2.1, clause 1). Systems read committed
//! state and propose; only a committed tick calls `apply`. A failed apply leaves reality
//! exactly at N-1 (Vol. V Ch. 3 §3.5.5).
//!
//! [`MemoryStore`] is the reference profile: the literal fact/triple model
//! (Vol. V Ch. 2 §2.2), a sorted map from [`FactKey`] to [`Fact`]. It is a valid store
//! because it satisfies the contract; a tiered columnar store would be another
//! (Vol. V Ch. 2 §2.2, Hybrid).

use crate::fact::{Fact, FactKey};
use crate::hash::{StateHash, StateHasher};
use crate::identity::EntityId;
use crate::system::CommittedView;
use std::collections::BTreeMap;

/// A batch of resolved, validated writes — the only thing [`RealityStore::apply`] accepts
/// (Vol. V Ch. 3 §3.1, stages 3-5).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct CommitBatch {
    /// The tick this batch commits (reality becomes N once applied).
    pub tick: u64,
    /// Facts to write or overwrite, in deterministic key order.
    pub writes: Vec<(FactKey, Fact)>,
    /// Facts to tombstone.
    pub tombstones: Vec<FactKey>,
}

impl CommitBatch {
    /// An empty batch for `tick`.
    pub fn new(tick: u64) -> Self {
        Self {
            tick,
            writes: Vec::new(),
            tombstones: Vec::new(),
        }
    }
}

/// The committed reality store, behind its contract (Vol. V Ch. 2 §2.1).
///
/// [`CommittedView`] (its supertrait) supplies the read surface systems get; this trait
/// adds the query slice, the single mutation path, and the state digest.
pub trait RealityStore: CommittedView {
    /// Every committed fact whose entity matches, in deterministic order — a minimal slice
    /// of the query surface (Vol. V Ch. 2 §2.1, clause 5).
    fn facts_of(&self, entity: EntityId) -> Vec<(FactKey, Fact)>;

    /// Apply a resolved, validated batch — the ONLY mutation path (Vol. V Ch. 2 §2.1,
    /// clause 1). Atomic; callers must not invoke it except from the commit stage
    /// (Vol. V Ch. 3 §3.1).
    fn apply(&mut self, batch: CommitBatch);

    /// The canonical, order-independent digest of committed reality (Vol. V Ch. 4 §4.2).
    fn state_hash(&self) -> StateHash;
}

/// The reference store: the fact/triple model as a sorted map (Vol. V Ch. 2 §2.2).
#[derive(Clone, Debug, Default)]
pub struct MemoryStore {
    facts: BTreeMap<FactKey, Fact>,
}

impl MemoryStore {
    /// An empty world.
    pub fn new() -> Self {
        Self::default()
    }

    /// Seed a fact directly at world construction, before tick 1.
    ///
    /// This is world-package loading, not simulation: it builds the initial committed
    /// state rather than running the world, so it is not a system write through the tick's
    /// mutation path (Vol. V Ch. 2 §2.1). Use only to construct the starting reality.
    pub fn seed(&mut self, key: FactKey, fact: Fact) {
        self.facts.insert(key, fact);
    }

    /// The number of committed facts.
    pub fn len(&self) -> usize {
        self.facts.len()
    }

    /// Whether the world holds no facts.
    pub fn is_empty(&self) -> bool {
        self.facts.is_empty()
    }
}

impl CommittedView for MemoryStore {
    fn read(&self, key: FactKey) -> Option<Fact> {
        self.facts.get(&key).copied()
    }
}

impl RealityStore for MemoryStore {
    fn facts_of(&self, entity: EntityId) -> Vec<(FactKey, Fact)> {
        self.facts
            .iter()
            .filter(|(k, _)| k.entity == entity)
            .map(|(k, v)| (*k, *v))
            .collect()
    }

    fn apply(&mut self, batch: CommitBatch) {
        for key in batch.tombstones {
            self.facts.remove(&key);
        }
        for (key, fact) in batch.writes {
            self.facts.insert(key, fact);
        }
    }

    fn state_hash(&self) -> StateHash {
        let mut hasher = StateHasher::new();
        for (key, fact) in &self.facts {
            // Canonical fact encoding: entity id, fact-type name, value. Provenance is
            // excluded — the digest summarises committed state values (Vol. V Ch. 4 §4.2).
            let name = key.fact_type.name().as_bytes();
            let mut buf = Vec::with_capacity(8 + name.len() + 9);
            buf.extend_from_slice(&key.entity.raw().to_le_bytes());
            buf.extend_from_slice(name);
            buf.extend_from_slice(&fact.value.canonical_bytes());
            hasher.add_fact(&buf);
        }
        hasher.finish()
    }
}
