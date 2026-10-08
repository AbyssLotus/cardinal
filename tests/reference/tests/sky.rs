//! The sky over Ashford (Vol. III Ch. 1 §1.10–1.11; Amendment A-35): cloud gathers in humid air,
//! dims the day, and holds back the day's warmth and the night's cold; saturated air rains, and
//! below the snow line it snows; fog lies over damp ground at dawn in still air, lifts as the day
//! warms, and shortens sight while it lies.

use kernel::domain::Domain;
use kernel::events::ChronicleEntry;
use kernel::fact::FactKey;
use kernel::store::MemoryStore;
use kernel::system::{CommittedView, System};
use kernel::tick::run_tick;
use kernel::value::Value;
use packages::WorldPackage;
use physical::schema::{CLOUD, FOG, ILLUMINATION, IN_VIEW, RAIN, SNOW, TEMPERATURE};
use physical::PhysicalDomain;
use reference::id::*;
use reference::{e, with_tick_seconds, without_minds, City};

/// Southfen.
const SOUTHFEN: u64 = 3;
/// Ten-minute ticks: six to the hour.
const HOUR: u64 = 6;

/// Ashford without minds at ten-minute ticks, with steady weather — and so still air (Amendment
/// A-36) — and the Vale's air at `humidity`.
fn ashford(humidity: i64) -> WorldPackage {
    let mut pkg = without_minds(with_tick_seconds(600));
    pkg.physical_rules.temperature_variability_centi_c = 0;
    pkg.physical_rules.humidity_variability = 0;
    pkg.physical_rules.pressure_variability = 0;
    pkg.physical_rules.humidity_baseline = humidity;
    pkg
}

fn int(s: &dyn CommittedView, id: u64, fact: kernel::fact::FactType) -> i64 {
    s.read(FactKey::new(e(id), fact))
        .and_then(|f| f.value.as_int())
        .unwrap_or(0)
}

#[test]
fn cloud_gathers_in_humid_air_and_dims_the_day() {
    // At 85% the Vale's sky is three-quarters cloud, which keeps out three-quarters of what it
    // covers: at noon, 44% of the clear day's light.
    let noon = |humidity: i64| {
        let mut city = City::from(ashford(humidity));
        city.run(12 * HOUR);
        (
            int(city.store(), VALE, CLOUD),
            int(city.store(), VALE, ILLUMINATION),
        )
    };
    let (clear_cloud, clear) = noon(6_000);
    let (cloud, dim) = noon(8_500);
    assert_eq!((clear_cloud, clear), (0, 7_631));
    assert_eq!(cloud, 7_500);
    assert_eq!(dim, 7_631 - 7_631 * 7_500 / 10_000 * 7_500 / 10_000);
}

#[test]
fn an_overcast_keeps_the_night_mild_and_the_day_cool() {
    // The Vale's second day, clear and under a full overcast: the overcast holds back 60% of the
    // day's 5 °C swing. (The season adds its own few hundredths of a degree a day.)
    let swing = |humidity: i64| {
        let mut city = City::from(ashford(humidity));
        city.run(24 * HOUR);
        let (mut lo, mut hi) = (i64::MAX, i64::MIN);
        for _ in 0..24 * HOUR {
            city.run(1);
            let t = int(city.store(), VALE, TEMPERATURE);
            (lo, hi) = (lo.min(t), hi.max(t));
        }
        hi - lo
    };
    let clear = swing(6_000);
    let overcast = swing(9_000);
    assert!((500..=520).contains(&clear), "{clear}");
    assert!((200..=220).contains(&overcast), "{overcast}");
}

/// Run some physical systems alone over the reality `pkg` loads, for `ticks` ticks, calling
/// `each` after every tick.
fn isolated(pkg: WorldPackage, only: &[&str], ticks: u64, mut each: impl FnMut(u64, &MemoryStore)) {
    let city = City::from(pkg.clone());
    let mut store = city.store().clone();
    let domain = PhysicalDomain::new(packages::physical_config(&pkg));
    let domains: [&dyn Domain; 1] = [&domain];
    let systems: Vec<Box<dyn System>> = domain
        .systems()
        .into_iter()
        .filter(|s| only.contains(&s.id().name()))
        .collect();
    let mut chronicle: Vec<ChronicleEntry> = Vec::new();
    for t in 1..=ticks {
        run_tick(&mut store, &domains, &systems, t, 7, &mut chronicle).expect("commits");
        chronicle.clear();
        each(t, &store);
    }
}

