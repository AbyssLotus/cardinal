//! Bodies, facing, and motion (Vol. III Ch. 1, *Bodies, Facing, and Motion*; Amendment A-3):
//! a container's heading turns its contents with it, "to your left" is answerable, a body's
//! size is what it fills, and motion is a segment — written when it starts and when it ends,
//! never in between, yet visible at every tick.

use kernel::domain::Domain;
use kernel::events::ChronicleEntry;
use kernel::fact::{Cause, Fact, FactKey, FactType, Provenance, SystemId};
use kernel::identity::EntityId;
use kernel::proposal::Proposal;
use kernel::spatial::Aabb;
use kernel::store::MemoryStore;
use kernel::system::{Cadence, CommittedView, System, TickContext};
use kernel::tick::run_tick;
use kernel::time::SimClock;
use kernel::value::Value;
use physical::index::PLACEMENT_READS;
use physical::motion::{arrival, course, depart, is_moving, speed, velocity};
use physical::nearby::{in_box, within};
use physical::schema::{
    BODY_SIZE, CONTAINED_IN, HEADING, MOTION_END, MOTION_START, MOTION_TARGET, POSITION,
};
use physical::space::{distance, heading_in, local_position, position_in, relative_bearing};
use physical::{PhysicalConfig, PhysicalDomain};

const HARBOUR: u64 = 1;
const SHIP: u64 = 2;
const CAPTAIN: u64 = 10;
const BOSUN: u64 = 11;

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

fn put(s: &mut MemoryStore, id: u64, frame: u64, at: [i64; 3]) {
    seed(s, id, CONTAINED_IN, Value::Entity(e(frame)));
    seed(s, id, POSITION, Value::Vec3(at));
}

/// One-second ticks; the environment barely matters here.
fn config() -> PhysicalConfig {
    PhysicalConfig {
        clock: SimClock::new(1_000),
        day_length_seconds: 86_400,
        environment_step_seconds: 60,
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
    }
}

/// A ship moored in a harbour, its bow pointing north, with the captain 5 m forward of the
/// mast and the bosun 2 m to starboard (right).
fn ship() -> MemoryStore {
    let mut s = MemoryStore::new();
    put(&mut s, SHIP, HARBOUR, [10_000, 0, 0]);
    put(&mut s, CAPTAIN, SHIP, [0, 500, 0]);
    put(&mut s, BOSUN, SHIP, [200, 0, 0]);
    s
}

#[test]
fn turning_a_ship_turns_its_deck() {
    let mut s = ship();
    // Bow north: the captain, 5 m forward, is 5 m north of the ship's mast in the harbour.
    assert_eq!(
        position_in(&s, e(CAPTAIN), e(HARBOUR)),
        Some([10_000, 500, 0])
    );
    // The ship comes about to face east. Nothing about the crew is written...
    seed(&mut s, SHIP, HEADING, Value::Int(9_000));
    assert_eq!(
        local_position(&s, e(CAPTAIN)),
        [0, 500, 0],
        "still 5 m forward"
    );
    // ...yet "forward" now points east, and starboard points south.
    assert_eq!(
        position_in(&s, e(CAPTAIN), e(HARBOUR)),
        Some([10_500, 0, 0])
    );
    assert_eq!(
        position_in(&s, e(BOSUN), e(HARBOUR)),
        Some([10_000, -200, 0])
    );
    // The crew's spacing on the deck is the same whichever way the ship points.
    assert_eq!(distance(&s, e(CAPTAIN), e(BOSUN)), Some(538));
    // A heading part-way round turns them part-way, rounded to the centimetre.
    seed(&mut s, SHIP, HEADING, Value::Int(3_000)); // 30° east of north
    assert_eq!(
        position_in(&s, e(CAPTAIN), e(HARBOUR)),
        Some([10_250, 433, 0])
    );
}

