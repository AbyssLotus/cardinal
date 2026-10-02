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
//! hosts ([`HAS_PORTAL`], §1.5), and a location's several overlapping regions ([`IN_REGION`],
//! §1.7) — are set-valued, which the store and the owning [`crate::PhysicalDomain`]'s
//! cardinality declaration support directly.

use kernel::fact::FactType;

// ---- Space -----------------------------------------------------------------------------

/// An entity's immediate container — the region or container it exists *within*
/// (Vol. III Ch. 1 §1.8). Single-valued: the innermost enclosing entity. Hierarchical
/// containment (planet ⊃ continent ⊃ region ⊃ …) is walked by following this link upward;
/// it is state, not immutable structure (§1.8, Dynamic Containment). Value is an entity ref.
pub const CONTAINED_IN: FactType = FactType::new("physical.space.contained_in");

/// The regions an entity belongs to *beyond* its containment chain — its overlapping
/// classifications (Vol. III Ch. 1 §1.7). A **cardinality-many** relationship: one farmhouse
/// lies in a county (its container) and also in a watershed, a climate zone, and a fox's
/// territory, none of which nest inside one another. Value is an entity ref to a region.
///
/// Membership means *lies wholly within*, and it is inherited: whatever an entity contains
/// lies in that entity's regions too, and a region that is itself a member of a larger region
/// passes that membership on (see [`crate::regions`]). Containment stays the single hierarchy
/// that gives a location its coordinate frame (§1.8); this fact carries every other
/// grouping, so "Cardinal intentionally avoids forcing locations into a single hierarchy"
/// (§1.7) without a second frame to reconcile. A region need not be geometric or contiguous:
/// an island nation is the islands that name it here (§1.7). Membership is state, not
/// structure — a herd entering a territory is an Add, leaving it a Remove (§1.8, Dynamic
/// Containment).
pub const IN_REGION: FactType = FactType::new("physical.space.in_region");

/// A location's elevation, as fixed-point centimetres above a world datum (Vol. III Ch. 1
/// §1.3, elevation). May be negative (below the datum). A spatial property of place.
pub const ELEVATION: FactType = FactType::new("physical.space.elevation");

/// An entity's position within its immediate container: a three-component value
/// ([`Value::Vec3`](kernel::value::Value::Vec3)) of fixed-point centimetres `[x, y, z]` in the
/// container's local frame — `+x` to the container's right (east, if it faces north), `+y`
/// ahead (north), `+z` up (Vol. III Ch. 1 §1.3; Amendment A-3). It is where the entity's
/// **base** sits: the point it stands or rests on. One fact, written atomically.
///
/// Space is representation-independent (§1.4) — coordinates are one representation a world may
/// choose; consumers ask spatial questions (`crate::space`, `crate::nearby`) rather than reading
/// this directly, not least because while an entity is in motion this fact holds where its
/// current segment *began* ([`MOTION_TARGET`]). Positions compose up the containment hierarchy,
/// rotated by each container's [`HEADING`], to give the relative position of any two entities
/// in the same hierarchy. Absent means the container's origin.
pub const POSITION: FactType = FactType::new("physical.space.position");

/// A body's size, as a three-component value of centimetres `[half_width, half_depth, height]`
/// (Amendment A-3): it spans `half_width` to either side of its base along its own x axis,
/// `half_depth` fore and aft along its y axis, and `height` upward from its base. Turned by the
/// body's [`HEADING`]. Absent means the body is a point. Size is what answers *does it fit*,
/// *what does it overlap*, and *what is it resting on*.
pub const BODY_SIZE: FactType = FactType::new("physical.body.size");

/// A body's facing, in hundredths of a degree, as a compass bearing within its container's
/// frame: 0 faces `+y` (north), 9000 faces `+x` (east), increasing clockwise; always in
/// `0..36000` (Amendment A-3). A container's heading also orients the frame of everything in it
/// — when a ship turns, its deck turns with it. Absent means 0. Pitch and roll are not yet
/// represented.
pub const HEADING: FactType = FactType::new("physical.body.heading");

