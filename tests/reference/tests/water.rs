//! Water (Vol. III Ch. 1 §1.10–1.11; Amendment A-39): the sea at the harbour rises and falls with
//! the tide, higher at springs than at neaps; rain on wet land swells the Brack over its banks
//! and floods the far field, and dry land drinks it instead; a flood cuts off what lies beyond
//! it; a pond too deep to wade bears a walker once it freezes hard.

use kernel::fact::{Cause, FactKey, FactType, SystemId};
use kernel::proposal::{Change, Proposal};
use kernel::system::{Cadence, CommittedView, System, TickContext};
use packages::WorldPackage;
use physical::schema::{FLOOD_DEPTH, GROUND_WATER, ICE, TRAVEL_BLOCKED, WATER_LEVEL};
use reference::id::*;
use reference::{e, with_tick_seconds, without_minds, City};

const VALE: u64 = 1;
const SEA: u64 = 506;
const BRACK: u64 = 605;
const FAR_FIELD: u64 = 602;
const POND: u64 = 307;

/// Ashford without minds in steady weather, the Vale's air at `humidity`, at ticks of `seconds`.
fn ashford(seconds: u64, humidity: i64) -> WorldPackage {
    let mut pkg = without_minds(with_tick_seconds(seconds));
    pkg.physical_rules.temperature_variability_centi_c = 0;
    pkg.physical_rules.humidity_variability = 0;
    pkg.physical_rules.pressure_variability = 0;
    pkg.physical_rules.humidity_baseline = humidity;
    pkg
}

/// The Vale's ground soaked through on the first tick.
struct Soak;

const SOAK_WRITES: &[FactType] = &[GROUND_WATER];

impl System for Soak {
    fn id(&self) -> SystemId {
        SystemId::new("test.soak")
    }
    fn reads(&self) -> &'static [FactType] {
        &[]
    }
    fn writes(&self) -> &'static [FactType] {
        SOAK_WRITES
    }
    fn cadence(&self) -> Cadence {
        Cadence::EveryTick
    }
    fn evaluate(&self, _view: &dyn CommittedView, ctx: &TickContext) -> Vec<Proposal> {
        if ctx.tick() != 1 {
            return Vec::new();
        }
        vec![Proposal::new(
            self.id(),
            FactKey::new(e(VALE), GROUND_WATER),
            ctx.basis_tick(),
            Change::Delta(10_000),
            Cause::new("soaked"),
        )]
    }
}

fn int(city: &City, id: u64, fact: FactType) -> i64 {
    city.int(id, fact).unwrap_or(0)
}

/// The sea's level every ten minutes for a day, from a world whose moon begins at `phase`.
fn a_day_of_sea(phase: i64) -> Vec<i64> {
    let mut pkg = ashford(600, 6_000);
    pkg.physical_rules.moon_phase_at_start = phase;
    let mut city = City::from(pkg);
    (0..144)
        .map(|_| {
            city.run(1);
            int(&city, SEA, WATER_LEVEL)
        })
        .collect()
}

#[test]
fn the_sea_rises_and_falls_with_the_tide() {
    // New moon: a spring tide, three metres from low water to high about the sea's five-metre
    // mean, high again 12 h 25 min after the first.
    let springs = a_day_of_sea(0);
    let range = |v: &[i64]| v.iter().max().unwrap() - v.iter().min().unwrap();
    assert!(
        (285..=300).contains(&range(&springs)),
        "{}",
        range(&springs)
    );
    let mean = springs.iter().sum::<i64>() / springs.len() as i64;
    assert!((480..=520).contains(&mean), "{mean}");
    let second_high = (36..108).max_by_key(|t| springs[*t]).unwrap() + 1;
    assert!(
        (73..=76).contains(&second_high),
        "high water at tick {second_high}"
    );
    // At the first quarter, a neap tide: half the range.
    let neaps = a_day_of_sea(2_500);
    assert!(
        range(&neaps) * 2 <= range(&springs) + 20,
        "{} against {}",
        range(&neaps),
        range(&springs)
    );
}

