//! Crossing the seam (Vol. V Ch. 5 §5.2; Appendix A, Ruling 19; Amendment A-24): when attention
//! arrives at the Lanes, its residents become organisms in a single tick — as hungry and as tired
//! as their records had them, asleep if they were, where they were — and then live in the scene,
//! keeping their own hours.

use kernel::store::RealityStore;
use kernel::system::CommittedView;
use kernel::value::Value;
use reference::id::*;
use reference::{e, with_tick_seconds, City};
use society::schema::{
    CITY, RESIDENT_AT, RESIDENT_CROSSING, RESIDENT_DOING, RESIDENT_FATIGUE, RESIDENT_HOME,
    RESIDENT_HUNGER, RESIDENT_UNTIL, RESIDENT_WORK, SCENE,
};

const HOUR: u64 = 60; // minute ticks

fn ashford() -> City {
    let mut pkg = with_tick_seconds(60);
    pkg.physical_rules.temperature_variability_centi_c = 0;
    City::from(pkg)
}

/// The record's state facts, and an organism's: never both for one person (Ruling 19).
fn one_tier_at_a_time(city: &City) {
    for who in RESIDENTS {
        let record = city.read(who, RESIDENT_HUNGER).is_some();
        let organism = city.read(who, living::schema::HUNGER).is_some()
            || city.read(who, physical::schema::CONTAINED_IN).is_some();
        assert!(
            !(record && organism),
            "{who} is both a record and an organism at tick {}",
            city.tick
        );
    }
}

fn place_of(city: &City, who: u64) -> Option<Value> {
    city.read(who, physical::schema::CONTAINED_IN)
}

#[test]
fn attention_brings_the_lanes_people_into_the_scene_as_they_were() {
    let mut city = ashford();
    city.run(3 * HOUR); // three in the morning: all asleep
    city.attend(LANES, SCENE);
    city.run(2); // the rung lands; the seam marks everyone at home in the Lanes
    let records: Vec<_> = RESIDENTS
        .iter()
        .map(|&who| {
            assert!(city.read(who, RESIDENT_CROSSING).is_some(), "{who} marked");
            (
                who,
                city.read(who, RESIDENT_HUNGER).unwrap(),
                city.read(who, RESIDENT_FATIGUE).unwrap(),
                city.read(who, RESIDENT_HOME).unwrap(),
                city.read(who, RESIDENT_WORK),
            )
        })
        .collect();
    one_tier_at_a_time(&city);
    city.run(1); // every domain takes them up at once
    one_tier_at_a_time(&city);
    for (who, hunger, fatigue, home, work) in records {
        // The levels carry over exactly: rate, anchor, and all.
        assert_eq!(
            city.read(who, living::schema::HUNGER),
            Some(hunger),
            "{who}"
        );
        assert_eq!(
            city.read(who, living::schema::FATIGUE),
            Some(fatigue),
            "{who}"
        );
        assert_eq!(place_of(&city, who), Some(home), "{who} is where they were");
        assert_eq!(city.int(who, living::schema::HEALTH), Some(10_000));
        assert!(city.read(who, minds::schema::WALK_SPEED).is_some(), "{who}");
        // Asleep in the record, lying down in the scene.
        assert_eq!(
            city.read(who, living::schema::REST),
            Some(Value::Bool(true))
        );
        assert_eq!(
            city.read(who, minds::schema::RESTING),
            Some(Value::Bool(true))
        );
        // They know where they are, and feel what their body has.
        assert_eq!(
            city.store()
                .read(kernel::fact::FactKey::pair(
                    e(who),
                    information::schema::PLACE_OF,
                    e(who)
                ))
                .map(|f| f.value),
            Some(home)
        );
        assert_eq!(
            city.read(who, information::schema::FELT_HUNGER),
            Some(hunger)
        );
        // The record keeps who they are, and nothing else.
        assert_eq!(city.read(who, RESIDENT_HOME), Some(home));
        assert_eq!(city.read(who, RESIDENT_WORK), work);
        for fact in [
            RESIDENT_DOING,
            RESIDENT_AT,
            RESIDENT_UNTIL,
            RESIDENT_HUNGER,
            RESIDENT_FATIGUE,
            RESIDENT_CROSSING,
        ] {
            assert!(city.read(who, fact).is_none(), "{who} keeps {fact:?}");
        }
    }
    // Hunger goes on rising at the same rate: an hour later, the same value either tier would
    // have given.
    city.run(HOUR);
    one_tier_at_a_time(&city);
    for who in RESIDENTS {
        assert!(city.int(who, living::schema::HUNGER).unwrap() > 0, "{who}");
    }
}

