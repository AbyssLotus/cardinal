//! Determinism harness — twin-run reproducibility check
//! (Vol. V Ch. 4 §4.2; Vol. V Ch. 10 §10.4).
//!
//! Proves the core determinism contract the way the roadmap's CI gate will: run a world
//! from a fixed seed, hash committed state after every tick, and require that a second run
//! from the same seed produces a bit-identical sequence of per-tick [`StateHash`] values.
//! The same harness must also *detect* nondeterminism — a run from a different seed must
//! diverge — so the check is a real discriminator, not a tautology.
//!
//! The world here is a deterministic stand-in: a small keyed store advanced by a seeded
//! substream RNG. It exists to exercise the harness now, before the real tick loop and
//! domains exist. When the engine lands, its systems plug into the identical twin-run
//! comparator (`run` + `first_divergence` below).

use kernel::hash::{StateHash, StateHasher};
use kernel::identity::EntityId;
use kernel::rng::SubstreamKey;
use std::collections::BTreeMap;

/// SplitMix64 — a fixed, self-contained deterministic generator standing in for the
/// kernel's future seeded substream RNG (Vol. V Ch. 4 §4.1). Same input, same output.
fn splitmix64(mut z: u64) -> u64 {
    z = z.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// A single deterministic draw for a world seed and substream key `(system, tick, scope)`
/// (Vol. V Ch. 4 §4.1): reproducible, and never touching OS entropy (Door 3).
fn draw(seed: u64, key: SubstreamKey) -> u64 {
    let mixed = seed
        ^ splitmix64(u64::from(key.system()))
        ^ splitmix64(key.tick())
        ^ splitmix64(key.scope());
    splitmix64(mixed)
}

/// A minimal deterministic world: entities (by raw id) mapped to a scalar value.
struct MockWorld {
    facts: BTreeMap<u64, u64>,
}

impl MockWorld {
    /// A fixed initial world — the same starting reality every run begins from.
    fn seeded_initial() -> Self {
        let mut facts = BTreeMap::new();
        for id in 0..8u64 {
            facts.insert(id, id.wrapping_mul(1000));
        }
        Self { facts }
    }

    /// The canonical, order-independent digest of committed state (Vol. V Ch. 4 §4.2).
    ///
    /// Each fact is encoded with fixed field order and fixed-width little-endian integers,
    /// so equal facts always produce equal bytes; the hasher combines them order-free.
    fn state_hash(&self) -> StateHash {
        let mut h = StateHasher::new();
        let mut buf = [0u8; 16];
        for (id, value) in &self.facts {
            buf[0..8].copy_from_slice(&id.to_le_bytes());
            buf[8..16].copy_from_slice(&value.to_le_bytes());
            h.add_fact(&buf);
        }
        h.finish()
    }

    /// Advance committed reality by one deterministic tick.
    ///
    /// A stand-in "system": every entity's value is mixed with a seeded draw keyed by
    /// `(system, tick, entity)`, and a fresh entity is admitted on a fixed cadence so the
    /// fact-set size changes over time. All mutation is a function of prior state and seed.
    fn tick(&mut self, seed: u64, tick_no: u64) {
        const SYSTEM: u32 = 1;
        let ids: Vec<u64> = self.facts.keys().copied().collect();
        for id in ids {
            let key = SubstreamKey::new(SYSTEM, tick_no, id);
            let d = draw(seed, key);
            let entry = self.facts.get_mut(&id).expect("id present");
            *entry = entry.wrapping_add(d);
        }
        // Admit a new entity every 5th tick (deterministic cadence).
        if tick_no % 5 == 0 {
            let new_id = EntityId::from_raw(1000 + tick_no);
            let key = SubstreamKey::new(SYSTEM, tick_no, new_id.raw());
            self.facts.insert(new_id.raw(), draw(seed, key));
        }
    }
}

/// Run a world from `seed` for `ticks` ticks, returning the per-tick state-hash sequence
/// (index 0 is the initial world, before any tick).
fn run(seed: u64, ticks: u64) -> Vec<StateHash> {
    let mut world = MockWorld::seeded_initial();
    let mut seq = Vec::with_capacity(ticks as usize + 1);
    seq.push(world.state_hash());
    for t in 1..=ticks {
        world.tick(seed, t);
        seq.push(world.state_hash());
    }
    seq
}

/// The first tick at which two hash sequences differ, or `None` if identical — the signal
/// a CI twin-run gate reports.
fn first_divergence(a: &[StateHash], b: &[StateHash]) -> Option<usize> {
    if a.len() != b.len() {
        return Some(a.len().min(b.len()));
    }
    a.iter().zip(b).position(|(x, y)| x != y)
}

#[test]
fn twin_run_is_bit_identical() {
    let a = run(42, 64);
    let b = run(42, 64);
    assert_eq!(a, b, "same seed must reproduce exactly");
    assert_eq!(first_divergence(&a, &b), None);
}

#[test]
fn harness_detects_nondeterminism() {
    // A different seed must diverge — proves the check is a real discriminator.
    let a = run(42, 64);
    let b = run(43, 64);
    assert_ne!(a, b);
    // Same fixed initial world, so tick 0 matches; divergence appears once the RNG bites.
    assert_eq!(a[0], b[0]);
    let d = first_divergence(&a, &b).expect("sequences must diverge");
    assert!(
        d >= 1,
        "divergence should appear after the initial state, at {d}"
    );
}

#[test]
fn initial_state_is_stable_and_nonempty() {
    let a = run(7, 0);
    let b = run(999, 0);
    // Tick 0 is the fixed initial world, independent of seed.
    assert_eq!(a[0], b[0]);
    assert_ne!(a[0], StateHash::EMPTY);
}
