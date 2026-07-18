//! Proposals — the tick's currency (Vol. V Ch. 3 §3.1, Proposals).
//!
//! Systems never mutate; they emit proposals (Vol. V Ch. 3 §3.5, invariant 3). Every
//! proposal names the fact it targets, the change it requests, the basis tick it read (for
//! conflict detection), and the event cause it asserts. No cause, no commit
//! (Vol. V Ch. 3 §3.1).

use crate::fact::{Cause, FactKey, SystemId};
use crate::value::Value;

/// The kind of change a proposal requests against a fact (Vol. V Ch. 3 §3.1).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Change {
    /// Bring a fact into existence with an initial value.
    Create(Value),
    /// Set the fact to an absolute value.
    Set(Value),
    /// Add a signed amount to an integer fact (fixed-point deltas compose by summation).
    Delta(i64),
    /// Remove the fact; identity is retained via tombstoning (Vol. V Ch. 2 §2.1, clause 4).
    Tombstone,
}

/// One system's requested change to one fact, with its basis and cause
/// (Vol. V Ch. 3 §3.1).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Proposal {
    /// The system making the proposal.
    pub system: SystemId,
    /// The fact this proposal targets.
    pub target: FactKey,
    /// The committed tick the system read as its basis (conflict detection — §3.1).
    pub basis_tick: u64,
    /// The requested change.
    pub change: Change,
    /// The event this proposal asserts it participates in.
    pub cause: Cause,
}

impl Proposal {
    /// Construct a proposal.
    pub const fn new(
        system: SystemId,
        target: FactKey,
        basis_tick: u64,
        change: Change,
        cause: Cause,
    ) -> Self {
        Self {
            system,
            target,
            basis_tick,
            change,
            cause,
        }
    }
}
