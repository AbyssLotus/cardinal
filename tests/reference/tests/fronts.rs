//! Wind and fronts (Vol. III Ch. 1 §1.10, "wind flows"; Amendment A-36): the wind blows down the
//! difference in neighbouring climates' pressure brought to sea level, so it follows the weather
//! and not the lie of the land; and a climate's weather mixes slowly with its neighbours', and
//! rides the wind into the climate it blows toward.

use kernel::fact::{Cause, FactKey, FactType, SystemId};
use kernel::proposal::{Change, Proposal};
use kernel::store::RealityStore;
use kernel::system::{Cadence, CommittedView, System, TickContext};
use kernel::value::Value;
use packages::WorldPackage;
use physical::schema::{
    ANOMALY_SCALE, PRESSURE_ANOMALY, TEMPERATURE_ANOMALY, WIND_SPEED, WIND_TOWARD,
};
use reference::id::*;
use reference::{e, with_tick_seconds, without_minds, City};

const SOUTHFEN: u64 = 3;
/// Ten-minute ticks: six to the hour.
const HOUR: u64 = 6;

/// Ashford without minds at ten-minute ticks, with no weather of its own.
fn steady() -> WorldPackage {
    let mut pkg = without_minds(with_tick_seconds(600));
    pkg.physical_rules.temperature_variability_centi_c = 0;
    pkg.physical_rules.humidity_variability = 0;
    pkg.physical_rules.pressure_variability = 0;
    pkg
}

/// Weather over the moor on the first tick: its pressure `pressure` decapascals from normal, and
/// its air `cold` centidegrees colder.
struct Spell {
    pressure: i64,
    cold: i64,
}

const SPELL_WRITES: &[FactType] = &[PRESSURE_ANOMALY, TEMPERATURE_ANOMALY];

impl System for Spell {
    fn id(&self) -> SystemId {
        SystemId::new("test.spell")
    }
    fn reads(&self) -> &'static [FactType] {
        &[]
    }
    fn writes(&self) -> &'static [FactType] {
        SPELL_WRITES
    }
    fn cadence(&self) -> Cadence {
        Cadence::EveryTick
    }
    fn evaluate(&self, _view: &dyn CommittedView, ctx: &TickContext) -> Vec<Proposal> {
        if ctx.tick() != 1 {
            return Vec::new();
        }
        [
            (PRESSURE_ANOMALY, self.pressure),
            (TEMPERATURE_ANOMALY, -self.cold),
        ]
        .into_iter()
        .map(|(fact, by)| {
            Proposal::new(
                self.id(),
                FactKey::new(e(HIGHMOOR), fact),
                ctx.basis_tick(),
                Change::Delta(by * ANOMALY_SCALE),
                Cause::new("spell"),
            )
        })
        .collect()
    }
}

fn anomaly(city: &City, region: u64, fact: FactType) -> i64 {
    city.int(region, fact).unwrap_or(0) / ANOMALY_SCALE
}

#[test]
fn a_low_over_the_moor_draws_the_wind_toward_it() {
    // A low 200 decapascals deep over Highmoor: the Vale's air flows toward it at 3 cm/s for each
    // decapascal, about 6 m/s; the moor, lowest of all, is calm.
    let mut city = City::from(steady());
    city.attach(Box::new(Spell {
        pressure: -200,
        cold: 0,
    }));
    city.run(3);
    assert_eq!(
        city.read(VALE, WIND_TOWARD),
        Some(Value::Entity(e(HIGHMOOR)))
    );
    let wind = city.int(VALE, WIND_SPEED).unwrap();
    assert!((550..=600).contains(&wind), "{wind}");
    assert_eq!(city.int(HIGHMOOR, WIND_SPEED), Some(0));
    // A high instead, and the wind blows off the moor into the Vale.
    let mut city = City::from(steady());
    city.attach(Box::new(Spell {
        pressure: 200,
        cold: 0,
    }));
    city.run(3);
    assert_eq!(
        city.read(HIGHMOOR, WIND_TOWARD),
        Some(Value::Entity(e(VALE)))
    );
}

/// The coldest the Vale's and Southfen's weather runs over the half-day after a spell 6 °C cold
/// over the moor, with its pressure `pressure` from normal.
fn coldest_after(pkg: WorldPackage, pressure: i64) -> (i64, i64) {
    let mut city = City::from(pkg);
    city.attach(Box::new(Spell {
        pressure,
        cold: 600,
    }));
    let (mut vale, mut fen) = (0, 0);
    for _ in 0..12 * HOUR {
        city.run(1);
        vale = vale.min(anomaly(&city, VALE, TEMPERATURE_ANOMALY));
        fen = fen.min(anomaly(&city, SOUTHFEN, TEMPERATURE_ANOMALY));
    }
    (vale, fen)
}

#[test]
fn a_cold_high_over_the_moor_blows_its_cold_into_the_vale() {
    // Under a high, the moor's cold rides its wind down into the Vale, and on from there to the
    // fens, a little; under a low, the wind blows the other way, and the cold reaches the Vale
    // only as neighbouring weather slowly mixes.
    let (downwind, fen) = coldest_after(steady(), 200);
    let (upwind, _) = coldest_after(steady(), -200);
    assert!(
        downwind < -200,
        "the Vale under the moor's wind: {downwind}"
    );
    assert!(fen < 0 && fen > downwind, "the fens, further on: {fen}");
    assert!(upwind < 0 && upwind > -100, "the Vale upwind: {upwind}");
}

#[test]
fn without_fronts_each_climate_keeps_its_own_weather() {
    let mut pkg = steady();
    pkg.physical_rules.climate_mixing_seconds = 0;
    pkg.physical_rules.front_distance_cm = 0;
    assert_eq!(coldest_after(pkg, 200), (0, 0));
}

#[test]
fn the_same_weather_crosses_the_same_way() {
    let run = || {
        let mut pkg = without_minds(with_tick_seconds(600));
        pkg.physical_rules.pressure_variability = 120;
        let mut city = City::from(pkg);
        city.run(2 * 24 * HOUR);
        city.store().state_hash()
    };
    assert_eq!(run(), run());
}
