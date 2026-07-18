//! The Reality Store contract — Vol. V Ch. 2 §2.1.
//!
//! The store is the single source of committed truth. Its defining law: **`apply` is the
//! only mutation path in the entire engine** (Vol. V Ch. 2 §2.1, clause 1). Systems never
//! mutate — they read committed reality and emit proposals, and only a committed tick
//! calls `apply` (Vol. V Ch. 3 §3.1). A failed `apply` leaves reality exactly at N-1
//! (Vol. V Ch. 3 §3.5.5).

use crate::hash::StateHash;

/// The committed reality store, behind its contract (Vol. V Ch. 2 §2.1).
///
/// Any backend — the POC's simple map, a future tiered columnar store — is a valid store
/// iff it satisfies this contract and passes the conformance suite in `tests/kernel`
/// (Vol. V Ch. 2 §2.1). Read methods observe committed state only: there is no way to
/// observe uncommitted, in-flight proposals through this surface.
pub trait RealityStore {
    /// A read-only, point-in-time view of committed reality handed to a system for the
    /// duration of one hermetic evaluation (Vol. V Ch. 3 §3.1).
    type Snapshot;

    /// A single committed fact, keyed by identity.
    type Fact;

    /// A scoped query over committed reality.
    type Query;

    /// The result of a [`RealityStore::Query`].
    type QueryResult;

    /// A batch of validated mutations produced by a resolved, validated tick — the only
    /// thing [`RealityStore::apply`] will accept (Vol. V Ch. 3 §3.1, stages 3-5).
    type Commit;

    /// A serializable full-state capture for persistence (Vol. V Ch. 7).
    type SnapshotCapture;

    /// An append-only causal chronicle handle (Vol. V Ch. 6 §6.1).
    type History;

    /// Resolve committed reality into a snapshot a system may read hermetically for one
    /// evaluation (Vol. V Ch. 3 §3.1). The snapshot never exposes in-flight proposals.
    fn resolve(&self) -> Self::Snapshot;

    /// Read a single committed fact by identity, or `None` if no such fact is committed.
    fn read(&self, id: crate::identity::EntityId) -> Option<Self::Fact>;

    /// Answer a scoped query over committed reality, mutating nothing.
    fn query(&self, query: Self::Query) -> Self::QueryResult;

    /// Apply a validated commit — **the only mutation path in the engine**
    /// (Vol. V Ch. 2 §2.1, clause 1). On failure, reality MUST remain exactly at N-1
    /// (Vol. V Ch. 3 §3.5.5).
    fn apply(&mut self, commit: Self::Commit);

    /// Capture a serializable snapshot of full committed state for persistence
    /// (Vol. V Ch. 7). Pairs with the chronicle tail for two-road recovery.
    fn snapshot(&self) -> Self::SnapshotCapture;

    /// Borrow the append-only chronicle: the causal record of how committed reality came
    /// to be what it is (Vol. V Ch. 6 §6.1).
    fn history(&self) -> &Self::History;

    /// The canonical digest of committed reality as it now stands (Vol. V Ch. 4 §4.2).
    /// Twin runs are compared tick-by-tick on this value.
    fn state_hash(&self) -> StateHash;
}
