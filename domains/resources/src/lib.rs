//! # Resources domain -- Vol. III Ch. 3
//!
//! Owns (Appendix A): definitions, deposits, grades, non-living stocks, regeneration. First,
//! deposits that regrow and yield items when picked (Amendment A-15): a fruit tree's stock of
//! fruit, ripening at its declared rate up to a cap; each pick Physical Reality reports takes one
//! unit and brings a new item into the picker's hand.
//!
//! Must be cleanly absent when disabled (Vol. IV Ch. 2 selection): worlds that switch
//! this domain off carry no trace of it -- Pelagia is watching (Vol. IV Ch. 8 §8.3).
//!
//! **Domains never import domains** (Vol. V Ch. 1 §1.1, rule 2). This crate depends on
//! `kernel` and nothing else in the workspace; cross-domain effect happens through
//! committed proposals and events, never direct calls (Vol. III Ch. 12 §12.1). The picks it
//! judges, and the items it brings into being, are Physical Reality's facts, named by id.

use kernel::domain::{Domain, ResolveError, Resolved, ValidationError};
use kernel::fact::FactType;
use kernel::proposal::Change;
use kernel::system::System;
use kernel::time::SimClock;
use kernel::value::Value;

/// Fact-type declarations owned by the Resources domain (Appendix A), and the facts of other
/// owners it reads or proposes by id.
pub mod schema {
    use kernel::fact::FactType;

    /// A deposit's stock, in hundredths of a unit — so slow regrowth accumulates between whole
    /// units. A pick takes a whole unit.
    pub const STOCK: FactType = FactType::new("resources.stock");
    /// What each unit a deposit yields is made of (a material entity).
    pub const YIELD_MADE_OF: FactType = FactType::new("resources.yield.made_of");
    /// How big each unit is, as a body's size `[half-width, half-depth, height]`, cm.
    pub const YIELD_SIZE: FactType = FactType::new("resources.yield.size");
    /// How many units regrow per day.
    pub const YIELD_PER_DAY: FactType = FactType::new("resources.yield.per_day");
    /// The most units a deposit holds.
    pub const YIELD_CAP: FactType = FactType::new("resources.yield.cap");
    /// The tick of the last pick judged for a picker, so each pick yields once.
    pub const PICK_JUDGED: FactType = FactType::new("resources.pick_judged");
    /// Whole units delivered to a deposit, to be added to its stock (Amendment A-26): an intent
    /// open to anyone. Deliveries proposed in one tick add up; the deposit takes them in the next.
    pub const RESTOCK: FactType = FactType::new("resources.act.restock");

    /// How a deposit renews (Vol. III Ch. 3 §3.4; Amendment A-41): [`NONRENEWABLE`],
    /// [`RENEWABLE`], [`THRESHOLD`], or [`FLOW`]. Absent, renewable.
    pub const RENEWAL: FactType = FactType::new("resources.renewal");
    /// The stock, in whole units, below which a threshold deposit collapses and never regrows.
    pub const FLOOR: FactType = FactType::new("resources.floor");
    /// How rich a deposit is, in hundredths of a percent: the poorer, the more work each unit
    /// takes. Absent, full.
    pub const GRADE: FactType = FactType::new("resources.grade");
    /// The work each unit of a deposit takes at full grade, in the units of a blow's strength
    /// (Amendment A-27). Absent or zero: one go, one unit.
    pub const DIFFICULTY: FactType = FactType::new("resources.difficulty");
    /// That a deposit grows as living things do (Amendment A-41): `[from, full, water]` — no
    /// regrowth at or below `from` centidegrees, full regrowth from `full`, and only as much as
    /// the ground holds of `water` (hundredths of a percent of ground water).
    pub const GROWS: FactType = FactType::new("resources.grows");
    /// The work put into a deposit toward its next unit, by whoever works it.
    pub const WORK_DONE: FactType = FactType::new("resources.work_done");
    /// That a threshold deposit was taken below its floor and will never regrow.
    pub const COLLAPSED: FactType = FactType::new("resources.collapsed");

    /// Stock only falls: ore, stone.
    pub const NONRENEWABLE: i64 = 1;
    /// Regrows toward its cap at its rate.
    pub const RENEWABLE: i64 = 2;
    /// Regrows as renewable stock does, until it is taken below its floor; then never.
    pub const THRESHOLD: i64 = 3;
    /// Refills to its cap each step, and cannot be stockpiled at the source: a well.
    pub const FLOW: i64 = 4;

