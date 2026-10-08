//! Ashford's climate (Vol. III Ch. 1 §1.9–1.10; Amendments A-1, A-5): weather simulated once
//! per declared climate and inherited down the nesting; walled rooms sheltered, lit only through
//! their openings, their air lagging the outside; and the same day, the same weather, at any
//! tick length. And the year turns: seasons, longer and shorter days, and a moon (Amendment A-34).

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
use reference::{e, with_tick_seconds, without_minds, City};

/// The climates Ashford declares that are open to the sky.
const OUTDOOR: [u64; 3] = [VALE, HIGHMOOR, SOUTHFEN];

/// Run some physical systems alone over a copy of the reality `pkg` loads, for `ticks` ticks, calling `each` after every tick — so one effect can be measured in isolation.
fn isolated(
    pkg: packages::WorldPackage,
    only: &[&str],
    ticks: u64,
    mut each: impl FnMut(u64, &MemoryStore),
) -> MemoryStore {
    let city = City::from(without_minds(pkg.clone()));
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
        let c = City::from(without_minds(pkg.clone()));
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
    let mut city = City::quiet();
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
    // Ten-minute ticks; noon is tick 72. Ashford begins a month after the spring equinox, at 52°
    // north: the noon sun stands 49.6° high, and lights open ground at 76% of its peak
    // (Amendment A-34).
    let mut city = City::from(without_minds(with_tick_seconds(600)));
    city.run(72);
    let light = |city: &City, id| city.int(id, ILLUMINATION).unwrap_or(0);
    assert_eq!(light(&city, VALE), 7_631, "the noon sun outdoors");
    // The kitchen: the open front door (90 cm × 2.05 m) and the low window (40 × 50 cm) onto
    // the yard, against a 10 m × 10 m floor — about 2% of daylight.
    assert_eq!(light(&city, KITCHEN), 155);
    assert_eq!(light(&city, BEDROOM), 61, "one shut glass window");
    assert_eq!(light(&city, CELLAR), 91, "the open bulkhead");
    assert_eq!(light(&city, VAULT), 0, "no openings at all");
    // The mine, by its declared exposure.
    assert_eq!(light(&city, MINE_MOUTH), 3_052);
    assert_eq!(light(&city, CABIN), 763);
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
    // The yard swings the full 5 °C, coldest at five and warmest at three (Amendment A-35). The
    // kitchen follows with 2.07 °C, peaking 4 h 40 min after the yard; the granite hall with
    // 1.64 °C, 5 h 30 min after.
    assert_eq!([swing(vale), swing(kitchen), swing(hall)], [500, 207, 164]);
    assert_eq!(
        [peak(kitchen) - peak(vale), peak(hall) - peak(vale)],
        [28, 33]
    );
}

#[test]
fn pressure_falls_with_height_and_the_wind_blows_toward_the_moor() {
    let mut city = City::from(without_minds(with_tick_seconds(600)));
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
    // Nothing accumulates from rounding: at every step the Vale's air is what it began at — its
    // declared 14 °C, plus where the day's swing stood at midnight (Amendment A-35) — plus the
    // current weather anomaly, rounded.
    let start = temp(
        City::from(without_minds(with_tick_seconds(60))).store(),
        VALE,
    );
    assert_eq!(
        start, 1_578,
        "14 °C, and midnight 5/14 of the way back from three o'clock"
    );
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
            assert_eq!(temp(s, VALE), start + anomaly.round() as i64, "tick {t}");
        },
    );
}

