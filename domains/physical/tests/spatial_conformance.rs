//! The conformance rule of Amendment A-2: every spatial query answered through the store's
//! index returns exactly what the same query returns by scanning committed facts — before any
//! tick, and after ticks that move things, re-parent them between rooms, and take them out of
//! the world. Also: reading through the index is reading the facts it mirrors, so a system must
//! declare them.
//!
//! One store, two roads: [`Scanning`] wraps the store and hides its index, so the same committed
//! state is queried both ways.

use kernel::domain::Domain;
use kernel::events::ChronicleEntry;
use kernel::fact::{Cause, Fact, FactKey, FactType, Provenance, SystemId};
use kernel::identity::EntityId;
use kernel::proposal::{Change, Proposal};
use kernel::spatial::Aabb;
use kernel::store::MemoryStore;
use kernel::system::{Cadence, CommittedView, System, TickContext};
use kernel::tick::{run_tick, TickError};
use kernel::time::SimClock;
use kernel::value::Value;
use physical::index::PLACEMENT_READS;
use physical::motion::depart;
use physical::nearby::{contents, in_box, nearest, within};
use physical::schema::{
    BODY_SIZE, CONTAINED_IN, HEADING, MOTION_END, MOTION_START, MOTION_TARGET, POSITION,
};
use physical::{PhysicalConfig, PhysicalDomain};
use std::sync::Arc;

/// The same committed state, with the spatial index hidden: every query takes the scanning road.
struct Scanning<'a>(&'a MemoryStore);

impl CommittedView for Scanning<'_> {
    fn read(&self, key: FactKey) -> Option<Fact> {
        self.0.read(key)
    }
    fn read_all(&self, key: FactKey) -> Vec<Fact> {
        self.0.read_all(key)
    }
    fn entities_with(&self, fact_type: FactType) -> Vec<EntityId> {
        self.0.entities_with(fact_type)
    }
    fn tick(&self) -> u64 {
        self.0.tick()
    }
    // `spatial` keeps its default: no index.
}

/// A small deterministic generator (xorshift64*), so the random world is the same every run.
struct Gen(u64);
impl Gen {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }
    fn range(&mut self, lo: i64, hi: i64) -> i64 {
        lo + (self.next() % (hi - lo + 1) as u64) as i64
    }
}

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

const ROOT: u64 = 1;
const ROOMS: std::ops::Range<u64> = 100..110; // rooms, in the root
const ALCOVES: std::ops::Range<u64> = 200..206; // alcoves, inside rooms
const THINGS: std::ops::Range<u64> = 1000..1500; // people and objects

/// A world of rooms and alcoves with five hundred things scattered through them — some at
/// their container's origin (no position facts), some at large offsets.
fn world(g: &mut Gen) -> MemoryStore {
    let mut s = MemoryStore::new();
    let place = |s: &mut MemoryStore, g: &mut Gen, id: u64, frame: u64, spread: i64| {
        seed(s, id, CONTAINED_IN, Value::Entity(e(frame)));
        if g.next() % 5 != 0 {
            let z = if g.next() % 2 == 0 {
                g.range(-500, 500)
            } else {
                0
            };
            let at = [g.range(-spread, spread), g.range(-spread, spread), z];
            seed(s, id, POSITION, Value::Vec3(at));
        }
        // Amendment A-3 in the mix: some bodies have size, many face somewhere, and some are
        // part-way along a journey that started at tick 0.
        if g.next() % 3 == 0 {
            let size = [g.range(0, 300), g.range(0, 300), g.range(0, 250)];
            seed(s, id, BODY_SIZE, Value::Vec3(size));
        }
        if g.next() % 2 == 0 {
            seed(s, id, HEADING, Value::Int(g.range(0, 35_999)));
        }
        if id >= THINGS.start && g.next() % 4 == 0 {
            let to = [g.range(-spread, spread), g.range(-spread, spread), 0];
            seed(s, id, MOTION_TARGET, Value::Vec3(to));
            seed(s, id, MOTION_START, Value::Int(0));
            seed(s, id, MOTION_END, Value::Int(g.range(1, 10)));
        }
    };
    for room in ROOMS {
        place(&mut s, g, room, ROOT, 5_000);
    }
    for alcove in ALCOVES {
        let room = ROOMS.start + g.next() % (ROOMS.end - ROOMS.start);
        place(&mut s, g, alcove, room, 1_000);
    }
    for thing in THINGS {
        let frame = match g.next() % 10 {
            0 => ROOT,
            1..=2 => ALCOVES.start + g.next() % (ALCOVES.end - ALCOVES.start),
            _ => ROOMS.start + g.next() % (ROOMS.end - ROOMS.start),
        };
        place(&mut s, g, thing, frame, 2_000);
    }
    s
}

