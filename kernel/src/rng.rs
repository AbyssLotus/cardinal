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

    /// A value in `0..bound`. Panics if `bound` is zero.
    pub fn below(&mut self, bound: u64) -> u64 {
        assert!(bound > 0, "bound must be positive");
        self.next_u64() % bound
    }
}

/// SplitMix64 finaliser — a fixed, strong bit-mixing pass.
fn mix(mut z: u64) -> u64 {
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}
