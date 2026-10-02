//! A house, a yard, and things that move between them — under the world's own rules
//! (Vol. III Ch. 1 §1.5–1.11; Amendments A-3, A-4; Appendix A, Ruling 13).
//!
//! ```text
//!   site (100) — the ground frame: z = 0 is ground level
//!   ├── yard (1)                          open ground; a shed (40) stands in it
//!   └── house (2), at site (1000, 0)
//!       ├── upstairs     (5)  z = +300    bedroom window (1011): west wall, glass, shut
//!       ├── ground floor (4)  z =    0    front door (north), low window (west), stairs
//!       └── cellar       (3)  z = -300    bulkhead to the yard (east)
//!
//!   Every room is enclosed, 10 m × 10 m, 2.8 m high: walls everywhere but its openings.
//!   Front door:  yard 1001 ↔ ground 1002   90 cm wide, 2.05 m tall
//!   Low window:  yard 1003 ↔ ground 1004   40 cm wide, 30 cm off the floor
//!   Bulkhead:    yard 1005 ↔ cellar 1006   1 m wide, 1.2 m tall
//!   Stairs:      ground 1007 ↔ upstairs 1008;  ground 1009 ↔ cellar 1010
//! ```
//!
//! Nothing here moves a body directly. Each test only says where things *want* to go
//! ([`TRAVEL_TO`]) — as a player's command or an NPC's choice would — and the physical domain's
//! own systems carry it out: routing through openings that are open and that the body fits and
//! can reach, walking around what is solid, dropping what nothing holds up. The tests then ask
//! the engine's queries what became of everyone.

use kernel::domain::Domain;
use kernel::fact::{Cause, Fact, FactKey, FactType, Provenance, SystemId};
use kernel::identity::EntityId;
use kernel::proposal::{Change, Proposal};
use kernel::store::{MemoryStore, RealityStore};
use kernel::system::{Cadence, CommittedView, System, TickContext};
use kernel::tick::run_tick;
use kernel::time::SimClock;
use kernel::value::Value;
use physical::regions::is_within;
use physical::schema::{
    BODY_SIZE, CONTAINED_IN, ENCLOSED, FALL_HEIGHT, HAS_PORTAL, HEADING, LEADS_TO, MOBILE, OPAQUE,
    PORTAL_FAR_SIDE, PORTAL_OPEN, POSITION, SOLID, TRAVEL_BLOCKED, TRAVEL_SPEED, TRAVEL_TO,
};
use physical::shape::body_box;
use physical::sight::line_of_sight;
use physical::space::{height_above_ground, local_position, position_in};
use physical::{PhysicalConfig, PhysicalDomain};

// Places.
const SITE: u64 = 100;
const YARD: u64 = 1;
const HOUSE: u64 = 2;
const CELLAR: u64 = 3;
const GROUND: u64 = 4;
const UPSTAIRS: u64 = 5;
// Openings.
const DOOR_OUT: u64 = 1001;
const DOOR_IN: u64 = 1002;
const WINDOW_OUT: u64 = 1003;
const WINDOW_IN: u64 = 1004;
const BULKHEAD_OUT: u64 = 1005;
const BULKHEAD_IN: u64 = 1006;
const STAIRS_UP: u64 = 1007;
const STAIRS_DOWN: u64 = 1008;
const CELLAR_DOWN: u64 = 1009;
const CELLAR_UP: u64 = 1010;
const BEDROOM_WINDOW: u64 = 1011;
const BEDROOM_WINDOW_OUT: u64 = 1012;
// Bodies.
const CAT: u64 = 10;
const COURIER: u64 = 11;
const RACCOON: u64 = 12;
const SECOND_COURIER: u64 = 13;
const ARCHER: u64 = 20;
const WARDROBE: u64 = 30;
const SHED: u64 = 40;
const ROOFER: u64 = 41;
const STONE: u64 = 42;
const LOOKOUT: u64 = 50;

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

fn sized(s: &mut MemoryStore, id: u64, size: [i64; 3], heading: i64) {
    seed(s, id, BODY_SIZE, Value::Vec3(size));
    seed(s, id, HEADING, Value::Int(heading));
}

fn flag(s: &mut MemoryStore, id: u64, fact: FactType) {
    seed(s, id, fact, Value::Bool(true));
}

