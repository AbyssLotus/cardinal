//! Ashford's climate (Vol. III Ch. 1 §1.9–1.10; Amendments A-1, A-5): weather simulated once
//! per declared climate and inherited down the nesting; walled rooms sheltered, lit only through
//! their openings, their air lagging the outside; and the same day, the same weather, at any
//! tick length.

use kernel::domain::Domain;
use kernel::events::ChronicleEntry;
use kernel::fact::FactKey;
use kernel::store::MemoryStore;
use kernel::system::{CommittedView, System};
use kernel::tick::run_tick;
use physical::climate::ambient_temperature;
use physical::schema::{
    ANOMALY_SCALE, HUMIDITY, ILLUMINATION, PRESSURE, TEMPERATURE, TEMPERATURE_ANOMALY, WIND_TOWARD,
};
use physical::PhysicalDomain;
use reference::id::*;
use reference::{e, with_tick_seconds, City};

/// The climates Ashford declares that are open to the sky.
const OUTDOOR: [u64; 3] = [VALE, HIGHMOOR, SOUTHFEN];

/// Run some physical systems alone over a copy of the reality `pkg` loads, for `ticks` ticks, calling `each` after every tick — so one effect can be measured in isolation.
fn isolated(
    pkg: packages::WorldPackage,
    only: &[&str],
    ticks: u64,
    mut each: impl FnMut(u64, &MemoryStore),
) -> MemoryStore {
    let city = City::from(pkg.clone());
    let mut store = city.store().clone();
    let domain = PhysicalDomain::new(packages::physical_config(&pkg));
    let domains: [&dyn Domain; 1] = [&domain];
    let systems: Vec<Box<dyn System>> = domain
        .systems()
        .into_iter()
        .filter(|s| only.contains(&s.id().name()))
        .collect();
    assert_eq!(systems.len(), only.len(), "{only:?} exist");
    let mut chronicle: Vec<ChronicleEntry> = Vec::new();
    for t in 1..=ticks {
        run_tick(&mut store, &domains, &systems, t, 7, &mut chronicle).expect("commits");
        chronicle.clear();
        each(t, &store);
    }
    store
}

fn temp(s: &dyn CommittedView, id: u64) -> i64 {
    s.read(FactKey::new(e(id), TEMPERATURE))
        .and_then(|f| f.value.as_int())
        .unwrap()
}

/// The coldest-to-warmest swing of each of `ids` over one day of the day/night cycle alone.
fn day_swing(tick_seconds: u64, ids: &[u64]) -> Vec<i64> {
    let mut lo = vec![i64::MAX; ids.len()];
    let mut hi = vec![i64::MIN; ids.len()];
    isolated(
        with_tick_seconds(tick_seconds),
        &["physical.diurnal_cycle"],
        86_400 / tick_seconds,
        |_, s| {
            for (i, id) in ids.iter().enumerate() {
                lo[i] = lo[i].min(temp(s, *id));
                hi[i] = hi[i].max(temp(s, *id));
            }
        },
    );
    hi.iter().zip(&lo).map(|(h, l)| h - l).collect()
}

#[test]
fn the_day_is_the_same_at_any_tick_length() {
    // The Vale is open sky: the full 5 °C swing. The mine mouth is 40% open: 2 °C. Highmoor is
    // open but granite: thermal mass damps it to 2.4 °C. Identical whether Ashford ticks by
    // the hour, the minute, or the second.
    for tick_seconds in [3_600, 60, 1] {
        assert_eq!(
            day_swing(tick_seconds, &[VALE, MINE_MOUTH, HIGHMOOR, SOUTHFEN]),
            vec![500, 200, 241, 500],
            "ticks of {tick_seconds} s"
        );
    }
}

/// The spread (standard deviation, centidegrees) of the outdoor climates' weather anomaly,
/// sampled every hour of simulated time over `days` days, weather alone.
fn weather_spread(tick_seconds: u64, days: u64) -> f64 {
    let mut samples = Vec::new();
    let per_hour = 3_600 / tick_seconds;
    isolated(
        with_tick_seconds(tick_seconds),
        &["physical.temperature_weather"],
        days * 86_400 / tick_seconds,
        |t, s| {
            if t % per_hour == 0 && t > 2 * 86_400 / tick_seconds {
                for id in OUTDOOR {
                    let a = s
                        .read(FactKey::new(e(id), TEMPERATURE_ANOMALY))
                        .and_then(|f| f.value.as_int())
                        .unwrap_or(0);
                    samples.push(a as f64 / ANOMALY_SCALE as f64);
                }
            }
        },
    );
    let mean = samples.iter().sum::<f64>() / samples.len() as f64;
    (samples.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / samples.len() as f64).sqrt()
}