#[test]
fn cloud_comes_and_goes_and_leaves_nothing_behind() {
    // Changeable weather over ten days, the day and the sky alone, at one-minute ticks. At every
    // step the Vale's air is what it began at, plus the day's swing since, less exactly the share
    // of it the cloud committed then holds back: nothing accumulates as cloud comes and goes.
    let mut pkg = without_minds(with_tick_seconds(60));
    pkg.physical_rules.humidity_variability = 2_000;
    let config = packages::physical_config(&pkg);
    let curve = config.day_curve();
    let damp = config.sky.cloud_damp;
    let start = int(City::from(pkg.clone()).store(), VALE, TEMPERATURE);
    let (mut cloudy, mut clear) = (0, 0);
    isolated(
        pkg,
        &["physical.diurnal_cycle", "physical.precipitation"],
        10 * 1_440,
        |t, s| {
            let at = config.clock.ms_at(t);
            let swing = curve.level(at) - curve.amplitude_centi_c / 2;
            let cloud = int(s, VALE, CLOUD);
            let held = -swing * cloud / 10_000 * damp / 10_000;
            let expected = start + curve.level(at) - curve.level(0) + held;
            assert_eq!(int(s, VALE, TEMPERATURE), expected, "tick {t}");
            if cloud > 0 {
                cloudy += 1;
            } else {
                clear += 1;
            }
        },
    );
    assert!(
        cloudy > 1_000 && clear > 1_000,
        "{cloudy} cloudy, {clear} clear"
    );
}

#[test]
fn saturated_air_rains_and_in_frost_it_snows() {
    // At 95% it pours 5 mm an hour on the Vale. Declared at five below, the same air brings snow.
    let mut city = City::from(ashford(9_500));
    city.run(2);
    assert_eq!(int(city.store(), VALE, RAIN), 500);
    assert_eq!(int(city.store(), VALE, SNOW), 0);
    let mut pkg = ashford(9_500);
    pkg.regions
        .iter_mut()
        .find(|r| r.id == VALE)
        .unwrap()
        .temperature_centi_c = -500;
    let mut city = City::from(pkg);
    city.run(2);
    assert_eq!(int(city.store(), VALE, SNOW), 500);
    assert_eq!(city.read(VALE, RAIN), None);
    // And dry air rains on nobody.
    let mut city = City::from(ashford(6_000));
    city.run(2);
    assert_eq!(city.read(VALE, RAIN), None);
}

#[test]
fn fog_lies_over_the_fens_at_dawn_and_lifts_by_afternoon() {
    // Southfen at 86%, in still air: at five, the coldest of the day, the air near the ground is
    // past 90% and fog lies over the fens; by three it has lifted.
    let mut pkg = ashford(6_000);
    pkg.humidities.retain(|(c, _)| *c != SOUTHFEN);
    pkg.humidities.push((SOUTHFEN, 8_600));
    let mut city = City::from(pkg);
    city.run(5 * HOUR);
    let dawn = int(city.store(), SOUTHFEN, FOG);
    assert!(dawn > 3_000, "fog at dawn {dawn}");
    city.run(10 * HOUR);
    assert_eq!(city.read(SOUTHFEN, FOG), None, "lifted by three");
    // The Vale's drier air makes none.
    assert_eq!(city.read(VALE, FOG), None);
}

#[test]
fn fog_hides_the_apple_tree() {
    // Six in the morning, Erin in the yard. In clear air she sees the apple tree fourteen metres
    // off; in the fog of a damp dawn she cannot, though it is light enough. By noon the fog has
    // lifted and she sees it again.
    let sees_the_tree = |city: &City| {
        city.store()
            .read_all(FactKey::new(e(ERIN), IN_VIEW))
            .iter()
            .any(|f| f.value == Value::Entity(e(TREE)))
    };
    let mut clear = City::from(ashford(6_000));
    clear.run(6 * HOUR);
    assert!(sees_the_tree(&clear), "in clear air");
    let mut foggy = City::from(ashford(8_900));
    foggy.run(6 * HOUR);
    assert!(int(foggy.store(), VALE, FOG) > 3_000);
    assert!(
        int(foggy.store(), VALE, ILLUMINATION) > 50,
        "light enough to see by"
    );
    assert!(!sees_the_tree(&foggy), "lost in the fog");
    foggy.run(6 * HOUR);
    assert!(sees_the_tree(&foggy), "the fog lifted");
}