/// One face of an opening: a portal in `host`, leading to `dest`, at `at`, of `size`, turned to
/// `heading` so its width runs along its wall.
fn face(
    s: &mut MemoryStore,
    id: u64,
    host: u64,
    dest: u64,
    at: [i64; 3],
    size: [i64; 3],
    heading: i64,
) {
    put(s, id, host, at);
    sized(s, id, size, heading);
    seed(s, id, LEADS_TO, Value::Entity(e(dest)));
    seed(s, host, HAS_PORTAL, Value::Entity(e(id)));
}

/// Both faces of an opening, linked, so whatever passes through one emerges at the other.
fn pair(s: &mut MemoryStore, a: u64, b: u64) {
    seed(s, a, PORTAL_FAR_SIDE, Value::Entity(e(b)));
    seed(s, b, PORTAL_FAR_SIDE, Value::Entity(e(a)));
}

/// A free-moving person-sized body.
fn person(s: &mut MemoryStore, id: u64, frame: u64, at: [i64; 3]) {
    put(s, id, frame, at);
    sized(s, id, [25, 15, 175], 0);
    flag(s, id, MOBILE);
}

fn house() -> MemoryStore {
    let mut s = MemoryStore::new();
    put(&mut s, YARD, SITE, [0, 0, 0]);
    put(&mut s, HOUSE, SITE, [1000, 0, 0]);
    for (room, z) in [(CELLAR, -300), (GROUND, 0), (UPSTAIRS, 300)] {
        put(&mut s, room, HOUSE, [0, 0, z]);
        sized(&mut s, room, [500, 500, 280], 0);
        flag(&mut s, room, ENCLOSED);
    }

    // The front door, in the north wall; a door is opaque when shut.
    let door = [45, 10, 205];
    face(&mut s, DOOR_OUT, YARD, GROUND, [1000, 500, 0], door, 0);
    face(&mut s, DOOR_IN, GROUND, YARD, [0, 500, 0], door, 0);
    pair(&mut s, DOOR_OUT, DOOR_IN);
    flag(&mut s, DOOR_OUT, OPAQUE);
    flag(&mut s, DOOR_IN, OPAQUE);
    // A low window in the west wall: 40 cm wide, its sill 30 cm off the floor, glass.
    let window = [20, 10, 50];
    face(
        &mut s,
        WINDOW_OUT,
        YARD,
        GROUND,
        [500, 0, 30],
        window,
        9_000,
    );
    face(
        &mut s,
        WINDOW_IN,
        GROUND,
        YARD,
        [-500, 0, 30],
        window,
        9_000,
    );
    pair(&mut s, WINDOW_OUT, WINDOW_IN);
    // The bulkhead down to the cellar, east wall.
    let bulkhead = [50, 10, 120];
    face(
        &mut s,
        BULKHEAD_OUT,
        YARD,
        CELLAR,
        [1500, 0, 0],
        bulkhead,
        9_000,
    );
    face(
        &mut s,
        BULKHEAD_IN,
        CELLAR,
        YARD,
        [500, 0, 0],
        bulkhead,
        9_000,
    );
    pair(&mut s, BULKHEAD_OUT, BULKHEAD_IN);
    // Stairs.
    let stairs = [50, 50, 220];
    face(
        &mut s,
        STAIRS_UP,
        GROUND,
        UPSTAIRS,
        [400, -400, 0],
        stairs,
        0,
    );
    face(
        &mut s,
        STAIRS_DOWN,
        UPSTAIRS,
        GROUND,
        [400, -400, 0],
        stairs,
        0,
    );
    pair(&mut s, STAIRS_UP, STAIRS_DOWN);
    face(
        &mut s,
        CELLAR_DOWN,
        GROUND,
        CELLAR,
        [-400, -400, 0],
        stairs,
        0,
    );
    face(
        &mut s,
        CELLAR_UP,
        CELLAR,
        GROUND,
        [-400, -400, 0],
        stairs,
        0,
    );
    pair(&mut s, CELLAR_DOWN, CELLAR_UP);
    // The bedroom window: west wall upstairs, 80 cm wide, sill 80 cm up, glass, shut. Its yard
    // face hangs 3.8 m above the ground.
    let bedroom_window = [40, 10, 100];
    face(
        &mut s,
        BEDROOM_WINDOW,
        UPSTAIRS,
        YARD,
        [-500, 0, 80],
        bedroom_window,
        9_000,
    );
    face(
        &mut s,
        BEDROOM_WINDOW_OUT,
        YARD,
        UPSTAIRS,
        [500, 0, 380],
        bedroom_window,
        9_000,
    );
    pair(&mut s, BEDROOM_WINDOW, BEDROOM_WINDOW_OUT);
    for w in [BEDROOM_WINDOW, BEDROOM_WINDOW_OUT] {
        seed(&mut s, w, PORTAL_OPEN, Value::Bool(false));
    }

    // Who and what is about. (Each test places the courier itself: seeding is world
    // generation, and seeding a second container would give it two, not move it.)
    put(&mut s, CAT, YARD, [600, 300, 0]);
    sized(&mut s, CAT, [10, 20, 35], 0);
    flag(&mut s, CAT, MOBILE);
    put(&mut s, RACCOON, CELLAR, [0, 0, 0]);
    sized(&mut s, RACCOON, [12, 25, 40], 0);
    flag(&mut s, RACCOON, MOBILE);
    person(&mut s, ARCHER, UPSTAIRS, [-430, 0, 0]);
    seed(&mut s, ARCHER, HEADING, Value::Int(27_000)); // facing west, out of the window
    s
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
        nav_cell_cm: 50,
    }
}