/// Ask everything both ways and require identical answers.
fn assert_conforms(store: &MemoryStore, g: &mut Gen) {
    assert!(store.spatial().is_some(), "the index road must exist");
    let scan = Scanning(store);
    let frames: Vec<u64> = std::iter::once(ROOT).chain(ROOMS).chain(ALCOVES).collect();
    for &f in &frames {
        assert_eq!(
            contents(store, e(f)),
            contents(&scan, e(f)),
            "contents of {f}"
        );
        for _ in 0..3 {
            let a = [
                g.range(-6_000, 6_000),
                g.range(-6_000, 6_000),
                g.range(-600, 600),
            ];
            let b = [
                g.range(-6_000, 6_000),
                g.range(-6_000, 6_000),
                g.range(-600, 600),
            ];
            let bx = Aabb::new(a, b);
            assert_eq!(
                in_box(store, e(f), &bx),
                in_box(&scan, e(f), &bx),
                "in_box {f} {bx:?}"
            );
        }
    }
    // Twenty-five centres per pass: the scanning road measures everyone, so it is the slow one.
    for _ in 0..25 {
        let center = THINGS.start + g.next() % (THINGS.end - THINGS.start);
        for radius in [0, 150, 1_000, 4_000, 20_000, i64::MAX / 4] {
            assert_eq!(
                within(store, e(center), radius),
                within(&scan, e(center), radius),
                "within {radius} of {center}"
            );
        }
        for k in [1, 5, 40] {
            assert_eq!(
                nearest(store, e(center), k, 1_000_000),
                nearest(&scan, e(center), k, 1_000_000),
                "nearest {k} to {center}"
            );
        }
    }
}

fn indexed(mut store: MemoryStore) -> MemoryStore {
    let domain = PhysicalDomain::new(config());
    store.install_spatial_index(domain.spatial_projector().expect("physical owns space"));
    store
}

#[test]
fn indexed_answers_equal_scanned_answers() {
    let mut g = Gen(0x5eed);
    let store = indexed(world(&mut g));
    assert_conforms(&store, &mut g);
}

#[test]
fn an_index_installed_before_seeding_equals_one_built_after() {
    // Seeding into an indexed store re-places incrementally; installing afterwards builds in
    // one pass. Both must index the same world.
    let mut g = Gen(77);
    let after = indexed(world(&mut g));
    let mut before = MemoryStore::new();
    let domain = PhysicalDomain::new(config());
    before.install_spatial_index(domain.spatial_projector().unwrap());
    let mut g = Gen(77);
    let fresh = world(&mut g);
    for entity in 0..2_000 {
        for (key, fact) in kernel::store::RealityStore::facts_of(&fresh, e(entity)) {
            before.seed(key, fact);
        }
    }
    let (a, b) = (after.spatial().unwrap(), before.spatial().unwrap());
    for id in 0..2_000 {
        assert_eq!(a.placement(e(id)), b.placement(e(id)), "placement of {id}");
        assert_eq!(a.children(e(id)), b.children(e(id)), "children of {id}");
        assert_eq!(a.subframes(e(id)), b.subframes(e(id)), "subframes of {id}");
    }
}

