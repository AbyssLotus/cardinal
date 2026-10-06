//! # The Ashford harness
//!
//! Every engine feature is tested in one fictional world — Ashford, a city in the Vale
//! (`worlds/ashford.world`) — rather than in a small world built per feature. A test loads the
//! city exactly as a frontend would, gives its people something to do through the front door,
//! lets the world run, and asks the engine's own queries what became of everyone.
//!
//! - [`id`] names Ashford's places, people, and things, so tests read as the world does.
//! - [`City`] is a running Ashford: its committed reality, its systems, and a front door.
//! - The **front door** ([`City::go`], [`City::open`], [`City::close`], [`City::face`]) queues
//!   orders that a decider proposes on the next tick — intents, never moves: Physical Reality
//!   alone carries them out (Appendix A, Ruling 13), and the kernel refuses any other route
//!   (Amendment A-5).
//!
//! Tests that need a different clock or a smaller domain selection edit the parsed package
//! before loading ([`package`], [`City::from`]) — the world itself never changes.

use kernel::events::ChronicleEntry;
use kernel::fact::{Cause, FactKey, FactType, SystemId};
use kernel::identity::EntityId;
use kernel::proposal::{Change, Proposal};
use kernel::store::MemoryStore;
use kernel::system::{Cadence, CommittedView, System, TickContext};
use kernel::value::Value;
use packages::{engine_version, load, parse_world, LoadedWorld, WorldPackage};
use physical::schema::{
    ACT_CLOSE, ACT_FACE, ACT_OPEN, ACT_REFUSED, CONTAINED_IN, TRAVEL_BLOCKED, TRAVEL_SPEED,
    TRAVEL_TO,
};
use std::cell::RefCell;
use std::rc::Rc;

/// The world file, as shipped.
pub const ASHFORD: &str = include_str!("../../../worlds/ashford.world");

