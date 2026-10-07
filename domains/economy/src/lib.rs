//! # Economy domain -- Vol. III Ch. 4
//!
//! Owns (Appendix A): holdings, transfers, exchanges, prices-as-events, money, markets. First,
//! production (Amendment A-17; Appendix A, Ruling 17): recipes the world declares, and makers
//! who make them at a workplace from the inputs they carry. The inputs leave the world when the
//! making begins; the product arrives when its time has passed. Then owners (Amendment A-19;
//! Appendix A, Ruling 18): who owns a thing, and claiming what no one owns.
//!
//! Must be cleanly absent when disabled (Vol. IV Ch. 2 selection): worlds that switch
//! this domain off carry no trace of it -- Pelagia is watching (Vol. IV Ch. 8 §8.3).
//!
//! **Domains never import domains** (Vol. V Ch. 1 §1.1, rule 2). This crate depends on
//! `kernel` and nothing else in the workspace; cross-domain effect happens through
//! committed proposals and events, never direct calls (Vol. III Ch. 12 §12.1). Where things
//! are, what they are made of, and their arriving and leaving are Physical Reality's facts,
//! named here by id.

use kernel::domain::{Domain, ResolveError, Resolved, ValidationError};
use kernel::fact::{FactType, SystemId};
use kernel::proposal::Change;
use kernel::system::System;
use kernel::time::SimClock;
use kernel::value::Value;

/// Fact-type declarations owned by the Economy domain (Appendix A), and the facts of other
/// owners it reads or proposes by id.
pub mod schema {
    use kernel::fact::FactType;

    /// How many things of a material a recipe needs: a fact about the pair (recipe, material).
    pub const RECIPE_NEEDS: FactType = FactType::new("economy.recipe.needs");
    /// The material a recipe's product is made of.
    pub const RECIPE_MAKES: FactType = FactType::new("economy.recipe.makes");
    /// The size of a recipe's product, as a body's size `[half-width, half-depth, height]`, cm.
    pub const RECIPE_SIZE: FactType = FactType::new("economy.recipe.size");
    /// Where a recipe is made: a workplace, such as a hearth.
    pub const RECIPE_AT: FactType = FactType::new("economy.recipe.at");
    /// How long a recipe's product is in the making, in seconds.
    pub const RECIPE_TAKES: FactType = FactType::new("economy.recipe.takes");
    /// A request that a maker make a recipe: the decider's intent (Ruling 17).
    pub const ACT_MAKE: FactType = FactType::new("economy.act.make");
    /// The recipe a maker has in the making.
    pub const MAKING: FactType = FactType::new("economy.making");
    /// The tick at which what a maker has in the making is done.
    pub const READY_AT: FactType = FactType::new("economy.making.ready_at");
    /// The last thing a maker made.
    pub const MADE: FactType = FactType::new("economy.made");
    /// The recipe a maker last asked to make and could not: away from the workplace, without
    /// the inputs, or with something already in the making. The next making clears it.
    pub const MAKE_REFUSED: FactType = FactType::new("economy.make.refused");

    /// Who owns a thing (Amendment A-19): an entity reference to its owner.
    pub const OWNER: FactType = FactType::new("economy.owner");
    /// A request that a claimant claim a thing no one owns: the decider's intent (Ruling 18).
    pub const ACT_CLAIM: FactType = FactType::new("economy.act.claim");
    /// The thing a claimant last asked to claim and could not: not carried, or owned already.
    /// The next claim granted clears it.
    pub const CLAIM_REFUSED: FactType = FactType::new("economy.claim.refused");

    /// Every fact type this domain owns.
    pub const OWNED: &[FactType] = &[
        OWNER,
        ACT_CLAIM,
        CLAIM_REFUSED,
        RECIPE_NEEDS,
        RECIPE_MAKES,
        RECIPE_SIZE,
        RECIPE_AT,
        RECIPE_TAKES,
        ACT_MAKE,
        MAKING,
        READY_AT,
        MADE,
        MAKE_REFUSED,
    ];

    /// Where a thing is: the place or holder that contains it — **Physical Reality's**.
    pub const CONTAINED_IN: FactType = FactType::new("physical.space.contained_in");
    /// What a thing is made of — **Physical Reality's**.
    pub const MADE_OF: FactType = FactType::new("physical.material.made_of");
    /// A body's size — **Physical Reality's**.
    pub const BODY_SIZE: FactType = FactType::new("physical.body.size");
    /// A request that a thing arrive in the world — **Physical Reality's**.
    pub const ACT_ARRIVE: FactType = FactType::new("physical.act.arrive");
    /// A request that a thing leave the world — **Physical Reality's**.
    pub const ACT_LEAVE: FactType = FactType::new("physical.act.leave");
}

