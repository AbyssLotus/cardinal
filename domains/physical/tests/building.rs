//! Getting into a building and moving around inside it (Vol. III Ch. 1 §1.5 connectivity,
//! §1.6 "Above" / "Below" / "Reachable", §1.7-1.8 regions and containment, §1.11 danger).
//!
//! ```text
//!   site (100) -- the ground frame: z = 0 is ground level
//!   ├── yard (1)                         cat (10), courier (11), raccoon (12) start here
//!   └── house (2)
//!       ├── upstairs     (5)  z = +300   bedroom window (1011) -> yard: a way OUT, 4 m up
//!       ├── ground floor (4)  z =    0   front door, window, stairs up, cellar stairs
//!       └── cellar       (3)  z = -300   cellar entrance (bulkhead) to the yard
//!
//!   From the yard:  front door (1001) -> ground floor
//!                   window     (1003) -> ground floor   (sill 1 m up)
//!                   bulkhead   (1005) -> cellar
//! ```
//!
//! Every opening is a pair of one-way portals, one on each face, exactly as a world file
//! declares them. The engine has no movement system yet -- nothing in Physical Reality decides
//! to walk (that is a decision, §1.3) -- so [`Walk`] below stands in for whoever would: it
//! proposes a containment change through the ordinary tick, and the tests then ask the engine's
//! own queries what became of the walker.

use kernel::domain::Domain;
use kernel::events::ChronicleEntry;
use kernel::fact::{Cause, Fact, FactKey, FactType, Provenance, SystemId};
use kernel::identity::EntityId;
use kernel::proposal::{Change, Proposal};
use kernel::store::MemoryStore;
use kernel::system::{Cadence, CommittedView, System, TickContext};
use kernel::tick::run_tick;
use kernel::time::SimClock;
use kernel::value::Value;
use physical::regions::is_within;
use physical::schema::{
    CONTAINED_IN, HAS_PORTAL, LEADS_TO, PORTAL_DANGER, PORTAL_DANGER_OVERRIDE, POSITION_X,
    POSITION_Y, POSITION_Z,
};
use physical::space::{
    can_reach, distance, height_above_ground, portal_destination, portals_in, position_in, route,
};
use physical::{PhysicalConfig, PhysicalDomain};

// Places.
const SITE: u64 = 100;
const YARD: u64 = 1;
const HOUSE: u64 = 2;
const CELLAR: u64 = 3;
const GROUND: u64 = 4;
const UPSTAIRS: u64 = 5;
// Things that move.
const CAT: u64 = 10;
const COURIER: u64 = 11;
const RACCOON: u64 = 12;
// Openings: (outside face, inside face) pairs, plus the stairs and the upstairs window.
const FRONT_DOOR_OUT: u64 = 1001;
const FRONT_DOOR_IN: u64 = 1002;
const WINDOW_OUT: u64 = 1003;
const WINDOW_IN: u64 = 1004;
const BULKHEAD_OUT: u64 = 1005;
const BULKHEAD_IN: u64 = 1006;
const STAIRS_UP: u64 = 1007;
const STAIRS_DOWN: u64 = 1008;
const CELLAR_STAIRS_DOWN: u64 = 1009;
const CELLAR_STAIRS_UP: u64 = 1010;
const BEDROOM_WINDOW: u64 = 1011;

fn e(id: u64) -> EntityId {
    EntityId::from_raw(id)
}

fn seed(s: &mut MemoryStore, entity: u64, ft: FactType, v: Value) {
    s.seed(
        FactKey::new(e(entity), ft),
        Fact::new(
            v,
            Provenance::new(SystemId::new("worldgen"), 0, Cause::new("seed")),
        ),
    );
}

/// Place `entity` inside `container` at local (x, y, z) centimetres.
fn place(s: &mut MemoryStore, entity: u64, container: u64, x: i64, y: i64, z: i64) {
    seed(s, entity, CONTAINED_IN, Value::Entity(e(container)));
    seed(s, entity, POSITION_X, Value::Int(x));
    seed(s, entity, POSITION_Y, Value::Int(y));
    seed(s, entity, POSITION_Z, Value::Int(z));
}