/// A decider standing in for a player or an NPC: on the given tick, it asks bodies to travel
/// and opens doors. It never moves anything itself (Ruling 13).
struct Decide {
    at: u64,
    go: Vec<(u64, u64, i64)>, // (body, destination, speed cm/s)
    open: Vec<u64>,           // portals to open
}

impl System for Decide {
    fn id(&self) -> SystemId {
        SystemId::new("test.decide")
    }
    fn reads(&self) -> &'static [FactType] {
        &[]
    }
    fn writes(&self) -> &'static [FactType] {
        &[TRAVEL_TO, TRAVEL_SPEED, PORTAL_OPEN]
    }
    fn cadence(&self) -> Cadence {
        Cadence::EveryTick
    }
    fn evaluate(&self, _view: &dyn CommittedView, ctx: &TickContext) -> Vec<Proposal> {
        if ctx.tick() != self.at {
            return Vec::new();
        }
        let mut out = Vec::new();
        let mut push = |id: u64, fact: FactType, v: Value| {
            out.push(Proposal::new(
                self.id(),
                FactKey::new(e(id), fact),
                ctx.basis_tick(),
                Change::Set(v),
                Cause::new("decided"),
            ));
        };
        for &(body, dest, speed) in &self.go {
            push(body, TRAVEL_TO, Value::Entity(e(dest)));
            push(body, TRAVEL_SPEED, Value::Int(speed));
        }
        for &p in &self.open {
            push(p, PORTAL_OPEN, Value::Bool(true));
        }
        out
    }
}

/// A running world: the house's committed state, the physical domain, and its spatial index.
struct World {
    store: MemoryStore,
    domain: PhysicalDomain,
    tick: u64,
}

impl World {
    fn new(store: MemoryStore) -> Self {
        let domain = PhysicalDomain::new(config());
        let mut store = store;
        store.install_spatial_index(domain.spatial_projector().unwrap());
        Self {
            store,
            domain,
            tick: 0,
        }
    }

    /// Advance `ticks` one-second ticks, with `decide` (if any) acting on the first of them.
    fn run(&mut self, ticks: u64, decide: Option<Decide>) {
        let domains: [&dyn Domain; 1] = [&self.domain];
        let mut systems = self.domain.systems();
        if let Some(mut d) = decide {
            d.at = self.tick + 1;
            systems.push(Box::new(d));
        }
        for _ in 0..ticks {
            self.tick += 1;
            run_tick(
                &mut self.store,
                &domains,
                &systems,
                self.tick,
                7,
                &mut Vec::new(),
            )
            .expect("the tick commits");
        }
    }

    /// The same world with no spatial index: every query takes the scanning road.
    fn unindexed(store: MemoryStore) -> Self {
        Self {
            store,
            domain: PhysicalDomain::new(config()),
            tick: 0,
        }
    }

    fn room_of(&self, id: u64) -> u64 {
        match self
            .store
            .read(FactKey::new(e(id), CONTAINED_IN))
            .map(|f| f.value)
        {
            Some(Value::Entity(r)) => r.raw(),
            other => panic!("{id} is nowhere: {other:?}"),
        }
    }

