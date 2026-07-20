//! Exposure across a whole loaded world (Vol. III Ch. 1 §1.6): open ground, a partly
//! sheltered cave mouth, a sealed deep chamber, and an enclosed cabin -- each attenuating
//! surface weather by how open it is to the sky.

use kernel::events::ChronicleEntry;
use kernel::fact::FactKey;
use kernel::identity::EntityId;
use kernel::system::CommittedView;
use packages::{engine_version, load, parse_world, LoadedWorld};
use physical::schema::{ILLUMINATION, TEMPERATURE};

const CAVERN: &str = include_str!("../../../worlds/cavern.world");

fn illumination(world: &LoadedWorld, region: u64) -> i64 {
    world
        .store()
        .read(FactKey::new(EntityId::from_raw(region), ILLUMINATION))
        .and_then(|f| f.value.as_int())
        .unwrap_or(0)
}

fn temperature(world: &LoadedWorld, region: u64) -> i64 {
    world
        .store()
        .read(FactKey::new(EntityId::from_raw(region), TEMPERATURE))
        .and_then(|f| f.value.as_int())
        .unwrap_or(0)
}

#[test]
fn exposure_gradient_shapes_light_and_temperature() {
    let pkg = parse_world(CAVERN).unwrap();
    let mut world = load(&pkg, engine_version()).unwrap();
    let mut chronicle: Vec<ChronicleEntry> = Vec::new();
    // Run to midday (tick 12 of the 24-tick day): the sun is at its peak.
    for t in 1..=12 {
        world.tick(t, 42, &mut chronicle).expect("tick commits");
    }

    // Illumination follows exposure: open meadow full, cave mouth 40%, sealed chamber dark,
    // cabin dim.
    assert_eq!(
        illumination(&world, 1),
        10000,
        "open meadow gets full midday sun"
    );
    assert_eq!(illumination(&world, 2), 4000, "cave mouth gets 40%");
    assert_eq!(illumination(&world, 3), 0, "sealed chamber is dark");
    assert_eq!(
        illumination(&world, 4),
        800,
        "cabin gets a little window light"
    );

    // The sealed chamber's temperature never moved from its seeded local mean.
    assert_eq!(
        temperature(&world, 3),
        1000,
        "sealed chamber is thermally stable"
    );
    // The open meadow warmed toward midday.
    assert_ne!(
        temperature(&world, 1),
        1500,
        "open meadow's temperature moved"
    );
}
