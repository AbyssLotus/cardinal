//! Multi-region environment: many regions each carry their own temperature, illumination,
//! and humidity, evolving independently under their own weather substreams, and the whole
//! world stays deterministic (Vol. V Ch. 3-4).

use kernel::domain::Domain;
use kernel::events::ChronicleEntry;
use kernel::fact::{Cause, Fact, FactKey, Provenance, SystemId};
use kernel::identity::EntityId;
use kernel::store::{MemoryStore, RealityStore};
use kernel::system::CommittedView;
use kernel::tick::run_tick;
use kernel::value::Value;
use physical::schema::{ABSOLUTE_ZERO_CENTI_C, TEMPERATURE};
use physical::{PhysicalConfig, PhysicalDomain};

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

fn regions(n: u64) -> Vec<EntityId> {
    (1..=n).map(EntityId::from_raw).collect()
}

fn seeded_world(rs: &[EntityId]) -> MemoryStore {
    let mut store = MemoryStore::new();
    for (i, r) in rs.iter().enumerate() {
        let temp = 1500 + (i as i64) * 200;
        store.seed(
            FactKey::new(*r, TEMPERATURE),
            Fact::new(
                Value::Int(temp),
                Provenance::new(SystemId::new("worldgen"), 0, Cause::new("seed")),
            ),
        );
    }
    store
}

fn run(seed: u64, ticks: u64, rs: &[EntityId]) -> Vec<[u8; 32]> {
    let mut store = seeded_world(rs);
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
fn regions_evolve_independently_and_validly() {
    let rs = regions(4);
    let mut store = seeded_world(&rs);
    let domain = PhysicalDomain::new(config());
    let domains: [&dyn Domain; 1] = [&domain];
    let systems = domain.systems();
    let mut chronicle = Vec::new();

    for t in 1..=100 {
        run_tick(&mut store, &domains, &systems, t, 42, &mut chronicle).expect("tick commits");
    }

    let temps: Vec<i64> = rs
        .iter()
        .map(|r| {
            store
                .read(FactKey::new(*r, TEMPERATURE))
                .unwrap()
                .value
                .as_int()
                .unwrap()
        })
        .collect();

    for t in &temps {
        assert!(*t >= ABSOLUTE_ZERO_CENTI_C);
    }
    assert!(temps.iter().any(|t| *t != temps[0]));
    // Several environmental facts are committed per region per tick.
    assert!(chronicle.len() > rs.len() * 100);
}

#[test]
fn multi_region_world_is_deterministic() {
    let rs = regions(4);
    assert_eq!(run(42, 120, &rs), run(42, 120, &rs));
    assert_ne!(run(42, 120, &rs), run(43, 120, &rs));
}
