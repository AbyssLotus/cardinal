//! Fact-type declarations owned by the Physical Reality domain (Appendix A; Vol. III Ch. 1).
//!
//! Physical Reality owns the stage: space (position, containment, elevation), connectivity,
//! and environmental state (Vol. III Ch. 1 §1.3). Every fact type appears exactly once, in
//! its owner's schema — one fact, one owner (Appendix A). Values are fixed-point integers,
//! never floats, so committed state carries no floating-point nondeterminism (Vol. V Ch. 4);
//! scales are this domain's convention. Consumers read these facts freely, by id
//! (Vol. III Ch. 12 §12.1).
//!
//! Both cardinalities are represented here. Cardinality-one facts (immediate containment,
//! scalar fields) hold at most one value per entity; cardinality-many facts — a region's
//! several neighbours in a topology ([`ADJACENT_TO`], §1.5) and the several portals a region
//! hosts ([`HAS_PORTAL`], §1.5) — are set-valued, which the store and the owning
//! [`crate::PhysicalDomain`]'s cardinality declaration support directly. A location's several
//! overlapping regions (Vol. III Ch. 1 §1.7) will layer on the same set-valued foundation.

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

/// An entity's local position within its immediate container, as fixed-point centimetres
/// along each axis (Vol. III Ch. 1 §1.3, position). Space is representation-independent
/// (§1.4) -- coordinates are one representation a world may choose; consumers ask spatial
/// questions rather than depending on this storage (see `crate::space`). Positions compose up
/// the containment hierarchy to give the relative position of any two loaded entities.
/// Frames are assumed axis-aligned (no inter-frame rotation); orientation is a later fact.
pub const POSITION_X: FactType = FactType::new("physical.space.position_x");
/// Local position along the Y axis; see [`POSITION_X`].
pub const POSITION_Y: FactType = FactType::new("physical.space.position_y");
/// Local position along the Z axis (height); see [`POSITION_X`].
pub const POSITION_Z: FactType = FactType::new("physical.space.position_z");

/// The regions directly connected to this one in a topology (Vol. III Ch. 1 §1.5). A
/// **cardinality-many** relationship: a region has several neighbours. Value is an entity
/// ref; seed both directions for an undirected edge. Distinct topologies (roads, rivers)
/// would be distinct fact types layered over the same regions, not a single graph
/// (Vol. III Ch. 1 §1.5, No Single Topology).
pub const ADJACENT_TO: FactType = FactType::new("physical.space.adjacent_to");

/// The destination a portal leads to -- the region on its far side (Vol. III Ch. 1 §1.5,
/// spatial connectivity; "Connected", §1.6). A portal is any located connection between
/// regions: a door, window, hatch, staircase, or a magical gate into a pocket region. The
/// portal is an entity, located in its host region (contained_in + position); this fact is
/// its far side. Single-valued and changeable -- a gangplank or a re-targetable gate just
/// Sets a new destination. This is CONNECTIVITY, distinct from adjacency: two rooms may
/// border yet be disconnected if no portal joins them (§1.5, "adjacent yet effectively
/// disconnected").
pub const LEADS_TO: FactType = FactType::new("physical.space.leads_to");

/// The set of portals a region hosts -- its exits (Vol. III Ch. 1 §1.5). A **cardinality-many**
/// relationship: a region may have several portals (a room with two doors and a window). Each
/// value is a portal entity, itself located within the region and carrying a [`LEADS_TO`]
/// destination.
pub const HAS_PORTAL: FactType = FactType::new("physical.space.has_portal");

/// How dangerous it is to traverse a portal, 0..=10000 (Vol. III Ch. 1 §1.11, physical
/// constraints). The *effective* danger, written every tick by the danger system: if a world
/// pins [`PORTAL_DANGER_OVERRIDE`] the system echoes it; otherwise it derives danger from the
/// portal's height above the ground (a 3rd-storey window is perilous, a ground-floor door is
/// not) -- with room for weather to raise it later (a storm-lashed ledge). Consumers read
/// this fact.
pub const PORTAL_DANGER: FactType = FactType::new("physical.space.portal_danger");