/// Ashford's entities by name. Ranges group them: 1–14 the nesting, 100s Alice's house, 200s the
/// Hill, 300s the manor, 400s the mine, 500s the harbour, 600s Millside, 700s materials, 900s
/// overlapping regions, 1000s openings, 2000s people, animals, and things that move.
pub mod id {
    #![allow(missing_docs)]
    // The nesting.
    pub const REACH: u64 = 9;
    pub const VALE: u64 = 1;
    pub const HIGHMOOR: u64 = 2;
    pub const SOUTHFEN: u64 = 3;
    pub const ASHFORD: u64 = 10;
    pub const OLD_TOWN: u64 = 11;
    pub const HILL: u64 = 12;
    pub const HARBOUR: u64 = 13;
    pub const MILLSIDE: u64 = 14;
    // Alice's house.
    pub const YARD: u64 = 100;
    pub const HOUSE: u64 = 101;
    pub const CELLAR: u64 = 102;
    pub const KITCHEN: u64 = 103;
    pub const BEDROOM: u64 = 104;
    pub const SHED: u64 = 110;
    pub const STONE: u64 = 111;
    pub const TABLE: u64 = 120;
    pub const LAMP: u64 = 121;
    pub const HEARTH: u64 = 122;
    pub const FLOUR_BIN: u64 = 123;
    pub const APPLE: u64 = 3000;
    pub const TREE: u64 = 3100;
    pub const VIALS: [u64; 4] = [3001, 3002, 3003, 3004];
    pub const BOTTLE: u64 = 3005;
    pub const BILBERRIES: u64 = 3104;
    pub const MILLERS_LOAVES: u64 = 3109;
    pub const WAGON: u64 = 130;
    pub const CART: u64 = 140;
    pub const DOOR_OUT: u64 = 1001;
    pub const DOOR_IN: u64 = 1002;
    pub const WINDOW_OUT: u64 = 1003;
    pub const WINDOW_IN: u64 = 1004;
    pub const BULKHEAD_OUT: u64 = 1005;
    pub const BULKHEAD_IN: u64 = 1006;
    pub const STAIRS_UP: u64 = 1007;
    pub const STAIRS_DOWN: u64 = 1008;
    pub const CELLAR_DOWN: u64 = 1009;
    pub const CELLAR_UP: u64 = 1010;
    pub const BEDROOM_WINDOW: u64 = 1011;
    pub const BEDROOM_WINDOW_OUT: u64 = 1012;
    // The manor.
    pub const GROUNDS: u64 = 300;
    pub const HALL: u64 = 301;
    pub const UNDERCROFT: u64 = 302;
    pub const GALLERY: u64 = 303;
    pub const VAULT: u64 = 304;
    pub const MANOR: u64 = 305;
    pub const MANOR_DOOR_OUT: u64 = 1101;
    pub const MANOR_DOOR_IN: u64 = 1102;
    pub const UNDERCROFT_STAIRS: u64 = 1103;
    pub const UNDERCROFT_STAIRS_UP: u64 = 1104;
    pub const GALLERY_STAIRS: u64 = 1105;
    pub const GALLERY_STAIRS_DOWN: u64 = 1106;
    pub const GALLERY_WINDOW: u64 = 1107;
    // The Hill.
    pub const COTTAGE: u64 = 200;
    pub const COTTAGE_KITCHEN: u64 = 201;
    pub const LOFT: u64 = 202;
    pub const COTTAGE_TABLE: u64 = 203;
    pub const BOULDER: u64 = 210;
    pub const MARKER: u64 = 211;
    pub const CAIRN: u64 = 2090;
    pub const COTTAGE_DOOR_OUT: u64 = 1201;
    pub const COTTAGE_DOOR_IN: u64 = 1202;
    pub const MINE_MOUTH: u64 = 400;
    pub const GALLERY_DEEP: u64 = 401;
    pub const CABIN: u64 = 402;
    // The Harbour.
    pub const HERON: u64 = 500;
    pub const GATE: u64 = 510;
    pub const WELL: u64 = 511;
    pub const STABLES: u64 = 512;
    // Millside.
    pub const FARM: u64 = 600;
    pub const FARMHOUSE: u64 = 601;
    pub const FAR_FIELD: u64 = 602;
    pub const WOOD: u64 = 603;
    pub const MILL: u64 = 604;
    pub const TEMPERATE: u64 = 900;
    pub const WATERSHED: u64 = 901;
    pub const FOX_RUN: u64 = 902;
    pub const FROST_HOLLOW: u64 = 903;
    // Materials.
    pub const TIMBER: u64 = 700;
    pub const GLASS: u64 = 701;
    pub const IRON: u64 = 702;
    pub const GRANITE: u64 = 703;
    pub const APPLE_STUFF: u64 = 704;
    pub const POPPY: u64 = 705;
    pub const FLOUR: u64 = 706;
    pub const PIE: u64 = 707;
    // Recipes.
    pub const APPLE_PIE: u64 = 950;
    // People, animals, and things that move.
    pub const ALICE: u64 = 2001;
    pub const BOB: u64 = 2002;
    pub const CAROL: u64 = 2003;
    pub const DAVE: u64 = 2004;
    pub const ERIN: u64 = 2005;
    pub const CAT: u64 = 2010;
    pub const COURIER: u64 = 2011;
    pub const RACCOON: u64 = 2012;
    pub const FINN: u64 = 2013;
    pub const GWEN: u64 = 2014;
    pub const STEWARD: u64 = 2015;
    pub const NED: u64 = 2016;
    pub const WARDROBE: u64 = 2020;
    pub const RIDER: u64 = 2030;
    pub const HAL: u64 = 2040;
    pub const IDA: u64 = 2041;
    pub const CAPTAIN: u64 = 2050;
    pub const BOSUN: u64 = 2051;
    pub const GUARD: u64 = 2052;
    pub const FARMER: u64 = 2060;
    pub const FOX: u64 = 2061;
    pub const MILLER: u64 = 2062;
    pub const VILLAGER: u64 = 2070;
    pub const JORY: u64 = 2080;
    pub const KIT: u64 = 2081;
    pub const LENA: u64 = 2082;
    pub const HIKER: u64 = 2083;
    pub const NELL: u64 = 2084;
}