    fn flag(&self, id: u64, fact: FactType) -> bool {
        matches!(
            self.store.read(FactKey::new(e(id), fact)).map(|f| f.value),
            Some(Value::Bool(true))
        )
    }

    fn travelling(&self, id: u64) -> bool {
        self.store.read(FactKey::new(e(id), TRAVEL_TO)).is_some()
    }
}

fn go(body: u64, dest: u64, speed: i64) -> Decide {
    Decide {
        at: 0,
        go: vec![(body, dest, speed)],
        open: vec![],
    }
}

#[test]
fn with_the_door_shut_the_cat_comes_in_the_window_and_lands_under_it() {
    let mut s = house();
    person(&mut s, COURIER, YARD, [900, 800, 0]);
    for d in [DOOR_OUT, DOOR_IN] {
        seed(&mut s, d, PORTAL_OPEN, Value::Bool(false));
    }
    let mut w = World::new(s);
    w.run(
        12,
        Some(Decide {
            at: 0,
            go: vec![(CAT, GROUND, 300), (COURIER, GROUND, 140)],
            open: vec![],
        }),
    );
    // The cat — small enough, and the sill low enough to hop — came in through the window,
    // dropped the 30 cm from the sill, and is on the floor right under it, done travelling.
    assert_eq!(w.room_of(CAT), GROUND);
    assert_eq!(local_position(&w.store, e(CAT)), [-500, 0, 0]);
    let fell = w.store.read(FactKey::new(e(CAT), FALL_HEIGHT)).unwrap();
    assert_eq!(fell.value, Value::Int(30));
    assert!(!w.travelling(CAT), "arrived");
    // The courier is too broad for that window and the door is shut: still outside, and the
    // world says so.
    assert_eq!(w.room_of(COURIER), YARD);
    assert!(w.flag(COURIER, TRAVEL_BLOCKED));
    assert!(
        w.travelling(COURIER),
        "the intent stands, waiting for a way"
    );

    // Someone opens the door. The courier goes in by it, and is no longer blocked.
    w.run(
        8,
        Some(Decide {
            at: 0,
            go: vec![],
            open: vec![DOOR_OUT, DOOR_IN],
        }),
    );
    assert_eq!(w.room_of(COURIER), GROUND);
    assert!(is_within(&w.store, e(COURIER), e(HOUSE)));
    assert!(!w.flag(COURIER, TRAVEL_BLOCKED));
    assert!(!w.travelling(COURIER));
    assert_eq!(
        local_position(&w.store, e(COURIER)),
        [0, 500, 0],
        "on the doormat"
    );
}

#[test]
fn two_people_cannot_squeeze_through_one_doorway_at_once() {
    let mut s = house();
    // Both are standing right at the door.
    person(&mut s, COURIER, YARD, [1000, 500, 0]);
    person(&mut s, SECOND_COURIER, YARD, [1000, 500, 0]);
    let mut w = World::new(s);
    w.run(
        1,
        Some(Decide {
            at: 0,
            go: vec![(COURIER, GROUND, 140), (SECOND_COURIER, GROUND, 140)],
            open: vec![],
        }),
    );
    // Same tick, same opening: the lower id goes first...
    assert_eq!(w.room_of(COURIER), YARD, "deciding takes the first tick");
    w.run(1, None);
    assert_eq!(w.room_of(COURIER), GROUND);
    assert_eq!(
        w.room_of(SECOND_COURIER),
        YARD,
        "...and the other waits its turn"
    );
    w.run(1, None);
    assert_eq!(w.room_of(SECOND_COURIER), GROUND);
}

