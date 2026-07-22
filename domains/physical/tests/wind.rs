//! Wind as a consequence of pressure gradients across the topology (Vol. III Ch. 1 §1.10).
//!
//! Two adjacent regions at very different elevations develop very different pressures; the
//! wind blows from the high-pressure (low, warm) region toward the low-pressure (high,
//! alpine) one. A multi-fact, topology-aware system read end to end.

use kernel::domain::Domain;
use kernel::events::ChronicleEntry;
use kernel::fact::{Cause, Fact, FactKey, Provenance, SystemId};
use kernel::identity::EntityId;
use kernel::store::MemoryStore;
use kernel::system::CommittedView;
use kernel::tick::run_tick;
use kernel::value::Value;
use physical::schema::{ADJACENT_TO, ELEVATION, TEMPERATURE, WIND_SPEED, WIND_TOWARD};
use physical::{PhysicalConfig, PhysicalDomain};

const ALPINE: EntityId = EntityId::from_raw(1); // high elevation -> low pressure
const LOWLAND: EntityId = EntityId::from_raw(2); // low elevation -> high pressure

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
        thermal_mass_reference: 1000,
    }
}

fn seeded_world() -> MemoryStore {
    let mut store = MemoryStore::new();
    let prov = Provenance::new(SystemId::new("worldgen"), 0, Cause::new("seed"));
    // Temperature marks each region so the domain's systems discover it (the loader seeds
    // this for every region; the value is irrelevant to wind, which follows pressure).
    store.seed(
        FactKey::new(ALPINE, TEMPERATURE),
        Fact::new(Value::Int(500), prov),
    );
    store.seed(
        FactKey::new(LOWLAND, TEMPERATURE),
        Fact::new(Value::Int(1500), prov),
    );
    // Elevations (centimetres): alpine at 2400 m, lowland at 100 m.
    store.seed(
        FactKey::new(ALPINE, ELEVATION),
        Fact::new(Value::Int(240000), prov),
    );
    store.seed(
        FactKey::new(LOWLAND, ELEVATION),
        Fact::new(Value::Int(10000), prov),
    );
    // Undirected border between them.
    store.seed(
        FactKey::new(ALPINE, ADJACENT_TO),
        Fact::new(Value::Entity(LOWLAND), prov),
    );
    store.seed(
        FactKey::new(LOWLAND, ADJACENT_TO),
        Fact::new(Value::Entity(ALPINE), prov),
    );
    store
}

fn run(store: &mut MemoryStore, ticks: u64, seed: u64) {
    let domain = PhysicalDomain::new(config());
    let domains: [&dyn Domain; 1] = [&domain];
    let systems = domain.systems();
    let mut chronicle: Vec<ChronicleEntry> = Vec::new();
    for t in 1..=ticks {
        run_tick(store, &domains, &systems, t, seed, &mut chronicle).expect("tick commits");
    }
}

fn wind_speed(store: &MemoryStore, region: EntityId) -> i64 {
    store
        .read(FactKey::new(region, WIND_SPEED))
        .and_then(|f| f.value.as_int())
        .unwrap_or(0)
}

fn wind_toward(store: &MemoryStore, region: EntityId) -> Option<u64> {
    match store
        .read(FactKey::new(region, WIND_TOWARD))
        .map(|f| f.value)
    {
        Some(Value::Entity(e)) => Some(e.raw()),
        _ => None,
    }
}

#[test]
fn wind_blows_from_high_pressure_to_low() {
    let mut store = seeded_world();
    run(&mut store, 60, 42);

    // The lowland (high pressure) blows toward the alpine region (low pressure).
    assert_eq!(wind_toward(&store, LOWLAND), Some(ALPINE.raw()));
    assert!(wind_speed(&store, LOWLAND) > 0, "lowland should have wind");

    // The alpine region has no lower-pressure neighbour, so it is calm.
    assert_eq!(wind_speed(&store, ALPINE), 0);
    assert_eq!(wind_toward(&store, ALPINE), None);
}

#[test]
fn wind_is_deterministic() {
    fn final_speed(seed: u64) -> i64 {
        let mut store = seeded_world();
        run(&mut store, 60, seed);
        wind_speed(&store, LOWLAND)
    }
    assert_eq!(final_speed(42), final_speed(42));
}
