//! Time has units (Vol. II Ch. 2, *Simulated Duration*, Amendment A-1): tick length is a
//! resolution choice, not a rule. These are the gate tests from the 3D-readiness audit
//! (`docs/audits/3d-world-readiness.md` §4.1 and Phase 1): the same world must keep the same
//! day, the same weather, and a bounded climate whether a tick lasts an hour or a second.

use kernel::domain::Domain;
use kernel::events::ChronicleEntry;
use kernel::fact::{Cause, Fact, FactKey, FactType, Provenance, SystemId};
use kernel::identity::EntityId;
use kernel::store::MemoryStore;
use kernel::system::{CommittedView, System};
use kernel::tick::run_tick;
use kernel::time::SimClock;
use kernel::value::Value;
use physical::schema::{ANOMALY_SCALE, EXPOSURE, TEMPERATURE, TEMPERATURE_ANOMALY};
use physical::{PhysicalConfig, PhysicalDomain};

const OPEN: u64 = 1;
const SHELTERED: u64 = 2;

fn e(id: u64) -> EntityId {
    EntityId::from_raw(id)
}

fn seed(s: &mut MemoryStore, id: u64, ft: FactType, v: i64) {
    s.seed(
        FactKey::new(e(id), ft),
        Fact::new(
            Value::Int(v),
            Provenance::new(SystemId::new("worldgen"), 0, Cause::new("seed")),
        ),
    );
}

fn int(s: &MemoryStore, id: u64, ft: FactType) -> i64 {
    s.read(FactKey::new(e(id), ft))
        .and_then(|f| f.value.as_int())
        .unwrap_or(0)
}

