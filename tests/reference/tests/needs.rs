//! Needs beyond warmth, and what they cost (Vol. III Ch. 2 §2.4; Appendix A, Rulings 9 and 15;
//! Amendment A-10): Ashford's people tire while awake and recover while they rest — but not while
//! they walk; the cold harms and then kills; a fall hurts beyond a safe drop; wounds heal slowly.
//! Their minds lie down when tired and get up when rested, when the cold calls, or when a routine
//! does once they are no longer tired. The dead neither see nor think.

use kernel::domain::Domain;
use kernel::events::ChronicleEntry;
use kernel::fact::{Cause, FactKey, FactType};
use kernel::store::{CommitBatch, RealityStore, Resolution};
use kernel::system::CommittedView;
use kernel::tick::run_tick;
use kernel::value::Value;
use living::schema::{BODY_HEAT, FATIGUE, HEALTH, SIGHT_RANGE};
use living::LivingDomain;
use minds::schema::{GOAL, RESTING};
use physical::schema::{IN_VIEW, TRAVEL_TO};
use reference::id::*;
use reference::{e, with_tick_seconds, City};

const HOUR: u64 = 6; // ten-minute ticks

fn int(city: &City, who: u64, fact: FactType) -> i64 {
    city.int(who, fact).unwrap()
}

/// Ashford at `tick_seconds` on a still day, adjusted by `change`.
fn ashford(tick_seconds: u64, change: impl FnOnce(&mut packages::WorldPackage)) -> City {
    let mut pkg = with_tick_seconds(tick_seconds);
    pkg.physical_rules.temperature_variability_centi_c = 0;
    change(&mut pkg);
    City::from(pkg)
}

/// As [`ashford`], without minds: for tests of the body alone.
fn quiet_ashford(tick_seconds: u64, change: impl FnOnce(&mut packages::WorldPackage)) -> City {
    ashford(tick_seconds, |p| {
        change(p);
        p.manifest.domains.retain(|d| d != "minds");
    })
}

/// A region's temperature, set before the world begins.
fn region_at(pkg: &mut packages::WorldPackage, region: u64, centi_c: i64) {
    pkg.regions
        .iter_mut()
        .find(|r| r.id == region)
        .unwrap()
        .temperature_centi_c = centi_c;
}

#[test]
fn tiring_and_resting_keep_the_same_pace_at_any_tick_length() {
    for tick_seconds in [60, 600] {
        let per_hour = 3_600 / tick_seconds;
        // Finn, awake for eight hours: 4.5% an hour — 7½ hundredths a one-minute tick, rounded
        // without bias.
        let mut city = quiet_ashford(tick_seconds, |_| {});
        city.run(8 * per_hour);
        let tired = int(&city, FINN, FATIGUE);
        assert!(
            (tired - 3_600).abs() <= 20,
            "ticks of {tick_seconds} s: {tired}"
        );
        // Gwen, worn out and lying down for four: 12.5% an hour off.
        let mut city = quiet_ashford(tick_seconds, |p| p.fatigue.push((GWEN, 8_000)));
        city.rest(GWEN, true);
        city.run(1); // the request lands
        let lay_down = int(&city, GWEN, FATIGUE);
        city.run(4 * per_hour);
        // 208⅓ a ten-minute tick, rounded without bias: within a few hundredths of a percent.
        let shed = lay_down - int(&city, GWEN, FATIGUE);
        assert!(
            (shed - 5_000).abs() <= 3,
            "ticks of {tick_seconds} s: shed {shed}"
        );
    }
}

#[test]
fn a_body_on_the_move_does_not_rest() {
    // The hiker is asked to rest and to walk to the cairn at once. Walking, she tires; only when
    // she has arrived and stands still does the rest begin.
    let mut city = quiet_ashford(1, |p| p.fatigue.push((HIKER, 5_000)));
    city.rest(HIKER, true).go(HIKER, CAIRN, 60);
    let walked = city.run_until(900, |c| !c.travelling(HIKER));
    assert!(walked > 60, "a long enough walk to tire on ({walked} s)");
    let arrived = int(&city, HIKER, FATIGUE);
    assert!(arrived > 5_000, "tired by walking: {arrived}");
    city.run(180);
    assert!(int(&city, HIKER, FATIGUE) < arrived, "rested once still");
}