#[test]
fn the_weather_has_the_declared_spread_at_any_tick_length() {
    // Ashford declares a 3 °C spread; hourly and per-minute ticks both deliver it.
    for (tick_seconds, days) in [(3_600, 90), (60, 30)] {
        let sd = weather_spread(tick_seconds, days);
        assert!(
            (225.0..=375.0).contains(&sd),
            "ticks of {tick_seconds} s: spread {sd:.0}, declared 300"
        );
    }
}

#[test]
fn a_decade_of_weather_stays_within_the_climate() {
    // Weather reverts toward normal: over ten years, no outdoor climate strays more than six
    // spreads from where it began.
    let mut pkg = with_tick_seconds(3_600);
    pkg.physical_rules.environment_step_seconds = 6 * 3_600;
    let start: Vec<i64> = {
        let c = City::from(pkg.clone());
        OUTDOOR.iter().map(|id| temp(c.store(), *id)).collect()
    };
    let mut worst = 0;
    isolated(
        pkg,
        &["physical.temperature_weather"],
        10 * 365 * 24,
        |_, s| {
            for (i, id) in OUTDOOR.iter().enumerate() {
                worst = worst.max((temp(s, *id) - start[i]).abs());
            }
        },
    );
    assert!(worst > 300, "weather happened ({worst})");
    assert!(worst < 1_800, "worst departure {worst}, bound 1800");
}

#[test]
fn weather_runs_on_climates_and_everything_else_inherits_it() {
    let mut city = City::new();
    city.run(60); // one environment step
    let s = city.store();
    // The weather has stepped on the declared climates...
    for id in [VALE, HIGHMOOR, SOUTHFEN, MINE_MOUTH] {
        assert!(
            s.read(FactKey::new(e(id), TEMPERATURE_ANOMALY)).is_some(),
            "{id}"
        );
    }
    // ...and nowhere else: not the city, not its districts, not the yard, not a single room.
    // Nor on the regions that only classify — the temperate zone, the watershed, the fox's run,
    // the frost hollow — which are not places with air of their own.
    for id in [
        ASHFORD,
        OLD_TOWN,
        HILL,
        YARD,
        HOUSE,
        KITCHEN,
        BEDROOM,
        LOFT,
        TEMPERATE,
        WATERSHED,
        FOX_RUN,
        FROST_HOLLOW,
    ] {
        assert!(
            s.read(FactKey::new(e(id), TEMPERATURE_ANOMALY)).is_none(),
            "{id}"
        );
    }
    // Erin in the yard feels the Vale's air; Hal feels Highmoor's.
    assert_eq!(ambient_temperature(s, e(ERIN)), Some(temp(s, VALE)));
    assert_eq!(ambient_temperature(s, e(HAL)), Some(temp(s, HIGHMOOR)));
    // Walled rooms have become indoor climates, starting from the air outside them, and Alice
    // feels her bedroom's.
    assert!(s.read(FactKey::new(e(BEDROOM), TEMPERATURE)).is_some());
    assert_eq!(ambient_temperature(s, e(ALICE)), Some(temp(s, BEDROOM)));
}

#[test]
fn indoors_is_lit_only_through_its_openings() {
    // Ten-minute ticks; noon is tick 72.
    let mut city = City::from(with_tick_seconds(600));
    city.run(72);
    let light = |city: &City, id| city.int(id, ILLUMINATION).unwrap_or(0);
    assert_eq!(light(&city, VALE), 10_000, "full sun outdoors");
    // The kitchen: the open front door (90 cm × 2.05 m) and the low window (40 × 50 cm) onto
    // the yard, against a 10 m × 10 m floor — about 2% of daylight.
    assert_eq!(light(&city, KITCHEN), 204);
    assert_eq!(light(&city, BEDROOM), 80, "one shut glass window");
    assert_eq!(light(&city, CELLAR), 120, "the open bulkhead");
    assert_eq!(light(&city, VAULT), 0, "no openings at all");
    // The mine, by its declared exposure.
    assert_eq!(light(&city, MINE_MOUTH), 4_000);
    assert_eq!(light(&city, CABIN), 1_000);
    assert_eq!(light(&city, GALLERY_DEEP), 0);
    // And at midnight, nothing is lit anywhere.
    city.run(72);
    for id in [VALE, KITCHEN, BEDROOM, CELLAR, MINE_MOUTH, CABIN] {
        assert_eq!(light(&city, id), 0, "{id} at midnight");
    }
}