#[test]
fn the_year_turns_warm_and_back_again() {
    // A year of the seasons alone, a day a tick (Amendment A-34). Ashford begins a month after
    // the spring equinox, at its year's mean. The Vale, open sky with nothing to store the heat,
    // is warmest three months on — a month after midsummer — 7 °C up, and coldest half a year
    // after that, 7 °C down; granite Highmoor answers less. A year on, both are where they began.
    let (mut vale, mut moor) = (Vec::new(), Vec::new());
    let start = temp(
        City::from(without_minds(with_tick_seconds(86_400))).store(),
        VALE,
    );
    isolated(
        with_tick_seconds(86_400),
        &["physical.seasons"],
        365,
        |_, s| {
            vale.push(temp(s, VALE));
            moor.push(temp(s, HIGHMOOR));
        },
    );
    // The middle of the days it stands at its best, in whole centidegrees.
    let day_of = |v: &[i64], best: i64| {
        let days: Vec<i64> = (0..v.len())
            .filter(|d| v[*d] == best)
            .map(|d| d as i64 + 1)
            .collect();
        (days[0] + days[days.len() - 1]) / 2
    };
    let (warm, cold) = (*vale.iter().max().unwrap(), *vale.iter().min().unwrap());
    assert!(
        (89..=93).contains(&day_of(&vale, warm)),
        "{}",
        day_of(&vale, warm)
    );
    assert!(
        (272..=276).contains(&day_of(&vale, cold)),
        "{}",
        day_of(&vale, cold)
    );
    assert!(
        (698..=700).contains(&(warm - start)) && (698..=700).contains(&(start - cold)),
        "{warm} {cold} from {start}"
    );
    let swing = |v: &[i64]| v.iter().max().unwrap() - v.iter().min().unwrap();
    assert!(swing(&moor) < swing(&vale), "stone answers less");
    assert_eq!(vale[364], start, "a year on");
}

#[test]
fn the_seasons_are_the_same_at_any_tick_length() {
    // Ten days of seasons at hour ticks and at ten-minute ticks: the same air, exactly.
    let after = |tick_seconds: u64| {
        let s = isolated(
            with_tick_seconds(tick_seconds),
            &["physical.seasons"],
            10 * 86_400 / tick_seconds,
            |_, _| {},
        );
        [temp(&s, VALE), temp(&s, HIGHMOOR), temp(&s, SOUTHFEN)]
    };
    let hourly = after(3_600);
    assert_eq!(hourly, after(600));
    assert!(hourly[0] > 1_400, "spring warms: {hourly:?}");
}

#[test]
fn the_days_lengthen_toward_midsummer() {
    // Late April at 52° north: fourteen hours of daylight. Two months on, at midsummer, sixteen
    // and a half. (Without the moon, whose light would count as day.)
    let mut lit = vec![0; 62];
    let mut pkg = with_tick_seconds(600);
    pkg.physical_rules.lunar_month_seconds = 0;
    isolated(pkg, &["physical.day_night_cycle"], 62 * 144, |t, s| {
        let light = s
            .read(FactKey::new(e(VALE), ILLUMINATION))
            .and_then(|f| f.value.as_int())
            .unwrap_or(0);
        if light > 0 {
            lit[((t - 1) / 144) as usize] += 1;
        }
    });
    let hours = |ticks: i64| ticks as f64 / 6.0;
    assert!((13.8..=14.2).contains(&hours(lit[0])), "{}", hours(lit[0]));
    assert!(
        (16.3..=16.7).contains(&hours(lit[61])),
        "{}",
        hours(lit[61])
    );
}

#[test]
fn a_full_moon_lights_the_yard_and_a_new_moon_does_not() {
    // Midnight, as the world begins. Under a full moon, standing 26° high in a late-April sky,
    // Erin in the yard can make out the apple tree; under Ashford's own new moon she cannot.
    let sees_the_tree = |phase: i64| {
        let mut pkg = without_minds(with_tick_seconds(60));
        pkg.physical_rules.moon_phase_at_start = phase;
        let mut city = City::from(pkg);
        city.run(2);
        let lit = city.int(VALE, ILLUMINATION).unwrap_or(0);
        let seen = city
            .store()
            .read_all(FactKey::new(e(ERIN), physical::schema::IN_VIEW))
            .iter()
            .any(|f| f.value == kernel::value::Value::Entity(e(TREE)));
        (lit, seen)
    };
    let (full, seen) = sees_the_tree(5_000);
    assert!(full >= 50, "moonlight {full}");
    assert!(seen, "by the full moon");
    assert_eq!(sees_the_tree(0), (0, false), "no moon");
}
