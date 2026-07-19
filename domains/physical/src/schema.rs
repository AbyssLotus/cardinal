//! Fact-type declarations owned by the Physical Reality domain (Appendix A; Vol. III Ch. 1).
//!
//! Physical Reality owns the stage: space (position, containment, elevation), connectivity,
//! and environmental state (Vol. III Ch. 1 §1.3). Every fact type appears exactly once, in
//! its owner's schema — one fact, one owner (Appendix A). Values are fixed-point integers,
//! never floats, so committed state carries no floating-point nondeterminism (Vol. V Ch. 4);
//! scales are this domain's convention. Consumers read these facts freely, by id
//! (Vol. III Ch. 12 §12.1).
//!
//! Many-valued spatial relationships — a location's several overlapping regions
//! (Vol. III Ch. 1 §1.7) and a region's several neighbours in a topology (§1.5) — await a
//! cardinality-many fact model; the single-valued facts below (immediate containment,
//! scalar fields) are what the current store represents.

use kernel::fact::FactType;

// ---- Space -----------------------------------------------------------------------------

/// An entity's immediate container — the region or container it exists *within*
/// (Vol. III Ch. 1 §1.8). Single-valued: the innermost enclosing entity. Hierarchical
/// containment (planet ⊃ continent ⊃ region ⊃ …) is walked by following this link upward;
/// it is state, not immutable structure (§1.8, Dynamic Containment). Value is an entity ref.
pub const CONTAINED_IN: FactType = FactType::new("physical.space.contained_in");

/// A location's elevation, as fixed-point centimetres above a world datum (Vol. III Ch. 1
/// §1.3, elevation). May be negative (below the datum). A spatial property of place.
pub const ELEVATION: FactType = FactType::new("physical.space.elevation");

// ---- Environmental state (facets of one shared environment, Vol. III Ch. 1 §1.10) ------

/// Ambient temperature of a location, as fixed-point centidegrees Celsius. Owned by Physical
/// Reality; consumed by Living Systems, Ecology, Economy (Appendix A). "A deer experiences
/// cold; the forest owns the temperature" (Vol. III Ch. 1 §1.10).
pub const TEMPERATURE: FactType = FactType::new("physical.environment.temperature");

/// Illumination at a location, as fixed-point hundredths of a percent (0..=10000). A field
/// that moves across the landscape with the sun (Vol. III Ch. 1 §1.10, Time and Change).
pub const ILLUMINATION: FactType = FactType::new("physical.environment.illumination");

/// Relative humidity at a location, as fixed-point hundredths of a percent (0..=10000)
/// (Vol. III Ch. 1 §1.10, environmental state).
pub const HUMIDITY: FactType = FactType::new("physical.environment.humidity");

// ---- Physical constants (laws of the mechanism, not tunable world rules) ----------------

/// Absolute zero in centidegrees Celsius (−273.15 °C) — the floor below which temperature is
/// physically meaningless. The Validate stage rejects any resolved temperature beneath it.
pub const ABSOLUTE_ZERO_CENTI_C: i64 = -27315;

/// The maximum value of a percentage field (illumination, humidity): 100.00%.
pub const PERCENT_FULL: i64 = 10000;