/// Where a moving body is heading: the end of its current straight segment, as a
/// three-component value of centimetres in its container's frame (Amendment A-3). Its
/// [`POSITION`] holds where the segment began. Present only while the body has a segment;
/// cleared when it settles.
pub const MOTION_TARGET: FactType = FactType::new("physical.motion.target");

/// The tick a body's current segment began (it was at its [`POSITION`] then).
pub const MOTION_START: FactType = FactType::new("physical.motion.start");

/// The tick a body's current segment ends (it is at its [`MOTION_TARGET`] from then on).
/// Between the two ticks its position is interpolated along the segment — derived, never
/// stored, so nothing is written while it travels.
pub const MOTION_END: FactType = FactType::new("physical.motion.end");

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

// ---- Constraints, ground, and travel (Vol. III Ch. 1 §1.11; Amendment A-4) --------------

/// Whether a body is **solid**: nothing may pass through it or stand inside it (Vol. III Ch. 1
/// §1.11, "Solid objects cannot occupy the same space"). Boolean; absent means not solid. A
/// solid body with a size is an obstacle to travel, and its top is something to stand on.
pub const SOLID: FactType = FactType::new("physical.body.solid");

/// Whether a body is **opaque**: it blocks sight (§1.11, "Walls block vision"). Boolean; absent
/// means transparent. Independent of [`SOLID`]: a glass pane is solid and not opaque. On a
/// portal it means "opaque when closed" — a shut door hides, a shut window does not.
pub const OPAQUE: FactType = FactType::new("physical.body.opaque");

/// Whether a region is **enclosed**: walled, so movement and sight cross its boundary only
/// through its portals (Amendment A-4). Boolean; absent means open — a field within a farm,
/// a glade within a forest.
pub const ENCLOSED: FactType = FactType::new("physical.space.enclosed");

/// Whether a portal is open (§1.11, "Closed doors prevent movement"). Boolean; absent means
/// open. A closed portal passes nothing, and passes sight only if it is not [`OPAQUE`].
pub const PORTAL_OPEN: FactType = FactType::new("physical.space.portal_open");

/// The other face of the same opening: the portal on the far side that whatever passes through
/// this one emerges from (Amendment A-4). An entity reference; absent means the far side is
/// found by nearness (the destination's portal back, closest to this one).
pub const PORTAL_FAR_SIDE: FactType = FactType::new("physical.space.portal_far_side");

/// Whether a body is **mobile**: a free body that falls when unsupported and can travel
/// (Amendment A-4). Boolean; absent means fixed in place — a room, a wall, a tree, a portal.
pub const MOBILE: FactType = FactType::new("physical.body.mobile");

/// A region's terrain grid spacing, in centimetres: the distance between neighbouring height
/// samples along each axis of the region's frame (Amendment A-4). Present only on a region with
/// terrain; together with [`TERRAIN_SAMPLE`] it is a heightfield whose sample `[column, row]`
/// stands at `(column × spacing, row × spacing)` in the region's frame.
pub const TERRAIN_SPACING: FactType = FactType::new("physical.terrain.spacing");

/// A region's terrain heights: a **cardinality-many** set of three-component samples
/// `[column, row, height]` (height in centimetres in the region's frame). Ground between samples
/// is interpolated; where a region has no sample, its ground is its frame's level floor. Stored
/// as a set so each sample is its own fact — terrain can be dug or raised one sample at a time —
/// and read by value range, never whole (`CommittedView::read_range`).
pub const TERRAIN_SAMPLE: FactType = FactType::new("physical.terrain.sample");

/// Where a body has been asked to go (Appendix A, Ruling 13): an entity reference — a region to
/// enter, or a thing to reach. Proposed by whatever decides (a player's action, an NPC's
/// choice); carried out by Physical Reality's travel system, which clears it on arrival.
pub const TRAVEL_TO: FactType = FactType::new("physical.travel.to");

