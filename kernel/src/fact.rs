//! Facts and their provenance — Vol. II Ch. 1; Vol. V Ch. 2 §2.1.
//!
//! The atom of committed reality is `(entity, fact_type, value, provenance)`
//! (Vol. V Ch. 2 §2.1). Provenance is never optional: every fact answers who, when, and
//! why (source system, tick, cause). A store may compress provenance; it may never shed it
//! (Vol. V Ch. 2 §2.1, clause 2).

use crate::identity::EntityId;
use crate::value::Value;

/// The type of a fact, namespaced by its owning domain (e.g. `"physical.env.temperature"`).
///
/// Each fact type has exactly one owning domain (Appendix A). The name is a stable static
/// string; ordering by it gives deterministic, hash-free iteration (Vol. V Ch. 4 §4.1).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct FactType(&'static str);

impl FactType {
    /// Declare a fact type from its stable, domain-namespaced name.
    pub const fn new(name: &'static str) -> Self {
        Self(name)
    }

    /// The fact type's stable name.
    pub const fn name(&self) -> &'static str {
        self.0
    }
}

/// The identity of a system, namespaced by its domain (e.g. `"physical.diurnal_cycle"`).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct SystemId(&'static str);

impl SystemId {
    /// Declare a system id from its stable, domain-namespaced name.
    pub const fn new(name: &'static str) -> Self {
        Self(name)
    }

    /// The system id's stable name.
    pub const fn name(&self) -> &'static str {
        self.0
    }

    /// A stable numeric hash of the name, for seeding RNG substreams (Vol. V Ch. 4 §4.1).
    ///
    /// FNV-1a 32-bit over the name — deterministic and platform-independent, so a system's
    /// stream is identical across runs.
    pub fn code(&self) -> u32 {
        let mut h: u32 = 0x811c_9dc5;
        for &b in self.0.as_bytes() {
            h ^= u32::from(b);
            h = h.wrapping_mul(0x0100_0193);
        }
        h
    }
}

/// The event a proposal asserts it participates in — chronicle honesty's source
/// (Vol. V Ch. 3 §3.1, Proposals). No proposal, no cause; no cause, no commit.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct Cause(&'static str);

impl Cause {
    /// Name the event kind this proposal participates in (e.g. `"diurnal_shift"`).
    pub const fn new(event: &'static str) -> Self {
        Self(event)
    }

    /// The event kind's stable name.
    pub const fn event(&self) -> &'static str {
        self.0
    }
}

/// Who wrote a fact, when, and why (Vol. V Ch. 2 §2.1, clause 2).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Provenance {
    /// The system whose proposal produced this fact.
    pub system: SystemId,
    /// The tick at which it was committed.
    pub tick: u64,
    /// The event the producing proposal asserted.
    pub cause: Cause,
}

impl Provenance {
    /// Assemble provenance from its three answers: who, when, why.
    pub const fn new(system: SystemId, tick: u64, cause: Cause) -> Self {
        Self {
            system,
            tick,
            cause,
        }
    }
}

/// The address of a single fact: an entity and a fact type (Vol. V Ch. 2 §2.1).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct FactKey {
    /// The entity the fact is about.
    pub entity: EntityId,
    /// The kind of fact.
    pub fact_type: FactType,
}

impl FactKey {
    /// Address a fact by its entity and type.
    pub const fn new(entity: EntityId, fact_type: FactType) -> Self {
        Self { entity, fact_type }
    }
}

/// A committed fact's value together with its provenance (Vol. V Ch. 2 §2.1).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Fact {
    /// The committed value.
    pub value: Value,
    /// Who wrote it, when, and why.
    pub provenance: Provenance,
}

impl Fact {
    /// Pair a value with its provenance.
    pub const fn new(value: Value, provenance: Provenance) -> Self {
        Self { value, provenance }
    }
}