#[test]
fn someone_who_walks_off_the_shed_roof_falls() {
    let mut s = house();
    // A 2.5 m shed in the yard, someone standing on its roof, and a stone 4 m to the north.
    put(&mut s, SHED, YARD, [300, -800, 0]);
    sized(&mut s, SHED, [150, 100, 250], 0);
    flag(&mut s, SHED, SOLID);
    person(&mut s, ROOFER, YARD, [300, -800, 250]);
    put(&mut s, STONE, YARD, [300, -400, 0]);
    let mut w = World::new(s);
    assert_eq!(height_above_ground(&w.store, e(ROOFER)), 250, "on the roof");
    w.run(1, Some(go(ROOFER, STONE, 140)));
    w.run(8, None);
    // It walked level off the edge, fell the shed's height, and finished beside the stone.
    let fell = w.store.read(FactKey::new(e(ROOFER), FALL_HEIGHT)).unwrap();
    assert_eq!(fell.value, Value::Int(250));
    assert_eq!(
        height_above_ground(&w.store, e(ROOFER)),
        0,
        "on the ground now"
    );
    assert!(!w.travelling(ROOFER));
    let at = local_position(&w.store, e(ROOFER));
    assert!((at[1] - -400).abs() <= 30, "beside the stone: {at:?}");
}

#[test]
fn the_archer_at_the_window_sees_the_yard_but_not_the_cellar_or_downstairs() {
    let mut s = house();
    person(&mut s, LOOKOUT, YARD, [100, 0, 0]);
    seed(&mut s, LOOKOUT, MOBILE, Value::Bool(false)); // stands still
    person(&mut s, COURIER, GROUND, [0, 0, 0]);
    let w = World::new(s);
    // Through the bedroom window — shut, but glass — down into the yard.
    assert!(line_of_sight(&w.store, e(ARCHER), e(LOOKOUT)));
    assert!(line_of_sight(&w.store, e(LOOKOUT), e(ARCHER)), "and back");
    // Not through the floor to the room below, nor down to the cellar.
    assert!(!line_of_sight(&w.store, e(ARCHER), e(COURIER)));
    assert!(!line_of_sight(&w.store, e(ARCHER), e(RACCOON)));
    // Draw the curtains (make the shut window opaque) and the yard is gone too.
    let mut s = house();
    person(&mut s, LOOKOUT, YARD, [100, 0, 0]);
    flag(&mut s, BEDROOM_WINDOW, OPAQUE);
    let w = World::new(s);
    assert!(!line_of_sight(&w.store, e(ARCHER), e(LOOKOUT)));
}

#[test]
fn a_solid_opaque_thing_in_between_blocks_the_view() {
    let mut s = house();
    person(&mut s, LOOKOUT, YARD, [100, 0, 0]);
    seed(&mut s, LOOKOUT, MOBILE, Value::Bool(false));
    let w = World::new(s.clone());
    assert!(line_of_sight(&w.store, e(ARCHER), e(LOOKOUT)));
    // A hay wagon pulls up between the house and the lookout.
    put(&mut s, 60, YARD, [300, 0, 0]);
    sized(&mut s, 60, [100, 200, 300], 0);
    flag(&mut s, 60, SOLID);
    flag(&mut s, 60, OPAQUE);
    let w = World::new(s);
    assert!(!line_of_sight(&w.store, e(ARCHER), e(LOOKOUT)));
}

#[test]
fn a_wardrobe_does_not_fit_through_any_way_in() {
    let mut s = house();
    person(&mut s, COURIER, YARD, [900, 800, 0]);
    put(&mut s, WARDROBE, YARD, [1100, 900, 0]);
    sized(&mut s, WARDROBE, [60, 30, 200], 0);
    flag(&mut s, WARDROBE, MOBILE);
    let mut w = World::new(s);
    w.run(1, Some(go(WARDROBE, GROUND, 100)));
    w.run(3, None);
    // 1.2 m wide: wider than the door (90 cm), the window, and the bulkhead (1 m).
    assert_eq!(w.room_of(WARDROBE), YARD);
    assert!(w.flag(WARDROBE, TRAVEL_BLOCKED));
    // The courier, 50 cm wide, is not blocked by the same door.
    let mut w2 = World::new(w.store.clone());
    w2.tick = w.tick;
    w2.run(1, Some(go(COURIER, GROUND, 140)));
    w2.run(6, None);
    assert_eq!(w2.room_of(COURIER), GROUND);
}

#[test]
fn someone_can_go_upstairs_and_the_engine_knows_how_high_they_are() {
    let mut s = house();
    person(&mut s, COURIER, GROUND, [0, 300, 0]);
    let mut w = World::new(s);
    w.run(1, Some(go(COURIER, UPSTAIRS, 140)));
    w.run(10, None);
    assert_eq!(w.room_of(COURIER), UPSTAIRS);
    assert!(
        is_within(&w.store, e(COURIER), e(HOUSE)),
        "still inside the building"
    );
    assert_eq!(height_above_ground(&w.store, e(COURIER)), 300);
    // At the top of the stairs, which in house coordinates is (400, −400, 300).
    assert_eq!(
        position_in(&w.store, e(COURIER), e(HOUSE)),
        Some([400, -400, 300])
    );
    assert!(!w.travelling(COURIER));
}