    /// Every fact type this domain owns.
    pub const OWNED: &[FactType] = &[
        STOCK,
        YIELD_MADE_OF,
        YIELD_SIZE,
        YIELD_PER_DAY,
        YIELD_CAP,
        PICK_JUDGED,
        RESTOCK,
        RENEWAL,
        FLOOR,
        GRADE,
        DIFFICULTY,
        GROWS,
        WORK_DONE,
        COLLAPSED,
    ];

    /// One whole unit of stock, in the stock's hundredths.
    pub const UNIT: i64 = 100;

    /// The deposit a body last picked from — **Physical Reality's** report.
    pub const PICKED: FactType = FactType::new("physical.body.picked");
    /// How hard that go was, as a blow's strength — **Physical Reality's** report (A-41).
    pub const PICK_EFFORT: FactType = FactType::new("physical.body.pick_effort");
    /// A place's air temperature — **Physical Reality's**.
    pub const TEMPERATURE: FactType = FactType::new("physical.environment.temperature");
    /// How wet a climate's ground is — **Physical Reality's** (A-38).
    pub const GROUND_WATER: FactType = FactType::new("physical.environment.ground_water");
    /// Where a thing is — **Physical Reality's**.
    pub const CONTAINED_IN: FactType = FactType::new("physical.space.contained_in");
    /// What a thing is made of — **Physical Reality's**.
    pub const MADE_OF: FactType = FactType::new("physical.material.made_of");
    /// A body's size — **Physical Reality's**.
    pub const BODY_SIZE: FactType = FactType::new("physical.body.size");
    /// A request that a thing arrive in the world — **Physical Reality's**.
    pub const ACT_ARRIVE: FactType = FactType::new("physical.act.arrive");

    /// A household's larder coming into the scene as its pantry, naming the home it stands in —
    /// **Society's** (Amendment A-26).
    pub const PANTRY_CROSSING: FactType = FactType::new("society.pantry.crossing");
    /// A household's pantry leaving the scene — **Society's**.
    pub const PANTRY_LEAVING: FactType = FactType::new("society.pantry.leaving");
    /// How many meals a household's larder holds, on its home — **Society's**.
    pub const LARDER: FactType = FactType::new("society.household.larder");
}

/// Hermetic transformations owned by the Resources domain (Vol. V Ch. 3 §3.1).
pub mod systems {
    use crate::schema::{
        ACT_ARRIVE, BODY_SIZE, COLLAPSED, CONTAINED_IN, DIFFICULTY, FLOOR, FLOW, GRADE,
        GROUND_WATER, GROWS, LARDER, MADE_OF, NONRENEWABLE, PANTRY_CROSSING, PANTRY_LEAVING,
        PICKED, PICK_EFFORT, PICK_JUDGED, RENEWAL, RESTOCK, STOCK, TEMPERATURE, THRESHOLD, UNIT,
        WORK_DONE, YIELD_CAP, YIELD_MADE_OF, YIELD_PER_DAY, YIELD_SIZE,
    };
    use kernel::fact::{Cause, FactKey, FactType, SystemId};
    use kernel::fixed::div_dither;
    use kernel::identity::EntityId;
    use kernel::proposal::{Change, Proposal};
    use kernel::system::{Cadence, CommittedView, System, TickContext};
    use kernel::value::Value;
    use std::collections::BTreeMap;

    const DEPOSIT_READS: &[FactType] = &[
        STOCK,
        YIELD_MADE_OF,
        YIELD_SIZE,
        YIELD_PER_DAY,
        YIELD_CAP,
        PICKED,
        PICK_JUDGED,
        RESTOCK,
        PICK_EFFORT,
        RENEWAL,
        FLOOR,
        GRADE,
        DIFFICULTY,
        GROWS,
        WORK_DONE,
        COLLAPSED,
        TEMPERATURE,
        GROUND_WATER,
        CONTAINED_IN,
    ];
    const DEPOSIT_WRITES: &[FactType] = &[
        STOCK,
        PICK_JUDGED,
        MADE_OF,
        BODY_SIZE,
        ACT_ARRIVE,
        RESTOCK,
        WORK_DONE,
        COLLAPSED,
    ];

    /// Full, in hundredths of a percent.
    const FULL: i64 = 10_000;

    /// The nearest of `thing` and what encloses it to carry `fact`, and its value.
    fn ambient(view: &dyn CommittedView, thing: EntityId, fact: FactType) -> Option<i64> {
        kernel::hierarchy::ancestry(view, thing, CONTAINED_IN)
            .into_iter()
            .find_map(|place| int(view, FactKey::new(place, fact)))
    }

