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
use living::schema::BODY_HEAT;
use packages::{engine_version, load, parse_world, LoadedWorld};
use physical::schema::TEMPERATURE;

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
