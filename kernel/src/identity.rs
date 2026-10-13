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

    /// A fixed offset within `-by..=by` derived from this id alone (SplitMix64 of it): the same
    /// for the same entity in every run and on every platform, and spread evenly across entities.
    /// For traits that differ from one entity to the next but never change — the hours a resident
    /// keeps (Amendment A-23) — without drawing on the world's randomness. Zero when `by` is not
    /// positive.
    pub const fn jitter(&self, by: i64) -> i64 {
        if by <= 0 {
            return 0;
        }
        let mut z = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^= z >> 31;
        (z % (2 * by as u64 + 1)) as i64 - by
    }
}

#[cfg(test)]
mod tests {
    use super::EntityId;

    #[test]
    fn ids_order_by_raw_value() {
        assert!(EntityId::from_raw(1) < EntityId::from_raw(2));
    }

    #[test]
    fn jitter_is_fixed_bounded_and_spread() {
        let offsets: Vec<i64> = (0..1_000)
            .map(|i| EntityId::from_raw(i).jitter(45))
            .collect();
        assert!(offsets.iter().all(|o| (-45..=45).contains(o)));
        assert_eq!(
            EntityId::from_raw(7).jitter(45),
            EntityId::from_raw(7).jitter(45)
        );
        assert_eq!(EntityId::from_raw(7).jitter(0), 0);
        // Spread: both halves well represented.
        let early = offsets.iter().filter(|o| **o < 0).count();
        assert!((400..600).contains(&early), "{early}");
    }
}