/// Economy facts only Economy's own systems may write (Rulings 17, 18): who owns what, the recipes
/// the world declares, and what is in the making. Everyone else proposes the intents to make and
/// to claim.
pub const RESTRICTED: &[FactType] = &[
    schema::OWNER,
    schema::CLAIM_REFUSED,
    schema::RECIPE_NEEDS,
    schema::RECIPE_MAKES,
    schema::RECIPE_SIZE,
    schema::RECIPE_AT,
    schema::RECIPE_TAKES,
    schema::MAKING,
    schema::READY_AT,
    schema::MADE,
    schema::MAKE_REFUSED,
];

/// Hermetic transformations owned by the Economy domain (Vol. V Ch. 3 §3.1).
pub mod systems {
    use crate::schema::{
        ACT_ARRIVE, ACT_CLAIM, ACT_LEAVE, ACT_MAKE, BODY_SIZE, CLAIM_REFUSED, CONTAINED_IN, MADE,
        MADE_OF, MAKE_REFUSED, MAKING, OWNER, READY_AT, RECIPE_AT, RECIPE_MAKES, RECIPE_NEEDS,
        RECIPE_SIZE, RECIPE_TAKES,
    };
    use kernel::fact::{Cause, FactKey, FactType, SystemId};
    use kernel::identity::EntityId;
    use kernel::proposal::{Change, Proposal};
    use kernel::system::{Cadence, CommittedView, System, TickContext};
    use kernel::value::Value;
    use std::collections::{BTreeMap, BTreeSet};

    const PRODUCTION_READS: &[FactType] = &[
        RECIPE_NEEDS,
        RECIPE_MAKES,
        RECIPE_SIZE,
        RECIPE_AT,
        RECIPE_TAKES,
        ACT_MAKE,
        MAKING,
        READY_AT,
        MAKE_REFUSED,
        CONTAINED_IN,
        MADE_OF,
    ];
    const PRODUCTION_WRITES: &[FactType] = &[
        ACT_MAKE,
        MAKING,
        READY_AT,
        MADE,
        MAKE_REFUSED,
        MADE_OF,
        BODY_SIZE,
        ACT_ARRIVE,
        ACT_LEAVE,
    ];

    /// Production (Vol. III Ch. 4 §4.4; Amendment A-17): every tick, each request to make is
    /// judged — the maker must stand where the recipe's workplace is, carry what it needs, and
    /// have nothing else in the making. If so, the inputs leave the world (the lowest ids of each
    /// material first) and the recipe is in the making until its time has passed; otherwise the
    /// request is refused. Whatever is done arrives: in the maker's hand if they stand at the
    /// workplace, otherwise at the workplace.
    pub struct Production {
        tick_ms: u64,
    }

    impl Production {
        /// Judge making in a world whose ticks are `tick_ms` long.
        pub const fn new(tick_ms: u64) -> Self {
            Self { tick_ms }
        }

        /// How many ticks `seconds` of making takes: rounded up, never fewer than one.
        fn ticks(&self, seconds: i64) -> u64 {
            let ms = seconds.max(0) as u64 * 1000;
            ms.div_ceil(self.tick_ms.max(1)).max(1)
        }
    }

    fn entity(view: &dyn CommittedView, key: FactKey) -> Option<EntityId> {
        match view.read(key)?.value {
            Value::Entity(e) => Some(e),
            _ => None,
        }
    }

    fn int(view: &dyn CommittedView, key: FactKey) -> Option<i64> {
        view.read(key).and_then(|f| f.value.as_int())
    }

    /// The place a workplace stands in.
    fn place_of(view: &dyn CommittedView, workplace: EntityId) -> Option<EntityId> {
        entity(view, FactKey::new(workplace, CONTAINED_IN))
    }

    impl System for Production {
        fn id(&self) -> SystemId {
            SystemId::new("economy.production")
        }
        fn reads(&self) -> &'static [FactType] {
            PRODUCTION_READS
        }
        fn writes(&self) -> &'static [FactType] {
            PRODUCTION_WRITES
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