#[test]
fn a_walker_goes_around_the_table_not_through_it() {
    let mut s = house();
    // A 2 m × 3 m table, waist high, in the middle of the ground floor; a lamp beyond it.
    put(&mut s, 61, GROUND, [0, 0, 0]);
    sized(&mut s, 61, [100, 150, 80], 0);
    flag(&mut s, 61, SOLID);
    put(&mut s, 62, GROUND, [300, 0, 0]);
    person(&mut s, COURIER, GROUND, [-300, 0, 0]);
    let mut w = World::new(s);
    let table = body_box(&w.store, e(61)).unwrap();
    w.run(1, Some(go(COURIER, 62, 140)));
    let mut ticks = 0;
    while w.travelling(COURIER) && ticks < 40 {
        w.run(1, None);
        ticks += 1;
        // Never inside the table's footprint (grown by the walker's own half-width).
        let at = local_position(&w.store, e(COURIER));
        assert!(
            !table.footprint_contains(at, 24),
            "tick {ticks}: walked into the table at {at:?}"
        );
    }
    assert!(!w.travelling(COURIER), "arrived within 40 s");
    let at = local_position(&w.store, e(COURIER));
    assert!(
        (at[0] - 300).abs() <= 30 && at[1].abs() <= 30,
        "beside the lamp: {at:?}"
    );
}

#[test]
fn leaving_the_building_is_recognised_too() {
    let mut s = house();
    person(&mut s, COURIER, GROUND, [0, 300, 0]);
    let mut w = World::new(s);
    w.run(1, Some(go(COURIER, YARD, 140)));
    w.run(5, None);
    assert_eq!(w.room_of(COURIER), YARD);
    assert!(!is_within(&w.store, e(COURIER), e(HOUSE)));
    // Out by the front door, on the yard side of it.
    assert_eq!(local_position(&w.store, e(COURIER)), [1000, 500, 0]);
}

#[test]
fn the_spatial_index_changes_nothing_that_happens() {
    // Amendment A-2's conformance rule, end to end: travel, gravity, support, and sight all ask
    // spatial questions; with or without the index, the world must come out bit-identical.
    let scene = || {
        let mut s = house();
        person(&mut s, COURIER, YARD, [900, 800, 0]);
        person(&mut s, ROOFER, YARD, [300, -800, 250]);
        put(&mut s, SHED, YARD, [300, -800, 0]);
        sized(&mut s, SHED, [150, 100, 250], 0);
        flag(&mut s, SHED, SOLID);
        put(&mut s, 61, GROUND, [0, 0, 0]);
        sized(&mut s, 61, [100, 150, 80], 0);
        flag(&mut s, 61, SOLID);
        s
    };
    let orders = || Decide {
        at: 0,
        go: vec![
            (CAT, UPSTAIRS, 300),
            (COURIER, CELLAR, 140),
            (ROOFER, GROUND, 140),
            (RACCOON, YARD, 200),
        ],
        open: vec![],
    };
    let mut indexed = World::new(scene());
    let mut scanned = World::unindexed(scene());
    assert!(scanned.store.spatial().is_none());
    for w in [&mut indexed, &mut scanned] {
        w.run(1, Some(orders()));
        w.run(40, None);
    }
    assert_eq!(indexed.store.state_hash(), scanned.store.state_hash());
    // And they really went somewhere.
    assert_eq!(indexed.room_of(COURIER), CELLAR);
    assert_eq!(indexed.room_of(ROOFER), GROUND);
    assert_eq!(indexed.room_of(RACCOON), YARD);
    for (a, b) in [
        (ARCHER, CAT),
        (COURIER, RACCOON),
        (ROOFER, CAT),
        (CAT, RACCOON),
    ] {
        assert_eq!(
            line_of_sight(&indexed.store, e(a), e(b)),
            line_of_sight(&scanned.store, e(a), e(b)),
            "sight {a} -> {b}"
        );
    }
}
