//! Exposure to the open sky attenuates surface weather (Vol. III Ch. 1 §1.6, Enclosed /
//! Exposed; §1.11, sheltered). Sealed spaces stay dark and stable; partial spaces are
//! dampened; open ground (or a region with no exposure fact) is unaffected.
//!
//! Edge cases exercised: a fully sealed chamber (deep cave / sealed vault), a partially
//! sheltered region (cave mouth / forest floor / room with a window), and open ground with
//! exposure left unspecified. Large vs small caves are the same mechanism at different
//! scales -- more regions, each with its own exposure -- so they need no special case.

use kernel::domain::Domain;
use kernel::events::ChronicleEntry;
use kernel::fact::{Cause, Fact, FactKey, Provenance, SystemId};
use kernel::identity::EntityId;
use kernel::store::MemoryStore;
use kernel::system::CommittedView;
use kernel::tick::run_tick;
use kernel::value::Value;
use physical::schema::{EXPOSURE, ILLUMINATION, TEMPERATURE};
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
    }
}

fn prov() -> Provenance {
    Provenance::new(SystemId::new("worldgen"), 0, Cause::new("seed"))
}

fn run(store: &mut MemoryStore, regions: &[EntityId], ticks: u64) {
    let domain = PhysicalDomain::new(regions.to_vec(), config());
    let domains: [&dyn Domain; 1] = [&domain];
    let systems = domain.systems();
    let mut chronicle: Vec<ChronicleEntry> = Vec::new();
    for t in 1..=ticks {
        run_tick(store, &domains, &systems, t, 42, &mut chronicle).expect("tick commits");
    }
}

fn illumination(store: &MemoryStore, region: EntityId) -> i64 {
    store
        .read(FactKey::new(region, ILLUMINATION))
        .and_then(|f| f.value.as_int())
        .unwrap_or(0)
}

fn temperature(store: &MemoryStore, region: EntityId) -> i64 {
    store
        .read(FactKey::new(region, TEMPERATURE))
        .and_then(|f| f.value.as_int())
        .unwrap_or(0)
}

#[test]
fn illumination_scales_with_exposure() {
    let open = EntityId::from_raw(1); // no exposure fact -> fully exposed
    let mouth = EntityId::from_raw(2); // 40% exposed (cave mouth / canopy)
    let deep = EntityId::from_raw(3); // sealed

    let mut store = MemoryStore::new();
    store.seed(
        FactKey::new(mouth, EXPOSURE),
        Fact::new(Value::Int(4000), prov()),
    );
    store.seed(
        FactKey::new(deep, EXPOSURE),
        Fact::new(Value::Int(0), prov()),
    );

    // Run to midday (tick 12 of a 24-tick day): the sun is at its peak.
    run(&mut store, &[open, mouth, deep], 12);

    assert_eq!(
        illumination(&store, open),
        10000,
        "open ground gets full sun"
    );
    assert_eq!(illumination(&store, mouth), 4000, "a cave mouth gets 40%");
    assert_eq!(illumination(&store, deep), 0, "a sealed chamber stays dark");
}

#[test]
fn a_sealed_chamber_stays_dark_and_thermally_stable() {
    let deep = EntityId::from_raw(1);
    let mut store = MemoryStore::new();
    store.seed(
        FactKey::new(deep, EXPOSURE),
        Fact::new(Value::Int(0), prov()),
    );
    // Seed the chamber at its local mean temperature.
    store.seed(
        FactKey::new(deep, TEMPERATURE),
        Fact::new(Value::Int(1000), prov()),
    );

    run(&mut store, &[deep], 48); // two full days

    // No day/night swing, no weather jitter, no light: the sealed chamber never moves.
    assert_eq!(temperature(&store, deep), 1000);
    assert_eq!(illumination(&store, deep), 0);
}

#[test]
fn an_exposed_region_moves_while_a_sealed_one_holds() {
    let open = EntityId::from_raw(1);
    let sealed = EntityId::from_raw(2);
    let mut store = MemoryStore::new();
    store.seed(
        FactKey::new(sealed, EXPOSURE),
        Fact::new(Value::Int(0), prov()),
    );
    // Both start at the same temperature.
    store.seed(
        FactKey::new(open, TEMPERATURE),
        Fact::new(Value::Int(1000), prov()),
    );
    store.seed(
        FactKey::new(sealed, TEMPERATURE),
        Fact::new(Value::Int(1000), prov()),
    );

    run(&mut store, &[open, sealed], 12);

    assert_ne!(
        temperature(&store, open),
        1000,
        "open ground warms toward midday"
    );
    assert_eq!(
        temperature(&store, sealed),
        1000,
        "the sealed region holds steady"
    );
}

#[test]
fn absent_exposure_is_fully_exposed() {
    // A region with no exposure fact behaves exactly as before the feature existed.
    let region = EntityId::from_raw(1);
    let mut store = MemoryStore::new();
    run(&mut store, &[region], 12);
    assert_eq!(illumination(&store, region), 10000);
}
