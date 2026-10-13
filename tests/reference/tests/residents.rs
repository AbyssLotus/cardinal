//! The city tier (Appendix A, Ruling 19; Amendment A-23): the Lanes' twelve residents are records,
//! not organisms. They sleep at home by night, walk to the workshop and work its hours, walk home,
//! and eat when hungry from larders their shifts keep stocked — and between one thing and the
//! next, they cost nothing.

use kernel::store::RealityStore;
use kernel::value::Value;
use reference::id::*;
use reference::{e, with_tick_seconds, City};
use society::schema::{
    AT_HOME, EATING, LARDER, RESIDENT_AT, RESIDENT_DOING, RESIDENT_HOME, RESIDENT_HUNGER, SLEEPING,
    WORKING,
};

const HOUR: u64 = 6; // ten-minute ticks

fn ashford() -> City {
    let mut pkg = with_tick_seconds(600);
    pkg.physical_rules.temperature_variability_centi_c = 0;
    City::from(pkg)
}

fn doing(city: &City, who: u64) -> i64 {
    city.int(who, RESIDENT_DOING).unwrap()
}

fn at(city: &City, who: u64) -> Option<Value> {
    city.read(who, RESIDENT_AT)
}

fn home(city: &City, who: u64) -> Option<Value> {
    city.read(who, RESIDENT_HOME)
}

#[test]
fn a_day_in_the_lanes() {
    let mut city = ashford();
    // Three in the morning: everyone asleep at home.
    city.run(3 * HOUR);
    for who in RESIDENTS {
        assert_eq!(doing(&city, who), SLEEPING, "{who} at 3am");
        assert_eq!(at(&city, who), home(&city, who));
    }
    // Eleven in the morning: the workers at the workshop, the others at home.
    city.run(8 * HOUR);
    for who in RESIDENTS {
        if IDLE_RESIDENTS.contains(&who) {
            assert_eq!(at(&city, who), home(&city, who), "{who} stays home");
        } else {
            assert_eq!(doing(&city, who), WORKING, "{who} at 11am");
            assert_eq!(at(&city, who), Some(Value::Entity(e(WORKSHOP))));
        }
    }
    // Nine in the evening: everyone home again.
    city.run(10 * HOUR);
    for who in RESIDENTS {
        assert_eq!(at(&city, who), home(&city, who), "{who} at 9pm");
        assert!(
            matches!(doing(&city, who), AT_HOME | EATING),
            "{who} at 9pm"
        );
    }
}

#[test]
fn they_eat_and_their_shifts_keep_the_larders() {
    let mut city = ashford();
    let (mut meals, mut earned, mut hungriest) = (0, 0, 0);
    let mut seen = 0;
    for _ in 0..4 * 24 * HOUR {
        city.run(1);
        for c in &city.chronicle[seen..] {
            match (c.fact_type(), c.cause().event()) {
                (f, "ate") if f == RESIDENT_HUNGER => meals += 1,
                (f, "larder") if f == LARDER => earned += 1,
                _ => {}
            }
        }
        seen = city.chronicle.len();
        for who in RESIDENTS {
            hungriest = hungriest.max(city.int(who, RESIDENT_HUNGER).unwrap());
        }
    }
    // About a meal a day each, and never near starving.
    assert!(meals >= 12 * 3, "{meals} meals in four days");
    assert!(hungriest < 9_000, "hungriest {hungriest}");
    assert!(earned > 0);
    for home in 151..=156 {
        let larder = city.int(home, LARDER).unwrap_or(0);
        assert!(larder >= 0, "{home}: {larder}");
    }
}

#[test]
fn between_one_thing_and_the_next_a_resident_costs_nothing() {
    // Within two hours of midnight everyone has eaten if hungry and gone to bed; until six the
    // rhythm attends to no one, and proposes nothing.
    let mut city = ashford();
    city.run(2 * HOUR);
    for _ in 0..3 * HOUR {
        city.run(1);
        let rhythm = city
            .meters()
            .iter()
            .find(|m| m.system.name() == "society.rhythm")
            .expect("metered");
        assert_eq!(rhythm.proposals, 0, "at tick {}", city.tick);
    }
}

#[test]
fn the_same_seed_makes_the_same_lanes() {
    let day = || {
        let mut city = ashford();
        city.run(2 * 24 * HOUR);
        city.store().state_hash()
    };
    assert_eq!(day(), day());
}

#[test]
fn a_resident_is_not_also_an_organism() {
    // One person, one tier at a time (Ruling 19).
    let mut pkg = reference::package();
    pkg.organisms.push(packages::model::OrganismSpec {
        id: 5001,
        region_id: 100,
        body_heat_centi_c: 3_700,
    });
    let err = packages::load(&pkg, packages::engine_version()).expect_err("double existence");
    assert!(format!("{err:?}").contains("Ruling 19"), "{err:?}");
}
