//! The city bench (Amendments A-23 to A-25): Ashford with `N` generated residents — districts of
//! homes and workshops, three to a home, most working in the next district along — run for a day,
//! with attention on the first `SCENE` districts from the start. Those districts are built with
//! doors and gates and their people live as organisms; everyone else is a record at the city tier;
//! and all day people cross between the two as they go to work and come home.
//!
//! Reports what loading costs, what a tick costs on average and at worst, how many facts the city
//! holds, and how many crossed into the scene and out of it: whether a city of a million, with a
//! live scene in it, fits a desktop.
//!
//! ```text
//! TICK_SECONDS=60 SCENE=1 cargo run --release -p reference --example city -- 100000 1000000
//! ```
//!
//! `METERS=1` breaks a tick down by system; `DAYS` runs longer than a day; `HOMES` sets homes to a district (1,000, three
//! residents to each), and so how many people a district in the scene holds.

use kernel::system::CommittedView;
use reference::generate::{self, CitySpec};
use reference::{with_tick_seconds, City};
use std::time::Instant;

fn env(name: &str, default: u64) -> u64 {
    std::env::var(name)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

fn main() {
    let sizes: Vec<u64> = std::env::args()
        .skip(1)
        .filter_map(|a| a.parse().ok())
        .collect();
    let sizes = if sizes.is_empty() {
        vec![10_000, 100_000]
    } else {
        sizes
    };
    let tick_seconds = env("TICK_SECONDS", 60);
    let scene = env("SCENE", 1);
    let ticks = env("DAYS", 1) * 86_400 / tick_seconds;
    println!(
        "{:>9} {:>7} {:>8} {:>9} {:>9} {:>10} {:>10} {:>8} {:>8} {:>9}",
        "residents",
        "load s",
        "ms/tick",
        "worst ms",
        "segments",
        "facts",
        "organisms",
        "in",
        "out",
        "hungriest"
    );
    for n in sizes {
        let mut pkg = with_tick_seconds(tick_seconds);
        pkg.physical_rules.temperature_variability_centi_c = 0;
        let town = generate::city(
            &mut pkg,
            CitySpec {
                residents: n,
                homes_per_district: env("HOMES", 1_000),
                per_home: 3,
                per_workshop: 20,
                walkable_districts: scene,
            },
        );
        let t = Instant::now();
        let mut city = City::from(pkg);
        let loaded = t.elapsed();
        for &district in town.districts.iter().take(scene as usize) {
            city.attend(district, society::schema::SCENE);
        }
        let (mut total, mut worst) = (0.0f64, 0.0f64);
        let (mut segments, mut into, mut out_of) = (0u64, 0u64, 0u64);
        let mut by_system: std::collections::BTreeMap<&str, u128> = Default::default();
        for _ in 0..ticks {
            city.chronicle.clear();
            let t = Instant::now();
            city.run(1);
            let ms = t.elapsed().as_secs_f64() * 1_000.0;
            total += ms;
            worst = worst.max(ms);
            for c in &city.chronicle {
                let (fact, why) = (c.fact_type(), c.cause().event());
                if fact == society::schema::RESIDENT_UNTIL {
                    segments += 1;
                } else if fact == living::schema::HUNGER && why == "crossed" {
                    into += 1;
                } else if fact == society::schema::RESIDENT_IN_SCENE && why == "left_the_scene" {
                    out_of += 1;
                }
            }
            for m in city.meters() {
                *by_system.entry(m.system.name()).or_default() += m.nanos;
            }
        }
        let organisms = city
            .store()
            .entities_with(society::schema::RESIDENT_IN_SCENE)
            .len();
        // The hungriest anyone is at the end, at either tier: a city that cannot feed itself shows
        // here first.
        let hungriest = town
            .residents
            .iter()
            .filter_map(|&(who, _, _)| {
                city.int(who, society::schema::RESIDENT_HUNGER)
                    .or_else(|| city.int(who, living::schema::HUNGER))
            })
            .max()
            .unwrap_or(0);
        println!(
            "{:>9} {:>7.1} {:>8.2} {:>9.2} {:>9} {:>10} {:>10} {:>8} {:>8} {:>9}",
            n,
            loaded.as_secs_f64(),
            total / ticks as f64,
            worst,
            segments,
            city.store().len(),
            organisms,
            into,
            out_of,
            hungriest
        );
        if std::env::var("METERS").is_ok() {
            let mut rows: Vec<_> = by_system.into_iter().collect();
            rows.sort_by(|a, b| b.1.cmp(&a.1));
            for (name, nanos) in rows.iter().take(8) {
                println!(
                    "    {:>8.2} ms/tick  {name}",
                    *nanos as f64 / 1e6 / ticks as f64
                );
            }
        }
    }
}
