//! Labour that takes from the world (Amendment A-45): Millside feeds itself — the farmer carries
//! wheat across open ground to the mill, the miller grinds it while the Brack runs, his daughter
//! bakes the flour — with the miller's loaves no longer filling themselves. A miner fetches a
//! hammer before working the iron seam; a digger digs clay where the ground is clay; and Carol
//! mends her damaged crate with logs from the woodpile and the hammer.
//!
//! And every store filled by someone (Amendment A-47): nothing made fills itself; the Hill folk
//! keep the forge in ore and charcoal, the smith makes bars, the quarry sends granite to the mill
//! and the wood logs to Alice's yard; the steward fetches the manor's bread from the mill; the
//! harbour eats fish; and after a gale, Carol mends her crate from the woodpile.

use kernel::fact::{Cause, FactKey, FactType, SystemId};
use kernel::proposal::{Change, Proposal};
use kernel::system::{Cadence, CommittedView, System, TickContext};
use kernel::value::Value;
use packages::model::{
    BodySpec, ContainmentSpec, DepositSpec, JobSpec, JobWork, MadeOfSpec, PositionSpec,
};
use packages::WorldPackage;
use physical::schema::{CONTAINED_IN, INTEGRITY, MADE_OF};
use reference::id::*;
use reference::{e, well_fed, with_tick_seconds, City};
use std::collections::BTreeSet;

const MILL: u64 = 604;
const GALLERY: u64 = 401;
const MINE_MOUTH: u64 = 400;
const HILL: u64 = 12;
const IRON_SEAM: u64 = 3120;
const WHEAT: u64 = 717;
const IRON_ORE: u64 = 715;
const CLAY: u64 = 713;
const TIMBER: u64 = 700;
const FARMER: u64 = 2060;
const BAKER: u64 = 2063;

/// What lies in `place` that is made of `material`.
fn lying_in(city: &City, place: u64, material: u64) -> usize {
    let s = city.store();
    s.entities_with(CONTAINED_IN)
        .into_iter()
        .filter(|t| {
            s.read(FactKey::new(*t, CONTAINED_IN)).map(|f| f.value) == Some(Value::Entity(e(place)))
        })
        .filter(|t| {
            s.read_all(FactKey::new(*t, MADE_OF))
                .iter()
                .any(|m| m.value == Value::Entity(e(material)))
        })
        .count()
}

/// How many things `maker` made in what has been chronicled since `from`.
fn made_by(city: &City, maker: u64, from: u64) -> usize {
    city.chronicle
        .iter()
        .filter(|c| {
            c.tick() >= from && c.subject() == e(maker) && c.fact_type() == economy::schema::MADE
        })
        .count()
}

/// Give `who` the job `job` alone, carrying `material` from `from` to `to`, at all hours.
fn carry_job(pkg: &mut WorldPackage, who: u64, job: u64, material: u64, from: u64, to: u64) {
    pkg.jobs.push(JobSpec {
        id: job,
        work: JobWork::Carry { material, from },
        to,
        keep: 2,
        hours: (0, 24),
    });
    pkg.roles.retain(|(w, _)| *w != who);
    pkg.roles.push((who, job));
}

/// Let `who` know of `things` besides what it knows.
fn tell(pkg: &mut WorldPackage, who: u64, things: &[u64]) {
    match pkg.knows.iter_mut().find(|(m, _)| *m == who) {
        Some((_, known)) => known.extend_from_slice(things),
        None => pkg.knows.push((who, things.to_vec())),
    }
}

/// Put `who` in `place` at `at`.
fn stand(pkg: &mut WorldPackage, who: u64, place: u64, at: [i64; 2]) {
    for c in &mut pkg.containment {
        if c.child_id == who {
            c.parent_id = place;
        }
    }
    for o in &mut pkg.organisms {
        if o.id == who {
            o.region_id = place;
        }
    }
    move_to(pkg, who, place, at);
}

fn move_to(pkg: &mut WorldPackage, thing: u64, place: u64, at: [i64; 2]) {
    for c in &mut pkg.containment {
        if c.child_id == thing {
            c.parent_id = place;
        }
    }
    pkg.positions.retain(|p| p.entity_id != thing);
    pkg.positions.push(PositionSpec {
        entity_id: thing,
        x: at[0],
        y: at[1],
        z: Some(0),
    });
}

