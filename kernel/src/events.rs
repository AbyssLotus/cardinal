//! The chronicle — Vol. V Ch. 6 §6.1.
//!
//! Every committed change records why it happened. The chronicle is append-only and
//! causally linked: each entry names the proposal cause that produced it, forming the
//! audit trail the causal debugger and the historian read (Vol. V Ch. 6 §6.1). Entries are
//! written in the `chronicle` stage, after commit — never before (Vol. V Ch. 3 §3.1).

use crate::fact::{Cause, FactType};
use crate::identity::EntityId;

/// A single append-only chronicle entry: a committed change and the cause that produced it
/// (Vol. V Ch. 6 §6.1).
///
/// Entries are immutable once written and causally linked to the proposal's asserted
/// cause, so any state can be explained by walking its causes backward.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ChronicleEntry {
    tick: u64,
    subject: EntityId,
    fact_type: FactType,
    cause: Cause,
}

impl ChronicleEntry {
    /// Record that `subject`'s `fact_type` changed at `tick`, for the given `cause`.
    ///
    /// Constructed only in the `chronicle` stage of a committed tick (Vol. V Ch. 3 §3.1);
    /// entries are never fabricated ahead of commit.
    pub const fn new(tick: u64, subject: EntityId, fact_type: FactType, cause: Cause) -> Self {
        Self {
            tick,
            subject,
            fact_type,
            cause,
        }
    }

    /// The tick at which the recorded change was committed.
    pub const fn tick(&self) -> u64 {
        self.tick
    }

    /// The entity the recorded change concerns.
    pub const fn subject(&self) -> EntityId {
        self.subject
    }

    /// The fact type that changed.
    pub const fn fact_type(&self) -> FactType {
        self.fact_type
    }

    /// The event cause the producing proposal asserted.
    pub const fn cause(&self) -> Cause {
        self.cause
    }
}
