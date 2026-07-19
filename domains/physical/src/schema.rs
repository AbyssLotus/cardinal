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

/// The regions directly connected to this one in a topology (Vol. III Ch. 1 §1.5). A
/// **cardinality-many** relationship: a region has several neighbours. Value is an entity
/// ref; seed both directions for an undirected edge. Distinct topologies (roads, rivers)
/// would be distinct fact types layered over the same regions, not a single graph
/// (Vol. III Ch. 1 §1.5, No Single Topology).
pub const ADJACENT_TO: FactType = FactType::new("physical.space.adjacent_to");

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

/// Atmospheric pressure at a location, in decapascals (hPa x 10) as fixed-point
/// (Vol. III Ch. 1 §1.10). Falls with elevation and drifts with weather; the pressure
/// gradient between adjacent regions is what drives wind.
pub const PRESSURE: FactType = FactType::new("physical.environment.pressure");

/// Wind speed at a location, in centimetres per second (Vol. III Ch. 1 §1.10, "Wind flows").
/// Magnitude only; direction is [`WIND_TOWARD`].
pub const WIND_SPEED: FactType = FactType::new("physical.environment.wind_speed");

/// The neighbouring region the wind blows toward — wind's direction expressed *in the
/// graph* rather than as a compass bearing, since space is representation-independent
/// (Vol. III Ch. 1 §1.4; "downwind" is a spatial relation, §1.6). An entity ref to the
/// downwind region; absent when the air is calm.
pub const WIND_TOWARD: FactType = FactType::new("physical.environment.wind_toward");

// ---- Physical constants (laws of the mechanism, not tunable world rules) ----------------

/// Absolute zero in centidegrees Celsius (−273.15 °C) — the floor below which temperature is
/// physically meaningless. The Validate stage rejects any resolved temperature beneath it.
pub const ABSOLUTE_ZERO_CENTI_C: i64 = -27315;

/// The maximum value of a percentage field (illumination, humidity): 100.00%.
pub const PERCENT_FULL: i64 = 10000;

/// Ceiling for atmospheric pressure (decapascals) — clamps the field to a sane range, well
/// above any real surface pressure (~1013 hPa = 10130 decapascals).
pub const MAX_PRESSURE: i64 = 20000;

/// Ceiling for wind speed (centimetres/second) — clamps the field well above any real wind.
pub const MAX_WIND: i64 = 100000;