/// A world-authored fixed danger for a portal, 0..=10000 (optional). When present it pins
/// [`PORTAL_DANGER`] to this value regardless of height or weather -- a magically warded gate
/// that is always deadly, or a padded chute that never is.
pub const PORTAL_DANGER_OVERRIDE: FactType = FactType::new("physical.space.portal_danger_override");

/// How open a location is to the outside sky, in hundredths of a percent (0..=10000)
/// (Vol. III Ch. 1 §1.6, Enclosed / Exposed; §1.11, sheltered). Full is open ground under
/// open sky; a forest floor or cave mouth is partial; a sealed chamber is 0. Surface-weather
/// systems scale their effect by this, so enclosed places get muted swings, no rain, and
/// darkness. A location with no exposure fact is treated as fully exposed.
pub const EXPOSURE: FactType = FactType::new("physical.space.exposure");

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

// ---- Materials (Vol. III Ch. 1 §1.9) ---------------------------------------------------
//
// Materials describe what reality is *composed of*. Cardinal never prescribes a material
// catalogue: a material is an entity that exposes *properties*, not a name (§1.9, Designer
// Note "Properties Over Names"). A physical object references the materials it is made of
// through [`MADE_OF`]; higher domains (Resources, Conflict, Knowledge — Appendix A) reason
// about the properties, never about "wood" or "steel". Composites are expected, not
// exceptional, so [`MADE_OF`] is cardinality-many. Each property below is optional: a
// material exposes only the characteristics it has, and a consumer asks through
// [`crate::materials`] rather than depending on which facts are present.

/// The materials a physical object is composed of (Vol. III Ch. 1 §1.9). A **cardinality-many**
/// relationship: an object may be a composite of several materials (a house of timber, glass,
/// and steel). Each value is a material entity — itself carrying the property facts below.
pub const MADE_OF: FactType = FactType::new("physical.material.made_of");

/// A material's density, in kilograms per cubic metre (Vol. III Ch. 1 §1.9). A property of the
/// material entity, read by consumers that reason about mass and buoyancy.
pub const MATERIAL_DENSITY: FactType = FactType::new("physical.material.density");

/// A material's hardness / structural strength, 0..=10000 (a normalized property, hundredths
/// of a percent). Governs whether it survives applied stress: a structure fails at its
/// weakest material, not because of what it is named (§1.9, the bridge that "fails because the
/// material cannot support the required stress").
pub const MATERIAL_HARDNESS: FactType = FactType::new("physical.material.hardness");

/// A material's specific heat capacity, in joules per kilogram-kelvin (Vol. III Ch. 1 §1.9).
/// How much energy it takes to change the material's temperature — its thermal inertia. Read
/// by environmental simulation; a high-capacity mass resists the diurnal swing.
pub const MATERIAL_THERMAL_CAPACITY: FactType = FactType::new("physical.material.thermal_capacity");

/// A material's flammability, 0..=10000 (normalized). Zero is inert; higher ignites more
/// readily. Fire spreads where nearby materials satisfy ignition conditions (§1.9), so a
/// composite's fire behaviour follows its most flammable constituent.
pub const MATERIAL_FLAMMABILITY: FactType = FactType::new("physical.material.flammability");

/// A material's conductivity, 0..=10000 (normalized) — thermal/electrical transport
/// (Vol. III Ch. 1 §1.9). Read by consumers reasoning about heat flow or current.
pub const MATERIAL_CONDUCTIVITY: FactType = FactType::new("physical.material.conductivity");

/// A material's toxicity, 0..=10000 (normalized). Zero is inert; higher is more hazardous to
/// living systems (Vol. III Ch. 1 §1.9). A composite is as hazardous as its most toxic part.
pub const MATERIAL_TOXICITY: FactType = FactType::new("physical.material.toxicity");

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

/// Ceiling for a portal's danger value (0 = harmless, 10000 = as dangerous as the model goes).
pub const MAX_DANGER: i64 = 10000;

/// Ceiling for material density (kg/m³) — clamps the field well above any real or exotic
/// material (osmium ≈ 22 600; room left for programmable matter, Vol. III Ch. 1 §1.9).
pub const MAX_DENSITY: i64 = 1_000_000;

/// Ceiling for material specific heat capacity (J/(kg·K)) — clamps well above any real value
/// (water ≈ 4184, hydrogen ≈ 14 300).
pub const MAX_THERMAL_CAPACITY: i64 = 1_000_000;
