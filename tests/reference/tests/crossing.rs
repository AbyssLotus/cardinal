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

#[test]
fn a_round_trip_through_the_scene_changes_nothing() {
    // Law 14, scale must not change truth (Amendment A-25): the Lanes brought into the scene at
    // three in the morning and let go at half past, while everyone sleeps, leave the world exactly
    // as if no one had looked. Bodies, minds, and beliefs come and go; the record is the same to the
    // bit, and so is everything after.
    let mut looked = ashford();
    let mut not = ashford();
    looked.run(3 * HOUR);
    not.run(3 * HOUR);
    looked.attend(LANES, SCENE);
    looked.run(HOUR / 2);
    for who in RESIDENTS {
        assert!(
            looked.read(who, living::schema::HUNGER).is_some(),
            "{who} crossed"
        );
    }
    looked.attend(LANES, CITY);
    not.run(HOUR / 2);
    looked.run(3);
    not.run(3);
    one_tier_at_a_time(&looked);
    for who in RESIDENTS {
        assert!(
            looked.read(who, RESIDENT_HUNGER).is_some(),
            "{who} came back"
        );
        assert!(looked.read(who, living::schema::HUNGER).is_none(), "{who}");
        assert!(
            looked.read(who, minds::schema::WALK_SPEED).is_none(),
            "{who}"
        );
        assert!(
            looked.read(who, physical::schema::CONTAINED_IN).is_none(),
            "{who}"
        );
        assert_eq!(
            looked.read(who, RESIDENT_HUNGER),
            not.read(who, RESIDENT_HUNGER),
            "{who}'s hunger"
        );
        assert_eq!(
            looked.read(who, RESIDENT_FATIGUE),
            not.read(who, RESIDENT_FATIGUE),
            "{who}'s fatigue"
        );
    }
    // By morning both Lanes are going about the same day, fact for fact.
    looked.run(9 * HOUR);
    not.run(9 * HOUR);
    for who in RESIDENTS {
        for fact in [
            RESIDENT_DOING,
            RESIDENT_AT,
            RESIDENT_UNTIL,
            RESIDENT_HUNGER,
            RESIDENT_FATIGUE,
        ] {
            assert_eq!(
                looked.read(who, fact),
                not.read(who, fact),
                "{who} {fact:?}"
            );
        }
    }
    assert_eq!(looked.store().state_hash(), not.store().state_hash());
}

/// The Lanes' homes and their pantries (Amendment A-26).
const PANTRIES: [(u64, u64); 6] = [
    (151, 1511),
    (152, 1512),
    (153, 1513),
    (154, 1514),
    (155, 1515),
    (156, 1516),
];

#[test]
fn a_larder_in_the_scene_is_a_pantry_its_household_eats_from() {
    // Amendment A-26: attention on the Lanes sets each household's larder out as its pantry, a
    // store of meals in the home that its people know of and eat from. Shifts worked in the scene
    // restock it. When attention leaves, what is left is counted back into the larder.
    let mut city = ashford();
    city.run(3 * HOUR);
    let meals: Vec<i64> = PANTRIES
        .iter()
        .map(|&(home, _)| city.int(home, society::schema::LARDER).unwrap())
        .collect();
    city.attend(LANES, SCENE);
    city.run(3);
    for (&(home, pantry), &n) in PANTRIES.iter().zip(&meals) {
        assert!(city.read(home, society::schema::LARDER).is_none(), "{home}");
        assert_eq!(
            city.read(pantry, physical::schema::CONTAINED_IN),
            Some(Value::Entity(e(home)))
        );
        assert_eq!(city.int(pantry, resources::schema::STOCK), Some(n * 100));
    }
    // Its household knows it: where it is, whose, and what it holds.
    for who in RESIDENTS {
        let home = match city.read(who, RESIDENT_HOME) {
            Some(Value::Entity(h)) => h.raw(),
            _ => unreachable!(),
        };
        let pantry = PANTRIES.iter().find(|(h, _)| *h == home).unwrap().1;
        let belief = |fact| {
            city.store()
                .read(kernel::fact::FactKey::pair(e(who), fact, e(pantry)))
                .map(|f| f.value)
        };
        assert_eq!(
            belief(information::schema::PLACE_OF),
            Some(Value::Entity(e(home)))
        );
        assert_eq!(
            belief(information::schema::OWNER),
            Some(Value::Entity(e(home)))
        );
    }

    // A day in the scene: people eat from their own pantries, and work keeps them stocked.
    let (mut eaten, mut restocked, mut hungriest) = (0, 0, 0);
    let mut seen = city.chronicle.len();
    for _ in 0..24 * HOUR {
        city.run(1);
        for c in &city.chronicle[seen..] {
            match (c.fact_type(), c.cause().event()) {
                (f, "picked") if f == physical::schema::PICKED => eaten += 1,
                (f, "restocked") if f == resources::schema::RESTOCK => restocked += 1,
                _ => {}
            }
        }
        seen = city.chronicle.len();
        for who in RESIDENTS {
            hungriest = hungriest.max(city.int(who, living::schema::HUNGER).unwrap_or(0));
        }
    }
    assert!(eaten >= 6, "{eaten} meals taken from pantries");
    assert!(restocked >= 1, "no shift restocked a pantry");
    assert!(hungriest < 9_000, "hungriest {hungriest}");

    // Attention leaves: each pantry counted back into its larder, exactly — its stock and any
    // delivery not yet taken in, as they stood when it was marked to leave.
    city.attend(LANES, CITY);
    city.run(2); // the rung lands; the pantries are marked
    let held: Vec<i64> = PANTRIES
        .iter()
        .map(|&(_, p)| {
            assert!(
                city.read(p, society::schema::PANTRY_LEAVING).is_some(),
                "{p}"
            );
            city.int(p, resources::schema::STOCK).unwrap() / 100
                + city.int(p, resources::schema::RESTOCK).unwrap_or(0)
        })
        .collect();
    city.run(1);
    for (i, &(home, pantry)) in PANTRIES.iter().enumerate() {
        assert!(
            city.read(pantry, resources::schema::STOCK).is_none(),
            "{pantry}"
        );
        assert!(city.read(pantry, physical::schema::CONTAINED_IN).is_none());
        assert_eq!(
            city.int(home, society::schema::LARDER),
            Some(held[i]),
            "{home}"
        );
    }
}
