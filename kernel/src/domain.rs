//! Domain registration and the owned-fact rules the kernel consults
//! (Vol. V Ch. 3 §3.1, stages 3-4; Appendix A).
//!
//! A domain owns a set of fact types and supplies the systems that propose changes to
//! them, the composition rule that resolves competing proposals against each owned fact
//! (Vol. IV Ch. 2), and the coherence checks that validate a resolved value. The kernel
//! calls these; a domain never writes another domain's facts (Appendix A, Ruling 9).

use crate::fact::{Cardinality, FactType, SystemId};
use crate::proposal::Change;
use crate::spatial::SpatialProjector;
use crate::system::System;
use crate::value::Value;
use std::sync::Arc;

/// The outcome of composing the proposals for one fact.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Resolved {
    /// The fact resolves to this committed value.
    Write(Value),
    /// The fact is removed (tombstoned).
    Tombstone,
}

/// A resolution failure: competing proposals with no declared way to compose
/// (Vol. V Ch. 3 §3.5, invariant 4). Surfaces as an aborted tick.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ResolveError {
    /// A human-readable reason resolution could not proceed.
    pub reason: &'static str,
}

impl ResolveError {
    /// Fail resolution with a reason.
    pub const fn new(reason: &'static str) -> Self {
        Self { reason }
    }
}

/// A coherence-check failure during the Validate stage (Vol. V Ch. 3 §3.1). Aborts the tick.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ValidationError {
    /// A human-readable reason the resolved value was rejected.
    pub reason: &'static str,
}

impl ValidationError {
    /// Reject a resolved value with a reason.
    pub const fn new(reason: &'static str) -> Self {
        Self { reason }
    }
}

/// A domain plugged into the kernel: what it owns, its systems, and its owned-fact rules.
pub trait Domain {
    /// The domain's stable name (e.g. `"physical"`).
    fn name(&self) -> &'static str;

    /// Whether this domain owns `fact_type` (Appendix A: one fact, one owner).
    fn owns(&self, fact_type: FactType) -> bool;

    /// The cardinality of an owned fact type (Vol. V Ch. 2 §2.1). Defaults to
    /// [`Cardinality::One`]; owners override for set-valued facts. The kernel uses this to
    /// store and resolve the fact: single-value replacement, or set accumulation.
    fn cardinality(&self, fact_type: FactType) -> Cardinality {
        let _ = fact_type;
        Cardinality::One
    }

    /// The systems this domain contributes to the tick.
    fn systems(&self) -> Vec<Box<dyn System>>;

    /// The placement rule for the store's spatial index, if this domain owns space
    /// (Amendment A-2; Appendix A: positions and containment are Physical Reality's). Whatever
    /// assembles a world installs it with `MemoryStore::install_spatial_index`. Defaults to
    /// none: a domain that owns no space has no say in where things are.
    fn spatial_projector(&self) -> Option<Arc<dyn SpatialProjector>> {
        None
    }

    /// Compose competing proposals against one owned fact into a single resolved outcome
    /// (Vol. V Ch. 3 §3.1, Resolve; Vol. IV Ch. 2). `current` is the committed value, if
    /// any; `changes` are the proposed changes to this fact in deterministic order. An
    /// undeclared conflict returns `Err` — resolution never improvises
    /// (Vol. V Ch. 3 §3.5, invariant 4).
    fn compose(
        &self,
        fact_type: FactType,
        current: Option<Value>,
        changes: &[Change],
    ) -> Result<Resolved, ResolveError>;

    /// Validate a resolved value for an owned fact before commit (Vol. V Ch. 3 §3.1,
    /// Validate). Returning `Err` aborts the tick; reality stays at N-1. The default
    /// accepts everything; domains override to enforce their invariants.
    fn validate(&self, fact_type: FactType, value: &Resolved) -> Result<(), ValidationError> {
        let _ = (fact_type, value);
        Ok(())
    }

    /// Validate the resolved value *set* of a cardinality-many owned fact before commit
    /// (Vol. V Ch. 3 §3.1, Validate). The owner sees the whole set an entity would hold,
    /// so it can enforce set-level coherence — member types, size bounds — that no
    /// per-value check could. Returning `Err` aborts the tick; reality stays at N-1. The
    /// default accepts everything; owners of set-valued facts override.
    fn validate_many(&self, fact_type: FactType, values: &[Value]) -> Result<(), ValidationError> {
        let _ = (fact_type, values);
        Ok(())
    }

    /// Whether this owner accepts a proposal to `fact_type` from `system` (Amendment A-5,
    /// *Owners may refuse writers*). An owner that restricts a fact to its own systems answers
    /// `false` for any other proposer, and the kernel fails the tick, named. The default accepts
    /// every proposer — most facts are open to anyone whose proposal the owner's composition
    /// and validation then judge (Appendix A, Ruling 9).
    fn accepts(&self, fact_type: FactType, system: SystemId) -> bool {
        let _ = (fact_type, system);
        true
    }
}
