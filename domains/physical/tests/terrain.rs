//! Ground (Vol. III Ch. 1 §1.10–1.11; Amendment A-4): a heightfield's surface between its
//! samples, how steep it is, a body that falls onto it, a ridge that hides one valley from the
//! next, and a walker who finds the pass when the cliff is too steep to climb.

use kernel::domain::Domain;
use kernel::fact::{Cause, Fact, FactKey, FactType, Provenance, SystemId};
use kernel::identity::EntityId;
use kernel::proposal::{Change, Proposal};
use kernel::store::MemoryStore;
use kernel::system::{Cadence, CommittedView, System, TickContext};
use kernel::tick::run_tick;
use kernel::time::SimClock;
use kernel::value::Value;
use physical::schema::{
    BODY_SIZE, CONTAINED_IN, FALL_HEIGHT, MOBILE, POSITION, TERRAIN_SAMPLE, TERRAIN_SPACING,
    TRAVEL_BLOCKED, TRAVEL_SPEED, TRAVEL_TO,
};
use physical::sight::line_of_sight;
use physical::space::local_position;
use physical::terrain::{ground, slope_percent, support};
use physical::{PhysicalConfig, PhysicalDomain};

const SITE: u64 = 100;
const LAND: u64 = 1;

fn e(id: u64) -> EntityId {
    EntityId::from_raw(id)
}

fn seed(s: &mut MemoryStore, id: u64, ft: FactType, v: Value) {
    s.seed(
        FactKey::new(e(id), ft),
        Fact::new(
            v,
            Provenance::new(SystemId::new("worldgen"), 0, Cause::new("seed")),
        ),
    );
}

/// Land in the site whose terrain is `rows` of heights, samples 10 m apart.
fn land(rows: &[&[i64]]) -> MemoryStore {
    let mut s = MemoryStore::new();
    seed(&mut s, LAND, CONTAINED_IN, Value::Entity(e(SITE)));
    seed(&mut s, LAND, POSITION, Value::Vec3([0, 0, 0]));
    seed(&mut s, LAND, TERRAIN_SPACING, Value::Int(1_000));
    for (r, row) in rows.iter().enumerate() {
        for (c, h) in row.iter().enumerate() {
            seed(
                &mut s,
                LAND,
                TERRAIN_SAMPLE,
                Value::Vec3([c as i64, r as i64, *h]),
            );
        }
    }
    s
}

fn person(s: &mut MemoryStore, id: u64, at: [i64; 3]) {
    seed(s, id, CONTAINED_IN, Value::Entity(e(LAND)));
    seed(s, id, POSITION, Value::Vec3(at));
    seed(s, id, BODY_SIZE, Value::Vec3([25, 15, 175]));
    seed(s, id, MOBILE, Value::Bool(true));
}

fn config() -> PhysicalConfig {
    PhysicalConfig {
        clock: SimClock::new(1_000),
        day_length_seconds: 86_400,
        environment_step_seconds: 3_600,
        diurnal_amplitude_centi_c: 400,
        temperature_variability_centi_c: 300,
        weather_persistence_seconds: 21_600,
        illumination_peak: 10000,
        humidity_baseline: 5500,
        humidity_variability: 800,
        pressure_sea_level: 10130,
        pressure_elevation_factor: 1,
        pressure_variability: 60,
        wind_gradient_divisor: 10,
        fall_danger_per_meter: 1500,
        thermal_mass_reference: 1000,
        gravity_cm_s2: 981,
        step_height_cm: 40,
        max_slope_percent: 100,
        nav_cell_cm: 100,
    }
}

/// Asks body `who` to go to `what` at walking pace, on tick 1.
struct Send {
    who: u64,
    what: u64,
}
impl System for Send {
    fn id(&self) -> SystemId {
        SystemId::new("test.send")
    }
    fn reads(&self) -> &'static [FactType] {
        &[]
    }
    fn writes(&self) -> &'static [FactType] {
        &[TRAVEL_TO, TRAVEL_SPEED]
    }
    fn cadence(&self) -> Cadence {
        Cadence::EveryTick
    }
    fn evaluate(&self, _view: &dyn CommittedView, ctx: &TickContext) -> Vec<Proposal> {
        if ctx.tick() != 1 {
            return Vec::new();
        }
        let p = |fact, v| {
            Proposal::new(
                self.id(),
                FactKey::new(e(self.who), fact),
                ctx.basis_tick(),
                Change::Set(v),
                Cause::new("decided"),
            )
        };
        vec![
            p(TRAVEL_TO, Value::Entity(e(self.what))),
            p(TRAVEL_SPEED, Value::Int(200)),
        ]
    }
}