/// The entity with raw id `raw`.
pub fn e(raw: u64) -> EntityId {
    EntityId::from_raw(raw)
}

/// Ashford's package, parsed — for a test to adjust (clock, domain selection) before loading.
pub fn package() -> WorldPackage {
    parse_world(ASHFORD).expect("Ashford parses")
}

/// One order through the front door: on the next tick, propose `value` for `fact` of `who`.
type Order = (EntityId, FactType, Change);

/// The front door: proposes, on the next tick, whatever orders the test queued. A stand-in for
/// a player — it writes only intents, which are open to anyone (Ruling 13), and direction
/// (Amendment A-14): a mind it commands is directed first, and commanded the tick after, so the
/// mind has stood aside before the command arrives.
struct FrontDoor {
    orders: Rc<RefCell<Vec<Order>>>,
}

const FRONT_DOOR_READS: &[FactType] = &[minds::schema::WALK_SPEED, minds::schema::DIRECTED];

const FRONT_DOOR_WRITES: &[FactType] = &[
    TRAVEL_TO,
    TRAVEL_SPEED,
    ACT_OPEN,
    ACT_CLOSE,
    ACT_FACE,
    living::schema::REST,
    physical::schema::ACT_TAKE,
    physical::schema::ACT_DROP,
    physical::schema::ACT_CONSUME,
    physical::schema::ACT_PICK,
    economy::schema::ACT_MAKE,
    economy::schema::ACT_CLAIM,
    minds::schema::DIRECTED,
];

impl System for FrontDoor {
    fn id(&self) -> SystemId {
        SystemId::new("frontdoor.orders")
    }
    fn reads(&self) -> &'static [FactType] {
        FRONT_DOOR_READS
    }
    fn writes(&self) -> &'static [FactType] {
        FRONT_DOOR_WRITES
    }
    fn cadence(&self) -> Cadence {
        Cadence::EveryTick
    }
    fn evaluate(&self, view: &dyn CommittedView, ctx: &TickContext) -> Vec<Proposal> {
        let propose = |who: EntityId, fact: FactType, change: Change| {
            Proposal::new(
                self.id(),
                FactKey::new(who, fact),
                ctx.basis_tick(),
                change,
                Cause::new("ordered"),
            )
        };
        // A mind not yet directed is directed now, and its orders wait a tick; everything else
        // is proposed as given.
        let undirected = |who: EntityId| {
            view.read(FactKey::new(who, minds::schema::WALK_SPEED))
                .is_some()
                && view
                    .read(FactKey::new(who, minds::schema::DIRECTED))
                    .map(|f| f.value)
                    != Some(Value::Bool(true))
        };
        let mut out = Vec::new();
        let mut waiting = Vec::new();
        let mut directing: Vec<EntityId> = Vec::new();
        for (who, fact, change) in self.orders.borrow_mut().drain(..) {
            if fact != minds::schema::DIRECTED && undirected(who) {
                if !directing.contains(&who) {
                    directing.push(who);
                    out.push(propose(
                        who,
                        minds::schema::DIRECTED,
                        Change::Set(Value::Bool(true)),
                    ));
                }
                waiting.push((who, fact, change));
            } else {
                out.push(propose(who, fact, change));
            }
        }
        self.orders.borrow_mut().extend(waiting);
        out
    }
}

/// A running Ashford.
pub struct City {
    world: LoadedWorld,
    orders: Rc<RefCell<Vec<Order>>>,
    /// The last committed tick.
    pub tick: u64,
    /// The seed every tick runs under.
    pub seed: u64,
    /// Everything chronicled so far.
    pub chronicle: Vec<ChronicleEntry>,
}