#[test]
fn the_cold_harms_and_then_kills_and_the_dead_stop() {
    // A killing frost on Highmoor: -25 °C.
    let mut city = quiet_ashford(600, |p| region_at(p, HIGHMOOR, -2_500));
    let mut died = None;
    let mut last = int(&city, HAL, HEALTH);
    for _ in 0..24 * HOUR {
        city.run(1);
        let health = int(&city, HAL, HEALTH);
        assert!(health <= last, "the cold only takes");
        last = health;
        if health == 0 && died.is_none() {
            died = Some(city.tick);
        }
    }
    let died = died.expect("Hal froze");
    assert!(
        (2 * HOUR..8 * HOUR).contains(&died),
        "died at hour {}",
        died / HOUR
    );
    let death = city.store().read(FactKey::new(e(HAL), HEALTH)).unwrap();
    assert_eq!(death.provenance.cause, Cause::new("died_of_cold"));
    // From then on: no metabolism, no fatigue, no senses, nothing in view.
    let since = |fact: FactType| {
        city.chronicle
            .iter()
            .filter(|c| c.subject() == e(HAL) && c.fact_type() == fact && c.tick() > died + 1)
            .count()
    };
    assert_eq!(
        (since(BODY_HEAT), since(FATIGUE), since(IN_VIEW)),
        (0, 0, 0)
    );
    assert_eq!(city.read(HAL, SIGHT_RANGE), None);
    assert!(city
        .store()
        .read_all(FactKey::new(e(HAL), IN_VIEW))
        .is_empty());
    // He is still there: a body on the moor.
    assert_eq!(city.room_of(HAL), HIGHMOOR);
    // Ida, in the fen, is untouched.
    assert_eq!(int(&city, IDA, HEALTH), 10_000);
}

#[test]
fn a_fall_hurts_beyond_a_safe_drop() {
    let mut city = quiet_ashford(1, |_| {});
    city.close(BOB, DOOR_IN);
    city.run(2);
    // The cat hops 30 cm down from the sill; Carol walks off the 2.5 m shed roof; Nell steps off
    // her 3 m boulder onto ground a metre lower still.
    city.go(CAT, KITCHEN, 300)
        .go(CAROL, STONE, 140)
        .go(NELL, MARKER, 140);
    city.run(40 + 61);
    assert_eq!(int(&city, CAT, HEALTH), 10_000, "a short hop does no harm");
    let carol = int(&city, CAROL, HEALTH);
    assert!(
        (8_995..=9_005).contains(&carol),
        "0.5 m beyond safe: {carol}"
    );
    let nell = int(&city, NELL, HEALTH);
    assert!((5_995..=6_005).contains(&nell), "2 m beyond safe: {nell}");
    let hurt = city.store().read(FactKey::new(e(CAROL), HEALTH)).unwrap();
    assert!(
        matches!(hurt.provenance.cause, c if c == Cause::new("fall") || c == Cause::new("healing"))
    );
}

#[test]
fn wounds_heal_slowly_while_nothing_harms() {
    // Living alone over Ashford, Finn's health set to half: 1% back an hour.
    let pkg = with_tick_seconds(600);
    let mut store = City::from(pkg.clone()).store().clone();
    let mut batch = CommitBatch::new(1);
    batch.resolutions.push(Resolution::One {
        key: FactKey::new(e(FINN), HEALTH),
        fact: kernel::fact::Fact::new(
            Value::Int(5_000),
            kernel::fact::Provenance::new(
                kernel::fact::SystemId::new("test.wound"),
                1,
                Cause::new("wounded"),
            ),
        ),
    });
    store.apply(batch);
    let domain = LivingDomain::new(packages::living_config(&pkg).unwrap());
    let domains: [&dyn Domain; 1] = [&domain];
    let mut chronicle: Vec<ChronicleEntry> = Vec::new();
    for t in 2..=1 + 10 * HOUR {
        run_tick(
            &mut store,
            &domains,
            &domain.systems(),
            t,
            7,
            &mut chronicle,
        )
        .unwrap();
    }
    let health = store
        .read(FactKey::new(e(FINN), HEALTH))
        .and_then(|f| f.value.as_int())
        .unwrap();
    assert!(
        (5_995..=6_005).contains(&health),
        "ten hours healed {health}"
    );
}

/// The ticks at which `who` lay down and got up, with why, over `ticks`.
fn sleeps(city: &mut City, who: u64, ticks: u64) -> Vec<(u64, bool, Cause)> {
    let mut out = Vec::new();
    for _ in 0..ticks {
        city.run(1);
        if let Some(f) = city
            .store()
            .read(FactKey::new(e(who), RESTING))
            .filter(|f| f.provenance.tick == city.tick)
        {
            out.push((city.tick, f.value == Value::Bool(true), f.provenance.cause));
        }
        let got_up = city
            .chronicle
            .iter()
            .rev()
            .take_while(|c| c.tick() == city.tick)
            .any(|c| c.subject() == e(who) && c.fact_type() == RESTING);
        if got_up && city.store().read(FactKey::new(e(who), RESTING)).is_none() {
            let why = city
                .chronicle
                .iter()
                .rev()
                .find(|c| c.subject() == e(who) && c.fact_type() == RESTING)
                .unwrap()
                .cause();
            out.push((city.tick, false, why));
        }
    }
    out
}

#[test]
fn erin_sleeps_the_night_and_gets_up_at_six() {
    // She begins the night rested; within the sleeping hours, once a little tired, she lies down,
    // and stays down until the hours end, rested.
    let mut city = ashford(600, |_| {});
    let rests = sleeps(&mut city, ERIN, 18 * HOUR);
    assert!(rests.len() >= 2, "{rests:?}");
    let (down, true, ref why) = rests[0] else {
        panic!("{rests:?}")
    };
    assert_eq!(*why, Cause::new("bedtime"));
    assert!(down < 6 * HOUR, "lay down at hour {}", down / HOUR);
    let (up, false, ref why) = rests[1] else {
        panic!("{rests:?}")
    };
    assert_eq!(*why, Cause::new("rested"));
    assert_eq!(up / HOUR, 6, "got up at {}", up);
    // And she is not tired again by evening.
    assert_eq!(rests.len(), 2, "{rests:?}");
}