#[test]
fn millside_bakes_its_own_bread() {
    // Two days at ten-minute ticks. The farmer brings wheat across the farm and Millside's open
    // ground into the mill; the miller grinds it while the Brack turns his wheel; his daughter
    // bakes the flour. Nobody in the mill goes hungry, though the loaves no longer fill
    // themselves.
    let mut city = City::from(with_tick_seconds(600));
    let mut wheat_in_the_mill = 0;
    let mut farmer_at_the_mill = false;
    for _ in 0..2 * 144 {
        city.run(1);
        wheat_in_the_mill = wheat_in_the_mill.max(lying_in(&city, MILL, WHEAT));
        farmer_at_the_mill |= city.room_of(FARMER) == MILL;
    }
    assert!(farmer_at_the_mill, "the farmer crossed to the mill");
    assert!(
        wheat_in_the_mill >= 2,
        "wheat brought in: {wheat_in_the_mill}"
    );
    let flour = made_by(&city, MILLER, 0);
    let bread = made_by(&city, BAKER, 0);
    assert!(flour >= 3, "{flour} scoops ground");
    assert!(bread >= 2, "{bread} loaves baked");
    for who in [MILLER, BAKER] {
        assert!(
            city.int(who, living::schema::HUNGER).unwrap_or(0) < 8_000,
            "{who} is not starving"
        );
    }
}

#[test]
fn a_miner_fetches_a_hammer_before_working_the_seam() {
    // Carol is set to carry iron ore from the seam to the mine mouth. Ore is too hard for her
    // hands; she knows of the hammer, lying in the gallery, and fetches it first.
    let mut pkg = well_fed(with_tick_seconds(60));
    stand(&mut pkg, CAROL, GALLERY, [0, 0]);
    move_to(&mut pkg, HAMMER, GALLERY, [-200, 100]);
    carry_job(&mut pkg, CAROL, 9_960, IRON_ORE, IRON_SEAM, MINE_MOUTH);
    tell(
        &mut pkg,
        CAROL,
        &[GALLERY, IRON_SEAM, HAMMER, HILL, MINE_MOUTH],
    );
    let mut city = City::from(pkg);
    let mut held_the_hammer = false;
    for _ in 0..6 * 60 {
        city.run(1);
        held_the_hammer |= city.read(HAMMER, CONTAINED_IN) == Some(Value::Entity(e(CAROL)));
        if lying_in(&city, MINE_MOUTH, IRON_ORE) >= 1 {
            break;
        }
    }
    assert!(held_the_hammer, "she took up the hammer");
    assert!(
        lying_in(&city, MINE_MOUTH, IRON_ORE) >= 1,
        "ore at the mine mouth"
    );
}

#[test]
fn a_digger_digs_clay_where_the_ground_is_clay() {
    // Erin is set to carry clay from the yard to the kitchen. Nothing made of clay is anywhere she
    // knows — but she knows the yard's ground is clay. She digs it, by hand, and carries it in.
    let mut pkg = well_fed(with_tick_seconds(60));
    carry_job(&mut pkg, ERIN, 9_961, CLAY, YARD, KITCHEN);
    let mut city = City::from(pkg);
    for _ in 0..4 * 60 {
        city.run(1);
        if lying_in(&city, KITCHEN, CLAY) >= 1 {
            break;
        }
    }
    assert!(
        city.store().entities_with(physical::schema::PIT).len() >= 1,
        "she broke ground"
    );
    assert!(lying_in(&city, KITCHEN, CLAY) >= 1, "clay in the kitchen");
}

/// Half the crate's integrity knocked out of it at the first tick.
struct Knock;

const KNOCK_WRITES: &[FactType] = &[INTEGRITY];

impl System for Knock {
    fn id(&self) -> SystemId {
        SystemId::new("test.knock")
    }
    fn reads(&self) -> &'static [FactType] {
        &[]
    }
    fn writes(&self) -> &'static [FactType] {
        KNOCK_WRITES
    }
    fn cadence(&self) -> Cadence {
        Cadence::EveryTick
    }
    fn evaluate(&self, _view: &dyn CommittedView, ctx: &TickContext) -> Vec<Proposal> {
        if ctx.tick() != 1 {
            return Vec::new();
        }
        vec![Proposal::new(
            self.id(),
            FactKey::new(e(CRATE), INTEGRITY),
            ctx.basis_tick(),
            Change::Delta(-5_000),
            Cause::new("knocked"),
        )]
    }
}