impl City {
    /// Ashford as shipped — everyone with a mind of their own (Amendment A-14) — under seed 1.
    pub fn new() -> Self {
        Self::from(package())
    }

    /// Ashford without minds: everyone stays put and does only what a test commands. For tests of
    /// space, sight, ground, and the body, where a city going about its business would only be in
    /// the way.
    pub fn quiet() -> Self {
        Self::from(without_minds(package()))
    }

    /// Ashford from an adjusted package, under seed 1.
    pub fn from(package: WorldPackage) -> Self {
        let mut world = load(&package, engine_version()).expect("Ashford loads");
        let orders = Rc::new(RefCell::new(Vec::new()));
        world.attach(Box::new(FrontDoor {
            orders: Rc::clone(&orders),
        }));
        Self {
            world,
            orders,
            tick: 0,
            seed: 1,
            chronicle: Vec::new(),
        }
    }

    /// The committed reality, for the engine's queries.
    pub fn store(&self) -> &MemoryStore {
        self.world.store()
    }

    /// Advance `ticks` ticks.
    pub fn run(&mut self, ticks: u64) {
        for _ in 0..ticks {
            self.tick += 1;
            self.world
                .tick(self.tick, self.seed, &mut self.chronicle)
                .unwrap_or_else(|err| panic!("tick {} failed: {err:?}", self.tick));
        }
    }

    /// Advance one tick, returning the kernel's verdict instead of insisting on success — for
    /// tests of what the world refuses.
    pub fn try_run(&mut self) -> Result<(), kernel::tick::TickError> {
        self.world
            .tick(self.tick + 1, self.seed, &mut self.chronicle)
            .map(|()| self.tick += 1)
    }

    /// Attach another system — a rogue or a probe — under the same law as everything else.
    pub fn attach(&mut self, system: Box<dyn System>) {
        self.world.attach(system);
    }

    /// Run at least one tick — so orders just given have landed — and then until `done` holds
    /// or `limit` ticks have passed; the number of ticks run.
    pub fn run_until(&mut self, limit: u64, done: impl Fn(&City) -> bool) -> u64 {
        for n in 1..=limit {
            self.run(1);
            if done(self) {
                return n;
            }
        }
        limit
    }

    fn order(&mut self, who: u64, fact: FactType, value: Value) {
        self.orders
            .borrow_mut()
            .push((e(who), fact, Change::Set(value)));
    }

    /// Ask `who` to travel to `dest` at `speed` centimetres per second.
    pub fn go(&mut self, who: u64, dest: u64, speed: i64) -> &mut Self {
        self.order(who, TRAVEL_TO, Value::Entity(e(dest)));
        self.order(who, TRAVEL_SPEED, Value::Int(speed));
        self
    }

    /// Ask `who` to open `door`.
    pub fn open(&mut self, who: u64, door: u64) -> &mut Self {
        self.order(who, ACT_OPEN, Value::Entity(e(door)));
        self
    }

    /// Ask `who` to shut `door`.
    pub fn close(&mut self, who: u64, door: u64) -> &mut Self {
        self.order(who, ACT_CLOSE, Value::Entity(e(door)));
        self
    }

    /// Ask `who` to take hold of `thing` (Appendix A, Ruling 16).
    pub fn take(&mut self, who: u64, thing: u64) -> &mut Self {
        self.order(who, physical::schema::ACT_TAKE, Value::Entity(e(thing)));
        self
    }

    /// Ask `who` to put down `thing` where it stands.
    pub fn drop_(&mut self, who: u64, thing: u64) -> &mut Self {
        self.order(who, physical::schema::ACT_DROP, Value::Entity(e(thing)));
        self
    }

    /// Ask `who` to pick from `deposit` — a fruit tree (Amendment A-15).
    pub fn pick(&mut self, who: u64, deposit: u64) -> &mut Self {
        self.order(who, physical::schema::ACT_PICK, Value::Entity(e(deposit)));
        self
    }

