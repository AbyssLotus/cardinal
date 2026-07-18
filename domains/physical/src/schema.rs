//! Fact-type declarations owned by the Physical Reality domain (Appendix A).
//!
//! Every fact type appears exactly once, in its owner's schema — one fact, one owner
//! (Appendix A). Boundary disputes are settled in Appendix A *before* code, argued from its
//! twelve rulings as precedent.

use kernel::fact::FactType;

/// Environmental temperature of a region, as fixed-point centidegrees Celsius (hundredths
/// of a degree). Fixed-point, not float, so committed state carries no floating-point
/// nondeterminism (Vol. V Ch. 4). Owned by Physical Reality; consumed by Living Systems,
/// Ecology, and Economy (Appendix A, environmental state).
pub const TEMPERATURE: FactType = FactType::new("physical.environment.temperature");

/// Absolute zero in centidegrees Celsius (−273.15 °C). The Validate stage rejects any
/// resolved temperature below this floor (Vol. V Ch. 3 §3.1) — a physical coherence rule
/// the domain owns.
pub const ABSOLUTE_ZERO_CENTI_C: i64 = -27315;