/// Each tick, shuffles twenty things: re-parents some to another room, sends others walking
/// somewhere nearby (a motion segment), and takes a few out of the world entirely (tombstoning
/// their containment).
struct Shuffle;
impl System for Shuffle {
    fn id(&self) -> SystemId {
        SystemId::new("test.shuffle")
    }
    fn reads(&self) -> &'static [FactType] {
        PLACEMENT_READS
    }
    fn writes(&self) -> &'static [FactType] {
        &[
            CONTAINED_IN,
            POSITION,
            MOTION_TARGET,
            MOTION_START,
            MOTION_END,
        ]
    }
    fn cadence(&self) -> Cadence {
        Cadence::EveryTick
    }
    fn evaluate(&self, view: &dyn CommittedView, ctx: &TickContext) -> Vec<Proposal> {
        let mut rng = ctx.rng(0);
        let mut out = Vec::new();
        let mut chosen = std::collections::BTreeSet::new();
        while chosen.len() < 20 {
            chosen.insert(THINGS.start + rng.below(THINGS.end - THINGS.start));
        }
        for thing in chosen {
            let key = |ft| FactKey::new(e(thing), ft);
            let change = |ft, c| {
                Proposal::new(
                    self.id(),
                    key(ft),
                    ctx.basis_tick(),
                    c,
                    Cause::new("shuffle"),
                )
            };
            match rng.below(4) {
                0 => {
                    let room = ROOMS.start + rng.below(ROOMS.end - ROOMS.start);
                    out.push(change(CONTAINED_IN, Change::Set(Value::Entity(e(room)))));
                }
                1 => out.push(change(CONTAINED_IN, Change::Tombstone)),
                _ => {
                    let here = physical::space::local_position(view, e(thing));
                    let to = [
                        here[0] + rng.below(2_001) as i64 - 1_000,
                        here[1] + rng.below(2_001) as i64 - 1_000,
                        here[2],
                    ];
                    let clock = config().clock;
                    for (key, c) in depart(view, e(thing), to, 140, &clock) {
                        out.push(change(key.fact_type, c));
                    }
                }
            }
        }
        out
    }
}

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
        gravity_cm_s2: 981,
        step_height_cm: 40,
        max_slope_percent: 100,
        nav_cell_cm: 50,
    }
}

#[test]
fn the_index_stays_exact_as_things_move_between_rooms() {
    let mut g = Gen(4242);
    let mut store = indexed(world(&mut g));
    let domain = PhysicalDomain::new(config());
    let domains: [&dyn Domain; 1] = [&domain];
    let systems: Vec<Box<dyn System>> = vec![Box::new(Shuffle)];
    let mut chronicle: Vec<ChronicleEntry> = Vec::new();
    for tick in 1..=6 {
        run_tick(&mut store, &domains, &systems, tick, 9, &mut chronicle).expect("commits");
        assert_eq!(store.tick(), tick, "the store knows its committed tick");
        assert_conforms(&store, &mut g);
    }
}

/// Asks a proximity question while declaring only containment — not the position facts the
/// index also mirrors.
struct HalfDeclared;
impl System for HalfDeclared {
    fn id(&self) -> SystemId {
        SystemId::new("test.half_declared")
    }
    fn reads(&self) -> &'static [FactType] {
        &[CONTAINED_IN]
    }
    fn writes(&self) -> &'static [FactType] {
        &[]
    }
    fn cadence(&self) -> Cadence {
        Cadence::EveryTick
    }
    fn evaluate(&self, view: &dyn CommittedView, _ctx: &TickContext) -> Vec<Proposal> {
        let _ = within(view, e(THINGS.start), 1_000);
        Vec::new()
    }
}

#[test]
fn reading_through_the_index_requires_declaring_what_it_mirrors() {
    let mut g = Gen(1);
    let mut store = indexed(world(&mut g));
    let domain = PhysicalDomain::new(config());
    let domains: [&dyn Domain; 1] = [&domain];
    let systems: Vec<Box<dyn System>> = vec![Box::new(HalfDeclared)];
    let err = run_tick(&mut store, &domains, &systems, 1, 0, &mut Vec::new())
        .expect_err("the index is not a back door around the read set");
    assert!(
        matches!(err, TickError::UndeclaredRead { fact_type, .. } if fact_type == POSITION),
        "got {err:?}"
    );
}

#[test]
fn the_physical_domain_supplies_the_placement_rule() {
    let domain = PhysicalDomain::new(config());
    let projector = domain.spatial_projector().expect("physical owns space");
    assert!(projector.watches().contains(&CONTAINED_IN));
    // And a domain's projector is shareable — installing it twice is fine.
    let mut s = MemoryStore::new();
    s.install_spatial_index(Arc::clone(&projector));
    s.install_spatial_index(projector);
}
