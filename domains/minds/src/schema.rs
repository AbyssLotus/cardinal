//! Decision systems' fact types, the beliefs they read, and the intents they propose (Appendix
//! A, Ruling 14; Amendment A-9).
//!
//! A mind's reads are the information layer's facts and its own — nothing else. The test at the
//! bottom of `lib.rs` holds every system here to that.

use kernel::fact::FactType;

// ---- Owned: what a mind is committed to, and why.

/// How fast a mind walks when it chooses to go somewhere, cm/s. Having one is having a mind.
pub const WALK_SPEED: FactType = FactType::new("mind.walk_speed");

/// A routine the world gives a mind: a fact about a pair — the mind, and where the routine takes
/// it — whose value is `[from, to, 0]`, seconds into the day (a window that wraps past midnight
/// when `from > to`).
pub const ROUTINE: FactType = FactType::new("mind.routine");

/// Where the mind has committed to go (an entity reference): a place, or a thing in a place.
pub const GOAL: FactType = FactType::new("mind.goal");

/// Why: [`REASON_WARMTH`] or [`REASON_ROUTINE`]. Part of the decision trace.
pub const REASON: FactType = FactType::new("mind.goal.reason");

/// How strongly: the decisive score the goal won with. Part of the decision trace.
pub const SCORE: FactType = FactType::new("mind.goal.score");

/// The plan's current step toward the goal: [`STEP_GO`], [`STEP_APPROACH`], or [`STEP_OPEN`].
/// Its provenance tick is when the step was taken.
pub const STEP: FactType = FactType::new("mind.step");

/// The opening the plan's step concerns (an entity reference), when it concerns one.
pub const VIA: FactType = FactType::new("mind.step.via");

/// The thing the mind last arrived at and stands by (an entity reference).
pub const AT: FactType = FactType::new("mind.at");

/// Whether the mind has lain down to rest (Amendment A-10): its own record of the rest it asked
/// its body for, and part of the trace.
pub const RESTING: FactType = FactType::new("mind.resting");

/// A mind's temperament (Amendment A-11): three traits, each in hundredths of a percent, as
/// `[a, b, c]`. How compatible two minds are — 100% less the mean difference of their traits —
/// bounds how fond they can grow of each other.
pub const TEMPERAMENT: FactType = FactType::new("mind.temperament");

/// Whether an outside controller — a player, a recorded model, a test — is directing this entity
/// (Amendment A-14). While true, its own mind stands aside and proposes nothing. Anyone may set it.
pub const DIRECTED: FactType = FactType::new("mind.directed");

/// What the plan's current step acts on (an entity reference): the thing to take or pick from, the
/// thing to put down, the recipe to make (Amendment A-18). A step about something else starts
/// over.
pub const OBJECT: FactType = FactType::new("mind.step.object");

/// What the mind tried to get and could not (Amendment A-18): a fact about a pair — the mind,
/// and the thing — holding the tick it gave up. Until it learns something newer of where the thing
/// is, it does not try again.
pub const GAVE_UP_ON: FactType = FactType::new("mind.gave_up_on");

/// How much seeing a place it has never stood in is worth to a mind (Amendment A-19): its
/// curiosity, a disposition the world declares. Absent: incurious.
pub const CURIOSITY: FactType = FactType::new("mind.curiosity");

/// How much a mind likes a material (Amendment A-19): a fact about a pair — the mind, and the
/// material — holding the worth to it of a thing made of it.
pub const LIKES: FactType = FactType::new("mind.likes");

/// Where a mind keeps what it owns: its home, a place (Amendment A-19).
pub const HOME: FactType = FactType::new("mind.home");

/// Every fact type decision systems own.
pub const OWNED: &[FactType] = &[
    OBJECT,
    GAVE_UP_ON,
    CURIOSITY,
    LIKES,
    HOME,
    WALK_SPEED,
    ROUTINE,
    GOAL,
    REASON,
    SCORE,
    STEP,
    VIA,
    AT,
    RESTING,
    TEMPERAMENT,
    DIRECTED,
    ABOUT,
];