#[test]
fn without_a_bedtime_erin_lies_down_when_tired_and_gets_up_rested() {
    // A world with no sleeping hours, and Erin at 6% an hour: rested at midnight, tired by noon,
    // she lies down in the yard, and gets up in the afternoon.
    let mut city = ashford(600, |p| {
        p.minds_rules.as_mut().unwrap().sleep_hours = (0, 0);
        p.living_rules.as_mut().unwrap().tire_per_hour = 600;
    });
    let rests = sleeps(&mut city, ERIN, 18 * HOUR);
    assert!(rests.len() >= 2, "{rests:?}");
    let (down, true, ref why) = rests[0] else {
        panic!("{rests:?}")
    };
    assert_eq!(*why, Cause::new("tired"));
    assert!(
        (11 * HOUR..13 * HOUR).contains(&down),
        "lay down at hour {}",
        down / HOUR
    );
    let (up, false, ref why) = rests[1] else {
        panic!("{rests:?}")
    };
    assert_eq!(*why, Cause::new("rested"));
    assert!(
        (15 * HOUR..18 * HOUR).contains(&up),
        "got up at hour {}",
        up / HOUR
    );
}

#[test]
fn the_cold_gets_erin_up_and_indoors() {
    // A bitter night in the Vale, 8 °C, and Erin worn out at midnight: she lies down in the yard.
    // Long before she is rested the cold gets her up, and she goes in.
    let mut city = ashford(600, |p| {
        region_at(p, VALE, 800);
        p.fatigue.push((ERIN, 9_500));
    });
    let rests = sleeps(&mut city, ERIN, 8 * HOUR);
    let (_, true, _) = rests[0] else {
        panic!("{rests:?}")
    };
    let (up, false, ref why) = rests[1] else {
        panic!("{rests:?}")
    };
    assert_eq!(*why, Cause::new("called_away"));
    assert!(up < 6 * HOUR, "the cold woke her at hour {}", up / HOUR);
    assert_eq!(city.room_of(ERIN), KITCHEN);
}

#[test]
fn a_tired_guard_lets_his_routine_wait() {
    // The guard begins exhausted, and rest comes slowly tonight: at six he is still tired, so the
    // gate waits; once he is no longer tired, it calls him.
    let mut city = ashford(600, |p| {
        p.fatigue.push((GUARD, 10_000));
        p.living_rules.as_mut().unwrap().rest_per_hour = 300;
    });
    city.run(7 * HOUR);
    assert_eq!(
        city.store()
            .read(FactKey::new(e(GUARD), GOAL))
            .map(|f| f.value),
        None
    );
    assert_eq!(
        city.store()
            .read(FactKey::new(e(GUARD), RESTING))
            .map(|f| f.value),
        Some(Value::Bool(true))
    );
    let mut called = None;
    for _ in 0..5 * HOUR {
        city.run(1);
        if called.is_none() && city.store().read(FactKey::new(e(GUARD), GOAL)).is_some() {
            called = Some(city.tick);
        }
    }
    let called = called.expect("the gate called him");
    assert!(
        (9 * HOUR..11 * HOUR).contains(&called),
        "at hour {}",
        called / HOUR
    );
    assert_eq!(
        city.read(GUARD, minds::schema::AT),
        Some(Value::Entity(e(GATE))),
        "and he kept it"
    );
}

#[test]
fn the_dead_neither_see_nor_think() {
    // A killing frost in the Vale. The courier, who knows nowhere warmer, freezes in the yard.
    let mut city = ashford(600, |p| region_at(p, VALE, -2_500));
    let mut died = None;
    for _ in 0..24 * HOUR {
        city.run(1);
        if died.is_none() && int(&city, COURIER, HEALTH) == 0 {
            died = Some(city.tick);
        }
    }
    let died = died.expect("the courier froze");
    // After the tick perception felt it, nothing about him is perceived, believed, or decided —
    // and nothing walks him anywhere.
    let after: Vec<&ChronicleEntry> = city
        .chronicle
        .iter()
        .filter(|c| c.subject() == e(COURIER) && c.tick() > died + 2)
        .filter(|c| {
            let name = c.fact_type().name();
            name.starts_with("info.") || name.starts_with("mind.") || name.starts_with("physical.")
        })
        .collect();
    assert!(after.is_empty(), "{after:?}");
    assert_eq!(city.read(COURIER, TRAVEL_TO), None);
    assert_eq!(
        city.store()
            .read(FactKey::new(e(COURIER), information::schema::FELT_HEALTH))
            .map(|f| f.value),
        Some(Value::Int(0))
    );
}
