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

    /// Every fact type this domain owns.
    pub const OWNED: &[FactType] = &[
        STOCK,
        YIELD_MADE_OF,
        YIELD_SIZE,
        YIELD_PER_DAY,
        YIELD_CAP,
        PICK_JUDGED,
        RESTOCK,
    ];

    /// One whole unit of stock, in the stock's hundredths.
    pub const UNIT: i64 = 100;

    /// The deposit a body last picked from — **Physical Reality's** report.
    pub const PICKED: FactType = FactType::new("physical.body.picked");
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
        ACT_ARRIVE, BODY_SIZE, LARDER, MADE_OF, PANTRY_CROSSING, PANTRY_LEAVING, PICKED,
        PICK_JUDGED, RESTOCK, STOCK, UNIT, YIELD_CAP, YIELD_MADE_OF, YIELD_PER_DAY, YIELD_SIZE,
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
    ];
    const DEPOSIT_WRITES: &[FactType] =
        &[STOCK, PICK_JUDGED, MADE_OF, BODY_SIZE, ACT_ARRIVE, RESTOCK];

    /// Deposits (Vol. III Ch. 3; Amendment A-15): every tick, each pick Physical Reality reported
    /// and this domain has not judged takes one whole unit from the deposit — the lower picker id
    /// first, while there is one — and brings a new item, made of the deposit's material and of
    /// its size, into the picker's hand. Every `regrow_ticks` ticks the stock regrows at the
    /// deposit's daily rate, up to its cap. A bare deposit yields nothing. Units delivered to it
    /// (Amendment A-26) are added before anything is picked.
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
                if regrowing {
                    let per_day = int(view, FactKey::new(deposit, YIELD_PER_DAY)).unwrap_or(0);
                    let cap = int(view, FactKey::new(deposit, YIELD_CAP)).unwrap_or(0) * UNIT;
                    let grow = div_dither(
                        per_day as i128 * UNIT as i128 * self.regrow_ms as i128,
                        86_400_000,
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
