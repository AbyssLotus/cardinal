//! Deterministic scheduling — Vol. V Ch. 3 §3.2.
//!
//! Execution order is DERIVED, not authored: from each system's declared read/write sets
//! the scheduler builds a dependency DAG and produces a total order that is deterministic
//! and published (Vol. V Ch. 3 §3.2). A system may not read a fact it did not declare, and
//! ordering never depends on wall-clock or hash-map iteration (Vol. V Ch. 4 §4.1, Doors 1 & 4).

/// A system's declared read and write sets, from which the scheduler derives a
/// deterministic order (Vol. V Ch. 3 §3.2).
///
/// The declaration is the contract the DAG is built from: a system that touches a fact
/// outside its declared sets is a scheduling-law violation. Fields are private in the
/// scaffold — the concrete fact-type key lands with the store's fact model (Vol. V Ch. 2),
/// and freezing it early would couple the scheduler to a shape not yet decided.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct ReadWriteSet {
    _reserved: (),
}

impl ReadWriteSet {
    /// An empty declaration: a system that reads and writes nothing. Placeholder until the
    /// fact-type key exists (Vol. V Ch. 2); real declarations name the facts they touch.
    pub const fn empty() -> Self {
        Self { _reserved: () }
    }
}
