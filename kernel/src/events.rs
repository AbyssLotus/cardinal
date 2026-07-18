//! The chronicle — Vol. V Ch. 6 §6.1.
//!
//! Every committed change records why it happened. The chronicle is append-only and
//! causally linked: each entry names the proposal cause that produced it, forming the
//! audit trail the causal debugger and the historian read (Vol. V Ch. 6 §6.1). Entries are
//! written in the `chronicle` stage, after commit — never before (Vol. V Ch. 3 §3.1).

use crate::identity::EntityId;

/// A single append-only chronicle entry: a committed change and the entity it concerns
/// (Vol. V Ch. 6 §6.1).
///
/// Entries are immutable once written and causally linked to the proposal that produced
/// them, so any state can be explained by walking its causes backward. The cause link is
/// scaffolded here as the subject and tick; the full proposal-cause reference lands with
/// the tick's resolve/commit stages (Vol. V Ch. 3 §3.1).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ChronicleEntry {
    tick: u64,
    subject: EntityId,
}

impl ChronicleEntry {
    /// Record a committed change to `subject` at `tick`.
    ///
    /// Constructed only in the `chronicle` stage of a committed tick (Vol. V Ch. 3 §3.1);
    /// entries are never fabricated ahead of commit.
    pub const fn new(tick: u64, subject: EntityId) -> Self {
        Self { tick, subject }
    }

    /// The tick at which the recorded change was committed.
    pub const fn tick(&self) -> u64 {
        self.tick
    }

    /// The entity the recorded change concerns.
    pub const fn subject(&self) -> EntityId {
        self.subject
    }
}
