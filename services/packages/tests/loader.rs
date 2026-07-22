//! Loading a world from a data file and running it (Vol. IV Ch. 1).
//!
//! The world file lives at the repository root under `worlds/`; the engine reads it through
//! the loader, never importing world content as code (Vol. IV Ch. 1, invariant 6).

use kernel::events::ChronicleEntry;
use kernel::fact::FactKey;
use kernel::identity::EntityId;
use kernel::store::RealityStore;
use kernel::system::CommittedView;
use packages::version::Version;
use packages::{engine_version, load, parse_world, LoadError};
use physical::schema::TEMPERATURE;

const WILDERNESS: &str = include_str!("../../../worlds/wilderness.world");

#[test]
fn parses_the_wilderness_world_file() {
    let pkg = parse_world(WILDERNESS).expect("world file parses");
    assert_eq!(pkg.manifest.id, "world.wilderness");
    assert_eq!(pkg.manifest.domains, vec!["physical".to_string()]);
    assert_eq!(pkg.physical_rules.ticks_per_day, 24);
    assert_eq!(pkg.physical_rules.weather_max_swing_centi_c, 50);
    assert_eq!(pkg.regions.len(), 3);
    assert_eq!(pkg.regions[0].temperature_centi_c, 1500);
}

#[test]
fn loads_and_seeds_committed_state() {
    let pkg = parse_world(WILDERNESS).unwrap();
    let world = load(&pkg, engine_version()).expect("loads");
    let t2 = world
        .store()
        .read(FactKey::new(EntityId::from_raw(2), TEMPERATURE))
        .expect("region 2 seeded")
        .value
        .as_int()
        .unwrap();
    assert_eq!(t2, 1800);
}

#[test]
fn loaded_world_ticks_deterministically() {
    fn run(seed: u64) -> Vec<[u8; 32]> {
        let pkg = parse_world(WILDERNESS).unwrap();
        let mut world = load(&pkg, engine_version()).unwrap();
        let mut chronicle: Vec<ChronicleEntry> = Vec::new();
        let mut seq = vec![*world.store().state_hash().as_bytes()];
        for t in 1..=120 {
            world.tick(t, seed, &mut chronicle).expect("tick commits");
            seq.push(*world.store().state_hash().as_bytes());
        }
        seq
    }
    // A world loaded from a file runs through the real deterministic tick loop.
    assert_eq!(run(42), run(42));
    assert_ne!(run(42), run(7));
}

#[test]
fn rejects_engine_out_of_range() {
    let pkg = parse_world(WILDERNESS).unwrap();
    // The package requires <1.0; a 2.x engine must be refused (invariant 10).
    let err = load(&pkg, Version::new(2, 0, 0)).unwrap_err();
    assert!(matches!(err, LoadError::EngineMismatch { .. }));
}

#[test]
fn rejects_world_without_physical() {
    let text = "[manifest]\nid = world.void\nversion = 0.1.0\nengine = >=0.0, <1.0\ndomains =\n\n[rules.physical]\nticks_per_day = 24\ndiurnal_amplitude_centi_c = 500\nweather_max_swing_centi_c = 50\nillumination_peak = 10000\nhumidity_baseline = 6000\nhumidity_swing = 100\nhumidity_drying_divisor = 8\npressure_sea_level = 10130\npressure_elevation_factor = 1\npressure_weather_swing = 20\npressure_settle_divisor = 8\nwind_gradient_divisor = 10\nfall_danger_per_meter = 1500\nthermal_mass_reference = 1500\n";
    let pkg = parse_world(text).unwrap();
    let err = load(&pkg, engine_version()).unwrap_err();
    assert!(matches!(err, LoadError::PhysicalNotSelected));
}

#[test]
fn missing_required_rule_is_rejected_with_no_default() {
    // weather_max_swing_centi_c omitted -> rejected; the engine never invents a default
    // (Vol. IV Ch. 2, "The Godlike Default" anti-pattern).
    let text = "[manifest]\nid = w\nversion = 0.1.0\nengine = >=0.0, <1.0\ndomains = physical\n\n[rules.physical]\nticks_per_day = 24\ndiurnal_amplitude_centi_c = 500\nillumination_peak = 10000\nhumidity_baseline = 6000\nhumidity_swing = 100\nhumidity_drying_divisor = 8\npressure_sea_level = 10130\npressure_elevation_factor = 1\npressure_weather_swing = 20\npressure_settle_divisor = 8\nwind_gradient_divisor = 10\nfall_danger_per_meter = 1500\nthermal_mass_reference = 1500\n";
    assert!(parse_world(text).is_err());
}
