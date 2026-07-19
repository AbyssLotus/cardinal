//! Fact-type declarations owned by the Living Systems domain, and the cross-domain reads it
//! consumes (Appendix A; Vol. III Ch. 12 §12.1).
//!
//! One fact, one owner (Appendix A). Living Systems owns vital state; ambient temperature is
//! Physical Reality's, consumed here by its stable id — a published contract, not an import.
//! This crate has ZERO code dependency on the physical crate (Vol. III Ch. 12, invariant 1);
//! the two domains meet only in committed reality.

use kernel::fact::FactType;

/// An organism's body heat, as fixed-point centidegrees Celsius — vital-state physiology
/// (Vol. III Ch. 2 §, vital state), the substrate of the "warmth" need that tracks the
/// environment (Vol. III Ch. 2). Owned by Living Systems; fixed-point, not float, so
/// committed state carries no floating-point nondeterminism (Vol. V Ch. 4).
pub const BODY_HEAT: FactType = FactType::new("living.vital.body_heat");

/// Ambient temperature of a region — **Physical Reality's** fact (Appendix A), consumed
/// here by its stable, permanent id. Naming an id is not importing code: consumers read
/// committed facts freely, with no knowledge required by the owner (Vol. III Ch. 12 §12.1).
/// If no enabled domain owns this id, package validation catches the dangling read
/// (Vol. IV Ch. 2).
pub const AMBIENT_TEMPERATURE: FactType = FactType::new("physical.environment.temperature");

/// Absolute zero in centidegrees Celsius (−273.15 °C) — the floor below which body heat is
/// physically meaningless. The Validate stage rejects any resolved body heat beneath it.
pub const BODY_HEAT_FLOOR_CENTI_C: i64 = -27315;
