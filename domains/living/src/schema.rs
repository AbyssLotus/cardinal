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
