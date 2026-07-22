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
//! temperature and its several neighbours. Alongside the triples it maintains, per commit:
//! a fact-type index serving [`CommittedView::entities_with`] (queries are the product,
//! Vol. V Ch. 2 §2.1, clause 5), and a running [`StateHasher`] so the per-tick digest is
//! O(changes), not a rescan of all reality (Vol. V Ch. 4 §4.2).

use crate::fact::{Fact, FactKey, FactType, Provenance};
use crate::hash::{StateHash, StateHasher};
use crate::identity::EntityId;
use crate::system::CommittedView;
use crate::value::Value;
use std::collections::{BTreeMap, BTreeSet};

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
/// (Vol. V Ch. 2 §2.2), with a fact-type index and an incrementally-maintained digest.
#[derive(Clone, Debug, Default)]
pub struct MemoryStore {
    // Per fact key, the committed values (each with provenance). A cardinality-one fact holds
    // at most one; a cardinality-many fact holds a set. Sorted throughout for determinism.
    facts: BTreeMap<FactKey, BTreeMap<Value, Provenance>>,
    // Which entities bear each fact type — the index behind `entities_with`
    // (Vol. V Ch. 2 §2.1, clause 5). Maintained on every seed and apply; an entity is present
    // exactly when it holds at least one value of that type.
    by_type: BTreeMap<FactType, BTreeSet<EntityId>>,
    // Running digest of committed state, updated per change so `state_hash` is O(1)
    // (Vol. V Ch. 4 §4.2). Provenance is excluded — the digest summarises committed values.
    hasher: StateHasher,
}

/// The canonical triple encoding fed to the state hasher: entity id (8 bytes LE), fact-type
/// name, value (9 fixed bytes). The fixed-width head and tail make the variable-width name
/// unambiguous (Vol. V Ch. 4 §4.2, canonicalisation).
fn triple_bytes(key: FactKey, value: &Value) -> Vec<u8> {
    let name = key.fact_type.name().as_bytes();
    let mut buf = Vec::with_capacity(8 + name.len() + 9);
    buf.extend_from_slice(&key.entity.raw().to_le_bytes());
    buf.extend_from_slice(name);
    buf.extend_from_slice(&value.canonical_bytes());
    buf
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
        let values = self.facts.entry(key).or_default();
        if values.insert(fact.value, fact.provenance).is_none() {
            // A genuinely new triple: fold it into the digest and the type index. Re-seeding
            // an existing value only refreshes provenance, which the digest excludes.
            self.hasher.add_fact(&triple_bytes(key, &fact.value));
            self.by_type
                .entry(key.fact_type)
                .or_default()
                .insert(key.entity);
        }
    }

    /// The total number of committed values (triples) across all facts.
    pub fn len(&self) -> usize {
        self.facts.values().map(BTreeMap::len).sum()
    }

    /// Whether the world holds no committed values.
    pub fn is_empty(&self) -> bool {
        self.facts.values().all(BTreeMap::is_empty)
    }

    /// Drop every value at `key`, unfolding each from the digest and index. The shared
    /// removal path for tombstones and whole-set replacement.
    fn clear_key(&mut self, key: FactKey) {
        if let Some(values) = self.facts.remove(&key) {
            for value in values.keys() {
                self.hasher.remove_fact(&triple_bytes(key, value));
            }
            if let Some(entities) = self.by_type.get_mut(&key.fact_type) {
                entities.remove(&key.entity);
                if entities.is_empty() {
                    self.by_type.remove(&key.fact_type);
                }
            }
        }
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

    fn entities_with(&self, fact_type: FactType) -> Vec<EntityId> {
        self.by_type
            .get(&fact_type)
            .map(|entities| entities.iter().copied().collect())
            .unwrap_or_default()
    }
}

impl RealityStore for MemoryStore {
    fn facts_of(&self, entity: EntityId) -> Vec<(FactKey, Fact)> {
        // Keys sort entity-first, so one range walk visits exactly this entity's facts
        // (Vol. V Ch. 2 §2.1, clause 5) — no scan of the rest of reality.
        let mut out = Vec::new();
        let start = FactKey::new(entity, FactType::new(""));
        for (key, values) in self.facts.range(start..) {
            if key.entity != entity {
                break;
            }
            for (v, p) in values {
                out.push((*key, Fact::new(*v, *p)));
            }
        }
        out
    }

