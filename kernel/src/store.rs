//! The Reality Store contract and a reference in-memory implementation — Vol. V Ch. 2 §2.1.
//!
//! The store is the single source of committed truth. Its defining law: `apply` is the only
//! mutation path in the engine (Vol. V Ch. 2 §2.1, clause 1). Systems read committed state
//! and propose; only a committed tick calls `apply`. A failed apply leaves reality exactly
//! at N-1 (Vol. V Ch. 3 §3.5.5).
//!
//! [`MemoryStore`] is the reference profile: the literal fact/triple model
//! (Vol. V Ch. 2 §2.2). It holds a set of `(entity, fact_type, value)` triples, so a fact
//! type may be cardinality-one (at most one value per entity) or cardinality-many
//! (set-valued) without changing the contract — the same store represents a region's one
//! temperature and its several neighbours.

use crate::fact::{Fact, FactKey, Provenance};
use crate::hash::{StateHash, StateHasher};
use crate::identity::EntityId;
use crate::system::CommittedView;
use crate::value::Value;
use std::collections::BTreeMap;

/// One resolved outcome for a single fact key, ready to commit (Vol. V Ch. 3 §3.1,
/// stages 3-5). Produced by resolution; consumed only by [`RealityStore::apply`].
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Resolution {
    /// Replace the single value at this key (a cardinality-one fact).
    One {
        /// The fact key being written.
        key: FactKey,
        /// The resolved value and its provenance.
        fact: Fact,
    },
    /// Replace the entire value set at this key (a cardinality-many fact). An empty set
    /// removes the fact.
    Many {
        /// The fact key being written.
        key: FactKey,
        /// The resolved set of values, each with provenance.
        facts: Vec<Fact>,
    },
    /// Remove all values at this key (tombstone the fact).
    Clear {
        /// The fact key being removed.
        key: FactKey,
    },
}

/// A batch of resolved, validated outcomes — the only thing [`RealityStore::apply`] accepts
/// (Vol. V Ch. 3 §3.1, stages 3-5).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct CommitBatch {
    /// The tick this batch commits (reality becomes N once applied).
    pub tick: u64,
    /// The per-key resolutions to apply, in deterministic key order.
    pub resolutions: Vec<Resolution>,
}

impl CommitBatch {
    /// An empty batch for `tick`.
    pub fn new(tick: u64) -> Self {
        Self {
            tick,
            resolutions: Vec::new(),
        }
    }
}

/// The committed reality store, behind its contract (Vol. V Ch. 2 §2.1).
///
/// [`CommittedView`] (its supertrait) supplies the read surface systems get; this trait adds
/// the query slice, the single mutation path, and the state digest.
pub trait RealityStore: CommittedView {
    /// Every committed fact whose entity matches, in deterministic order — a minimal slice
    /// of the query surface (Vol. V Ch. 2 §2.1, clause 5). Cardinality-many facts contribute
    /// one entry per value.
    fn facts_of(&self, entity: EntityId) -> Vec<(FactKey, Fact)>;

    /// Apply a resolved, validated batch — the ONLY mutation path (Vol. V Ch. 2 §2.1,
    /// clause 1). Atomic; callers must not invoke it except from the commit stage
    /// (Vol. V Ch. 3 §3.1).
    fn apply(&mut self, batch: CommitBatch);

    /// The canonical, order-independent digest of committed reality (Vol. V Ch. 4 §4.2).
    fn state_hash(&self) -> StateHash;
}

/// The reference store: the fact/triple model as a sorted map of value sets
/// (Vol. V Ch. 2 §2.2).
#[derive(Clone, Debug, Default)]
pub struct MemoryStore {
    // Per fact key, the committed values (each with provenance). A cardinality-one fact holds
    // at most one; a cardinality-many fact holds a set. Sorted throughout for determinism.
    facts: BTreeMap<FactKey, BTreeMap<Value, Provenance>>,
}

impl MemoryStore {
    /// An empty world.
    pub fn new() -> Self {
        Self::default()
    }

    /// Seed a value at world construction, before tick 1.
    ///
    /// This is world-package loading, not simulation: it builds the initial committed state
    /// rather than running the world, so it is not a system write through the tick's mutation
    /// path (Vol. V Ch. 2 §2.1). Values accumulate, so seeding a cardinality-many fact means
    /// calling this once per value.
    pub fn seed(&mut self, key: FactKey, fact: Fact) {
        self.facts
            .entry(key)
            .or_default()
            .insert(fact.value, fact.provenance);
    }

    /// The total number of committed values (triples) across all facts.
    pub fn len(&self) -> usize {
        self.facts.values().map(BTreeMap::len).sum()
    }

    /// Whether the world holds no committed values.
    pub fn is_empty(&self) -> bool {
        self.facts.values().all(BTreeMap::is_empty)
    }
}

impl CommittedView for MemoryStore {
    fn read(&self, key: FactKey) -> Option<Fact> {
        self.facts
            .get(&key)
            .and_then(|m| m.iter().next())
            .map(|(v, p)| Fact::new(*v, *p))
    }

    fn read_all(&self, key: FactKey) -> Vec<Fact> {
        self.facts
            .get(&key)
            .map(|m| m.iter().map(|(v, p)| Fact::new(*v, *p)).collect())
            .unwrap_or_default()
    }
}

impl RealityStore for MemoryStore {
    fn facts_of(&self, entity: EntityId) -> Vec<(FactKey, Fact)> {
        let mut out = Vec::new();
        for (key, values) in &self.facts {
            if key.entity == entity {
                for (v, p) in values {
                    out.push((*key, Fact::new(*v, *p)));
                }
            }
        }
        out
    }

    fn apply(&mut self, batch: CommitBatch) {
        for resolution in batch.resolutions {
            match resolution {
                Resolution::One { key, fact } => {
                    let mut set = BTreeMap::new();
                    set.insert(fact.value, fact.provenance);
                    self.facts.insert(key, set);
                }
                Resolution::Many { key, facts } => {
                    if facts.is_empty() {
                        self.facts.remove(&key);
                    } else {
                        let set = facts.into_iter().map(|f| (f.value, f.provenance)).collect();
                        self.facts.insert(key, set);
                    }
                }
                Resolution::Clear { key } => {
                    self.facts.remove(&key);
                }
            }
        }
    }

    fn state_hash(&self) -> StateHash {
        let mut hasher = StateHasher::new();
        for (key, values) in &self.facts {
            let name = key.fact_type.name().as_bytes();
            for value in values.keys() {
                // Canonical triple encoding: entity id, fact-type name, value. Provenance is
                // excluded — the digest summarises committed state values (Vol. V Ch. 4 §4.2).
                let mut buf = Vec::with_capacity(8 + name.len() + 9);
                buf.extend_from_slice(&key.entity.raw().to_le_bytes());
                buf.extend_from_slice(name);
                buf.extend_from_slice(&value.canonical_bytes());
                hasher.add_fact(&buf);
            }
        }
        hasher.finish()
    }
}