    /// Ask `who` to make `recipe` — bake a pie at the hearth (Amendment A-17).
    pub fn make(&mut self, who: u64, recipe: u64) -> &mut Self {
        self.order(who, economy::schema::ACT_MAKE, Value::Entity(e(recipe)));
        self
    }

    /// Ask `who` to claim `thing`, which no one owns (Amendment A-19).
    pub fn claim(&mut self, who: u64, thing: u64) -> &mut Self {
        self.order(who, economy::schema::ACT_CLAIM, Value::Entity(e(thing)));
        self
    }

    /// Ask `who` to consume `thing` — eat it, drink it.
    pub fn consume(&mut self, who: u64, thing: u64) -> &mut Self {
        self.order(who, physical::schema::ACT_CONSUME, Value::Entity(e(thing)));
        self
    }

    /// Stop directing `who`: its own mind decides again (Amendment A-14).
    pub fn release(&mut self, who: u64) -> &mut Self {
        self.orders
            .borrow_mut()
            .push((e(who), minds::schema::DIRECTED, Change::Tombstone));
        self
    }

    /// Whether `who` is being directed.
    pub fn directed(&self, who: u64) -> bool {
        self.read(who, minds::schema::DIRECTED) == Some(Value::Bool(true))
    }

    /// Ask `who` to lie down and rest, or to get up (Appendix A, Ruling 15).
    pub fn rest(&mut self, who: u64, yes: bool) -> &mut Self {
        self.order(who, living::schema::REST, Value::Bool(yes));
        self
    }

    /// Ask `who` to face compass `degrees`.
    pub fn face(&mut self, who: u64, degrees: i64) -> &mut Self {
        self.order(who, ACT_FACE, Value::Int(degrees * 100));
        self
    }

    /// The place `who` is directly in.
    pub fn room_of(&self, who: u64) -> u64 {
        match self.read(who, CONTAINED_IN) {
            Some(Value::Entity(r)) => r.raw(),
            other => panic!("{who} is nowhere: {other:?}"),
        }
    }

    /// The committed value of `fact` for `who`, if any.
    pub fn read(&self, who: u64, fact: FactType) -> Option<Value> {
        self.store()
            .read(FactKey::new(e(who), fact))
            .map(|f| f.value)
    }

    /// The committed integer value of `fact` for `who`, if any.
    pub fn int(&self, who: u64, fact: FactType) -> Option<i64> {
        self.read(who, fact).and_then(|v| v.as_int())
    }

    /// Whether `who` still has somewhere to go.
    pub fn travelling(&self, who: u64) -> bool {
        self.read(who, TRAVEL_TO).is_some()
    }

    /// Whether `who`'s travel is reported blocked.
    pub fn blocked(&self, who: u64) -> bool {
        self.read(who, TRAVEL_BLOCKED) == Some(Value::Bool(true))
    }

    /// What `who` last could not do, if anything.
    pub fn refused(&self, who: u64) -> Option<u64> {
        match self.read(who, ACT_REFUSED) {
            Some(Value::Entity(x)) => Some(x.raw()),
            _ => None,
        }
    }
}

impl Default for City {
    fn default() -> Self {
        Self::new()
    }
}

/// `package` without decision systems: nobody acts unless commanded.
pub fn without_minds(mut package: WorldPackage) -> WorldPackage {
    package.manifest.domains.retain(|d| d != "minds");
    package
}

/// `package` in which nobody grows hungry: for tests of other drives — warmth, love, a habit —
/// that run past the hours when hunger would send people to the orchard (Amendment A-16).
pub fn well_fed(mut package: WorldPackage) -> WorldPackage {
    if let Some(rules) = package.living_rules.as_mut() {
        rules.hunger_per_hour = 0;
    }
    package
}

/// Ashford's package with ticks of `seconds` (Amendment A-1: the same city at any resolution).
pub fn with_tick_seconds(seconds: u64) -> WorldPackage {
    let mut p = package();
    p.clock.tick_ms = seconds * 1000;
    p
}