/// The goal is warmth: a place remembered warmer than here.
pub const REASON_WARMTH: i64 = 1;
/// The goal is a routine the world gives the mind for this hour.
pub const REASON_ROUTINE: i64 = 2;
/// The goal answers a need that arose — to be with someone, for instance (Amendment A-11).
pub const REASON_NEED: i64 = 3;
/// The goal is a thing to fetch and consume, for a need met by a dose of what it is made of
/// (Amendment A-13).
pub const REASON_DOSE: i64 = 4;
/// The goal is something to eat: food, or a tree that bears it (Amendment A-16).
pub const REASON_HUNGER: i64 = 5;

/// The goal is the mind's work: the job it holds (Amendment A-18).
pub const REASON_WORK: i64 = 6;
/// The goal is a place the mind knows a way to but has never stood in (Amendment A-19).
pub const REASON_CURIOSITY: i64 = 7;
/// The goal is a thing the mind wants: one it owns, to keep at home, or one no one owns, made of
/// something it likes, to claim (Amendment A-19).
pub const REASON_WANT: i64 = 8;

/// A need met by its object's presence (Living Systems' mechanism code, Amendment A-11).
pub const MET_BY_PRESENCE: i64 = 1;
/// A need met by a dose of its object (Living Systems' mechanism code, Amendment A-13).
pub const MET_BY_DOSE: i64 = 2;

/// Travelling toward the goal.
pub const STEP_GO: i64 = 0;
/// Walking to an opening on the way, to open it.
pub const STEP_APPROACH: i64 = 1;
/// Opening it.
pub const STEP_OPEN: i64 = 2;
/// Walking up to a thing, in its place, to take it (Amendment A-13).
pub const STEP_FETCH: i64 = 3;
/// Taking it.
pub const STEP_TAKE: i64 = 4;
/// Consuming it.
pub const STEP_CONSUME: i64 = 5;
/// Picking from a deposit — a tree (Amendment A-16).
pub const STEP_PICK: i64 = 6;
/// Putting something down where the mind stands — in a store (Amendment A-18).
pub const STEP_DROP: i64 = 7;
/// Making a recipe at its workplace (Amendment A-18).
pub const STEP_MAKE: i64 = 8;
/// Claiming a thing no one owns (Amendment A-19).
pub const STEP_CLAIM: i64 = 9;

/// What a consuming goal is for: the substance a craving wants (an entity reference). Absent for
/// hunger, which any food meets.
pub const ABOUT: FactType = FactType::new("mind.goal.about");

// ---- Read: the information layer's beliefs, by their published ids (never imported).

