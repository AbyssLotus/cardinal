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

/// Whether the holder believes an opening is open. A fact about a pair.
pub const OPEN: FactType = FactType::new("info.belief.open");

/// How warm the holder found a place when it stood there, in centidegrees Celsius. A fact
/// about a pair: the holder's memory of that place's air.
pub const WARMTH_OF: FactType = FactType::new("info.belief.warmth_of");

/// How warm the holder's own body feels, in centidegrees Celsius.
pub const FELT_BODY_HEAT: FactType = FactType::new("info.self.body_heat");

/// Where the holder knows it is trying to go (an entity reference): its own travel intent.
pub const GOING_TO: FactType = FactType::new("info.self.going_to");

/// Whether the holder knows its way is blocked.
pub const BLOCKED: FactType = FactType::new("info.self.blocked");

/// What the holder last tried and could not do (an entity reference).
pub const REFUSED: FactType = FactType::new("info.self.refused");

/// Every fact type this layer owns.
pub const OWNED: &[FactType] = &[
    IN_SIGHT,
    PLACE_OF,
    LEADS_TO,
    OPEN,
    WARMTH_OF,
    FELT_BODY_HEAT,
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
/// How far an organism can see, centimetres — **Living Systems'**. Whoever has it perceives.
pub const LIVING_SIGHT_RANGE: FactType = FactType::new("living.sense.sight_range");