#[test]
fn rain_on_wet_land_floods_the_far_field_and_dry_land_drinks_it() {
    // An hour of the hardest rain. On land already soaked, it all runs off into the Brack, which
    // tops its banks and floods the far field. On dry land, the ground drinks it, and the Brack
    // barely rises.
    let after_an_hour = |soaked: bool| {
        let mut city = City::from(ashford(60, 10_000));
        if soaked {
            city.attach(Box::new(Soak));
        }
        city.run(60);
        (
            int(&city, BRACK, WATER_LEVEL),
            int(&city, FAR_FIELD, FLOOD_DEPTH),
        )
    };
    let (river, flood) = after_an_hour(true);
    assert!(river > 150, "over its banks: {river} cm");
    assert!(flood > 50, "the far field under {flood} cm");
    let (dry_river, dry_flood) = after_an_hour(false);
    assert!(
        dry_river < 150 && dry_flood == 0,
        "{dry_river} cm, {dry_flood}"
    );
    // The same at minute ticks and ten-minute ticks.
    let mut coarse = City::from(ashford(600, 10_000));
    coarse.attach(Box::new(Soak));
    coarse.run(6);
    let level = int(&coarse, BRACK, WATER_LEVEL);
    assert!((level - river).abs() <= 10, "{level} against {river}");
}

#[test]
fn the_flood_cuts_off_the_far_field() {
    // Erin sets out for the far field, a kilometre off. Flooded deeper than she can wade, there is
    // no way into it; dry, she gets there.
    let reaches = |flooded: bool| {
        let mut city = City::from(ashford(60, if flooded { 10_000 } else { 6_000 }));
        if flooded {
            city.attach(Box::new(Soak));
        }
        city.run(30);
        city.go(ERIN, FAR_FIELD, 140);
        city.run_until(4 * 60, |c| {
            !c.travelling(ERIN) || c.read(ERIN, TRAVEL_BLOCKED).is_some()
        });
        (
            city.room_of(ERIN) == FAR_FIELD,
            city.read(ERIN, TRAVEL_BLOCKED).is_some(),
        )
    };
    assert_eq!(reaches(true), (false, true), "cut off");
    assert_eq!(reaches(false).0, true, "dry, she gets there");
}

#[test]
fn a_frozen_pond_bears_a_walker() {
    // The manor's pond is a metre deep: no one walks into it. In a hard frost — the Vale declared
    // fifteen below — ice forms two millimetres an hour for each degree, and by midday it is
    // thick enough to bear Erin, who walks out onto it.
    let mut pkg = ashford(60, 6_000);
    pkg.regions
        .iter_mut()
        .find(|r| r.id == VALE)
        .unwrap()
        .temperature_centi_c = -1_500;
    let mut city = City::from(pkg);
    city.go(ERIN, POND, 140);
    city.run(5);
    assert!(
        city.read(ERIN, TRAVEL_BLOCKED).is_some(),
        "too deep to wade"
    );
    city.run(4 * 60);
    assert!(int(&city, POND, ICE) >= 80, "{} mm", int(&city, POND, ICE));
    city.go(ERIN, POND, 140);
    city.run_until(60, |c| c.room_of(ERIN) == POND);
    assert_eq!(city.room_of(ERIN), POND, "out on the ice");
}

#[test]
fn what_lies_in_the_river_is_soaked() {
    // The hammer, dropped in the Brack on a dry day, is wet through at once.
    let mut pkg = ashford(60, 6_000);
    for c in &mut pkg.containment {
        if c.child_id == HAMMER {
            c.parent_id = BRACK;
        }
    }
    let mut city = City::from(pkg);
    city.run(2);
    assert_eq!(int(&city, HAMMER, physical::schema::WETNESS), 10_000);
}