/// A one-way portal located in `host` at (x, y, z) that leads to `dest`.
fn portal(s: &mut MemoryStore, id: u64, host: u64, dest: u64, x: i64, y: i64, z: i64) {
    place(s, id, host, x, y, z);
    seed(s, id, LEADS_TO, Value::Entity(e(dest)));
    seed(s, host, HAS_PORTAL, Value::Entity(e(id)));
}

/// The house in the module diagram, with the cat, courier, and raccoon out in the yard.
fn house() -> MemoryStore {
    let mut s = MemoryStore::new();
    place(&mut s, YARD, SITE, 0, 0, 0);
    place(&mut s, HOUSE, SITE, 1000, 0, 0);
    place(&mut s, CELLAR, HOUSE, 0, 0, -300);
    place(&mut s, GROUND, HOUSE, 0, 0, 0);
    place(&mut s, UPSTAIRS, HOUSE, 0, 0, 300);

    portal(&mut s, FRONT_DOOR_OUT, YARD, GROUND, 1000, 500, 0);
    portal(&mut s, FRONT_DOOR_IN, GROUND, YARD, 0, 500, 0);
    portal(&mut s, WINDOW_OUT, YARD, GROUND, 1000, 200, 100);
    portal(&mut s, WINDOW_IN, GROUND, YARD, 0, 200, 100);
    portal(&mut s, BULKHEAD_OUT, YARD, CELLAR, 1000, 800, 0);
    portal(&mut s, BULKHEAD_IN, CELLAR, YARD, 0, 800, 0);
    portal(&mut s, STAIRS_UP, GROUND, UPSTAIRS, 600, 100, 0);
    portal(&mut s, STAIRS_DOWN, UPSTAIRS, GROUND, 600, 100, 0);
    portal(&mut s, CELLAR_STAIRS_DOWN, GROUND, CELLAR, 300, 100, 0);
    portal(&mut s, CELLAR_STAIRS_UP, CELLAR, GROUND, 300, 100, 0);
    portal(&mut s, BEDROOM_WINDOW, UPSTAIRS, YARD, 0, 200, 100);

    // Stairs are pinned harmless, as a world file does in [portal_danger]. Derived danger is
    // height above ground (§1.11), which cannot tell a staircase from a sheer drop: the top of
    // these stairs is 3 m up and would otherwise rate like a 3 m fall.
    for stairs in [STAIRS_UP, STAIRS_DOWN, CELLAR_STAIRS_DOWN, CELLAR_STAIRS_UP] {
        seed(&mut s, stairs, PORTAL_DANGER_OVERRIDE, Value::Int(0));
    }

    place(&mut s, CAT, YARD, 500, 200, 0);
    place(&mut s, COURIER, YARD, 900, 500, 0);
    place(&mut s, RACCOON, YARD, 900, 800, 0);
    s
}

/// Stands in for a mover the engine does not have yet: each `(entity, portal)` pair steps
/// through that portal -- but only if the portal is in the room the entity is actually in.
/// The walker arrives on the floor just inside the far face of the opening, or at the room's
/// origin if the opening has none.
///
/// The model does not link an opening's two faces -- a door is two independent one-way portals
/// -- so "the far face" is found spatially: of the destination's portals that lead back to
/// where the walker came from, the one nearest the portal just used. A ground floor with both a
/// door and a window onto the yard has two candidates, and only distance tells them apart.
struct Walk(Vec<(u64, u64)>);

