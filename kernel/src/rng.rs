//! The seeded stream forest — Vol. V Ch. 4 §4.1.
//!
//! All randomness is deterministic and reproducible. Substreams are issued per
//! `(system, tick, scope-key)`, so the same world seed replays exactly
//! (Vol. V Ch. 4 §4.1). Systems receive a stream; they cannot construct one, and no
//! system ever touches OS entropy (Vol. V Ch. 4 §4.1, Door 3).

/// The key identifying one deterministic random substream: `(system, tick, scope)`
/// (Vol. V Ch. 4 §4.1).
///
/// Two draws with the same [`SubstreamKey`] under the same world seed produce the same
/// bytes — the reproducibility that makes twin-run determinism testable.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct SubstreamKey {
    system: u32,
    tick: u64,
    scope: u64,
}

impl SubstreamKey {
    /// Construct a substream key from its `(system, tick, scope)` components.
    pub const fn new(system: u32, tick: u64, scope: u64) -> Self {
        Self {
            system,
            tick,
            scope,
        }
    }

    /// The identifier of the system drawing from this substream.
    pub const fn system(&self) -> u32 {
        self.system
    }

    /// The tick this substream belongs to.
    pub const fn tick(&self) -> u64 {
        self.tick
    }

    /// The scope key isolating this draw from others in the same system and tick.
    pub const fn scope(&self) -> u64 {
        self.scope
    }
}

/// A deterministic random substream (SplitMix64), seeded from a [`SubstreamKey`] and the
/// world seed (Vol. V Ch. 4 §4.1). Reproducible; never OS entropy.
#[derive(Clone, Debug)]
pub struct Rng {
    state: u64,
}

impl Rng {
    /// Issue the substream for `key` under world `seed`. Kernel-internal: systems receive
    /// an [`Rng`] from their tick context, never construct one (Vol. V Ch. 3 §3.3).
    pub(crate) fn for_substream(seed: u64, key: SubstreamKey) -> Self {
        let mut state = seed;
        state = mix(state ^ u64::from(key.system()));
        state = mix(state ^ key.tick());
        state = mix(state ^ key.scope());
        Self { state }
    }

    /// The next 64-bit value in the substream.
    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        mix(self.state)
    }

    /// A value in `0..bound`, uniformly. Panics if `bound` is zero.
    ///
    /// Uses Lemire's multiply-shift with rejection: a plain `% bound` would bias toward the
    /// low end of the range whenever `bound` does not divide `2^64`, and an engine whose
    /// whole premise is bit-exact fairness cannot ship a skewed die (Vol. V Ch. 4 §4.1). The
    /// rejection zone is entered only for the fraction of draws that would bias the result,
    /// so the expected number of `next_u64` calls is barely above one, and the sequence
    /// stays fully deterministic.
    pub fn below(&mut self, bound: u64) -> u64 {
        assert!(bound > 0, "bound must be positive");
        let mut product = u128::from(self.next_u64()) * u128::from(bound);
        let mut low = product as u64;
        if low < bound {
            // Reject the low zone that would otherwise be over-represented.
            let threshold = bound.wrapping_neg() % bound;
            while low < threshold {
                product = u128::from(self.next_u64()) * u128::from(bound);
                low = product as u64;
            }
        }
        (product >> 64) as u64
    }
}

/// SplitMix64 finaliser — a fixed, strong bit-mixing pass.
fn mix(mut z: u64) -> u64 {
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}
