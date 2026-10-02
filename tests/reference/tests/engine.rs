//! The engine's own guarantees, held over Ashford (Vol. V Ch. 3–4, Ch. 7; Vol. IV Ch. 1–2, 7;
//! Amendments A-2, A-5): the same seed replays the same city; the spatial index is invisible —
//! a city run without it is the same city, and every question it answers it answers exactly as
//! a scan would; the index is no back door around a system's declared reads; a package that
//! breaks the rules is refused with every problem named; and a tick that would break an
//! invariant commits nothing.

use kernel::domain::{Domain, ValidationError};
use kernel::events::ChronicleEntry;
use kernel::fact::{Cause, Fact, FactKey, FactType, Provenance, SystemId};
use kernel::identity::EntityId;
use kernel::proposal::{Change, Proposal};
use kernel::spatial::Aabb;
use kernel::store::{CommitBatch, MemoryStore, RealityStore, Resolution};
use kernel::system::{Cadence, CommittedView, System, TickContext};
use kernel::tick::{run_tick, TickError};
use kernel::time::SimClock;
use kernel::value::Value;
use living::LivingDomain;
use packages::{engine_version, load, parse_world, Layer, LoadError, Problem, Version};
use physical::motion::depart;
use physical::nearby::{contents, in_box, nearest, within};
use physical::regions::regions_of;
use physical::schema::{CONTAINED_IN, IN_REGION, POSITION, TEMPERATURE, TRAVEL_SPEED, TRAVEL_TO};
use physical::PhysicalDomain;
use reference::id::*;
use reference::{e, package, City};

/// Sends a few of Ashford's people on their way, so a run has movement in it.
fn set_off(city: &mut City) {
    city.go(BOB, BEDROOM, 140)
        .go(CAT, KITCHEN, 300)
        .go(VILLAGER, LOFT, 120)
        .go(HIKER, CAIRN, 200)
        .go(STEWARD, UNDERCROFT, 140);
}

#[test]
fn the_same_seed_replays_the_same_city() {
    let run = || {
        let mut city = City::new();
        set_off(&mut city);
        city.run(300);
        (city.store().state_hash(), city.chronicle)
    };
    let (a, b) = (run(), run());
    assert_eq!(a.0, b.0);
    assert!(a.1 == b.1, "the chronicles match entry for entry");
}

#[test]
fn another_seed_brings_other_weather_but_the_same_footsteps() {
    let run = |seed| {
        let mut city = City::new();
        city.seed = seed;
        set_off(&mut city);
        city.run(300);
        city
    };
    let (one, two) = (run(1), run(2));
    assert_ne!(one.store().state_hash(), two.store().state_hash());
    assert_ne!(one.read(VALE, TEMPERATURE), two.read(VALE, TEMPERATURE));
    // Walking is not left to chance.
    for who in [BOB, CAT, VILLAGER, HIKER, STEWARD] {
        assert_eq!(one.read(who, POSITION), two.read(who, POSITION), "{who}");
        assert_eq!(one.room_of(who), two.room_of(who), "{who}");
    }
}

fn seeded(fact: FactType, who: u64, value: Value) -> (FactKey, Fact) {
    (
        FactKey::new(e(who), fact),
        Fact::new(
            value,
            Provenance::new(SystemId::new("test.setup"), 0, Cause::new("ordered")),
        ),
    )
}

#[test]
fn a_city_run_without_its_index_is_the_same_city() {
    // Ashford as loaded (indexed), and the very same facts in a store with no index: every
    // spatial question then takes the scanning road. Both run the world's own systems for a
    // minute, with people on the move; tick for tick the two realities are identical.
    let pkg = package();
    let mut indexed = City::new().store().clone();
    for (key, fact) in [
        seeded(TRAVEL_TO, BOB, Value::Entity(e(BEDROOM))),
        seeded(TRAVEL_SPEED, BOB, Value::Int(140)),
        seeded(TRAVEL_TO, CAT, Value::Entity(e(KITCHEN))),
        seeded(TRAVEL_SPEED, CAT, Value::Int(300)),
        seeded(TRAVEL_TO, HIKER, Value::Entity(e(CAIRN))),
        seeded(TRAVEL_SPEED, HIKER, Value::Int(200)),
    ] {
        indexed.seed(key, fact);
    }
    let mut scanning = MemoryStore::new();
    for id in 0..10_000 {
        for (key, fact) in indexed.facts_of(e(id)) {
            scanning.seed(key, fact);
        }
    }
    assert!(indexed.spatial().is_some() && scanning.spatial().is_none());
    assert_eq!(indexed.state_hash(), scanning.state_hash());

    let physical = PhysicalDomain::new(packages::physical_config(&pkg));
    let living = LivingDomain::new(packages::living_config(&pkg).unwrap());
    let domains: [&dyn Domain; 2] = [&physical, &living];
    let systems: Vec<Box<dyn System>> = physical
        .systems()
        .into_iter()
        .chain(living.systems())
        .collect();
    let (mut ca, mut cb): (Vec<ChronicleEntry>, Vec<ChronicleEntry>) = (Vec::new(), Vec::new());
    for t in 1..=60 {
        run_tick(&mut indexed, &domains, &systems, t, 1, &mut ca).unwrap();
        run_tick(&mut scanning, &domains, &systems, t, 1, &mut cb).unwrap();
        assert_eq!(indexed.state_hash(), scanning.state_hash(), "tick {t}");
    }
    assert!(ca == cb);
    assert_ne!(
        indexed.read(FactKey::new(e(BOB), POSITION)),
        City::new().store().read(FactKey::new(e(BOB), POSITION)),
        "Bob moved"
    );
}