/// How fast a body has been asked to travel, in centimetres per simulated second (Ruling 13).
pub const TRAVEL_SPEED: FactType = FactType::new("physical.travel.speed");

/// Whether a body's travel is currently **blocked**: no open way it fits through leads where
/// it was asked to go (Ruling 13). Boolean, written by the travel system — set when the way
/// closes, cleared when it opens — so a decider can notice and reconsider. Absent means not
/// blocked.
pub const TRAVEL_BLOCKED: FactType = FactType::new("physical.travel.blocked");

/// A decider's intent that a body open a door (Appendix A, Ruling 13 as amended by A-5): an
/// entity reference to the portal. Carried out by Physical Reality only if the body is within
/// reach of it — both faces of the opening then open together — and cleared either way.
pub const ACT_OPEN: FactType = FactType::new("physical.act.open");

/// A decider's intent that a body shut a door; as [`ACT_OPEN`].
pub const ACT_CLOSE: FactType = FactType::new("physical.act.close");

/// A decider's intent that a body turn to face a compass bearing (hundredths of a degree, as
/// [`HEADING`]). Carried out for a mobile body; cleared either way.
pub const ACT_FACE: FactType = FactType::new("physical.act.face");

/// What a body last tried and could not do (Amendment A-5): an entity reference to the door it
/// could not reach, or to itself if it could not turn. Written by Physical Reality so a decider
/// can notice and reconsider; cleared by the body's next act that succeeds.
pub const ACT_REFUSED: FactType = FactType::new("physical.act.refused");

/// How far a body fell in its most recent fall, in centimetres (Amendment A-4): written when the
/// fall begins, so a consumer — Living Systems judging an injury (Appendix A, Ruling 9) — can
/// read what gravity did without Physical Reality deciding what it meant.
pub const FALL_HEIGHT: FactType = FactType::new("physical.body.fall_height");

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

/// Weather's current departure from a region's normal temperature, in **sub-units**:
/// [`ANOMALY_SCALE`] × centidegrees (i.e. millionths of a degree). A mean-reverting quantity
/// the weather system steps (Vol. III Ch. 1 §1.10; Amendment A-1); temperature carries its
/// rounded, damped effect. Kept at fine resolution so the small per-step change at a short tick
/// length is never rounded away. Absent means "normal" (zero).
pub const TEMPERATURE_ANOMALY: FactType = FactType::new("physical.environment.temperature_anomaly");

/// Weather's current departure from a region's baseline humidity, in sub-units
/// ([`ANOMALY_SCALE`] × hundredths of a percent). See [`TEMPERATURE_ANOMALY`].
pub const HUMIDITY_ANOMALY: FactType = FactType::new("physical.environment.humidity_anomaly");

/// Weather's current departure from a region's baseline pressure, in sub-units
/// ([`ANOMALY_SCALE`] × decapascals). See [`TEMPERATURE_ANOMALY`].
pub const PRESSURE_ANOMALY: FactType = FactType::new("physical.environment.pressure_anomaly");

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

/// How many sub-units make one unit of a field, for the weather anomaly facts
/// ([`TEMPERATURE_ANOMALY`] and friends). A representation constant, not a world rule: it sets
/// how finely the anomaly is carried, never how weather behaves.
pub const ANOMALY_SCALE: i64 = 10_000;

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

/// Ceiling for a body size component, in centimetres (10 000 km) — clamps the field well above
/// any body a world might declare, from a grain of sand to a planet's crust.
pub const MAX_SIZE: i64 = 1_000_000_000;

/// Ceiling for a travel speed, in centimetres per second (10 km/s) — well above anything a world
/// moves by walking, riding, or sailing.
pub const MAX_SPEED: i64 = 1_000_000;

/// Ceiling for material specific heat capacity (J/(kg·K)) — clamps well above any real value
/// (water ≈ 4184, hydrogen ≈ 14 300).
pub const MAX_THERMAL_CAPACITY: i64 = 1_000_000;