            // What is done arrives.
            for maker in view.entities_with(MAKING) {
                let ready = int(view, FactKey::new(maker, READY_AT)).unwrap_or(0);
                if (ctx.tick() as i64) < ready {
                    continue;
                }
                push(FactKey::new(maker, MAKING), Change::Tombstone, "done");
                push(FactKey::new(maker, READY_AT), Change::Tombstone, "done");
                let Some(recipe) = entity(view, FactKey::new(maker, MAKING)) else {
                    continue;
                };
                let Some(workplace) = entity(view, FactKey::new(recipe, RECIPE_AT)) else {
                    continue;
                };
                let Some(place) = place_of(view, workplace) else {
                    continue; // the workplace is gone, and the work with it
                };
                let at_work = entity(view, FactKey::new(maker, CONTAINED_IN)) == Some(place);
                let thing = ctx.new_entity();
                if let Some(m) = entity(view, FactKey::new(recipe, RECIPE_MAKES)) {
                    push(
                        FactKey::new(thing, MADE_OF),
                        Change::Add(Value::Entity(m)),
                        "made",
                    );
                }
                if let Some(size) = view.read(FactKey::new(recipe, RECIPE_SIZE)) {
                    push(
                        FactKey::new(thing, BODY_SIZE),
                        Change::Set(size.value),
                        "made",
                    );
                }
                let into = if at_work { maker } else { workplace };
                push(
                    FactKey::new(thing, ACT_ARRIVE),
                    Change::Set(Value::Entity(into)),
                    "made",
                );
                push(
                    FactKey::new(maker, MADE),
                    Change::Set(Value::Entity(thing)),
                    "made",
                );
            }

            // Requests to make are judged.
            let makers = view.entities_with(ACT_MAKE);
            if makers.is_empty() {
                return out;
            }
            // What each maker carries, in ascending id.
            let mut carried: BTreeMap<EntityId, Vec<EntityId>> = BTreeMap::new();
            for thing in view.entities_with(CONTAINED_IN) {
                if let Some(holder) = entity(view, FactKey::new(thing, CONTAINED_IN)) {
                    if makers.contains(&holder) {
                        carried.entry(holder).or_default().push(thing);
                    }
                }
            }
            for maker in makers {
                push(
                    FactKey::new(maker, ACT_MAKE),
                    Change::Tombstone,
                    "considered",
                );
                let Some(recipe) = entity(view, FactKey::new(maker, ACT_MAKE)) else {
                    continue;
                };
                let here = entity(view, FactKey::new(maker, CONTAINED_IN));
                let workplace = entity(view, FactKey::new(recipe, RECIPE_AT));
                let at_work = here.is_some() && workplace.and_then(|w| place_of(view, w)) == here;
                let busy = view.read(FactKey::new(maker, MAKING)).is_some();
                // The inputs: for each material it needs, so many carried things made of it.
                let held = carried.remove(&maker).unwrap_or_default();
                let mut used: BTreeSet<EntityId> = BTreeSet::new();
                let mut short = false;
                for (material, needs) in view.read_about(recipe, RECIPE_NEEDS) {
                    let count = needs.value.as_int().unwrap_or(0).max(0) as usize;
                    let of_it: Vec<EntityId> = held
                        .iter()
                        .copied()
                        .filter(|t| !used.contains(t))
                        .filter(|t| {
                            view.read_all(FactKey::new(*t, MADE_OF))
                                .iter()
                                .any(|m| m.value == Value::Entity(material))
                        })
                        .take(count)
                        .collect();
                    short |= of_it.len() < count;
                    used.extend(of_it);
                }
                if !at_work || busy || short {
                    let why = if !at_work {
                        "away_from_work"
                    } else if busy {
                        "already_making"
                    } else {
                        "lacks_inputs"
                    };
                    push(
                        FactKey::new(maker, MAKE_REFUSED),
                        Change::Set(Value::Entity(recipe)),
                        why,
                    );
                    continue;
                }
                for input in used {
                    push(
                        FactKey::new(input, ACT_LEAVE),
                        Change::Set(Value::Bool(true)),
                        "used_in_making",
                    );
                }
                let takes = int(view, FactKey::new(recipe, RECIPE_TAKES)).unwrap_or(0);
                let ready = ctx.tick() + self.ticks(takes);
                push(
                    FactKey::new(maker, MAKING),
                    Change::Set(Value::Entity(recipe)),
                    "making",
                );
                push(
                    FactKey::new(maker, READY_AT),
                    Change::Set(Value::Int(ready as i64)),
                    "making",
                );
                if view.read(FactKey::new(maker, MAKE_REFUSED)).is_some() {
                    push(
                        FactKey::new(maker, MAKE_REFUSED),
                        Change::Tombstone,
                        "making",
                    );
                }
            }
            out
        }
    }

    const HOLDINGS_READS: &[FactType] = &[ACT_CLAIM, OWNER, CONTAINED_IN, CLAIM_REFUSED];
    const HOLDINGS_WRITES: &[FactType] = &[ACT_CLAIM, OWNER, CLAIM_REFUSED];

    /// Holdings (Vol. III Ch. 4 §4.4; Amendment A-19): every tick, each claim is judged — the
    /// claimant must carry the thing, and no one may own it already. If so, the claimant owns it;
    /// otherwise the claim is refused.
    pub struct Holdings;

    impl System for Holdings {
        fn id(&self) -> SystemId {
            SystemId::new("economy.holdings")
        }
        fn reads(&self) -> &'static [FactType] {
            HOLDINGS_READS
        }
        fn writes(&self) -> &'static [FactType] {
            HOLDINGS_WRITES
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
            let mut granted: BTreeSet<EntityId> = BTreeSet::new();
            for claimant in view.entities_with(ACT_CLAIM) {
                push(
                    FactKey::new(claimant, ACT_CLAIM),
                    Change::Tombstone,
                    "considered",
                );
                let Some(thing) = entity(view, FactKey::new(claimant, ACT_CLAIM)) else {
                    continue;
                };
                let carried = entity(view, FactKey::new(thing, CONTAINED_IN)) == Some(claimant);
                let owned = view.read(FactKey::new(thing, OWNER)).is_some();
                if carried && !owned && granted.insert(thing) {
                    push(
                        FactKey::new(thing, OWNER),
                        Change::Set(Value::Entity(claimant)),
                        "claimed",
                    );
                    if view.read(FactKey::new(claimant, CLAIM_REFUSED)).is_some() {
                        push(
                            FactKey::new(claimant, CLAIM_REFUSED),
                            Change::Tombstone,
                            "claimed",
                        );
                    }
                } else {
                    let why = if carried {
                        "owned_already"
                    } else {
                        "not_carried"
                    };
                    push(
                        FactKey::new(claimant, CLAIM_REFUSED),
                        Change::Set(Value::Entity(thing)),
                        why,
                    );
                }
            }
            out
        }
    }
}