fn run(s: &mut MemoryStore, ticks: u64, extra: Vec<Box<dyn System>>) {
    let domain = PhysicalDomain::new(config());
    s.install_spatial_index(domain.spatial_projector().unwrap());
    let domains: [&dyn Domain; 1] = [&domain];
    let mut systems = domain.systems();
    systems.extend(extra);
    for t in 1..=ticks {
        run_tick(s, &domains, &systems, t, 3, &mut Vec::new()).expect("commits");
    }
}

#[test]
fn ground_blends_between_samples_and_knows_its_slope() {
    // A slope rising 3 m over 10 m to the east, then level.
    let s = land(&[&[0, 300, 300], &[0, 300, 300]]);
    assert_eq!(ground(&s, e(LAND), 0, 0), 0);
    assert_eq!(ground(&s, e(LAND), 500, 500), 150, "halfway up");
    assert_eq!(ground(&s, e(LAND), 1_000, 0), 300);
    assert_eq!(ground(&s, e(LAND), 1_500, 1_000), 300, "on the level");
    assert_eq!(
        slope_percent(&s, e(LAND), 500, 500),
        Some(30),
        "a 30% grade"
    );
    assert_eq!(slope_percent(&s, e(LAND), 1_500, 500), Some(0));
    assert_eq!(
        ground(&s, e(LAND), 9_000, 0),
        0,
        "beyond the samples: level floor"
    );
}

#[test]
fn a_body_dropped_over_a_hillside_lands_on_it() {
    let mut s = land(&[&[0, 300, 300], &[0, 300, 300]]);
    // Someone 5 m up in the air over the middle of the slope.
    person(&mut s, 10, [500, 500, 650]);
    run(&mut s, 3, vec![]);
    let at = local_position(&s, e(10));
    assert_eq!(at, [500, 500, 150], "standing on the hillside");
    assert_eq!(
        s.read(FactKey::new(e(10), FALL_HEIGHT)).unwrap().value,
        Value::Int(500)
    );
    assert_eq!(support(&s, e(10), 40), 150);
}

#[test]
fn a_ridge_hides_one_valley_from_the_next() {
    // A 6 m ridge running north–south at x = 20 m.
    let mut s = land(&[&[0, 0, 600, 0, 0], &[0, 0, 600, 0, 0], &[0, 0, 600, 0, 0]]);
    person(&mut s, 10, [500, 1_000, 0]); // west valley
    person(&mut s, 11, [3_500, 1_000, 0]); // east valley
    person(&mut s, 12, [2_000, 1_000, 600]); // on the ridgeline
    assert!(!line_of_sight(&s, e(10), e(11)), "the ridge is in the way");
    assert!(line_of_sight(&s, e(12), e(10)), "the ridge sees west");
    assert!(line_of_sight(&s, e(12), e(11)), "and east");
}

#[test]
fn a_walker_finds_the_pass_when_the_cliff_is_too_steep() {
    // A 15 m cliff along x = 20 m — a 150% grade, too steep to walk — except to the north,
    // where the land is level: a pass, between y = 20 m and 30 m.
    let mut s = land(&[
        &[0, 0, 1_500, 0, 0],
        &[0, 0, 1_500, 0, 0],
        &[0, 0, 0, 0, 0],
        &[0, 0, 0, 0, 0],
    ]);
    person(&mut s, 10, [500, 500, 0]);
    // A cairn on the far side.
    seed(&mut s, 20, CONTAINED_IN, Value::Entity(e(LAND)));
    seed(&mut s, 20, POSITION, Value::Vec3([3_500, 500, 0]));
    let domain = PhysicalDomain::new(config());
    s.install_spatial_index(domain.spatial_projector().unwrap());
    let domains: [&dyn Domain; 1] = [&domain];
    let mut systems = domain.systems();
    systems.push(Box::new(Send { who: 10, what: 20 }));
    let mut furthest_north = 0;
    for t in 1..=60 {
        run_tick(&mut s, &domains, &systems, t, 3, &mut Vec::new()).expect("commits");
        let at = local_position(&s, e(10));
        furthest_north = furthest_north.max(at[1]);
        // Never on the cliff face.
        if (1_000..3_000).contains(&at[0]) && at[1] < 2_000 {
            panic!("tick {t}: climbing the cliff at {at:?}");
        }
    }
    assert!(s.read(FactKey::new(e(10), TRAVEL_BLOCKED)).is_none());
    assert!(s.read(FactKey::new(e(10), TRAVEL_TO)).is_none(), "arrived");
    assert!(furthest_north >= 2_000, "went round by the pass");
    let at = local_position(&s, e(10));
    assert!(
        (at[0] - 3_500).abs() <= 50 && (at[1] - 500).abs() <= 50,
        "at the cairn: {at:?}"
    );
}