impl System for Walk {
    fn id(&self) -> SystemId {
        SystemId::new("test.walk")
    }
    fn reads(&self) -> &'static [FactType] {
        &[
            CONTAINED_IN,
            HAS_PORTAL,
            LEADS_TO,
            POSITION_X,
            POSITION_Y,
            POSITION_Z,
        ]
    }
    fn writes(&self) -> &'static [FactType] {
        &[CONTAINED_IN, POSITION_X, POSITION_Y, POSITION_Z]
    }
    fn cadence(&self) -> Cadence {
        Cadence::EveryTick
    }
    fn evaluate(&self, view: &dyn CommittedView, ctx: &TickContext) -> Vec<Proposal> {
        let mut out = Vec::new();
        for &(walker, through) in &self.0 {
            let (walker, through) = (e(walker), e(through));
            let Some(Value::Entity(here)) = view
                .read(FactKey::new(walker, CONTAINED_IN))
                .map(|f| f.value)
            else {
                continue;
            };
            // You can only use an opening that is in the room you are standing in.
            if !portals_in(view, here).contains(&through) {
                continue;
            }
            let Some(there) = portal_destination(view, through) else {
                continue;
            };
            let far_face = portals_in(view, there)
                .into_iter()
                .filter(|&p| portal_destination(view, p) == Some(here))
                .min_by_key(|&p| (distance(view, through, p).unwrap_or(i64::MAX), p));
            let coord = |axis| {
                far_face
                    .and_then(|p| view.read(FactKey::new(p, axis)))
                    .and_then(|f| f.value.as_int())
                    .unwrap_or(0)
            };
            let arrive = [
                (CONTAINED_IN, Value::Entity(there)),
                (POSITION_X, Value::Int(coord(POSITION_X))),
                (POSITION_Y, Value::Int(coord(POSITION_Y))),
                (POSITION_Z, Value::Int(0)), // on the floor, not on the sill
            ];
            for (fact_type, value) in arrive {
                out.push(Proposal::new(
                    self.id(),
                    FactKey::new(walker, fact_type),
                    ctx.basis_tick(),
                    Change::Set(value),
                    Cause::new("step_through"),
                ));
            }
        }
        out
    }
}

/// Run tick `n`: the physical domain's own systems (which keep portal danger current) plus
/// the given steps.
fn walk(s: &mut MemoryStore, n: u64, steps: &[(u64, u64)]) {
    let domain = PhysicalDomain::new(PhysicalConfig {
        clock: SimClock::new(3_600_000),
        day_length_seconds: 86_400,
        environment_step_seconds: 3600,
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
    });
    let domains: [&dyn Domain; 1] = [&domain];
    let mut systems = domain.systems();
    systems.push(Box::new(Walk(steps.to_vec())));
    let mut chronicle: Vec<ChronicleEntry> = Vec::new();
    run_tick(s, &domains, &systems, n, 0, &mut chronicle).expect("tick commits");
}

fn room_of(s: &MemoryStore, entity: u64) -> u64 {
    match s
        .read(FactKey::new(e(entity), CONTAINED_IN))
        .map(|f| f.value)
    {
        Some(Value::Entity(room)) => room.raw(),
        other => panic!("{entity} has no room: {other:?}"),
    }
}

#[test]
fn something_can_enter_through_the_door_the_window_and_the_cellar_entrance() {
    let mut s = house();
    // The yard's ways in are exactly the three openings, each leading somewhere in the house.
    assert_eq!(
        portals_in(&s, e(YARD)),
        vec![e(FRONT_DOOR_OUT), e(WINDOW_OUT), e(BULKHEAD_OUT)]
    );
    for opening in [FRONT_DOOR_OUT, WINDOW_OUT, BULKHEAD_OUT] {
        let inside = portal_destination(&s, e(opening)).unwrap();
        assert!(
            is_within(&s, inside, e(HOUSE)),
            "{opening} leads into the house"
        );
    }

    // Nobody is inside yet.
    for who in [CAT, COURIER, RACCOON] {
        assert!(!is_within(&s, e(who), e(HOUSE)));
    }

    // One tick: the cat through the window, the courier through the door, the raccoon down
    // the cellar entrance.
    walk(
        &mut s,
        1,
        &[
            (CAT, WINDOW_OUT),
            (COURIER, FRONT_DOOR_OUT),
            (RACCOON, BULKHEAD_OUT),
        ],
    );

    // All three are now recognised as inside the building -- and in which room.
    for who in [CAT, COURIER, RACCOON] {
        assert!(is_within(&s, e(who), e(HOUSE)), "{who} is inside");
    }
    assert_eq!(room_of(&s, CAT), GROUND);
    assert_eq!(room_of(&s, COURIER), GROUND);
    assert_eq!(room_of(&s, RACCOON), CELLAR);

    // Each landed just inside the opening it used: the cat under the window, the courier on
    // the doormat, the raccoon at the foot of the bulkhead -- in house coordinates.
    assert_eq!(position_in(&s, e(CAT), e(HOUSE)), Some([0, 200, 0]));
    assert_eq!(position_in(&s, e(COURIER), e(HOUSE)), Some([0, 500, 0]));
    assert_eq!(position_in(&s, e(RACCOON), e(HOUSE)), Some([0, 800, -300]));
}