#[test]
fn carol_mends_her_crate_with_logs_from_the_woodpile() {
    // The crate is Carol's, and half smashed. A woodpile stands in the yard, and the hammer lies
    // by the crate. With no job to call her, she wakes at dawn, fetches the hammer, and mends the
    // crate with what timber is to hand — the torch beside it, then logs from the pile, one at a
    // time — until it is whole. Never the shed, the wardrobe, or the door: fixtures, an opening.
    let mut pkg = well_fed(with_tick_seconds(60));
    pkg.roles.retain(|(w, _)| *w != CAROL);
    pkg.owners.push((CRATE, CAROL));
    let pile = 9_500;
    pkg.containment.push(ContainmentSpec {
        child_id: pile,
        parent_id: YARD,
    });
    pkg.positions.push(PositionSpec {
        entity_id: pile,
        x: 400,
        y: -600,
        z: Some(0),
    });
    pkg.bodies.push(BodySpec {
        entity_id: pile,
        half_width: 60,
        half_depth: 40,
        height: 60,
    });
    pkg.made_of.push(MadeOfSpec {
        object_id: pile,
        material_id: TIMBER,
    });
    pkg.deposits.push(DepositSpec {
        id: pile,
        made_of: TIMBER,
        size: [50, 8, 8],
        per_day: 0,
        cap: 20,
        stock: 20,
        ..Default::default()
    });
    tell(&mut pkg, CAROL, &[CRATE, HAMMER, pile]);
    let mut city = City::from(pkg);
    city.attach(Box::new(Knock));
    let mut mended_at = None;
    for t in 0..8 * 60 {
        city.run(1);
        if t > 5 && city.read(CRATE, INTEGRITY).is_none() {
            mended_at = Some(t);
            break;
        }
    }
    assert!(mended_at.is_some(), "the crate is whole again");
    let logs = 20 - city.int(pile, resources::schema::STOCK).unwrap_or(0) / resources::schema::UNIT;
    assert!(logs >= 2, "{logs} logs went to it");
}

const CABIN: u64 = 402;
const WOODPILE: u64 = 112;
const BREAD: u64 = 709;
const HALL: u64 = 301;
const IRON: u64 = 702;
const CHARCOAL: u64 = 719;
const GRANITE: u64 = 703;
const FISH: u64 = 720;
const LOG: u64 = 700;
const JORY: u64 = 2080;
const KIT: u64 = 2081;
const WOODCUTTER: u64 = 2083;
const NELL: u64 = 2084;
const SMITH: u64 = 2070;
const STEWARD: u64 = 2015;
const GUARD: u64 = 2052;
const CAPTAIN: u64 = 2050;
const BOSUN: u64 = 2051;

#[test]
fn nothing_made_fills_itself() {
    // Every store of what people make — bread, flour, smoked fish, a pie — is filled only by
    // what is brought to it (Amendment A-47). What renews is what grows: fruit, berries, wheat,
    // timber, fish.
    let pkg = reference::package();
    let grows = [704, 708, 717, 700, 720];
    for d in &pkg.deposits {
        let renews = d.per_day > 0 || d.renewal == Some(4);
        assert!(
            !renews || grows.contains(&d.made_of),
            "deposit {} of {} fills itself",
            d.id,
            d.made_of
        );
    }
}

/// How many things made of `material` lie in `place`, at most, over `days` at ten-minute ticks;
/// and the city at the end.
fn most_in(days: u64, watch: &[(u64, u64)]) -> (Vec<usize>, City) {
    let mut city = City::from(with_tick_seconds(600));
    let mut most = vec![0; watch.len()];
    for _ in 0..days * 144 {
        city.run(1);
        for (i, (place, material)) in watch.iter().enumerate() {
            most[i] = most[i].max(lying_in(&city, *place, *material));
        }
    }
    (most, city)
}

