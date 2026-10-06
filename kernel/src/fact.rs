//! Facts and their provenance — Vol. II Ch. 1; Vol. V Ch. 2 §2.1.
//!
//! The atom of committed reality is `(entity, fact_type, value, provenance)`
//! (Vol. V Ch. 2 §2.1). Provenance is never optional: every fact answers who, when, and
//! why (source system, tick, cause). A store may compress provenance; it may never shed it
//! (Vol. V Ch. 2 §2.1, clause 2).

use crate::identity::EntityId;
use crate::value::Value;

/// How many values a fact type may hold for a single entity (Vol. V Ch. 2 §2.1; the
/// cardinality-one / cardinality-many distinction of a fact store).
///
/// A cardinality-one fact is a function of its entity — one temperature per region. A
/// cardinality-many fact is set-valued — a region's several neighbours, a location's several
/// overlapping regions (Vol. III Ch. 1 §1.5, §1.7). The owning domain declares which via
/// `Domain::cardinality`; the kernel stores and resolves each accordingly.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Cardinality {
    /// At most one value per entity. Set/Delta/Tombstone changes; the value is replaced.
    One,
    /// A set of values per entity. Add/Remove changes; values accumulate as a set.
    Many,
}

/// The type of a fact, namespaced by its owning domain (e.g. `"physical.env.temperature"`).
///
/// Each fact type has exactly one owning domain (Appendix A). The name is a stable static
/// string. A fact type also carries a code computed from its name when it is declared, and is
/// compared and ordered by that code first and its name only to break a tie: the same order on
/// every run and platform (Vol. V Ch. 4 §4.1), without comparing long names on every lookup.
#[derive(Clone, Copy)]
pub struct FactType {
    code: u64,
    name: &'static str,
}

impl FactType {
    /// Sorts before every fact type: where a walk over all of one entity's facts begins.
    pub const MIN: FactType = FactType { code: 0, name: "" };

    /// Declare a fact type from its stable, domain-namespaced name.
    pub const fn new(name: &'static str) -> Self {
        Self {
            code: fnv1a_64(name),
            name,
        }
    }

    /// The fact type's stable name.
    pub const fn name(&self) -> &'static str {
        self.name
    }

    /// Whether two names are the same name: the same static string, or equal text.
    fn same_name(&self, other: &Self) -> bool {
        std::ptr::eq(self.name, other.name) || self.name == other.name
    }
}

/// FNV-1a 64-bit over a name, at compile time: a fact type's code.
const fn fnv1a_64(name: &str) -> u64 {
    let bytes = name.as_bytes();
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    let mut i = 0;
    while i < bytes.len() {
        h ^= bytes[i] as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
        i += 1;
    }
    h
}

impl PartialEq for FactType {
    fn eq(&self, other: &Self) -> bool {
        self.code == other.code && self.same_name(other)
    }
}

impl Eq for FactType {}

impl Ord for FactType {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.code.cmp(&other.code).then_with(|| {
            if self.same_name(other) {
                std::cmp::Ordering::Equal
            } else {
                self.name.cmp(other.name)
            }
        })
    }
}

impl PartialOrd for FactType {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl std::hash::Hash for FactType {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.code.hash(state);
    }
}

impl std::fmt::Debug for FactType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("FactType").field(&self.name).finish()
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

/// The address of a single fact: an entity and a fact type (Vol. V Ch. 2 §2.1) — and, for a
/// fact about a pair, the second entity it is about (Amendment A-7).
///
/// Keys order by entity, then fact type, then the second entity (a fact of one entity first), so
/// all of an entity's facts — and all of a holder's facts of one type — are contiguous.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct FactKey {
    /// The entity the fact belongs to: for a fact about a pair, its holder.
    pub entity: EntityId,
    /// The kind of fact.
    pub fact_type: FactType,
    /// For a fact about a pair, the second entity — what Erin's belief is *about*, whom she
    /// trusts. `None` for a fact of one entity.
    pub about: Option<EntityId>,
}

impl FactKey {
    /// Address a fact of one entity by its entity and type.
    pub const fn new(entity: EntityId, fact_type: FactType) -> Self {
        Self {
            entity,
            fact_type,
            about: None,
        }
    }

    /// Address a fact about a pair: `holder`'s `fact_type` about `about` (Amendment A-7).
    pub const fn pair(holder: EntityId, fact_type: FactType, about: EntityId) -> Self {
        Self {
            entity: holder,
            fact_type,
            about: Some(about),
        }
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

#[cfg(test)]
mod tests {
    use super::FactType;

    #[test]
    fn min_sorts_before_every_fact_type() {
        for name in ["", "a", "physical.space.contained_in", "zzzz", "mind.goal"] {
            assert!(FactType::MIN <= FactType::new(name), "{name}");
        }
    }

    #[test]
    fn fact_types_compare_by_name_whatever_their_code() {
        // Equal names are equal however they were declared; different names never are, and
        // the order is total and the same every run.
        let a = FactType::new("physical.space.contained_in");
        let b = FactType::new(concat!("physical.space.", "contained_in"));
        assert_eq!(a, b);
        assert_eq!(a.cmp(&b), std::cmp::Ordering::Equal);
        let c = FactType::new("physical.space.portal_open");
        assert_ne!(a, c);
        assert_eq!(a.cmp(&c), c.cmp(&a).reverse());
    }
}
