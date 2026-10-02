//! Overlapping regions (Vol. III Ch. 1 §1.7): a location belongs to many regions at once, by
//! containment and by membership, and membership is state that systems change through the tick.
//!
//! The package-level scenario (a farmstead laid under a watershed, a climate zone, a territory,
//! and a discontiguous frost hollow) lives in `services/packages/tests/farmstead.rs`; these
//! tests pin the edges the domain itself must hold: cycles terminate, membership is dynamic,
//! a member must be an entity, and a system that asks about regions must declare what it reads.

use kernel::domain::Domain;
use kernel::events::ChronicleEntry;
use kernel::fact::{Cause, Fact, FactKey, FactType, Provenance, SystemId};
use kernel::identity::EntityId;
use kernel::proposal::{Change, Proposal};
use kernel::store::MemoryStore;
use kernel::system::{Cadence, CommittedView, System, TickContext};
use kernel::tick::{run_tick, TickError};
use kernel::time::SimClock;
use kernel::value::Value;
use physical::regions::{is_within, members_of, memberships, overlaps, regions_of};
use physical::schema::{CONTAINED_IN, IN_REGION};
use physical::{PhysicalConfig, PhysicalDomain};
use std::collections::BTreeSet;

fn e(id: u64) -> EntityId {
    EntityId::from_raw(id)
}

fn link(s: &mut MemoryStore, entity: u64, ft: FactType, target: u64) {
    s.seed(
        FactKey::new(e(entity), ft),
        Fact::new(
            Value::Entity(e(target)),
            Provenance::new(SystemId::new("worldgen"), 0, Cause::new("seed")),
        ),
    );
}

fn ids(set: BTreeSet<EntityId>) -> Vec<u64> {
    set.into_iter().map(|e| e.raw()).collect()
}

fn config() -> PhysicalConfig {
    PhysicalConfig {
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
        gravity_cm_s2: 981,
        step_height_cm: 40,
        max_slope_percent: 100,
        nav_cell_cm: 50,
    }
}

/// Run one tick of `systems` with Physical Reality as the owner of every fact they touch.
fn tick(store: &mut MemoryStore, n: u64, systems: Vec<Box<dyn System>>) -> Result<(), TickError> {
    let domain = PhysicalDomain::new(config());
    let domains: [&dyn Domain; 1] = [&domain];
    let mut chronicle: Vec<ChronicleEntry> = Vec::new();
    run_tick(store, &domains, &systems, n, 0, &mut chronicle)
}

#[test]
fn both_links_compose_into_one_answer() {
    // Hierarchy: 10 (a deer) in 2 (a meadow) in 1 (a valley).
    // Memberships: the meadow lies in 900 (a territory); the valley in 901 (a climate zone).
    let mut s = MemoryStore::new();
    link(&mut s, 10, CONTAINED_IN, 2);
    link(&mut s, 2, CONTAINED_IN, 1);
    link(&mut s, 2, IN_REGION, 900);
    link(&mut s, 1, IN_REGION, 901);

    // Direct memberships are only what the entity names itself.
    assert!(memberships(&s, e(10)).is_empty());
    assert_eq!(memberships(&s, e(2)), vec![e(900)]);

    // The full answer mixes both links at every level: the deer inherits its meadow's
    // territory and its valley's climate zone.
    assert_eq!(ids(regions_of(&s, e(10))), vec![1, 2, 900, 901]);
    assert!(is_within(&s, e(10), e(900)));
    assert!(!is_within(&s, e(10), e(10)), "nothing lies within itself");

    // And downward: the climate zone takes in the valley and everything under it.
    assert_eq!(ids(members_of(&s, e(901))), vec![1, 2, 10]);
}

#[test]
fn a_membership_cycle_terminates_and_never_reports_self() {
    // Two names for one extent: 900 declared within 901 and 901 within 900. A location in
    // either lies in both; neither lies within itself.
    let mut s = MemoryStore::new();
    link(&mut s, 900, IN_REGION, 901);
    link(&mut s, 901, IN_REGION, 900);
    link(&mut s, 5, IN_REGION, 900);

    assert_eq!(ids(regions_of(&s, e(900))), vec![901]);
    assert_eq!(ids(regions_of(&s, e(5))), vec![900, 901]);
    assert_eq!(ids(members_of(&s, e(900))), vec![5, 901]);
    assert!(overlaps(&s, e(900), e(901)));
}