/// A world ticking every `tick_seconds`, with its environment stepping every `step_seconds`.
fn config(tick_seconds: u64, step_seconds: u64) -> PhysicalConfig {
    PhysicalConfig {
        clock: SimClock::new(tick_seconds * 1000),
        day_length_seconds: 86_400,
        environment_step_seconds: step_seconds,
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

/// Only the system named `id`, so one effect can be measured in isolation.
fn only(domain: &PhysicalDomain, id: &str) -> Vec<Box<dyn System>> {
    domain
        .systems()
        .into_iter()
        .filter(|s| s.id().name() == id)
        .collect()
}

/// The coldest-to-warmest swing of an open and a half-sheltered region over one simulated day,
/// day/night cycle only.
fn day_swing(tick_seconds: u64) -> (i64, i64) {
    let domain = PhysicalDomain::new(config(tick_seconds, tick_seconds));
    let domains: [&dyn Domain; 1] = [&domain];
    let systems = only(&domain, "physical.diurnal_cycle");
    let mut s = MemoryStore::new();
    seed(&mut s, OPEN, TEMPERATURE, 1500);
    seed(&mut s, SHELTERED, TEMPERATURE, 1500);
    seed(&mut s, SHELTERED, EXPOSURE, 5000);
    let mut chronicle: Vec<ChronicleEntry> = Vec::new();
    let (mut lo, mut hi) = ([i64::MAX; 2], [i64::MIN; 2]);
    for tick in 1..=86_400 / tick_seconds {
        run_tick(&mut s, &domains, &systems, tick, 1, &mut chronicle).unwrap();
        chronicle.clear();
        for (i, region) in [OPEN, SHELTERED].into_iter().enumerate() {
            let t = int(&s, region, TEMPERATURE);
            lo[i] = lo[i].min(t);
            hi[i] = hi[i].max(t);
        }
    }
    (hi[0] - lo[0], hi[1] - lo[1])
}

#[test]
fn the_day_is_the_same_length_and_size_at_any_tick_length() {
    // The audit measured a half-sheltered region's swing collapse from 1.96 C to 0.00 C as
    // ticks shortened from an hour to a minute. Damping the absolute level and differencing
    // keeps it whole: 4.00 C in the open, 2.00 C at half exposure, at every resolution.
    for tick_seconds in [3600, 60, 1] {
        assert_eq!(
            day_swing(tick_seconds),
            (400, 200),
            "ticks of {tick_seconds} s"
        );
    }
}

/// The spread (standard deviation, in centidegrees) of weather's temperature anomaly across
/// `regions` independent regions after `days` simulated days, weather only.
fn weather_spread(tick_seconds: u64, regions: u64, days: u64) -> f64 {
    let domain = PhysicalDomain::new(config(tick_seconds, tick_seconds));
    let domains: [&dyn Domain; 1] = [&domain];
    let systems = only(&domain, "physical.temperature_weather");
    let mut s = MemoryStore::new();
    for r in 1..=regions {
        seed(&mut s, r, TEMPERATURE, 1500);
    }
    let mut chronicle: Vec<ChronicleEntry> = Vec::new();
    for tick in 1..=days * 86_400 / tick_seconds {
        run_tick(&mut s, &domains, &systems, tick, 11, &mut chronicle).unwrap();
        chronicle.clear();
    }
    let levels: Vec<f64> = (1..=regions)
        .map(|r| int(&s, r, TEMPERATURE_ANOMALY) as f64 / ANOMALY_SCALE as f64)
        .collect();
    let mean = levels.iter().sum::<f64>() / levels.len() as f64;
    (levels.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / levels.len() as f64).sqrt()
}

#[test]
fn the_weather_has_the_same_spread_at_any_tick_length() {
    // The world declares a 3.00 C spread. Hour-long ticks and minute-long ticks both deliver
    // it — where the old per-tick swing gave 1.6 C a day at hourly ticks and 7.9 C at
    // minute ticks. (Two hundred regions: the sample spread is within about 5% of the truth.)
    let hourly = weather_spread(3600, 200, 3);
    let by_minute = weather_spread(60, 200, 3);
    for (label, sd) in [("hourly", hourly), ("by the minute", by_minute)] {
        assert!(
            (255.0..=345.0).contains(&sd),
            "{label}: spread {sd:.0} cd, declared 300"
        );
    }
}

#[test]
fn a_decade_of_weather_stays_within_its_climate() {
    // The shipped random walk sent regions that began at 15 C to between -55 C and +74 C in a
    // single year. The weather now reverts toward normal: across a simulated decade, no region
    // strays more than six declared spreads from its normal at any step.
    let domain = PhysicalDomain::new(config(3600, 6 * 3600));
    let domains: [&dyn Domain; 1] = [&domain];
    let systems = only(&domain, "physical.temperature_weather");
    let mut s = MemoryStore::new();
    let regions = 40;
    for r in 1..=regions {
        seed(&mut s, r, TEMPERATURE, 1500);
    }
    let mut chronicle: Vec<ChronicleEntry> = Vec::new();
    let mut worst = 0i64;
    for tick in 1..=10 * 365 * 24 {
        run_tick(&mut s, &domains, &systems, tick, 5, &mut chronicle).unwrap();
        chronicle.clear();
        if tick % 6 == 0 {
            for r in 1..=regions {
                worst = worst.max((int(&s, r, TEMPERATURE) - 1500).abs());
            }
        }
    }
    assert!(worst > 300, "weather happened (worst departure {worst} cd)");
    assert!(worst < 6 * 300, "worst departure {worst} cd, bound 1800");
}

#[test]
fn temperature_is_exactly_normal_plus_weather() {
    // Telescoping means nothing accumulates: after any number of steps a region's temperature
    // is its seeded normal plus the rounded current anomaly — no residue from past rounding.
    let domain = PhysicalDomain::new(config(60, 60));
    let domains: [&dyn Domain; 1] = [&domain];
    let systems = only(&domain, "physical.temperature_weather");
    let mut s = MemoryStore::new();
    seed(&mut s, OPEN, TEMPERATURE, 1500);
    let mut chronicle: Vec<ChronicleEntry> = Vec::new();
    for tick in 1..=5_000 {
        run_tick(&mut s, &domains, &systems, tick, 3, &mut chronicle).unwrap();
        chronicle.clear();
        let anomaly = int(&s, OPEN, TEMPERATURE_ANOMALY) as f64 / ANOMALY_SCALE as f64;
        assert_eq!(
            int(&s, OPEN, TEMPERATURE),
            1500 + anomaly.round() as i64,
            "tick {tick}"
        );
    }
}