#[test]
fn in_the_scene_they_keep_their_own_hours() {
    let mut city = ashford();
    city.run(3 * HOUR);
    city.attend(LANES, SCENE);
    // By eleven the workers have got up, gone out of their doors, along the street, and into
    // the workshop; the others are at home.
    city.run(8 * HOUR);
    one_tier_at_a_time(&city);
    for who in RESIDENTS {
        let home = city.read(who, RESIDENT_HOME);
        if IDLE_RESIDENTS.contains(&who) {
            assert_eq!(place_of(&city, who), home, "{who} stays home");
        } else {
            assert_eq!(
                place_of(&city, who),
                Some(Value::Entity(e(WORKSHOP))),
                "{who} at work by eleven"
            );
        }
    }
    // And by nine in the evening they are home again.
    city.run(10 * HOUR);
    for who in RESIDENTS {
        assert_eq!(place_of(&city, who), city.read(who, RESIDENT_HOME), "{who}");
    }
}

#[test]
fn a_resident_crosses_on_arriving_where_the_scene_is() {
    // Attention on the workshop alone: nobody is there at three in the morning, so nobody
    // crosses; the workers cross as each arrives for the day, and the others stay records.
    let mut city = ashford();
    city.run(3 * HOUR);
    city.attend(WORKSHOP, SCENE);
    city.run(HOUR);
    for who in RESIDENTS {
        assert!(
            city.read(who, RESIDENT_HUNGER).is_some(),
            "{who} still a record"
        );
    }
    let mut crossed = Vec::new();
    for _ in 0..7 * HOUR {
        city.run(1);
        one_tier_at_a_time(&city);
        for who in RESIDENTS {
            if !crossed.contains(&who) && city.read(who, living::schema::HUNGER).is_some() {
                assert_eq!(place_of(&city, who), Some(Value::Entity(e(WORKSHOP))));
                crossed.push(who);
            }
        }
    }
    crossed.sort_unstable();
    let workers: Vec<u64> = RESIDENTS
        .into_iter()
        .filter(|w| !IDLE_RESIDENTS.contains(w))
        .collect();
    assert_eq!(crossed, workers);
}

#[test]
fn a_place_that_keeps_the_city_tier_keeps_its_people() {
    // The Lanes in the scene, but one home kept at the city tier: its household stays records.
    let mut city = ashford();
    city.run(3 * HOUR);
    city.attend(151, CITY).attend(LANES, SCENE);
    city.run(3);
    for who in RESIDENTS {
        let home = city.read(who, RESIDENT_HOME);
        let record = city.read(who, RESIDENT_HUNGER).is_some();
        assert_eq!(record, home == Some(Value::Entity(e(151))), "{who}");
    }
}

#[test]
fn the_same_seed_crosses_the_same_way() {
    let run = || {
        let mut city = ashford();
        city.run(3 * HOUR);
        city.attend(LANES, SCENE);
        city.run(6 * HOUR);
        city.store().state_hash()
    };
    assert_eq!(run(), run());
}

#[test]
fn a_resident_does_not_begin_in_the_scene() {
    // Ruling 19 at load: a resident's home is at the city tier.
    let mut pkg = reference::package();
    pkg.residency.retain(|(place, _)| *place != LANES);
    let err = packages::load(&pkg, packages::engine_version()).expect_err("homes in the scene");
    assert!(format!("{err:?}").contains("city tier"), "{err:?}");
}