#[test]
fn bearings_are_relative_to_where_you_face() {
    // A guard in a courtyard; the gate is 10 m east, the well 10 m north.
    let mut s = MemoryStore::new();
    put(&mut s, 20, HARBOUR, [0, 0, 0]); // guard
    put(&mut s, 21, HARBOUR, [1_000, 0, 0]); // gate
    put(&mut s, 22, HARBOUR, [0, 1_000, 0]); // well
    put(&mut s, 23, HARBOUR, [0, -1_000, 0]); // stables
                                              // Facing north: the well ahead, the gate on the right, the stables behind.
    assert_eq!(relative_bearing(&s, e(20), e(22)), Some(0));
    assert_eq!(relative_bearing(&s, e(20), e(21)), Some(9_000));
    assert_eq!(relative_bearing(&s, e(20), e(23)), Some(18_000));
    // The guard turns to face the gate: the gate is ahead, the well on the left.
    seed(&mut s, 20, HEADING, Value::Int(9_000));
    assert_eq!(relative_bearing(&s, e(20), e(21)), Some(0));
    assert_eq!(relative_bearing(&s, e(20), e(22)), Some(-9_000));
}

#[test]
fn facing_composes_through_turned_containers() {
    // The captain faces the bow; the ship faces east; so the captain faces east in the harbour.
    let mut s = ship();
    seed(&mut s, SHIP, HEADING, Value::Int(9_000));
    assert_eq!(heading_in(&s, e(CAPTAIN), e(HARBOUR)), Some(9_000));
    // Turning to face the stern (180° on deck) makes them face west.
    seed(&mut s, CAPTAIN, HEADING, Value::Int(18_000));
    assert_eq!(heading_in(&s, e(CAPTAIN), e(HARBOUR)), Some(27_000));
}

#[test]
fn a_body_fills_its_size_not_just_its_base() {
    // A 2 m × 1 m table whose base point is at the room's origin, and a cup 4 m away.
    let mut s = MemoryStore::new();
    put(&mut s, 30, HARBOUR, [0, 0, 0]);
    seed(&mut s, 30, BODY_SIZE, Value::Vec3([100, 50, 80]));
    put(&mut s, 31, HARBOUR, [400, 0, 0]);
    // A box over the table's far end (base point outside it) still finds the table...
    let far_end = Aabb::new([60, -10, 0], [120, 10, 10]);
    assert_eq!(in_box(&s, e(HARBOUR), &far_end), vec![e(30)]);
    // ...but turned 90°, the table's long side runs north–south and no longer reaches it.
    seed(&mut s, 30, HEADING, Value::Int(9_000));
    assert!(in_box(&s, e(HARBOUR), &far_end).is_empty());
    let north_end = Aabb::new([-10, 60, 0], [10, 120, 10]);
    assert_eq!(in_box(&s, e(HARBOUR), &north_end), vec![e(30)]);
    // Height counts: a box above the 80 cm tabletop misses it.
    assert!(in_box(&s, e(HARBOUR), &Aabb::new([-5, -5, 90], [5, 5, 200])).is_empty());
}

/// Sends the walker (40) north 14 m at a walking pace of 1.4 m/s, on tick 1 only.
struct SetOut;
impl System for SetOut {
    fn id(&self) -> SystemId {
        SystemId::new("test.set_out")
    }
    fn reads(&self) -> &'static [FactType] {
        PLACEMENT_READS
    }
    fn writes(&self) -> &'static [FactType] {
        &[POSITION, MOTION_TARGET, MOTION_START, MOTION_END]
    }
    fn cadence(&self) -> Cadence {
        Cadence::EveryTick
    }
    fn evaluate(&self, view: &dyn CommittedView, ctx: &TickContext) -> Vec<Proposal> {
        if ctx.tick() != 1 {
            return Vec::new();
        }
        depart(view, e(40), [0, 1_400, 0], 140, &config().clock)
            .into_iter()
            .map(|(key, change)| {
                Proposal::new(
                    self.id(),
                    key,
                    ctx.basis_tick(),
                    change,
                    Cause::new("set_out"),
                )
            })
            .collect()
    }
}

