//! End-to-end vertical slice: a region's temperature living through the real tick loop
//! (Vol. V Ch. 3 §3.1), reconciled by the physical domain's composition rule and guarded
//! by its validation — run deterministically through the kernel's state-hash harness
//! (Vol. V Ch. 4 §4.2).

use kernel::domain::Domain;
use kernel::events::ChronicleEntry;
use kernel::fact::{Cause, Fact, FactKey, FactType, Provenance, SystemId};
use kernel::identity::EntityId;
use kernel::proposal::{Change, Proposal};
use kernel::store::{MemoryStore, RealityStore};
use kernel::system::{Cadence, CommittedView, System, TickContext};
use kernel::tick::{run_tick, TickError};
use kernel::value::Value;
use physical::schema::{ABSOLUTE_ZERO_CENTI_C, TEMPERATURE};
use physical::PhysicalDomain;

const REGION: EntityId = EntityId::from_raw(1);
const SEED_TEMP_CENTI_C: i64 = 2000; // 20.00 C

/// A freshly seeded single-region world at 20.00 C.
fn seeded_world() -> MemoryStore {
    let mut store = MemoryStore::new();
    store.seed(
        FactKey::new(REGION, TEMPERATURE),
        Fact::new(
            Value::Int(SEED_TEMP_CENTI_C),
            Provenance::new(SystemId::new("worldgen"), 0, Cause::new("seed")),
        ),
    );
    store
}

/// Run the physical slice for `ticks` ticks and return the per-tick state-hash sequence.
fn run(seed: u64, ticks: u64) -> Vec<[u8; 32]> {
    let mut store = seeded_world();
    let domain = PhysicalDomain::new(REGION, 24, 500, 50); // ±5.00 C day, ±0.50 C weather
    let domains: [&dyn Domain; 1] = [&domain];
    let systems = domain.systems();
    let mut chronicle: Vec<ChronicleEntry> = Vec::new();

    let mut seq = vec![*store.state_hash().as_bytes()];
    for t in 1..=ticks {
        run_tick(&mut store, &domains, &systems, t, seed, &mut chronicle).expect("tick commits");
        seq.push(*store.state_hash().as_bytes());
    }
    seq
}

#[test]
fn tick_loop_commits_and_chronicles() {
    let mut store = seeded_world();
    let domain = PhysicalDomain::new(REGION, 24, 500, 50);
    let domains: [&dyn Domain; 1] = [&domain];
    let systems = domain.systems();
    let mut chronicle = Vec::new();

    run_tick(&mut store, &domains, &systems, 1, 42, &mut chronicle).expect("tick commits");

    // The fact still exists, stayed physically valid, and the change was chronicled.
    let temp = store
        .read(FactKey::new(REGION, TEMPERATURE))
        .expect("temperature committed")
        .value
        .as_int()
        .expect("temperature is integer");
    assert!(temp >= ABSOLUTE_ZERO_CENTI_C);
    assert_eq!(chronicle.len(), 1);
    assert_eq!(chronicle[0].fact_type(), TEMPERATURE);
    assert_eq!(chronicle[0].subject(), REGION);
}

#[test]
fn same_seed_is_bit_identical() {
    // The real tick loop, twin-run: reproducible to the bit (Vol. V Ch. 4 §4.2).
    assert_eq!(run(42, 240), run(42, 240));
}

#[test]
fn different_seed_diverges() {
    // Weather draws from the seeded substream, so a different seed yields a different world.
    let a = run(42, 240);
    let b = run(43, 240);
    assert_ne!(a, b);
    assert_eq!(a[0], b[0]); // identical seeded initial state
}

/// A test-only system that proposes an impossible plunge in temperature, to prove the
/// Validate stage rejects it and the tick aborts with reality intact (Vol. V Ch. 3 §3.5.5).
struct FreezeRay;

impl System for FreezeRay {
    fn id(&self) -> SystemId {
        SystemId::new("test.freeze_ray")
    }
    fn reads(&self) -> &'static [FactType] {
        &[TEMPERATURE]
    }
    fn writes(&self) -> &'static [FactType] {
        &[TEMPERATURE]
    }
    fn cadence(&self) -> Cadence {
        Cadence::EveryTick
    }
    fn evaluate(&self, _view: &dyn CommittedView, ctx: &TickContext) -> Vec<Proposal> {
        vec![Proposal::new(
            self.id(),
            FactKey::new(REGION, TEMPERATURE),
            ctx.tick(),
            Change::Delta(-1_000_000_000),
            Cause::new("test_freeze"),
        )]
    }
}

#[test]
fn validation_failure_aborts_the_tick() {
    let mut store = seeded_world();
    let domain = PhysicalDomain::new(REGION, 24, 500, 50);
    let domains: [&dyn Domain; 1] = [&domain];
    // Physical still owns/validates TEMPERATURE; the rogue system provides the bad proposal.
    let systems: Vec<Box<dyn System>> = vec![Box::new(FreezeRay)];
    let mut chronicle = Vec::new();

    let before = store.state_hash();
    let result = run_tick(&mut store, &domains, &systems, 1, 42, &mut chronicle);

    assert!(matches!(result, Err(TickError::Validate(_))));
    // Failure aborts the tick: reality is exactly as it was, nothing chronicled.
    assert_eq!(store.state_hash(), before);
    assert!(chronicle.is_empty());
    assert_eq!(
        store
            .read(FactKey::new(REGION, TEMPERATURE))
            .unwrap()
            .value
            .as_int()
            .unwrap(),
        SEED_TEMP_CENTI_C
    );
}
