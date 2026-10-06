//! The information layer's fact types, and the facts of other owners it perceives (Vol. II
//! Ch. 4; Amendments A-7, A-8).
//!
//! Beliefs about other things are **facts about a pair** (Amendment A-7): the holder is the
//! believer, the second entity what the belief is about —
//! `FactKey::pair(erin, PLACE_OF, bob)` is where Erin believes Bob is. A belief's provenance is
//! its pedigree: the tick it was last formed or confirmed, and how (its cause: `seen`, `felt`,
//! `lost_sight`, `known`).

use kernel::fact::FactType;

// ---- Owned by the information layer.

/// What a mind is watching now: a cardinality-many set of entity references, the information
/// layer's record of what was in view at its last perception. While something is in sight,
/// what the mind believes about it is current.
pub const IN_SIGHT: FactType = FactType::new("info.in_sight");

/// Where the holder believes something is: the place it was last seen in (an entity
/// reference). A fact about a pair; about the holder itself, where it knows it stands.
pub const PLACE_OF: FactType = FactType::new("info.belief.place_of");

/// Where the holder believes an opening leads (an entity reference). A fact about a pair.
pub const LEADS_TO: FactType = FactType::new("info.belief.leads_to");

/// What the holder believes a thing is made of (Amendment A-12): a fact about a pair, holding
/// the set of materials seen in it.
pub const MADE_OF: FactType = FactType::new("info.belief.made_of");

/// How hungry the holder feels, in hundredths of a percent (Amendment A-16).
pub const FELT_HUNGER: FactType = FactType::new("info.self.hunger");

/// How much the holder believes a material feeds a body (Amendment A-16): a fact about a pair —
/// the holder, and the material. Learned by seeing things made of it.
pub const NUTRITION: FactType = FactType::new("info.belief.nutrition");

/// What the holder believes a deposit yields (an entity reference to a material; a fact about a
/// pair).
pub const YIELDS: FactType = FactType::new("info.belief.yields");

/// How many units the holder believes a deposit holds — fruit on a tree (Amendment A-15). A fact
/// about a pair.
pub const STOCK: FactType = FactType::new("info.belief.stock");

/// Whether the holder believes an opening is open. A fact about a pair.
pub const OPEN: FactType = FactType::new("info.belief.open");

/// How warm the holder found a place when it stood there, in centidegrees Celsius. A fact
/// about a pair: the holder's memory of that place's air.
pub const WARMTH_OF: FactType = FactType::new("info.belief.warmth_of");

/// How warm the holder's own body feels, in centidegrees Celsius.
pub const FELT_BODY_HEAT: FactType = FactType::new("info.self.body_heat");

/// How tired the holder feels, in hundredths of a percent (Amendment A-10).
pub const FELT_FATIGUE: FactType = FactType::new("info.self.fatigue");

/// How well the holder feels, in hundredths of a percent (Amendment A-10); zero is dead.
pub const FELT_HEALTH: FactType = FactType::new("info.self.health");

/// How strongly the holder feels a need that arose, about its object (a fact about a pair),
/// in hundredths of a percent (Amendment A-11).
pub const FELT_NEED: FactType = FactType::new("info.self.need");

/// How the holder knows that need is met: 1, by its object's presence; 2, by a dose of it (a fact
/// about a pair).
pub const FELT_NEED_MET_BY: FactType = FactType::new("info.self.need_met_by");

/// How fond the holder is of another, in hundredths of a percent (a fact about a pair; Amendment
/// A-11). An opinion: the holder's own, one-way (Appendix A, Ruling 2).
pub const AFFECTION: FactType = FactType::new("info.affection");

/// Where the holder knows it is trying to go (an entity reference): its own travel intent.
pub const GOING_TO: FactType = FactType::new("info.self.going_to");

/// Whether the holder knows its way is blocked.
pub const BLOCKED: FactType = FactType::new("info.self.blocked");

/// What the holder last tried and could not do (an entity reference).
pub const REFUSED: FactType = FactType::new("info.self.refused");

/// The job the holder holds (Amendment A-18): known as one knows oneself.
pub const WORK_ROLE: FactType = FactType::new("info.self.work.role");
/// The material the holder's work carries.
pub const WORK_CARRIES: FactType = FactType::new("info.self.work.carries");
/// The recipe the holder's work makes.
pub const WORK_MAKES: FactType = FactType::new("info.self.work.makes");
/// Where the holder's work gets its goods.
pub const WORK_FROM: FactType = FactType::new("info.self.work.from");
/// The store the holder's work keeps.
pub const WORK_TO: FactType = FactType::new("info.self.work.to");
/// How many goods the holder's work keeps in its store.
pub const WORK_KEEP: FactType = FactType::new("info.self.work.keep");
/// The holder's working hours, as seconds into the day `[from, to, 0]`.
pub const WORK_HOURS: FactType = FactType::new("info.self.work.hours");

/// What the holder believes a recipe needs (Amendment A-18): a fact about a pair — the holder and
/// the recipe — holding a set of `[material id, how many, 0]`.
pub const RECIPE_NEEDS: FactType = FactType::new("info.belief.recipe.needs");
/// What the holder believes a recipe makes: a material (a fact about a pair).
pub const RECIPE_MAKES: FactType = FactType::new("info.belief.recipe.makes");
/// Where the holder believes a recipe is made: its workplace (a fact about a pair).
pub const RECIPE_AT: FactType = FactType::new("info.belief.recipe.at");

/// Whom the holder believes owns a thing (Amendment A-19): a fact about a pair. Absent: no owner
/// that it knows of.
pub const OWNER: FactType = FactType::new("info.belief.owner");