    /// How fast a growing deposit regrows now, in hundredths of a percent of its rate
    /// (Amendment A-41): as warm as it stands between its `from` and `full`, and as much as the
    /// ground holds of the water it needs. A deposit that does not grow regrows at its rate.
    fn growth(view: &dyn CommittedView, deposit: EntityId) -> i64 {
        let Some([from, full, water]) = view
            .read(FactKey::new(deposit, GROWS))
            .and_then(|f| f.value.as_vec3())
        else {
            return FULL;
        };
        let air = ambient(view, deposit, TEMPERATURE).unwrap_or(from);
        let warmth = ((air - from) * FULL / (full - from).max(1)).clamp(0, FULL);
        let wet = if water <= 0 {
            FULL
        } else {
            let ground = ambient(view, deposit, GROUND_WATER).unwrap_or(0);
            (ground * FULL / water).clamp(0, FULL)
        };
        warmth * wet / FULL
    }

    /// Deposits (Vol. III Ch. 3; Amendments A-15, A-41). Every tick, each go at a deposit Physical
    /// Reality reported and this domain has not judged puts its effort into the deposit, the
    /// lower picker id first. Once the work put in reaches what a unit takes — its difficulty
    /// over its grade; nothing, for fruit — one whole unit leaves the stock, and a new item, made
    /// of the deposit's material and of its size, arrives in the hand of the one whose go
    /// finished it. A bare deposit yields nothing. Units delivered to it (Amendment A-26) are
    /// added before anything is taken.
    ///
    /// Every `regrow_ticks` ticks the stock renews as its class says: a nonrenewable deposit
    /// never; a flow refills to its cap; a renewable or threshold deposit regrows at its daily
    /// rate up to its cap — as it grows, if it does (warmth and water) — and a threshold deposit
    /// taken below its floor has collapsed, and never regrows again.
    pub struct Deposits {
        regrow_ticks: u64,
        regrow_ms: u64,
    }

    impl Deposits {
        /// Regrow every `regrow_ticks` ticks, each step `regrow_ms` of simulated time.
        pub const fn new(regrow_ticks: u64, regrow_ms: u64) -> Self {
            Self {
                regrow_ticks,
                regrow_ms,
            }
        }
    }

    fn int(view: &dyn CommittedView, key: FactKey) -> Option<i64> {
        view.read(key).and_then(|f| f.value.as_int())
    }

