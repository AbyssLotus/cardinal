//! Canonical per-tick state hashing — Vol. V Ch. 4 §4.2.
//!
//! Build this early: every determinism test reduces to comparing this digest across twin
//! runs (Vol. V Ch. 4 §4.2; `tests/kernel`). The hash MUST be canonical — identical
//! committed reality yields an identical digest **regardless of the order facts are
//! visited**. Storage or iteration order must never leak into the digest; this is the
//! belt-and-suspenders companion to the ban on unordered iteration on the simulation path
//! (Vol. V Ch. 4 §4.1, Door 4).
//!
//! ## Design
//! [`StateHasher`] folds each committed fact's canonical byte encoding into two
//! order-independent accumulators — a wrapping sum and an XOR of a well-mixed derivative —
//! plus a fact count. Because addition and XOR are commutative and associative, the final
//! [`StateHash`] does not depend on the order facts were added. The hash is defined purely
//! in terms of the input bytes using fixed, self-contained mixing (FNV-1a for the per-fact
//! digest, SplitMix64 for the strengthening pass), so it is stable across platforms and
//! does not rely on the standard library's deliberately unspecified hashers.
//!
//! The caller owns *canonicalisation*: each fact must be encoded with fixed field order and
//! fixed-width, fixed-endian integers, so equal facts always produce equal bytes. A
//! production store may later swap this multiset construction for a cryptographic digest
//! over a canonically-ordered encoding; the [`StateHash`] type and its role in the harness
//! stay the same.

/// FNV-1a 64-bit offset basis.
const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
/// FNV-1a 64-bit prime.
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

/// Fixed, self-contained per-fact digest: FNV-1a over the fact's canonical bytes.
///
/// Chosen over `std`'s `DefaultHasher` because the standard hashers are explicitly not
/// guaranteed stable across releases or platforms; a determinism digest must be.
fn fnv1a_64(bytes: &[u8]) -> u64 {
    let mut hash = FNV_OFFSET;
    for &b in bytes {
        hash ^= u64::from(b);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

/// SplitMix64 finaliser — a strong, fixed bit-mixing pass that decorrelates the XOR
/// accumulator from the sum accumulator.
fn splitmix64(mut z: u64) -> u64 {
    z = z.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// A canonical, order-independent digest of committed reality at a single tick.
///
/// Two [`StateHash`] values are equal iff the committed state they summarise is equal.
/// This is the atom of the determinism harness (Vol. V Ch. 4 §4.2): twin runs pass when
/// their per-tick `StateHash` sequences match exactly.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct StateHash([u8; 32]);

impl StateHash {
    /// The digest of an empty world — the identity value before any fact is committed.
    ///
    /// Equal to `StateHasher::new().finish()`: with no facts, every accumulator is zero.
    pub const EMPTY: StateHash = StateHash([0u8; 32]);

    /// Construct a [`StateHash`] from a raw 32-byte digest.
    ///
    /// The bytes MUST come from a deterministic, order-independent reduction over committed
    /// facts (Vol. V Ch. 4 §4.2); [`StateHasher`] is the intended producer.
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Borrow the raw digest bytes.
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

/// Accumulates committed facts into a canonical, order-independent [`StateHash`].
///
/// Feed each committed fact's canonical byte encoding to [`StateHasher::add_fact`] in any
/// order, then call [`StateHasher::finish`]. The result is independent of the order facts
/// were added (Vol. V Ch. 4 §4.1, Door 4) and sensitive to any change in the set: altering,
/// adding, or removing a fact changes the digest.
#[derive(Clone, Debug, Default)]
pub struct StateHasher {
    /// Order-independent wrapping sum of per-fact digests.
    sum: u64,
    /// Order-independent XOR of the mixed per-fact digests.
    xor: u64,
    /// Number of facts folded in; distinguishes otherwise-coinciding accumulations.
    count: u64,
}

impl StateHasher {
    /// Create an empty hasher whose [`StateHasher::finish`] equals [`StateHash::EMPTY`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Fold one committed fact, given its canonical byte encoding, into the accumulators.
    ///
    /// The caller guarantees canonicalisation: equal facts must yield equal `bytes` (fixed
    /// field order, fixed-width little-endian integers). Accumulation is commutative, so the
    /// order of `add_fact` calls does not affect the final digest.
    pub fn add_fact(&mut self, bytes: &[u8]) {
        let h = fnv1a_64(bytes);
        self.sum = self.sum.wrapping_add(h);
        self.xor ^= splitmix64(h);
        self.count = self.count.wrapping_add(1);
    }

    /// Finalise the accumulators into a canonical [`StateHash`].
    ///
    /// Packs the sum, XOR, fact count, and their combination into the 32-byte digest. With
    /// no facts added this returns [`StateHash::EMPTY`].
    pub fn finish(&self) -> StateHash {
        let mut out = [0u8; 32];
        out[0..8].copy_from_slice(&self.sum.to_le_bytes());
        out[8..16].copy_from_slice(&self.xor.to_le_bytes());
        out[16..24].copy_from_slice(&self.count.to_le_bytes());
        out[24..32].copy_from_slice(&(self.sum ^ self.xor).to_le_bytes());
        StateHash::from_bytes(out)
    }
}

#[cfg(test)]
mod tests {
    use super::{StateHash, StateHasher};

    fn hash_facts(facts: &[&[u8]]) -> StateHash {
        let mut h = StateHasher::new();
        for f in facts {
            h.add_fact(f);
        }
        h.finish()
    }

    #[test]
    fn empty_differs_from_nonempty() {
        assert_ne!(StateHash::EMPTY, StateHash::from_bytes([1u8; 32]));
    }

    #[test]
    fn equal_bytes_equal_hash() {
        assert_eq!(
            StateHash::from_bytes([7u8; 32]),
            StateHash::from_bytes([7u8; 32])
        );
    }

    #[test]
    fn new_hasher_is_empty() {
        // Identity invariant: no facts -> the empty digest.
        assert_eq!(StateHasher::new().finish(), StateHash::EMPTY);
    }

    #[test]
    fn hash_is_order_independent() {
        // Door 4: visiting order must not leak into the digest (Vol. V Ch. 4 §4.1).
        let a = &b"entity:1|value:10"[..];
        let b = &b"entity:2|value:20"[..];
        let c = &b"entity:3|value:30"[..];
        assert_eq!(hash_facts(&[a, b, c]), hash_facts(&[c, a, b]));
        assert_eq!(hash_facts(&[a, b, c]), hash_facts(&[b, c, a]));
    }

    #[test]
    fn changing_a_fact_changes_the_hash() {
        let base = hash_facts(&[&b"entity:1|value:10"[..], &b"entity:2|value:20"[..]]);
        let changed = hash_facts(&[&b"entity:1|value:11"[..], &b"entity:2|value:20"[..]]);
        assert_ne!(base, changed);
    }

    #[test]
    fn adding_a_fact_changes_the_hash() {
        let base = hash_facts(&[&b"entity:1|value:10"[..]]);
        let bigger = hash_facts(&[&b"entity:1|value:10"[..], &b"entity:2|value:20"[..]]);
        assert_ne!(base, bigger);
    }
}