/// The same committed state with its index hidden: every query takes the scanning road.
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
}

/// A small deterministic generator (xorshift64*).
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
    fn pick(&mut self, from: &[u64]) -> u64 {
        from[(self.next() % from.len() as u64) as usize]
    }
}

/// Every place in Ashford something can be in — including the cart and the Heron, which move.
const FRAMES: &[u64] = &[
    REACH,
    VALE,
    HIGHMOOR,
    SOUTHFEN,
    ASHFORD,
    OLD_TOWN,
    HILL,
    HARBOUR,
    MILLSIDE,
    YARD,
    HOUSE,
    CELLAR,
    KITCHEN,
    BEDROOM,
    GROUNDS,
    MANOR,
    HALL,
    UNDERCROFT,
    GALLERY,
    VAULT,
    COTTAGE,
    COTTAGE_KITCHEN,
    LOFT,
    HERON,
    CART,
    FARM,
    FARMHOUSE,
    FAR_FIELD,
    WOOD,
    MILL,
];

/// Everyone and everything that gets picked up and put down.
const THINGS: &[u64] = &[
    ALICE,
    BOB,
    CAROL,
    DAVE,
    ERIN,
    CAT,
    COURIER,
    RACCOON,
    FINN,
    GWEN,
    STEWARD,
    WARDROBE,
    RIDER,
    HAL,
    IDA,
    CAPTAIN,
    BOSUN,
    GUARD,
    FARMER,
    FOX,
    MILLER,
    VILLAGER,
    JORY,
    KIT,
    LENA,
    HIKER,
    NELL,
    STONE,
    LAMP,
    TABLE,
    MARKER,
    COTTAGE_TABLE,
];