/// The rules making follows, from the world package (Vol. IV Ch. 2 §2.2).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct EconomyConfig {
    /// The world's clock.
    pub clock: SimClock,
}

/// The Economy domain, plugged into the kernel as the owner of production.
pub struct EconomyDomain {
    config: EconomyConfig,
    /// The ids of this domain's systems — the only proposers it accepts for its [`RESTRICTED`]
    /// facts.
    own: Vec<SystemId>,
}

impl EconomyDomain {
    /// Configure the domain.
    pub fn new(config: EconomyConfig) -> Self {
        let mut domain = Self {
            config,
            own: Vec::new(),
        };
        domain.own = domain.systems().iter().map(|s| s.id()).collect();
        domain
    }
}

impl Domain for EconomyDomain {
    fn name(&self) -> &'static str {
        "economy"
    }

    fn owns(&self, fact_type: FactType) -> bool {
        schema::OWNED.contains(&fact_type)
    }

    fn accepts(&self, fact_type: FactType, system: SystemId) -> bool {
        // Ruling 17: what is in the making moves only under this domain's own systems.
        !RESTRICTED.contains(&fact_type) || self.own.contains(&system)
    }

    fn systems(&self) -> Vec<Box<dyn System>> {
        vec![
            Box::new(systems::Production::new(self.config.clock.ms_at(1))),
            Box::new(systems::Holdings),
        ]
    }

    fn compose(
        &self,
        fact_type: FactType,
        _current: Option<Value>,
        changes: &[Change],
    ) -> Result<Resolved, ResolveError> {
        // A fresh request to make outlives the clearing of the one just judged (as Physical
        // Reality's intents do, Ruling 13).
        let wishes: Vec<Change> = changes
            .iter()
            .copied()
            .filter(|c| !matches!(c, Change::Tombstone))
            .collect();
        let intent = fact_type == schema::ACT_MAKE || fact_type == schema::ACT_CLAIM;
        let changes = if intent && !wishes.is_empty() {
            &wishes[..]
        } else {
            changes
        };
        match changes {
            [Change::Set(v) | Change::Create(v)] => Ok(Resolved::Write(*v)),
            [Change::Tombstone] => Ok(Resolved::Tombstone),
            [_] => Err(ResolveError::new("making is set or cleared, never nudged")),
            _ => Err(ResolveError::new("two competing changes to one making")),
        }
    }

    fn validate(&self, fact_type: FactType, value: &Resolved) -> Result<(), ValidationError> {
        let Resolved::Write(v) = value else {
            return Ok(());
        };
        let entity = [
            schema::ACT_MAKE,
            schema::MAKING,
            schema::MADE,
            schema::MAKE_REFUSED,
            schema::OWNER,
            schema::ACT_CLAIM,
            schema::CLAIM_REFUSED,
        ];
        if entity.contains(&fact_type) && !matches!(v, Value::Entity(_)) {
            return Err(ValidationError::new(
                "making and owning name a recipe, a thing, or a person",
            ));
        }
        if fact_type == schema::READY_AT && v.as_int().is_none() {
            return Err(ValidationError::new("a making is ready at a tick"));
        }
        Ok(())
    }
}
