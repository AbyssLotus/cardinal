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
    assert_eq!(pkg.clock.tick_ms, 3_600_000);
    assert_eq!(pkg.clock.day_seconds, 86_400);
    assert_eq!(pkg.physical_rules.temperature_variability_centi_c, 300);
    assert_eq!(pkg.physical_rules.weather_persistence_seconds, 21_600);
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
    // The wilderness, with its domain selection emptied: every rule is present, but the
    // mandatory Physical Reality domain is not selected.
    let text = WILDERNESS.replace("domains = physical", "domains =");
    let pkg = parse_world(&text).unwrap();
    let err = load(&pkg, engine_version()).unwrap_err();
    assert!(matches!(err, LoadError::PhysicalNotSelected));
}

#[test]
fn missing_required_rule_is_rejected_with_no_default() {
    // temperature_variability_centi_c omitted -> rejected, naming the rule; the engine never
    // invents a default (Vol. IV Ch. 2, "The Godlike Default" anti-pattern).
    let text: String = WILDERNESS
        .lines()
        .filter(|l| !l.starts_with("temperature_variability_centi_c"))
        .map(|l| format!("{l}\n"))
        .collect();
    let err = parse_world(&text).expect_err("a missing rule is an error");
    assert!(
        err.reason.contains("temperature_variability_centi_c"),
        "the error names the missing rule, got: {}",
        err.reason
    );
}