#[test]
fn an_opening_only_works_from_the_room_it_is_in() {
    let mut s = house();
    // From the yard the cat cannot take the stairs, and cannot climb in the bedroom window:
    // that window opens from the bedroom onto the yard, not the other way.
    walk(&mut s, 1, &[(CAT, STAIRS_UP), (CAT, BEDROOM_WINDOW)]);
    assert_eq!(room_of(&s, CAT), YARD);
    assert!(!is_within(&s, e(CAT), e(HOUSE)));
    // Upstairs is still reachable from the yard -- just not directly.
    assert!(can_reach(&s, e(YARD), e(UPSTAIRS)));
    assert_eq!(
        route(&s, e(YARD), e(UPSTAIRS)),
        Some(vec![e(FRONT_DOOR_OUT), e(STAIRS_UP)])
    );
}

#[test]
fn someone_can_go_upstairs_and_the_engine_knows_how_high_they_are() {
    let mut s = house();
    assert_eq!(
        height_above_ground(&s, e(COURIER)),
        0,
        "standing in the yard"
    );

    // Follow the engine's own route from the yard to upstairs, one opening per tick.
    let steps = route(&s, e(YARD), e(UPSTAIRS)).unwrap();
    for (n, step) in steps.iter().enumerate() {
        walk(&mut s, n as u64 + 1, &[(COURIER, step.raw())]);
    }

    assert_eq!(room_of(&s, COURIER), UPSTAIRS);
    assert!(
        is_within(&s, e(COURIER), e(HOUSE)),
        "still inside the building"
    );
    // Upstairs is stacked 3 m up the house, so the courier is 3 m above the ground...
    assert_eq!(height_above_ground(&s, e(COURIER)), 300);
    // ...standing at the top of the stairs, which in house coordinates is (600, 100, 300).
    assert_eq!(position_in(&s, e(COURIER), e(HOUSE)), Some([600, 100, 300]));

    // And the raccoon, once in the cellar, is 3 m *below* ground.
    walk(&mut s, 3, &[(RACCOON, BULKHEAD_OUT)]);
    assert_eq!(height_above_ground(&s, e(RACCOON)), -300);
}

#[test]
fn leaving_the_building_is_recognised_too() {
    let mut s = house();
    walk(&mut s, 1, &[(COURIER, FRONT_DOOR_OUT)]);
    assert!(is_within(&s, e(COURIER), e(HOUSE)));
    walk(&mut s, 2, &[(COURIER, FRONT_DOOR_IN)]);
    assert!(!is_within(&s, e(COURIER), e(HOUSE)));
    assert_eq!(room_of(&s, COURIER), YARD);
    // Back on the yard side of the front door, in yard coordinates.
    assert_eq!(position_in(&s, e(COURIER), e(YARD)), Some([1000, 500, 0]));
}

#[test]
fn the_quickest_way_down_from_upstairs_is_the_dangerous_one() {
    let mut s = house();
    walk(&mut s, 1, &[]); // one tick so the danger system writes each portal's danger

    // Shortest route from the bedroom to the yard is the window -- one step instead of two...
    assert_eq!(
        route(&s, e(UPSTAIRS), e(YARD)),
        Some(vec![e(BEDROOM_WINDOW)])
    );
    // ...but its sill is 4 m above the ground (3 m floor + 1 m sill), and the engine rates
    // the drop accordingly, while the stairs and the front door are harmless.
    assert_eq!(height_above_ground(&s, e(BEDROOM_WINDOW)), 400);
    let danger = |p: u64| {
        s.read(FactKey::new(e(p), PORTAL_DANGER))
            .and_then(|f| f.value.as_int())
            .unwrap()
    };
    assert_eq!(danger(BEDROOM_WINDOW), 6000); // 4 m x 1500 per metre
    assert_eq!(danger(STAIRS_DOWN), 0);
    assert_eq!(danger(FRONT_DOOR_IN), 0);
}

#[test]
fn a_lone_entity_is_its_own_ground() {
    // No container: the entity is the ground frame, so its height is 0 by definition.
    let s = house();
    assert_eq!(height_above_ground(&s, e(SITE)), 0);
    assert_eq!(route(&s, e(YARD), e(YARD)), Some(vec![]));
}
