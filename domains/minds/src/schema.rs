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

/// Every fact type decision systems own.
pub const OWNED: &[FactType] = &[WALK_SPEED, ROUTINE, GOAL, REASON, SCORE, STEP, VIA, AT];

/// The goal is warmth: a place remembered warmer than here.
pub const REASON_WARMTH: i64 = 1;
/// The goal is a routine the world gives the mind for this hour.
pub const REASON_ROUTINE: i64 = 2;

/// Travelling toward the goal.
pub const STEP_GO: i64 = 0;
/// Walking to an opening on the way, to open it.
pub const STEP_APPROACH: i64 = 1;
/// Opening it.
pub const STEP_OPEN: i64 = 2;

// ---- Read: the information layer's beliefs, by their published ids (never imported).

/// Where the mind believes something is (a fact about a pair).
pub const BELIEF_PLACE_OF: FactType = FactType::new("info.belief.place_of");
/// Where the mind believes an opening leads (a fact about a pair).
pub const BELIEF_LEADS_TO: FactType = FactType::new("info.belief.leads_to");
/// How warm the mind found a place (a fact about a pair).
pub const BELIEF_WARMTH_OF: FactType = FactType::new("info.belief.warmth_of");
/// How warm the mind's body feels.
pub const FELT_BODY_HEAT: FactType = FactType::new("info.self.body_heat");
/// Where the mind knows it is trying to go.
pub const GOING_TO: FactType = FactType::new("info.self.going_to");
/// Whether the mind knows its way is blocked.
pub const BLOCKED: FactType = FactType::new("info.self.blocked");
/// What the mind last could not do.
pub const REFUSED: FactType = FactType::new("info.self.refused");

// ---- Written: Physical Reality's intents, open to every decider (Ruling 13).

/// Where to travel.
pub const TRAVEL_TO: FactType = FactType::new("physical.travel.to");
/// How fast.
pub const TRAVEL_SPEED: FactType = FactType::new("physical.travel.speed");
/// What to open.
pub const ACT_OPEN: FactType = FactType::new("physical.act.open");