    impl System for Deposits {
        fn id(&self) -> SystemId {
            SystemId::new("resources.deposits")
        }
        fn reads(&self) -> &'static [FactType] {
            DEPOSIT_READS
        }
        fn writes(&self) -> &'static [FactType] {
            DEPOSIT_WRITES
        }
        fn cadence(&self) -> Cadence {
            Cadence::EveryTick
        }
        fn evaluate(&self, view: &dyn CommittedView, ctx: &TickContext) -> Vec<Proposal> {
            let mut out = Vec::new();
            let mut push = |key: FactKey, change: Change, why: &'static str| {
                out.push(Proposal::new(
                    self.id(),
                    key,
                    ctx.basis_tick(),
                    change,
                    Cause::new(why),
                ));
            };
            // The picks not yet judged, by deposit, pickers in ascending id.
            let mut picks: BTreeMap<EntityId, Vec<EntityId>> = BTreeMap::new();
            for picker in view.entities_with(PICKED) {
                let Some(f) = view.read(FactKey::new(picker, PICKED)) else {
                    continue;
                };
                let judged = int(view, FactKey::new(picker, PICK_JUDGED)).unwrap_or(0);
                if let (Value::Entity(deposit), true) = (f.value, f.provenance.tick as i64 > judged)
                {
                    picks.entry(deposit).or_default().push(picker);
                    let tick = Value::Int(f.provenance.tick as i64);
                    push(
                        FactKey::new(picker, PICK_JUDGED),
                        Change::Set(tick),
                        "pick_judged",
                    );
                }
            }
            let regrowing = ctx.tick() % self.regrow_ticks.max(1) == 0;
            for deposit in view.entities_with(STOCK) {
                let before = int(view, FactKey::new(deposit, STOCK)).unwrap_or(0);
                let mut stock = before;
                let worked = int(view, FactKey::new(deposit, WORK_DONE)).unwrap_or(0);
                let mut work = worked;
                let grade = int(view, FactKey::new(deposit, GRADE))
                    .unwrap_or(FULL)
                    .max(1);
                let per_unit = int(view, FactKey::new(deposit, DIFFICULTY))
                    .unwrap_or(0)
                    .max(0)
                    .saturating_mul(FULL)
                    / grade;
                if let Some(units) = int(view, FactKey::new(deposit, RESTOCK)) {
                    stock += units.max(0) * UNIT;
                    push(
                        FactKey::new(deposit, RESTOCK),
                        Change::Tombstone,
                        "restocked",
                    );
                }
                for picker in picks.remove(&deposit).unwrap_or_default() {
                    if stock < UNIT {
                        continue; // bare: the hand comes away empty
                    }
                    // The go's effort, toward the next unit: one go yields one unit at most.
                    if per_unit > 0 {
                        let effort = int(view, FactKey::new(picker, PICK_EFFORT)).unwrap_or(0);
                        work = (work + effort.max(0)).min(per_unit);
                        if work < per_unit {
                            continue;
                        }
                        work = 0;
                    }
                    stock -= UNIT;
                    let item = ctx.new_entity();
                    if let Some(Value::Entity(m)) = view
                        .read(FactKey::new(deposit, YIELD_MADE_OF))
                        .map(|f| f.value)
                    {
                        push(
                            FactKey::new(item, MADE_OF),
                            Change::Add(Value::Entity(m)),
                            "grown",
                        );
                    }
                    if let Some(size) = view
                        .read(FactKey::new(deposit, YIELD_SIZE))
                        .map(|f| f.value)
                    {
                        push(FactKey::new(item, BODY_SIZE), Change::Set(size), "grown");
                    }
                    push(
                        FactKey::new(item, ACT_ARRIVE),
                        Change::Set(Value::Entity(picker)),
                        "picked",
                    );
                }
                if work != worked {
                    let change = if work == 0 {
                        Change::Tombstone
                    } else {
                        Change::Set(Value::Int(work))
                    };
                    push(FactKey::new(deposit, WORK_DONE), change, "worked");
                }
                // A threshold stock taken below its floor collapses, for good.
                let renewal = int(view, FactKey::new(deposit, RENEWAL)).unwrap_or(0);
                let mut collapsed = view.read(FactKey::new(deposit, COLLAPSED)).is_some();
                let floor = int(view, FactKey::new(deposit, FLOOR)).unwrap_or(0) * UNIT;
                if renewal == THRESHOLD && !collapsed && stock < floor {
                    collapsed = true;
                    push(
                        FactKey::new(deposit, COLLAPSED),
                        Change::Set(Value::Bool(true)),
                        "taken_below_its_floor",
                    );
                }
                let cap = int(view, FactKey::new(deposit, YIELD_CAP)).unwrap_or(0) * UNIT;
                if regrowing && renewal == FLOW {
                    stock = stock.max(cap);
                } else if regrowing && renewal != NONRENEWABLE && !collapsed {
                    let per_day = int(view, FactKey::new(deposit, YIELD_PER_DAY)).unwrap_or(0);
                    let grow = div_dither(
                        per_day as i128
                            * UNIT as i128
                            * self.regrow_ms as i128
                            * growth(view, deposit) as i128,
                        86_400_000 * FULL as i128,
                        &mut ctx.rng(deposit.raw()),
                    ) as i64;
                    stock = (stock + grow).min(cap.max(stock));
                }
                if stock != before {
                    push(
                        FactKey::new(deposit, STOCK),
                        Change::Set(Value::Int(stock)),
                        "ripening",
                    );
                }
            }
            out
        }
    }

    /// How a household's larder stands in the scene as its pantry (Amendment A-26): what each
    /// meal is made of, and how big it is.
    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    pub struct PantryRules {
        /// The material a meal is made of.
        pub meal_material: EntityId,
        /// A meal's size, `[half-width, half-depth, height]`, cm.
        pub meal_size: [i64; 3],
    }

    const SEAM_READS: &[FactType] = &[PANTRY_CROSSING, PANTRY_LEAVING, LARDER];
    const SEAM_WRITES: &[FactType] = &[STOCK, YIELD_MADE_OF, YIELD_SIZE, YIELD_PER_DAY, YIELD_CAP];

    /// Resources' side of the seam for larders (Amendment A-26): a household's larder coming into
    /// the scene becomes its pantry, a deposit holding the larder's meals that does not regrow;
    /// a pantry leaving the scene is retired from this domain's keeping, and Society counts what
    /// is left back into the larder.
    pub struct Seam {
        rules: PantryRules,
    }

    impl Seam {
        /// Pantries of meals by `rules`.
        pub const fn new(rules: PantryRules) -> Self {
            Self { rules }
        }
    }

    impl System for Seam {
        fn id(&self) -> SystemId {
            SystemId::new("resources.seam")
        }
        fn reads(&self) -> &'static [FactType] {
            SEAM_READS
        }
        fn writes(&self) -> &'static [FactType] {
            SEAM_WRITES
        }
        fn cadence(&self) -> Cadence {
            Cadence::EveryTick
        }
        fn evaluate(&self, view: &dyn CommittedView, ctx: &TickContext) -> Vec<Proposal> {
            let r = &self.rules;
            let mut out: Vec<Proposal> = view
                .entities_with(PANTRY_LEAVING)
                .into_iter()
                .map(|pantry| {
                    Proposal::new(
                        self.id(),
                        FactKey::new(pantry, STOCK),
                        ctx.basis_tick(),
                        Change::Retire,
                        Cause::new("left_the_scene"),
                    )
                })
                .collect();
            for pantry in view.entities_with(PANTRY_CROSSING) {
                let Some(Value::Entity(home)) = view
                    .read(FactKey::new(pantry, PANTRY_CROSSING))
                    .map(|f| f.value)
                else {
                    continue;
                };
                let meals = int(view, FactKey::new(home, LARDER)).unwrap_or(0).max(0);
                for (fact, value) in [
                    (STOCK, Value::Int(meals * UNIT)),
                    (YIELD_MADE_OF, Value::Entity(r.meal_material)),
                    (YIELD_SIZE, Value::Vec3(r.meal_size)),
                    (YIELD_PER_DAY, Value::Int(0)),
                    (YIELD_CAP, Value::Int(0)),
                ] {
                    out.push(Proposal::new(
                        self.id(),
                        FactKey::new(pantry, fact),
                        ctx.basis_tick(),
                        Change::Set(value),
                        Cause::new("crossed"),
                    ));
                }
            }
            out
        }
    }
}

