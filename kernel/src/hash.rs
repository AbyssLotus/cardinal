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
//! order-independent 128-bit accumulators — a wrapping sum and an XOR of a well-mixed
//! derivative — plus a fact count. Because addition and XOR are commutative and
//! associative, the final [`StateHash`] does not depend on the order facts were added;
//! because both operations are invertible, a fact can also be *removed*
//! ([`StateHasher::remove_fact`]), which lets a store maintain the digest incrementally
//! across commits instead of rescanning all of reality each tick. The hash is defined
//! purely in terms of the input bytes using fixed, self-contained mixing (FNV-1a 128 for
//! the per-fact digest, SplitMix64 for the strengthening pass), so it is stable across
//! platforms and does not rely on the standard library's deliberately unspecified hashers.
//!
//! The caller owns *canonicalisation*: each fact must be encoded with fixed field order and
//! fixed-width, fixed-endian integers, so equal facts always produce equal bytes. Equal
//! committed states always produce equal digests; distinct states produce distinct digests
//! except with negligible probability (a 128-bit per-fact digest feeding a 256-bit
//! accumulator). This is a determinism check, not a cryptographic commitment — a production
//! store may later swap in a cryptographic digest; the [`StateHash`] type and its role in
//! the harness stay the same.

/// FNV-1a 128-bit offset basis.
const FNV128_OFFSET: u128 = 0x6c62_272e_07bb_0142_62b8_2175_6295_c58d;
/// FNV-1a 128-bit prime.
const FNV128_PRIME: u128 = 0x0000_0000_0100_0000_0000_0000_0000_013b;

/// Fixed, self-contained per-fact digest: FNV-1a 128 over the fact's canonical bytes.
///
/// Chosen over `std`'s `DefaultHasher` because the standard hashers are explicitly not
/// guaranteed stable across releases or platforms; a determinism digest must be. The
/// 128-bit variant keeps accidental per-fact collisions negligible where a 64-bit digest
/// would leave them merely unlikely.
fn fnv1a_128(bytes: &[u8]) -> u128 {
    let mut hash = FNV128_OFFSET;
    for &b in bytes {
        hash ^= u128::from(b);
        hash = hash.wrapping_mul(FNV128_PRIME);
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

/// Mix a 128-bit per-fact digest lane-wise for the XOR accumulator, so the two
/// accumulators never cancel each other's information.
fn mix128(h: u128) -> u128 {
    let lo = splitmix64(h as u64);
    let hi = splitmix64((h >> 64) as u64);
    (u128::from(hi) << 64) | u128::from(lo)
}

/// A large odd constant (2^128 / φ, forced odd) spreading the fact count across the sum
/// accumulator's bits at finalisation.
const COUNT_SPREAD: u128 = 0x9e37_79b9_7f4a_7c15_f39c_c060_5ced_c835;

/// A canonical, order-independent digest of committed reality at a single tick.
///
/// Equal committed states always yield equal digests; unequal states yield unequal digests
/// except with negligible probability. This is the atom of the determinism harness
/// (Vol. V Ch. 4 §4.2): twin runs pass when their per-tick `StateHash` sequences match
/// exactly.
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
/// adding, or removing a fact changes the digest. [`StateHasher::remove_fact`] exactly
/// inverts an earlier `add_fact` of the same bytes, so a store may keep one hasher alive
/// and update it per commit — the digest of the running hasher equals the digest of a
/// fresh rescan (the store's contract test holds it to that).
#[derive(Clone, Debug, Default)]
pub struct StateHasher {
    /// Order-independent wrapping sum of per-fact digests.
    sum: u128,
    /// Order-independent XOR of the mixed per-fact digests.
    xor: u128,
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
        let h = fnv1a_128(bytes);
        self.sum = self.sum.wrapping_add(h);
        self.xor ^= mix128(h);
        self.count = self.count.wrapping_add(1);
    }

    /// Remove one previously-added fact, given the same canonical byte encoding.
    ///
    /// The exact inverse of [`StateHasher::add_fact`]: subtracting from the sum and
    /// re-XORing the mixed digest restores the accumulators to their prior state. The
    /// caller must only remove facts it actually added — the hasher cannot detect a
    /// removal of bytes never folded in.
    pub fn remove_fact(&mut self, bytes: &[u8]) {
        let h = fnv1a_128(bytes);
        self.sum = self.sum.wrapping_sub(h);
        self.xor ^= mix128(h);
        self.count = self.count.wrapping_sub(1);
    }

    /// Finalise the accumulators into a canonical [`StateHash`].
    ///
    /// Packs the count-strengthened sum and the XOR accumulator into the 32-byte digest.
    /// With no facts added this returns [`StateHash::EMPTY`].
    pub fn finish(&self) -> StateHash {
        let sum = self
            .sum
            .wrapping_add(u128::from(self.count).wrapping_mul(COUNT_SPREAD));
        let mut out = [0u8; 32];
        out[0..16].copy_from_slice(&sum.to_le_bytes());
        out[16..32].copy_from_slice(&self.xor.to_le_bytes());
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

    #[test]
    fn remove_exactly_inverts_add() {
        // Incremental maintenance: add a, b, c, remove b -> identical to adding a, c.
        let a = &b"entity:1|value:10"[..];
        let b = &b"entity:2|value:20"[..];
        let c = &b"entity:3|value:30"[..];
        let mut h = StateHasher::new();
        h.add_fact(a);
        h.add_fact(b);
        h.add_fact(c);
        h.remove_fact(b);
        assert_eq!(h.finish(), hash_facts(&[a, c]));
        // Removing everything restores the empty digest.
        h.remove_fact(a);
        h.remove_fact(c);
        assert_eq!(h.finish(), StateHash::EMPTY);
    }
}
