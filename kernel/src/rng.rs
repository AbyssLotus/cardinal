//! The seeded stream forest — Vol. V Ch. 4 §4.1.
//!
//! All randomness is deterministic and reproducible. Substreams are issued per
//! `(system, tick, scope-key)`, counter-based and content-keyed, so the same world seed
//! replays exactly (Vol. V Ch. 4 §4.1). No system ever touches OS entropy
//! (Vol. V Ch. 4 §4.1, Door 3).

/// The key identifying one deterministic random substream: `(system, tick, scope)`
/// (Vol. V Ch. 4 §4.1).
///
/// Two draws with the same [`SubstreamKey`] under the same world seed produce the same
/// bytes — that reproducibility is what makes twin-run determinism testable.
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
