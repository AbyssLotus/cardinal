//! Canonical per-tick state hashing — Vol. V Ch. 4 §4.2.
//!
//! Build this early: every determinism test reduces to comparing this digest across twin
//! runs (Vol. V Ch. 4 §4.2; `tests/kernel`). The hash MUST be canonical — identical
//! committed reality yields an identical digest regardless of iteration order, which is
//! why unordered iteration is forbidden on the simulation path (Vol. V Ch. 4 §4.1, Door 4).

/// A canonical, order-independent digest of committed reality at a single tick.
///
/// Two [`StateHash`] values are equal iff the committed state they summarise is equal.
/// This is the atom of the determinism harness (Vol. V Ch. 4 §4.2): twin runs pass when
/// their per-tick `StateHash` sequences match exactly.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct StateHash([u8; 32]);

impl StateHash {
    /// The digest of an empty world — the identity value before any fact is committed.
    pub const EMPTY: StateHash = StateHash([0u8; 32]);

    /// Construct a [`StateHash`] from a raw 32-byte digest.
    ///
    /// The bytes MUST come from a deterministic, order-independent reduction over committed
    /// facts (Vol. V Ch. 4 §4.2); the canonical hasher is the only intended caller.
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Borrow the raw digest bytes.
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::StateHash;

    #[test]
    fn empty_differs_from_nonempty() {
        assert_ne!(StateHash::EMPTY, StateHash::from_bytes([1u8; 32]));
    }

    #[test]
    fn equal_bytes_equal_hash() {
        assert_eq!(StateHash::from_bytes([7u8; 32]), StateHash::from_bytes([7u8; 32]));
    }
}
