//! Permanent identity — Vol. II; Vol. V Ch. 2 §2.1.4.
//!
//! Ids are permanent and NEVER reused. A dead entity leaves a tombstone, not a freed slot;
//! reusing an id would let two distinct entities share one history and break provenance
//! (Vol. V Ch. 2 §2.1.4).

/// A permanent, globally-unique entity identifier that is never reused.
///
/// Once issued, an [`EntityId`] refers to exactly one entity for the life of the world,
/// even after that entity dies and is tombstoned (Vol. V Ch. 2 §2.1.4).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct EntityId(u64);

impl EntityId {
    /// Wrap a raw id value. Issuance is the store's responsibility; ids MUST be
    /// monotonically drawn and never recycled (Vol. V Ch. 2 §2.1.4).
    pub const fn from_raw(raw: u64) -> Self {
        Self(raw)
    }

    /// The raw numeric value of this id.
    pub const fn raw(&self) -> u64 {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::EntityId;

    #[test]
    fn ids_order_by_raw_value() {
        assert!(EntityId::from_raw(1) < EntityId::from_raw(2));
    }
}