/// The rules deposits follow, from the world package (Vol. IV Ch. 2 §2.2).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ResourcesConfig {
    /// The world's clock.
    pub clock: SimClock,
    /// How often deposits regrow, in seconds of simulated time.
    pub regrow_step_seconds: u64,
    /// How households' larders stand in the scene as pantries (Amendment A-26), if the world's
    /// residents have them.
    pub pantries: Option<systems::PantryRules>,
}

/// The Resources domain, plugged into the kernel as the owner of deposits and their stocks.
pub struct ResourcesDomain {
    config: ResourcesConfig,
}

impl ResourcesDomain {
    /// Configure the domain.
    pub fn new(config: ResourcesConfig) -> Self {
        Self { config }
    }
}

impl Domain for ResourcesDomain {
    fn name(&self) -> &'static str {
        "resources"
    }

    fn owns(&self, fact_type: FactType) -> bool {
        schema::OWNED.contains(&fact_type)
    }

    fn systems(&self) -> Vec<Box<dyn System>> {
        let c = self.config;
        let step = c.clock.step(c.regrow_step_seconds.saturating_mul(1000));
        let mut systems: Vec<Box<dyn System>> = vec![Box::new(systems::Deposits::new(
            step.period_ticks,
            step.dt_ms,
        ))];
        if let Some(pantries) = c.pantries {
            systems.push(Box::new(systems::Seam::new(pantries)));
        }
        systems
    }

    fn compose(
        &self,
        fact_type: FactType,
        _current: Option<Value>,
        changes: &[Change],
    ) -> Result<Resolved, ResolveError> {
        // Deliveries add up; the deposit taking in the last tick's clears it, and any made in
        // the same tick are the next to be taken in (Amendment A-26).
        if fact_type == schema::RESTOCK {
            let mut units = 0i64;
            let mut taken = false;
            for change in changes {
                match change {
                    Change::Delta(n) => units = units.saturating_add(*n),
                    Change::Tombstone => taken = true,
                    _ => return Err(ResolveError::new("a delivery is a number of units")),
                }
            }
            return match (units, taken) {
                (0, true) => Ok(Resolved::Tombstone),
                (0, false) => Err(ResolveError::new("an empty delivery")),
                (n, _) => Ok(Resolved::Write(Value::Int(n))),
            };
        }
        match changes {
            [Change::Set(v) | Change::Create(v)] => Ok(Resolved::Write(*v)),
            [Change::Tombstone] => Ok(Resolved::Tombstone),
            [_] => Err(ResolveError::new("a stock is set, never nudged")),
            _ => Err(ResolveError::new("two competing changes to one deposit")),
        }
    }

    fn validate(&self, fact_type: FactType, value: &Resolved) -> Result<(), ValidationError> {
        if fact_type == schema::STOCK {
            if let Resolved::Write(Value::Int(v)) = value {
                if *v < 0 {
                    return Err(ValidationError::new("a stock cannot fall below nothing"));
                }
            }
        }
        Ok(())
    }
}