    fn apply(&mut self, batch: CommitBatch) {
        for resolution in batch.resolutions {
            match resolution {
                Resolution::One { key, fact } => {
                    self.clear_key(key);
                    self.seed(key, fact);
                }
                Resolution::Many { key, facts } => {
                    self.clear_key(key);
                    for fact in facts {
                        self.seed(key, fact);
                    }
                }
                Resolution::Clear { key } => {
                    self.clear_key(key);
                }
            }
        }
    }

    fn state_hash(&self) -> StateHash {
        // O(1): the running hasher was maintained by every seed and apply; the contract test
        // below holds it equal to a full rescan (Vol. V Ch. 4 §4.2).
        self.hasher.finish()
    }
}

#[cfg(test)]
mod tests {
    use super::{CommitBatch, MemoryStore, RealityStore, Resolution};
    use crate::fact::{Cause, Fact, FactKey, FactType, Provenance, SystemId};
    use crate::hash::StateHasher;
    use crate::identity::EntityId;
    use crate::system::CommittedView;
    use crate::value::Value;

    const HEAT: FactType = FactType::new("test.heat");
    const NEIGHBOUR: FactType = FactType::new("test.neighbour");

    fn fact(v: i64) -> Fact {
        Fact::new(
            Value::Int(v),
            Provenance::new(SystemId::new("t"), 0, Cause::new("seed")),
        )
    }

    /// The digest a fresh rescan of `store` would produce — the reference the incremental
    /// hasher is held to.
    fn rescan_hash(store: &MemoryStore) -> crate::hash::StateHash {
        let mut hasher = StateHasher::new();
        for e in 0..100u64 {
            for (key, f) in store.facts_of(EntityId::from_raw(e)) {
                hasher.add_fact(&super::triple_bytes(key, &f.value));
            }
        }
        hasher.finish()
    }

    #[test]
    fn incremental_hash_matches_full_rescan() {
        // Seed, overwrite, accumulate a set, tombstone — after each commit the running
        // digest must equal a from-scratch rescan (Vol. V Ch. 4 §4.2).
        let e1 = EntityId::from_raw(1);
        let e2 = EntityId::from_raw(2);
        let mut store = MemoryStore::new();
        store.seed(FactKey::new(e1, HEAT), fact(10));
        store.seed(FactKey::new(e2, NEIGHBOUR), fact(1));
        store.seed(FactKey::new(e2, NEIGHBOUR), fact(3));
        assert_eq!(store.state_hash(), rescan_hash(&store));

        let mut batch = CommitBatch::new(1);
        batch.resolutions.push(Resolution::One {
            key: FactKey::new(e1, HEAT),
            fact: fact(11),
        });
        batch.resolutions.push(Resolution::Many {
            key: FactKey::new(e2, NEIGHBOUR),
            facts: vec![fact(3), fact(5)],
        });
        store.apply(batch);
        assert_eq!(store.state_hash(), rescan_hash(&store));

        let mut clear = CommitBatch::new(2);
        clear.resolutions.push(Resolution::Clear {
            key: FactKey::new(e1, HEAT),
        });
        store.apply(clear);
        assert_eq!(store.state_hash(), rescan_hash(&store));
    }

    #[test]
    fn entities_with_tracks_commits() {
        let e1 = EntityId::from_raw(1);
        let e2 = EntityId::from_raw(2);
        let mut store = MemoryStore::new();
        store.seed(FactKey::new(e2, HEAT), fact(20));
        store.seed(FactKey::new(e1, HEAT), fact(10));
        // Deterministic ascending order regardless of seed order.
        assert_eq!(store.entities_with(HEAT), vec![e1, e2]);
        assert!(store.entities_with(NEIGHBOUR).is_empty());

        // A tombstone removes the entity from the index.
        let mut batch = CommitBatch::new(1);
        batch.resolutions.push(Resolution::Clear {
            key: FactKey::new(e1, HEAT),
        });
        store.apply(batch);
        assert_eq!(store.entities_with(HEAT), vec![e2]);
    }

    #[test]
    fn facts_of_returns_only_that_entity() {
        let e1 = EntityId::from_raw(1);
        let e2 = EntityId::from_raw(2);
        let mut store = MemoryStore::new();
        store.seed(FactKey::new(e1, HEAT), fact(10));
        store.seed(FactKey::new(e1, NEIGHBOUR), fact(2));
        store.seed(FactKey::new(e2, HEAT), fact(20));
        let of_e1 = store.facts_of(e1);
        assert_eq!(of_e1.len(), 2);
        assert!(of_e1.iter().all(|(k, _)| k.entity == e1));
    }
}