#[test]
fn regions_with_no_shared_place_do_not_overlap() {
    // Regions are semantic, not geometric (§1.7): with no recorded place in both, two regions
    // do not overlap, whatever their shapes might do on a map.
    let mut s = MemoryStore::new();
    link(&mut s, 1, IN_REGION, 900);
    link(&mut s, 2, IN_REGION, 901);
    assert!(!overlaps(&s, e(900), e(901)));
    assert!(overlaps(&s, e(900), e(900)), "a region overlaps itself");

    // One shared place is enough.
    link(&mut s, 3, IN_REGION, 900);
    link(&mut s, 3, IN_REGION, 901);
    assert!(overlaps(&s, e(900), e(901)));
}

/// Moves a herd (entity 50) into a territory (900) on tick 1 and out again on tick 2 — the
/// shape of a migration proposal a higher domain would make (§1.8, Dynamic Containment).
struct Migrate;
impl System for Migrate {
    fn id(&self) -> SystemId {
        SystemId::new("test.migrate")
    }
    fn reads(&self) -> &'static [FactType] {
        &[]
    }
    fn writes(&self) -> &'static [FactType] {
        &[IN_REGION]
    }
    fn cadence(&self) -> Cadence {
        Cadence::EveryTick
    }
    fn evaluate(&self, _view: &dyn CommittedView, ctx: &TickContext) -> Vec<Proposal> {
        let territory = Value::Entity(e(900));
        let change = if ctx.tick() == 1 {
            Change::Add(territory)
        } else {
            Change::Remove(territory)
        };
        vec![Proposal::new(
            self.id(),
            FactKey::new(e(50), IN_REGION),
            ctx.basis_tick(),
            change,
            Cause::new("migration"),
        )]
    }
}

#[test]
fn membership_is_state_that_changes_through_the_tick() {
    let mut s = MemoryStore::new();
    link(&mut s, 50, IN_REGION, 800); // the herd's home range, untouched throughout
    assert!(!is_within(&s, e(50), e(900)));

    tick(&mut s, 1, vec![Box::new(Migrate)]).expect("entering commits");
    assert!(is_within(&s, e(50), e(900)));
    assert!(
        is_within(&s, e(50), e(800)),
        "entering one region leaves the others"
    );

    tick(&mut s, 2, vec![Box::new(Migrate)]).expect("leaving commits");
    assert!(!is_within(&s, e(50), e(900)));
    assert!(is_within(&s, e(50), e(800)));
}

/// Proposes a number, not a region, as a membership.
struct ScalarMember;
impl System for ScalarMember {
    fn id(&self) -> SystemId {
        SystemId::new("test.scalar_member")
    }
    fn reads(&self) -> &'static [FactType] {
        &[]
    }
    fn writes(&self) -> &'static [FactType] {
        &[IN_REGION]
    }
    fn cadence(&self) -> Cadence {
        Cadence::EveryTick
    }
    fn evaluate(&self, _view: &dyn CommittedView, ctx: &TickContext) -> Vec<Proposal> {
        vec![Proposal::new(
            self.id(),
            FactKey::new(e(50), IN_REGION),
            ctx.basis_tick(),
            Change::Add(Value::Int(900)),
            Cause::new("bad_membership"),
        )]
    }
}

#[test]
fn a_membership_must_name_an_entity() {
    let mut s = MemoryStore::new();
    let err = tick(&mut s, 1, vec![Box::new(ScalarMember)]).expect_err("validate refuses");
    assert!(matches!(err, TickError::Validate(_)), "got {err:?}");
    assert!(
        memberships(&s, e(50)).is_empty(),
        "a failed tick commits nothing"
    );
}

/// Asks a region question while declaring only the containment link — a mis-declared read set.
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
        let _ = regions_of(view, e(10));
        Vec::new()
    }
}

#[test]
fn asking_about_regions_requires_declaring_both_links() {
    // A system that saw only half the links would get a confidently wrong answer; the kernel
    // refuses the tick instead (Vol. V Ch. 3 §3.5).
    let mut s = MemoryStore::new();
    link(&mut s, 10, CONTAINED_IN, 2);
    link(&mut s, 10, IN_REGION, 900);
    let err = tick(&mut s, 1, vec![Box::new(HalfDeclared)]).expect_err("hermeticity");
    assert!(
        matches!(err, TickError::UndeclaredRead { fact_type, .. } if fact_type == IN_REGION),
        "got {err:?}"
    );
}