#[test]
fn the_hill_folk_keep_the_forge_and_the_mill_supplied() {
    // A working day at minute ticks (a mind takes a go a tick: at ten-minute ticks a lump of ore
    // takes hours, the known limit of minds that count in ticks): Jory brings ore from the seam
    // to the cabin, Kit fells logs and burns charcoal for it, and the smith makes iron bars of
    // them at the forge; Nell cuts granite on the Hill and brings it to the mill; Lena stacks logs
    // in Alice's yard.
    let mut city = City::from(with_tick_seconds(60));
    let mut most = [0; 2];
    for _ in 0..20 * 60 {
        city.run(1);
        most[0] = most[0].max(lying_in(&city, MILL, GRANITE));
        most[1] = most[1].max(lying_in(&city, WOODPILE, LOG));
    }
    let seam = city.int(IRON_SEAM, resources::schema::STOCK).unwrap_or(0) / 100;
    let charcoal = made_by(&city, KIT, 0);
    let bars = made_by(&city, SMITH, 0);
    let report = format!(
        "seam {seam}, charcoal {charcoal}, bars {bars}, granite {}, logs {}",
        most[0], most[1]
    );
    assert!(seam <= 1_996, "{report}");
    assert!(charcoal >= 1, "{report}");
    assert!(bars >= 1, "{report}");
    assert!(most[0] >= 2, "{report}");
    assert!(most[1] >= 2, "{report}");
    let _ = (JORY, WOODCUTTER, NELL, IRON, CABIN, CHARCOAL);
}

#[test]
fn the_steward_fetches_the_manors_bread_from_the_mill() {
    // The pantry no longer fills itself: the steward walks to the mill for bread and brings it
    // to the hall, and Ned and he eat it.
    let (most, _city) = most_in(2, &[(HALL, BREAD)]);
    assert!(most[0] >= 2, "bread in the hall: {}", most[0]);
    let _ = STEWARD;
}

#[test]
fn the_harbour_eats_fish() {
    // The ship's stores and the fish barrel are bare. The guard fishes off the quay; the Heron's
    // crew haul their nets. Over two days each eats fish, and none goes hungry.
    let mut pkg = with_tick_seconds(600);
    for d in &mut pkg.deposits {
        if d.id == 3106 || d.id == 3107 {
            d.stock = 0;
        }
    }
    let mut city = City::from(pkg);
    let mut ate: BTreeSet<u64> = BTreeSet::new();
    let mut seen = 0;
    for _ in 0..2 * 144 {
        city.run(1);
        for c in &city.chronicle[seen..] {
            if c.fact_type() == physical::schema::CONSUMED {
                if let Some(Value::Entity(t)) =
                    city.read(c.subject().raw(), physical::schema::CONSUMED)
                {
                    let fish = city
                        .store()
                        .read_all(FactKey::new(t, MADE_OF))
                        .iter()
                        .any(|m| m.value == Value::Entity(e(FISH)));
                    if fish {
                        ate.insert(c.subject().raw());
                    }
                }
            }
        }
        seen = city.chronicle.len();
    }
    for who in [GUARD, CAPTAIN, BOSUN] {
        assert!(ate.contains(&who), "{who} ate no fish: {ate:?}");
        assert!(city.int(who, living::schema::HUNGER).unwrap_or(0) < 8_000);
    }
}

#[test]
fn after_a_gale_carol_mends_her_crate_from_the_woodpile() {
    // A 32 m/s gale over the Vale for an hour in the night presses just past what timber bears,
    // and tears at the crate, which is Carol's. The woodcutter stacks logs on the woodpile in
    // Alice's yard from the morning, for just this. Carol takes up the hammer and works the torch and the logs into
    // the crate until it is whole. (The gale takes a little off the house too. Nobody here calls
    // it home, or they would be at it all week: a house is tonnes of timber.)
    let mut pkg = well_fed(with_tick_seconds(60));
    pkg.physical_rules.temperature_variability_centi_c = 0;
    pkg.storms.push((VALE, 1, 1, 3_200));
    pkg.owners.push((CRATE, CAROL));
    pkg.home.clear();
    // The gale tears at the sawpit too, the woodcutter's own; he would mend that first, with
    // the day's logs. Here it is no one's.
    pkg.owners.retain(|(thing, _)| *thing != 622);
    let mut city = City::from(pkg);
    let mut torn = 0;
    let mut logs = 0;
    let mut whole_again = false;
    for _ in 0..24 * 60 {
        city.run(1);
        let worn = 10_000 - city.int(CRATE, INTEGRITY).unwrap_or(10_000);
        torn = torn.max(worn);
        logs = logs.max(lying_in(&city, WOODPILE, LOG));
        if torn > 0 && worn == 0 {
            whole_again = true;
            break;
        }
    }
    assert!(torn >= 1_000, "the gale tore at it: {torn}");
    assert!(logs >= 1, "logs stacked on the woodpile");
    assert!(whole_again, "whole again");
}