/// Every fact type this layer owns.
pub const OWNED: &[FactType] = &[
    OWNER,
    WORK_ROLE,
    WORK_CARRIES,
    WORK_MAKES,
    WORK_FROM,
    WORK_TO,
    WORK_KEEP,
    WORK_HOURS,
    RECIPE_NEEDS,
    RECIPE_MAKES,
    RECIPE_AT,
    FELT_HUNGER,
    NUTRITION,
    YIELDS,
    STOCK,
    MADE_OF,
    IN_SIGHT,
    PLACE_OF,
    LEADS_TO,
    OPEN,
    WARMTH_OF,
    FELT_BODY_HEAT,
    FELT_FATIGUE,
    FELT_HEALTH,
    FELT_NEED,
    FELT_NEED_MET_BY,
    AFFECTION,
    GOING_TO,
    BLOCKED,
    REFUSED,
];

// ---- Perceived: other owners' facts, read by their published ids (Vol. III Ch. 12 §12.1).

/// What a body could see — **Physical Reality's**.
pub const PHYS_IN_VIEW: FactType = FactType::new("physical.sense.in_view");
/// An entity's immediate container — **Physical Reality's**.
pub const PHYS_CONTAINED_IN: FactType = FactType::new("physical.space.contained_in");
/// Where an opening leads — **Physical Reality's**.
pub const PHYS_LEADS_TO: FactType = FactType::new("physical.space.leads_to");
/// How hungry an organism is — **Living Systems'**.
pub const LIVING_HUNGER: FactType = FactType::new("living.vital.hunger");
/// How much a material feeds — **Physical Reality's** material property.
pub const PHYS_NUTRITION: FactType = FactType::new("physical.material.nutrition");
/// What a deposit yields — **Resources'**.
pub const RES_YIELD: FactType = FactType::new("resources.yield.made_of");
/// A deposit's stock in hundredths of a unit — **Resources'**.
pub const RES_STOCK: FactType = FactType::new("resources.stock");
/// What a thing is made of — **Physical Reality's** (a set of materials).
pub const PHYS_MADE_OF: FactType = FactType::new("physical.material.made_of");
/// Whether an opening is open (absent means open) — **Physical Reality's**.
pub const PHYS_PORTAL_OPEN: FactType = FactType::new("physical.space.portal_open");
/// A place's air temperature, centidegrees — **Physical Reality's**.
pub const PHYS_TEMPERATURE: FactType = FactType::new("physical.environment.temperature");
/// Where a body is asked to travel — **Physical Reality's** intent.
pub const PHYS_TRAVEL_TO: FactType = FactType::new("physical.travel.to");
/// Whether a body's travel is blocked — **Physical Reality's** report.
pub const PHYS_TRAVEL_BLOCKED: FactType = FactType::new("physical.travel.blocked");
/// What a body last could not do — **Physical Reality's** report.
pub const PHYS_ACT_REFUSED: FactType = FactType::new("physical.act.refused");
/// An organism's body heat, centidegrees — **Living Systems'**.
pub const LIVING_BODY_HEAT: FactType = FactType::new("living.vital.body_heat");
/// A need that arose, about its object — **Living Systems'** (a fact about a pair).
pub const LIVING_NEED: FactType = FactType::new("living.need");
/// That need's kind, an entity — **Living Systems'** (a fact about a pair).
pub const LIVING_NEED_KIND: FactType = FactType::new("living.need.kind");
/// How a kind of need is met — **Living Systems'** (on the kind).
pub const LIVING_KIND_MET_BY: FactType = FactType::new("living.need_kind.met_by");
/// A mind's temperament, `[a, b, c]` — **decision systems'**.
pub const MIND_TEMPERAMENT: FactType = FactType::new("mind.temperament");
/// Whether a body can move — **Physical Reality's**.
pub const PHYS_MOBILE: FactType = FactType::new("physical.body.mobile");
/// Whether a body is solid: a fixture, not a thing one carries off — **Physical Reality's**.
pub const PHYS_SOLID: FactType = FactType::new("physical.body.solid");
/// The deposit a body last picked from — **Physical Reality's** report.
pub const PHYS_PICKED: FactType = FactType::new("physical.body.picked");
/// How tired an organism is — **Living Systems'**.
pub const LIVING_FATIGUE: FactType = FactType::new("living.vital.fatigue");
/// How alive an organism is — **Living Systems'**; zero is dead.
pub const LIVING_HEALTH: FactType = FactType::new("living.vital.health");
/// Who owns a thing — **Economy's**.
pub const ECON_OWNER: FactType = FactType::new("economy.owner");
/// The job a person holds — **Society's**.
pub const SOC_ROLE: FactType = FactType::new("society.role");
/// A carry job's goods — **Society's**.
pub const SOC_JOB_CARRIES: FactType = FactType::new("society.job.carries");
/// A make job's recipe — **Society's**.
pub const SOC_JOB_MAKES: FactType = FactType::new("society.job.makes");
/// Where a carry job gets its goods — **Society's**.
pub const SOC_JOB_FROM: FactType = FactType::new("society.job.from");
/// A job's store — **Society's**.
pub const SOC_JOB_TO: FactType = FactType::new("society.job.to");
/// How many goods a job keeps — **Society's**.
pub const SOC_JOB_KEEP: FactType = FactType::new("society.job.keep");
/// A job's hours — **Society's**.
pub const SOC_JOB_HOURS: FactType = FactType::new("society.job.hours");
/// How far an organism can see, centimetres — **Living Systems'**. Whoever has it perceives.
pub const LIVING_SIGHT_RANGE: FactType = FactType::new("living.sense.sight_range");