/// Where the mind believes something is (a fact about a pair).
pub const BELIEF_PLACE_OF: FactType = FactType::new("info.belief.place_of");
/// How hungry the mind feels (Amendment A-16).
pub const FELT_HUNGER: FactType = FactType::new("info.self.hunger");
/// How much the mind believes a material feeds (a fact about a pair).
pub const BELIEF_NUTRITION: FactType = FactType::new("info.belief.nutrition");
/// What the mind believes a deposit yields (a fact about a pair).
pub const BELIEF_YIELDS: FactType = FactType::new("info.belief.yields");
/// How many units the mind believes a deposit holds (a fact about a pair).
pub const BELIEF_STOCK: FactType = FactType::new("info.belief.stock");
/// What the mind believes a thing is made of (a fact about a pair; a set of materials).
pub const BELIEF_MADE_OF: FactType = FactType::new("info.belief.made_of");
/// Where the mind believes an opening leads (a fact about a pair).
pub const BELIEF_LEADS_TO: FactType = FactType::new("info.belief.leads_to");
/// How warm the mind found a place (a fact about a pair).
pub const BELIEF_WARMTH_OF: FactType = FactType::new("info.belief.warmth_of");
/// How warm the mind's body feels.
pub const FELT_BODY_HEAT: FactType = FactType::new("info.self.body_heat");
/// How tired the mind feels (Amendment A-10).
pub const FELT_FATIGUE: FactType = FactType::new("info.self.fatigue");
/// How well the mind feels; zero is dead (Amendment A-10).
pub const FELT_HEALTH: FactType = FactType::new("info.self.health");
/// How strongly the mind feels a need that arose, about its object (a fact about a pair).
pub const FELT_NEED: FactType = FactType::new("info.self.need");
/// How that need is met, as the mind knows it — [`MET_BY_PRESENCE`] or [`MET_BY_DOSE`] (a fact
/// about a pair).
pub const FELT_NEED_MET_BY: FactType = FactType::new("info.self.need_met_by");
/// Where the mind knows it is trying to go.
pub const GOING_TO: FactType = FactType::new("info.self.going_to");
/// Whether the mind knows its way is blocked.
pub const BLOCKED: FactType = FactType::new("info.self.blocked");
/// What the mind last could not do.
pub const REFUSED: FactType = FactType::new("info.self.refused");
/// Whom the mind believes owns a thing (a fact about a pair; Amendment A-19).
pub const BELIEF_OWNER: FactType = FactType::new("info.belief.owner");
/// The job the mind holds (Amendment A-18).
pub const WORK_ROLE: FactType = FactType::new("info.self.work.role");
/// The material its work carries.
pub const WORK_CARRIES: FactType = FactType::new("info.self.work.carries");
/// The recipe its work makes.
pub const WORK_MAKES: FactType = FactType::new("info.self.work.makes");
/// Where its work gets its goods.
pub const WORK_FROM: FactType = FactType::new("info.self.work.from");
/// The store its work keeps.
pub const WORK_TO: FactType = FactType::new("info.self.work.to");
/// How many goods its work keeps in the store.
pub const WORK_KEEP: FactType = FactType::new("info.self.work.keep");
/// Its working hours, seconds into the day `[from, to, 0]`.
pub const WORK_HOURS: FactType = FactType::new("info.self.work.hours");
/// What the mind believes a recipe needs: a set of `[material id, how many, 0]` (a fact about a
/// pair).
pub const BELIEF_RECIPE_NEEDS: FactType = FactType::new("info.belief.recipe.needs");
/// What the mind believes a recipe makes (a fact about a pair).
pub const BELIEF_RECIPE_MAKES: FactType = FactType::new("info.belief.recipe.makes");
/// Where the mind believes a recipe is made (a fact about a pair).
pub const BELIEF_RECIPE_AT: FactType = FactType::new("info.belief.recipe.at");

// ---- Written: Physical Reality's intents, open to every decider (Ruling 13).

/// Where to travel.
pub const TRAVEL_TO: FactType = FactType::new("physical.travel.to");
/// How fast.
pub const TRAVEL_SPEED: FactType = FactType::new("physical.travel.speed");
/// What to open.
pub const ACT_OPEN: FactType = FactType::new("physical.act.open");
/// What to take hold of (Appendix A, Ruling 16).
pub const ACT_TAKE: FactType = FactType::new("physical.act.take");
/// What to consume.
pub const ACT_CONSUME: FactType = FactType::new("physical.act.consume");
/// What to pick from (Amendment A-15).
pub const ACT_PICK: FactType = FactType::new("physical.act.pick");
/// What to put down (Appendix A, Ruling 16).
pub const ACT_DROP: FactType = FactType::new("physical.act.drop");
/// What recipe to make — **Economy's** intent (Appendix A, Ruling 17).
pub const ACT_MAKE: FactType = FactType::new("economy.act.make");
/// What to claim — **Economy's** intent (Appendix A, Ruling 18).
pub const ACT_CLAIM: FactType = FactType::new("economy.act.claim");
/// To lie down and rest — **Living Systems'** intent (Appendix A, Ruling 15).
pub const REST: FactType = FactType::new("living.act.rest");