#[test]
fn indoor_air_follows_the_outdoors_slowly_and_stone_slowest() {
    // The day and night alone, with walls between the yard and two rooms: Alice's timber
    // kitchen and the manor's granite hall. Measured on the third day, once both have settled
    // into the rhythm. Each lags the yard by the world's four-hour coupling, stretched by the
    // heat its walls store per cubic metre (Amendment A-6) — granite's nearly twice timber's.
    let rooms = [VALE, KITCHEN, HALL];
    let mut days: [Vec<i64>; 3] = Default::default();
    isolated(
        with_tick_seconds(600),
        &["physical.diurnal_cycle", "physical.shelter"],
        3 * 144,
        |t, s| {
            if t > 2 * 144 {
                for (day, id) in days.iter_mut().zip(rooms) {
                    day.push(temp(s, id));
                }
            }
        },
    );
    let swing = |v: &[i64]| v.iter().max().unwrap() - v.iter().min().unwrap();
    let peak = |v: &[i64]| v.iter().enumerate().max_by_key(|(_, t)| **t).unwrap().0 as i64;
    let [vale, kitchen, hall] = &days;
    // The yard swings the full 5 °C. The kitchen follows with 2.1 °C, peaking 3 h 50 min after
    // the yard; the granite hall with 1.65 °C, 4 h 50 min after.
    assert_eq!([swing(vale), swing(kitchen), swing(hall)], [500, 211, 165]);
    assert_eq!(
        [peak(kitchen) - peak(vale), peak(hall) - peak(vale)],
        [23, 29]
    );
}

#[test]
fn pressure_falls_with_height_and_the_wind_blows_toward_the_moor() {
    let mut city = City::from(with_tick_seconds(600));
    city.run(3);
    let p = |id| city.int(id, PRESSURE).unwrap();
    // 1 decapascal per metre: Southfen (5 m) > the Vale (50 m) > Highmoor (400 m).
    assert!(p(SOUTHFEN) > p(VALE) && p(VALE) > p(HIGHMOOR));
    // Wind runs down the gradient: out of the Vale toward Highmoor; out of Southfen into the Vale.
    assert_eq!(
        city.read(VALE, WIND_TOWARD),
        Some(kernel::value::Value::Entity(e(HIGHMOOR)))
    );
    assert_eq!(
        city.read(SOUTHFEN, WIND_TOWARD),
        Some(kernel::value::Value::Entity(e(VALE)))
    );
    // And humidity stays within its range.
    for id in OUTDOOR {
        let h = city.int(id, HUMIDITY).unwrap();
        assert!((0..=10_000).contains(&h));
    }
}

#[test]
fn the_deep_gallery_holds_steady_while_the_mine_mouth_follows_the_sky() {
    // A month of days and weather. How far each place's air ranges follows how open it is to
    // the sky: the Vale fully, the mine mouth 40%, the cabin 10%, the deep gallery not at all.
    let ids = [VALE, MINE_MOUTH, CABIN, GALLERY_DEEP];
    let mut lo = [i64::MAX; 4];
    let mut hi = [i64::MIN; 4];
    isolated(
        with_tick_seconds(3_600),
        &["physical.diurnal_cycle", "physical.temperature_weather"],
        30 * 24,
        |_, s| {
            for (i, id) in ids.iter().enumerate() {
                lo[i] = lo[i].min(temp(s, *id));
                hi[i] = hi[i].max(temp(s, *id));
            }
        },
    );
    let range: Vec<i64> = hi.iter().zip(&lo).map(|(h, l)| h - l).collect();
    assert!(range[0] > range[1] && range[1] > range[2], "{range:?}");
    assert_eq!(range[3], 0, "the deep gallery never changes");
}

#[test]
fn the_vale_is_its_normal_plus_its_weather_exactly() {
    // Nothing accumulates from rounding: at every step the Vale's air is its seeded normal,
    // 14 °C, plus the current weather anomaly, rounded.
    isolated(
        with_tick_seconds(60),
        &["physical.temperature_weather"],
        5_000,
        |t, s| {
            let anomaly = s
                .read(FactKey::new(e(VALE), TEMPERATURE_ANOMALY))
                .and_then(|f| f.value.as_int())
                .unwrap_or(0) as f64
                / ANOMALY_SCALE as f64;
            assert_eq!(temp(s, VALE), 1_400 + anomaly.round() as i64, "tick {t}");
        },
    );
}
