//! End-to-end: environmental facts living through the real tick loop (Vol. V Ch. 3 §3.1),
//! reconciled by the physical domain's composition rules and guarded by its validation —
//! run deterministically through the kernel's state-hash harness (Vol. V Ch. 4 §4.2).

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
use physical::{PhysicalConfig, PhysicalDomain};

const REGION: EntityId = EntityId::from_raw(1);
const SEED_TEMP_CENTI_C: i64 = 2000; // 20.00 C

fn config() -> PhysicalConfig {
    PhysicalConfig {
        ticks_per_day: 24,
        diurnal_amplitude_centi_c: 500,
        weather_max_swing_centi_c: 50,
        illumination_peak: 10000,
        humidity_baseline: 6000,
        humidity_swing: 100,
        humidity_drying_divisor: 8,
        pressure_sea_level: 10130,
        pressure_elevation_factor: 1,
        pressure_weather_swing: 20,
        pressure_settle_divisor: 8,
        wind_gradient_divisor: 10,
        fall_danger_per_meter: 1500,
    }
}

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

fn run(seed: u64, ticks: u64) -> Vec<[u8; 32]> {
    let mut store = seeded_world();
    let domain = PhysicalDomain::new(config());
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
fn tick_loop_commits_and_chronicles_the_environment() {
    let mut store = seeded_world();
    let domain = PhysicalDomain::new(config());
    let domains: [&dyn Domain; 1] = [&domain];
    let systems = domain.systems();
    let mut chronicle = Vec::new();

    run_tick(&mut store, &domains, &systems, 1, 42, &mut chronicle).expect("tick commits");

    let temp = store
        .read(FactKey::new(REGION, TEMPERATURE))
        .expect("temperature committed")
        .value
        .as_int()
        .expect("temperature is integer");
    assert!(temp >= ABSOLUTE_ZERO_CENTI_C);
    // On tick 1, five environmental proposals commit: temperature is moved by two systems
    // (diurnal_shift and weather_perturbation), plus illumination, humidity, and pressure.
    // The chronicle records one entry per committed proposal, so both temperature causes are
    // kept (Vol. V Ch. 6 §6.1). Wind reads committed pressure, which does not exist until
    // pressure is first written, so wind begins on tick 2 (effects chain across ticks).
    assert_eq!(chronicle.len(), 5);
    assert!(chronicle.iter().any(|e| e.fact_type() == TEMPERATURE));
}

#[test]
fn same_seed_is_bit_identical() {
    assert_eq!(run(42, 240), run(42, 240));
}

#[test]
fn different_seed_diverges() {
    let a = run(42, 240);
    let b = run(43, 240);
    assert_ne!(a, b);
    assert_eq!(a[0], b[0]); // identical seeded initial state
}

/// A test-only system proposing an impossible plunge in temperature, to prove the Validate
/// stage rejects it and the tick aborts with reality intact (Vol. V Ch. 3 §3.5.5).
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
            ctx.basis_tick(),
            Change::Delta(-1_000_000_000),
            Cause::new("test_freeze"),
        )]
    }
}

#[test]
fn validation_failure_aborts_the_tick() {
    let mut store = seeded_world();
    let domain = PhysicalDomain::new(config());
    let domains: [&dyn Domain; 1] = [&domain];
    let systems: Vec<Box<dyn System>> = vec![Box::new(FreezeRay)];
    let mut chronicle = Vec::new();

    let before = store.state_hash();
    let result = run_tick(&mut store, &domains, &systems, 1, 42, &mut chronicle);

    assert!(matches!(result, Err(TickError::Validate(_))));
    assert_eq!(store.state_hash(), before);
    assert!(chronicle.is_empty());
}