#[test]
fn motion_is_written_when_it_starts_and_ends_and_never_between() {
    let mut s = MemoryStore::new();
    put(&mut s, 40, HARBOUR, [0, 0, 0]); // the walker
    put(&mut s, 41, HARBOUR, [0, 700, 0]); // a lamp-post halfway along
    let domain = PhysicalDomain::new(config());
    s.install_spatial_index(domain.spatial_projector().unwrap());
    let domains: [&dyn Domain; 1] = [&domain];
    // The domain's own systems include Settle, which closes the segment on arrival.
    let mut systems = domain.systems();
    systems.push(Box::new(SetOut));
    let clock = config().clock;

    let mut writes_per_tick = Vec::new();
    for tick in 1..=13 {
        let mut chronicle: Vec<ChronicleEntry> = Vec::new();
        run_tick(&mut s, &domains, &systems, tick, 0, &mut chronicle).unwrap();
        let motion_writes = chronicle.iter().filter(|c| c.subject() == e(40)).count();
        writes_per_tick.push(motion_writes);
        // The walker sets out during tick 1 (the proposal reads tick 0 and the segment starts
        // there), so it has walked one second's worth by the end of tick 1, and five by tick 5.
        if tick == 5 {
            assert_eq!(local_position(&s, e(40)), [0, 700, 0]);
            assert_eq!(within(&s, e(41), 10), vec![(e(40), 0)], "at the lamp-post");
        }
        if tick == 6 {
            assert!(is_moving(&s, e(40)));
            assert_eq!(local_position(&s, e(40)), [0, 840, 0]);
            assert_eq!(velocity(&s, e(40), &clock), [0, 140, 0]);
            assert_eq!(speed(&s, e(40), &clock), 140);
            assert_eq!(course(&s, e(40)), Some(0), "due north");
            assert_eq!(arrival(&s, e(40)), Some(10), "14 m at 1.4 m/s: ten seconds");
            assert!(within(&s, e(41), 10).is_empty(), "past the lamp-post now");
        }
    }
    // Four facts at departure (tick 1); nothing while walking; four when Settle closes the
    // arrived segment (it arrives at tick 10, and Settle, reading tick 10, closes it on 11).
    let expected: Vec<usize> = vec![4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 4, 0, 0];
    assert_eq!(writes_per_tick, expected);
    assert!(!is_moving(&s, e(40)));
    assert_eq!(local_position(&s, e(40)), [0, 1_400, 0]);
    assert_eq!(speed(&s, e(40), &clock), 0);
    assert!(
        s.read(FactKey::new(e(40), MOTION_TARGET)).is_none(),
        "segment closed"
    );
}

#[test]
fn a_moving_container_carries_its_contents_without_writing_them() {
    // A cart rolls 30 m east over ten ticks; a passenger sits in it. Only the cart has a
    // segment, yet the passenger's position in the world moves with it.
    let mut s = MemoryStore::new();
    put(&mut s, 50, HARBOUR, [0, 0, 0]); // cart
    put(&mut s, 51, 50, [0, 0, 100]); // passenger, on the seat
    seed(&mut s, 50, MOTION_TARGET, Value::Vec3([3_000, 0, 0]));
    seed(&mut s, 50, MOTION_START, Value::Int(0));
    seed(&mut s, 50, MOTION_END, Value::Int(10));
    let domain = PhysicalDomain::new(config());
    let domains: [&dyn Domain; 1] = [&domain];
    let systems: Vec<Box<dyn System>> = Vec::new();
    for tick in 1..=4 {
        run_tick(&mut s, &domains, &systems, tick, 0, &mut Vec::new()).unwrap();
    }
    assert_eq!(position_in(&s, e(51), e(HARBOUR)), Some([1_200, 0, 100]));
    assert_eq!(local_position(&s, e(51)), [0, 0, 100], "still on the seat");
}
