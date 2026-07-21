//! The first cross-domain interaction, end to end (Vol. III Ch. 12).
//!
//! A world with both `physical` and `living` selected: physical produces each region's
//! temperature; living's organisms read that committed temperature and defend their body
//! heat against it. The two domains never reference each other — they meet only in the
//! fact store.

use kernel::events::ChronicleEntry;
use kernel::fact::FactKey;
use kernel::identity::EntityId;
use kernel::store::RealityStore;
use kernel::system::CommittedView;
use kernel::value::Value;
use living::schema::BODY_HEAT;
use packages::{engine_version, load, parse_world, LoadedWorld};
use physical::schema::{ADJACENT_TO, CONTAINED_IN, TEMPERATURE, WIND_SPEED, WIND_TOWARD};

const MENAGERIE: &str = include_str!("../../../worlds/menagerie.world");

fn body_heat(world: &LoadedWorld, organism: u64) -> i64 {
    world
        .store()
        .read(FactKey::new(EntityId::from_raw(organism), BODY_HEAT))
        .expect("organism has body heat")
        .value
        .as_int()
        .unwrap()
}

fn temperature(world: &LoadedWorld, region: u64) -> i64 {
    world
        .store()
        .read(FactKey::new(EntityId::from_raw(region), TEMPERATURE))
        .expect("region has temperature")
        .value
        .as_int()
        .unwrap()
}

fn run(world: &mut LoadedWorld, ticks: u64, seed: u64) {
    let mut chronicle: Vec<ChronicleEntry> = Vec::new();
    for t in 1..=ticks {
        world.tick(t, seed, &mut chronicle).expect("tick commits");
    }
}

#[test]
fn body_heat_tracks_the_region_temperature() {
    let pkg = parse_world(MENAGERIE).unwrap();
    let mut world = load(&pkg, engine_version()).unwrap();
    run(&mut world, 300, 42);

    let frigid = body_heat(&world, 10); // organism in region 1 (-8 C)
    let warm = body_heat(&world, 20); //   organism in region 2 (26 C)

    // The organism in the frigid region ends colder than the one in the warm region, and
    // both are held below the 37.00 C set point by their environment -- a Living fact driven
    // entirely by reading a Physical fact.
    assert!(frigid < warm, "frigid={frigid} should be < warm={warm}");
    assert!(warm < 3700);
}

#[test]
fn cross_domain_world_is_deterministic() {
    fn hashes(seed: u64) -> Vec<[u8; 32]> {
        let pkg = parse_world(MENAGERIE).unwrap();
        let mut world = load(&pkg, engine_version()).unwrap();
        let mut chronicle: Vec<ChronicleEntry> = Vec::new();
        let mut seq = vec![*world.store().state_hash().as_bytes()];
        for t in 1..=150 {
            world.tick(t, seed, &mut chronicle).unwrap();
            seq.push(*world.store().state_hash().as_bytes());
        }
        seq
    }
    assert_eq!(hashes(42), hashes(42));
    assert_ne!(hashes(42), hashes(7));
}

#[test]
fn living_does_not_perturb_physical() {
    // Non-interference (Vol. III Ch. 12, invariant 7): the temperature trajectory must be
    // identical whether or not the living domain is running, because living only reads
    // temperature and writes its own body-heat facts.
    fn temp_trajectory(pkg: &packages::WorldPackage, seed: u64) -> Vec<(i64, i64)> {
        let mut world = load(pkg, engine_version()).unwrap();
        let mut chronicle: Vec<ChronicleEntry> = Vec::new();
        let mut traj = Vec::new();
        for t in 1..=120 {
            world.tick(t, seed, &mut chronicle).unwrap();
            traj.push((temperature(&world, 1), temperature(&world, 2)));
        }
        traj
    }

    let combined = parse_world(MENAGERIE).unwrap();
    let mut physical_only = combined.clone();
    physical_only.manifest.domains = vec!["physical".to_string()];
    physical_only.living_rules = None;
    physical_only.organisms = Vec::new();

    assert_eq!(
        temp_trajectory(&combined, 42),
        temp_trajectory(&physical_only, 42)
    );
}

#[test]
fn containment_hierarchy_is_seeded() {
    // The loader seeds the physical containment links the world file declares:
    // organism 10 -> region 1 -> continent 1000. Living reads the first hop to find its
    // region; the rest of the hierarchy is there for any consumer to walk.
    let pkg = parse_world(MENAGERIE).unwrap();
    let world = load(&pkg, engine_version()).unwrap();

    let org_in = world
        .store()
        .read(FactKey::new(EntityId::from_raw(10), CONTAINED_IN))
        .expect("organism containment seeded")
        .value;
    assert_eq!(org_in, Value::Entity(EntityId::from_raw(1)));

    let region_in = world
        .store()
        .read(FactKey::new(EntityId::from_raw(1), CONTAINED_IN))
        .expect("region containment seeded")
        .value;
    assert_eq!(region_in, Value::Entity(EntityId::from_raw(1000)));
}

#[test]
fn regions_are_adjacent_in_the_loaded_world() {
    // The [adjacency] section seeds a symmetric cardinality-many topology fact.
    let pkg = parse_world(MENAGERIE).unwrap();
    let world = load(&pkg, engine_version()).unwrap();

    fn neighbours(world: &LoadedWorld, region: u64) -> Vec<u64> {
        world
            .store()
            .read_all(FactKey::new(EntityId::from_raw(region), ADJACENT_TO))
            .into_iter()
            .filter_map(|f| match f.value {
                Value::Entity(e) => Some(e.raw()),
                _ => None,
            })
            .collect()
    }

    assert!(neighbours(&world, 1).contains(&2));
    assert!(neighbours(&world, 2).contains(&1)); // symmetric
}

#[test]
fn wind_develops_between_adjacent_regions_in_the_loaded_world() {
    // Region 1 (alpine, high) and region 2 (lowland, low) border each other; pressure falls
    // with elevation, so wind blows from the lowland toward the alpine region.
    let pkg = parse_world(MENAGERIE).unwrap();
    let mut world = load(&pkg, engine_version()).unwrap();
    let mut chronicle: Vec<ChronicleEntry> = Vec::new();
    for t in 1..=60 {
        world.tick(t, 42, &mut chronicle).expect("tick commits");
    }

    let toward = match world
        .store()
        .read(FactKey::new(EntityId::from_raw(2), WIND_TOWARD))
        .map(|f| f.value)
    {
        Some(Value::Entity(e)) => Some(e.raw()),
        _ => None,
    };
    let speed = world
        .store()
        .read(FactKey::new(EntityId::from_raw(2), WIND_SPEED))
        .and_then(|f| f.value.as_int())
        .unwrap_or(0);

    assert_eq!(
        toward,
        Some(1),
        "lowland wind should blow toward the alpine region"
    );
    assert!(speed > 0, "lowland should have a nonzero wind speed");
}
