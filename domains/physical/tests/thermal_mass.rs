//! Thermal mass damps the temperature swing (Vol. III Ch. 1 §1.9 materials meeting §1.10
//! environmental state): a region built of a high-heat-capacity material resists the day/night
//! swing that a bare region follows in full. Materials stop being inert data and start driving
//! the live environment — a stone hall stays even while the open field bakes and freezes.

use kernel::domain::Domain;
use kernel::events::ChronicleEntry;
use kernel::fact::{Cause, Fact, FactKey, Provenance, SystemId};
use kernel::identity::EntityId;
use kernel::store::MemoryStore;
use kernel::system::CommittedView;
use kernel::value::Value;
use physical::schema::{MADE_OF, MATERIAL_THERMAL_CAPACITY, TEMPERATURE};
use physical::{PhysicalConfig, PhysicalDomain};

const BARE: EntityId = EntityId::from_raw(1); // open field, no material
const HEAVY: EntityId = EntityId::from_raw(2); // built of a high-capacity material
const STONE: EntityId = EntityId::from_raw(700); // the material entity
const SEED_TEMP: i64 = 2000;

fn config() -> PhysicalConfig {
    PhysicalConfig {
        ticks_per_day: 24,
        diurnal_amplitude_centi_c: 1000,
        weather_max_swing_centi_c: 0, // no weather noise: the diurnal swing alone, deterministic
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
        // The swing halves at capacity == reference; stone's 3000 is 3x this, so the heavy
        // region keeps only reference/(reference+capacity) = 1000/4000 = 1/4 of the swing.
        thermal_mass_reference: 1000,
    }
}

fn prov() -> Provenance {
    Provenance::new(SystemId::new("worldgen"), 0, Cause::new("seed"))
}

fn deviation_at_midday() -> (i64, i64) {
    let mut store = MemoryStore::new();
    // Both regions start at the same temperature.
    store.seed(
        FactKey::new(BARE, TEMPERATURE),
        Fact::new(Value::Int(SEED_TEMP), prov()),
    );
    store.seed(
        FactKey::new(HEAVY, TEMPERATURE),
        Fact::new(Value::Int(SEED_TEMP), prov()),
    );
    // A stone material with a high heat capacity, and the heavy region built of it.
    store.seed(
        FactKey::new(STONE, MATERIAL_THERMAL_CAPACITY),
        Fact::new(Value::Int(3000), prov()),
    );
    store.seed(
        FactKey::new(HEAVY, MADE_OF),
        Fact::new(Value::Entity(STONE), prov()),
    );

    let domain = PhysicalDomain::new(config());
    let domains: [&dyn Domain; 1] = [&domain];
    let systems = domain.systems();
    let mut chronicle: Vec<ChronicleEntry> = Vec::new();

    // Run to midday (tick 12 of a 24-tick day), where the diurnal swing peaks.
    for t in 1..=12 {
        run_tick_ok(&mut store, &domains, &systems, t, &mut chronicle);
    }

    let temp = |e: EntityId| {
        store
            .read(FactKey::new(e, TEMPERATURE))
            .unwrap()
            .value
            .as_int()
            .unwrap()
    };
    (temp(BARE) - SEED_TEMP, temp(HEAVY) - SEED_TEMP)
}

fn run_tick_ok(
    store: &mut MemoryStore,
    domains: &[&dyn Domain],
    systems: &[Box<dyn kernel::system::System>],
    tick: u64,
    chronicle: &mut Vec<ChronicleEntry>,
) {
    kernel::tick::run_tick(store, domains, systems, tick, 7, chronicle).expect("tick commits");
}

#[test]
fn a_heavy_region_swings_less_than_a_bare_one() {
    let (bare_dev, heavy_dev) = deviation_at_midday();

    // The bare field follows the full diurnal swing: seed + amplitude at midday.
    assert_eq!(bare_dev, 1000, "bare region should take the full swing");

    // The stone-built region resists it: about a quarter of the swing (1/4 of 1000 ≈ 250),
    // allowing for per-tick integer rounding.
    assert!(heavy_dev > 0, "the heavy region still warms somewhat");
    assert!(
        heavy_dev < bare_dev,
        "thermal mass must damp the swing: heavy {heavy_dev} vs bare {bare_dev}"
    );
    assert!(
        (200..=300).contains(&heavy_dev),
        "heavy region should keep ~1/4 of the swing, got {heavy_dev}"
    );
}
