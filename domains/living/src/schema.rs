//! Fact-type declarations owned by the Living Systems domain, and the cross-domain reads it
//! consumes (Appendix A; Vol. III Ch. 12 §12.1).
//!
//! One fact, one owner (Appendix A). Living Systems owns vital state; the temperature and
//! containment it reads are Physical Reality's, consumed by their stable ids — published
//! contracts, not imports. This crate has ZERO code dependency on the physical crate
//! (Vol. III Ch. 12, invariant 1); the two domains meet only in committed reality.

use kernel::fact::FactType;

/// An organism's body heat, as fixed-point centidegrees Celsius — vital-state physiology
/// (Vol. III Ch. 2, vital state), the substrate of the "warmth" need that tracks the
/// environment. Owned by Living Systems; fixed-point, not float (Vol. V Ch. 4).
pub const BODY_HEAT: FactType = FactType::new("living.vital.body_heat");

/// How far an organism can see, in centimetres — its first sensory capability (Vol. III Ch. 2
/// §2.2; Amendment A-8). What it can perceive, never what it knows: Physical Reality reports
/// what lies in view within this range, and the information layer decides what is observed.
pub const SIGHT_RANGE: FactType = FactType::new("living.sense.sight_range");

/// How tired an organism is, in hundredths of a percent (0 rested, 10000 exhausted) — a need
/// (Vol. III Ch. 2 §2.4; Amendment A-10): it rises while the organism is awake and falls while it
/// rests, at rates the world declares.
pub const FATIGUE: FactType = FactType::new("living.vital.fatigue");

/// How hungry an organism is, in hundredths of a percent (0 fed, 10000 starved) — a need
/// (Amendment A-16): it rises at the world's rate, and eating lowers it by the food's nutrition.
pub const HUNGER: FactType = FactType::new("living.vital.hunger");

/// How much a unit of a material feeds a body — **Physical Reality's** material property.
pub const MATERIAL_NUTRITION: FactType = FactType::new("physical.material.nutrition");

/// How alive an organism is, in hundredths of a percent (Amendment A-10): harmed by cold and by
/// falls, recovering slowly when nothing harms it. Death is this reaching zero — an event, not a
/// flag (Vol. III Ch. 2 §2.8, invariant 1); from then on the organism's processing stops.
pub const HEALTH: FactType = FactType::new("living.vital.health");

/// A decider's intent that an organism rest (Appendix A, Ruling 15): a boolean, open to any
/// decider. Living Systems carries it out only for a body that is still.
pub const REST: FactType = FactType::new("living.act.rest");

/// The tick of the last fall this domain has judged, so each fall harms once (Amendment A-10).
pub const FALL_JUDGED: FactType = FactType::new("living.vital.fall_judged");

/// A need that arose (Amendment A-11): a fact about a pair — the organism, and what the need is
/// about — whose value is how strongly it is felt, in hundredths of a percent.
pub const NEED: FactType = FactType::new("living.need");

/// What kind of need that is (a fact about a pair): an entity the world declares, carrying the
/// kind's rules below.
pub const NEED_KIND: FactType = FactType::new("living.need.kind");

/// What makes a kind of need arise: [`ARISES_FROM_BOND`] or [`ARISES_FROM_DEPENDENCE`].
pub const KIND_ARISES: FactType = FactType::new("living.need_kind.arises");
/// What meets it: [`MET_BY_PRESENCE`] or [`MET_BY_DOSE`].
pub const KIND_MET_BY: FactType = FactType::new("living.need_kind.met_by");
/// How much it grows per hour while unmet, in hundredths of a percent.
pub const KIND_RISE: FactType = FactType::new("living.need_kind.rise");
/// How much it eases per hour while met.
pub const KIND_EASE: FactType = FactType::new("living.need_kind.ease");
/// How much health it costs per hour, felt in full.
pub const KIND_HARM: FactType = FactType::new("living.need_kind.harm");

/// A kind of need that arises from a bond to someone (Society's).
pub const ARISES_FROM_BOND: i64 = 1;
/// A kind of need that arises from a dependence on something (Amendment A-13).
pub const ARISES_FROM_DEPENDENCE: i64 = 2;
/// A kind of need met by its object's presence: in the same place, or in view.
pub const MET_BY_PRESENCE: i64 = 1;
/// A kind of need met by a dose of its object (Amendment A-13).
pub const MET_BY_DOSE: i64 = 2;

/// How dependent an organism is on a substance (Amendment A-13): a fact about a pair — the
/// organism, and the material — in hundredths of a percent.
pub const DEPENDENCE: FactType = FactType::new("living.dependence");

/// The tick of the last consumption this domain has judged, so each dose acts once.
pub const DOSE_JUDGED: FactType = FactType::new("living.vital.dose_judged");

/// For a kind that arises from dependence: the dependence past which it comes into being.
pub const KIND_ABOVE: FactType = FactType::new("living.need_kind.above");

/// What a body last consumed — **Physical Reality's** report.
pub const CONSUMED: FactType = FactType::new("physical.body.consumed");
/// What a thing is made of — **Physical Reality's** (a set of materials).
pub const MADE_OF: FactType = FactType::new("physical.material.made_of");
/// How strongly a dose of a material acts — **Physical Reality's** material property.
pub const MATERIAL_POTENCY: FactType = FactType::new("physical.material.potency");
/// How much dependence a dose builds — **Physical Reality's** material property.
pub const MATERIAL_HABIT: FactType = FactType::new("physical.material.habit");

/// A bond between two persons — **Society's** (a fact about a pair).
pub const SOCIETY_BOND: FactType = FactType::new("society.bond");

/// What a body could see — **Physical Reality's**.
pub const IN_VIEW: FactType = FactType::new("physical.sense.in_view");

/// The full measure of a need or of health, in hundredths of a percent.
pub const FULL: i64 = 10_000;

/// How far a body fell in its most recent fall, in centimetres — **Physical Reality's** report,
/// consumed by id to judge the harm (Appendix A, Ruling 9).
pub const FALL_HEIGHT: FactType = FactType::new("physical.body.fall_height");

/// The end of a moving body's current segment — **Physical Reality's**; present while it moves.
pub const MOTION_TARGET: FactType = FactType::new("physical.motion.target");

/// Where a body has been asked to travel — **Physical Reality's** intent; present while it goes.
pub const TRAVEL_TO: FactType = FactType::new("physical.travel.to");

/// An entity's immediate container — **Physical Reality's** fact (Appendix A), consumed here
/// by its stable id to learn which region an organism inhabits. Naming an id is not
/// importing code (Vol. III Ch. 12 §12.1).
pub const CONTAINED_IN: FactType = FactType::new("physical.space.contained_in");

/// Ambient temperature of a region — **Physical Reality's** fact (Appendix A), consumed here
/// by its stable id. If no enabled domain owns a consumed id, package validation catches the
/// dangling read (Vol. IV Ch. 2).
pub const AMBIENT_TEMPERATURE: FactType = FactType::new("physical.environment.temperature");

/// Absolute zero in centidegrees Celsius (−273.15 °C) — the floor below which body heat is
/// physically meaningless. The Validate stage rejects any resolved body heat beneath it.
pub const BODY_HEAT_FLOOR_CENTI_C: i64 = -27315;

/// Every fact type of needs that arise and their kinds (Amendment A-11).
pub const NEED_FACTS: &[FactType] = &[
    DEPENDENCE,
    DOSE_JUDGED,
    KIND_ABOVE,
    NEED,
    NEED_KIND,
    KIND_ARISES,
    KIND_MET_BY,
    KIND_RISE,
    KIND_EASE,
    KIND_HARM,
];