/// Ask everything both ways and require identical answers.
fn assert_conforms(store: &MemoryStore, g: &mut Gen) {
    let scan = Scanning(store);
    for &f in FRAMES {
        assert_eq!(
            contents(store, e(f)),
            contents(&scan, e(f)),
            "contents of {f}"
        );
        for _ in 0..3 {
            let a = [
                g.range(-12_000, 12_000),
                g.range(-12_000, 12_000),
                g.range(-800, 2_000),
            ];
            let b = [
                g.range(-12_000, 12_000),
                g.range(-12_000, 12_000),
                g.range(-800, 2_000),
            ];
            let bx = Aabb::new(a, b);
            assert_eq!(
                in_box(store, e(f), &bx),
                in_box(&scan, e(f), &bx),
                "in_box {f} {bx:?}"
            );
        }
    }
    for &center in THINGS {
        for radius in [0, 150, 1_000, 6_000, 600_000] {
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

/// Commits, as tick `tick`, a shuffle of eight things: some carried to another place, some sent
/// walking (a motion segment), some taken out of the world. A test of the *store* — that its
/// index stays exact under any committed change — so it commits directly, as the tick's commit
/// stage does; no system of Ashford's scatters things at random, nor may any but Physical's.
fn shuffle(store: &mut MemoryStore, tick: u64, g: &mut Gen) {
    let prov = Provenance::new(SystemId::new("test.shuffle"), tick, Cause::new("shuffle"));
    let clock = SimClock::new(1_000);
    let mut chosen = std::collections::BTreeSet::new();
    while chosen.len() < 8 {
        chosen.insert(g.pick(THINGS));
    }
    let mut changes: Vec<(FactKey, Change)> = Vec::new();
    for thing in chosen {
        let key = |ft| FactKey::new(e(thing), ft);
        match g.next() % 5 {
            0 | 1 => {
                let to = g.pick(&FRAMES[9..]);
                changes.push((key(CONTAINED_IN), Change::Set(Value::Entity(e(to)))));
            }
            2 => changes.push((key(CONTAINED_IN), Change::Tombstone)),
            _ => {
                let here = physical::space::local_position(&*store, e(thing));
                let to = [
                    here[0] + g.range(-2_000, 2_000),
                    here[1] + g.range(-2_000, 2_000),
                    here[2],
                ];
                changes.extend(depart(&*store, e(thing), to, 140, &clock));
            }
        }
    }
    changes.sort_by_key(|(k, _)| *k);
    changes.dedup_by_key(|(k, _)| *k);
    let mut batch = CommitBatch::new(tick);
    for (key, change) in changes {
        batch.resolutions.push(match change {
            Change::Set(v) => Resolution::One {
                key,
                fact: Fact::new(v, prov),
            },
            _ => Resolution::Clear { key },
        });
    }
    store.apply(batch);
}

#[test]
fn the_index_answers_exactly_what_a_scan_would_as_ashford_is_turned_over() {
    let mut g = Gen(0xa5f0_7d);
    let mut store = City::new().store().clone();
    assert!(store.spatial().is_some(), "Ashford is indexed");
    assert_conforms(&store, &mut g);
    for tick in 1..=8 {
        shuffle(&mut store, tick, &mut g);
        assert_conforms(&store, &mut g);
    }
}

/// Asks who is near Alice while declaring only containment — not the positions the index also
/// mirrors.
struct HalfDeclared;
impl System for HalfDeclared {
    fn id(&self) -> SystemId {
        SystemId::new("probe.half_declared")
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
        let _ = within(view, e(ALICE), 1_000);
        Vec::new()
    }
}

#[test]
fn the_index_is_no_back_door_around_a_systems_reads() {
    let mut city = City::new();
    city.attach(Box::new(HalfDeclared));
    let err = city
        .try_run()
        .expect_err("an undeclared read through the index");
    assert!(
        matches!(err, TickError::UndeclaredRead { fact_type, .. } if fact_type == POSITION),
        "got {err:?}"
    );
}

#[test]
fn an_index_built_as_ashford_loads_equals_one_built_afterwards() {
    // The loader installs the index first and seeds into it, placing each thing as it arrives;
    // installing it over the finished city builds it in one pass. Both must index one city.
    let loaded = City::new().store().clone();
    let mut after = MemoryStore::new();
    for id in 0..10_000 {
        for (key, fact) in loaded.facts_of(e(id)) {
            after.seed(key, fact);
        }
    }
    let domain = PhysicalDomain::new(packages::physical_config(&package()));
    after.install_spatial_index(domain.spatial_projector().expect("physical owns space"));
    let (a, b) = (loaded.spatial().unwrap(), after.spatial().unwrap());
    for id in 0..10_000 {
        assert_eq!(a.placement(e(id)), b.placement(e(id)), "placement of {id}");
        assert_eq!(a.children(e(id)), b.children(e(id)), "children of {id}");
        assert_eq!(a.subframes(e(id)), b.subframes(e(id)), "subframes of {id}");
    }
}

/// Asks which regions Alice is in while declaring only containment — not region membership.
struct HalfDeclaredRegions;
impl System for HalfDeclaredRegions {
    fn id(&self) -> SystemId {
        SystemId::new("probe.half_declared_regions")
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
        let _ = regions_of(view, e(ALICE));
        Vec::new()
    }
}

#[test]
fn asking_about_regions_means_declaring_both_links() {
    let mut city = City::new();
    city.attach(Box::new(HalfDeclaredRegions));
    let err = city
        .try_run()
        .expect_err("an undeclared read of membership");
    assert!(
        matches!(err, TickError::UndeclaredRead { fact_type, .. } if fact_type == IN_REGION),
        "got {err:?}"
    );
}

/// Proposes a cold beyond cold over the Vale.
struct ColdSnap;
impl System for ColdSnap {
    fn id(&self) -> SystemId {
        SystemId::new("rogue.cold_snap")
    }
    fn reads(&self) -> &'static [FactType] {
        &[]
    }
    fn writes(&self) -> &'static [FactType] {
        &[TEMPERATURE]
    }
    fn cadence(&self) -> Cadence {
        Cadence::EveryTick
    }
    fn evaluate(&self, _view: &dyn CommittedView, ctx: &TickContext) -> Vec<Proposal> {
        vec![Proposal::new(
            self.id(),
            FactKey::new(e(VALE), TEMPERATURE),
            ctx.basis_tick(),
            Change::Set(Value::Int(-30_000)),
            Cause::new("cold_snap"),
        )]
    }
}

#[test]
fn a_tick_that_would_break_physics_commits_nothing() {
    let mut city = City::new();
    city.go(BOB, BEDROOM, 140);
    city.attach(Box::new(ColdSnap));
    let before = city.store().state_hash();
    let err = city.try_run().expect_err("-300 °C is below absolute zero");
    assert_eq!(
        err,
        TickError::Validate(ValidationError::new(
            "temperature resolved below absolute zero"
        ))
    );
    assert_eq!(
        city.store().state_hash(),
        before,
        "not even Bob's first step"
    );
    assert_eq!(city.tick, 0);
}

// ---- Packages the loader refuses (Vol. IV Ch. 1–2, Ch. 7).

fn refusal(text: &str) -> LoadError {
    load(&parse_world(text).expect("parses"), engine_version()).expect_err("refused")
}

/// Ashford with `from` replaced by `to`, exactly once.
fn edited(from: &str, to: &str) -> String {
    let text = reference::ASHFORD;
    assert_eq!(
        text.matches(from).count(),
        1,
        "{from:?} is unique in Ashford"
    );
    text.replacen(from, to, 1)
}

fn problems(text: &str) -> Vec<Problem> {
    match refusal(text) {
        LoadError::Invalid(problems) => problems,
        other => panic!("refused, but not as invalid: {other:?}"),
    }
}

fn problem(layer: Layer, subject: u64, rule: &str) -> Problem {
    Problem {
        layer,
        subject,
        rule: rule.into(),
    }
}

#[test]
fn ashford_itself_is_valid() {
    assert_eq!(packages::validate(&package()), Vec::new());
}

#[test]
fn an_engine_outside_the_declared_range_is_refused() {
    let err = load(&package(), Version::new(2, 0, 0)).unwrap_err();
    assert!(matches!(err, LoadError::EngineMismatch { .. }), "{err:?}");
}

#[test]
fn a_world_without_physical_reality_is_refused() {
    let err = refusal(&edited("domains = physical, living", "domains = living"));
    assert!(matches!(err, LoadError::PhysicalNotSelected), "{err:?}");
}

#[test]
fn living_without_its_rules_is_refused() {
    let mut pkg = package();
    pkg.living_rules = None;
    let err = load(&pkg, engine_version()).unwrap_err();
    assert!(matches!(err, LoadError::LivingRulesMissing), "{err:?}");
}

#[test]
fn a_missing_rule_is_named_and_never_defaulted() {
    let text = edited("temperature_variability_centi_c = 300\n", "");
    let err = parse_world(&text).expect_err("a missing rule is an error");
    assert!(
        err.reason.contains("temperature_variability_centi_c"),
        "{}",
        err.reason
    );
}

#[test]
fn a_container_that_does_not_exist_is_named() {
    let text = edited("120 = 103               # the kitchen table", "120 = 9999");
    assert_eq!(
        problems(&text),
        vec![problem(
            Layer::Reference,
            TABLE,
            "its container 9999 is never declared"
        )]
    );
}

#[test]
fn a_material_that_does_not_exist_is_named() {
    let text = edited("103 = 700               # Alice's", "103 = 799  #");
    assert_eq!(
        problems(&text),
        vec![problem(
            Layer::Reference,
            KITCHEN,
            "material 799 is not declared in [materials]"
        )]
    );
}

#[test]
fn a_door_given_a_second_position_is_named() {
    // The front door's place is declared with the door, in [portals]; declaring it again in
    // [positions] is ambiguous.
    let text = edited("[positions]\n", "[positions]\n1001 = 0, 0, 0\n");
    assert_eq!(
        problems(&text),
        vec![problem(
            Layer::Reference,
            DOOR_OUT,
            "its position is declared twice (in [positions] and in [portals])"
        )]
    );
}

#[test]
fn only_a_portal_can_be_shut() {
    let text = edited("120 = solid             # tables", "120 = solid, closed");
    assert_eq!(
        problems(&text),
        vec![problem(
            Layer::Coherence,
            TABLE,
            "only a portal can be closed"
        )]
    );
}

#[test]
fn a_city_inside_its_own_district_is_named() {
    // Ashford put inside Old Town, which is inside Ashford.
    let text = edited("10 = 1                  # Ashford, in the Vale", "10 = 11");
    assert_eq!(
        problems(&text),
        vec![problem(
            Layer::World,
            ASHFORD,
            "containment loops back on itself through [10, 11]"
        )]
    );
}

#[test]
fn every_problem_is_named_at_once() {
    let text = edited("120 = 103               # the kitchen table", "120 = 9999");
    let text = text.replacen("120 = solid             # tables", "120 = solid, closed", 1);
    let text = text.replacen(
        "10 = 1                  # Ashford, in the Vale",
        "10 = 11",
        1,
    );
    let found = problems(&text);
    assert_eq!(found.len(), 3, "{found:#?}");
}
