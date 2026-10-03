//! # Living Systems domain -- Vol. III Ch. 2
//!
//! Owns (Appendix A): vital state, metabolism, lifecycle, capability, inheritance, death. The
//! first capability is sight range ([`schema::SIGHT_RANGE`], Amendment A-8).
//!
//! Must be cleanly absent when disabled (Vol. IV Ch. 2 selection): worlds that switch this
//! domain off carry no trace of it, and Physical Reality runs byte-identically whether or
//! not this domain is present (Vol. III Ch. 12, invariant 7).
//!
//! **Domains never import domains** (Vol. V Ch. 1 §1.1, rule 2; Vol. III Ch. 12, inv. 1).
//! This crate depends on `kernel` and nothing else in the workspace. Its first system,
//! [`systems::Thermoregulation`], *consumes* two Physical Reality facts — an organism's
//! containment ([`schema::CONTAINED_IN`], to learn its region) and that region's temperature
//! ([`schema::AMBIENT_TEMPERATURE`]) — but by their published ids, reading committed reality,
//! never calling or importing the physical crate (Vol. III Ch. 12 §12.1).
//!
//! ## First slice (Vol. V Ch. 10 §10.4)
//! The first cross-domain interaction: organisms whose [`schema::BODY_HEAT`] is defended
//! toward a metabolic set point while the temperature of the region they inhabit pulls it
//! toward ambient — two domains meeting only in the fact store.

pub mod composition;
pub mod schema;
pub mod systems;

use kernel::domain::{Domain, ResolveError, Resolved, ValidationError};
use kernel::fact::FactType;
use kernel::proposal::Change;
use kernel::system::System;
use kernel::time::SimClock;
use kernel::value::Value;

/// The Living Systems domain, plugged into the kernel (Appendix A owner of vital state).
///
/// Configured with one set of metabolic rules shared by every organism. It carries no
/// organism list: its single system discovers the organisms from committed reality — every
/// entity bearing body heat — each tick (Vol. V Ch. 2 §2.1, clause 5). Each organism's
/// region is a Physical containment fact it reads at run time, not domain configuration.
/// Per-species rules keyed on declared categories are a later refinement (Vol. IV Ch. 2
/// §2.2).
pub struct LivingDomain {
    config: LivingConfig,
}

/// The tunable metabolic rules the living domain consumes, all sourced from the world package
/// (Vol. IV Ch. 2 §2.2). Rates are time constants in simulated time (Vol. II Ch. 2,
/// Amendment A-1), so a body cools at the same pace whatever the world's tick length.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct LivingConfig {
    /// The world's clock: how much simulated time one tick lasts.
    pub clock: SimClock,
    /// How often metabolism steps, in seconds of simulated time (rounded to whole ticks).
    pub metabolism_step_seconds: u64,
    /// Metabolic set-point body heat, in centidegrees Celsius.
    pub set_point_centi_c: i64,
    /// Time constant of the pull toward the set point, in seconds: the time over which the
    /// body would close most of the gap on its own (larger = slower).
    pub warm_response_seconds: u64,
    /// Time constant of the pull toward the ambient temperature, in seconds (larger = slower).
    pub cold_response_seconds: u64,
}

impl LivingDomain {
    /// Configure the domain with shared metabolic rules.
    pub fn new(config: LivingConfig) -> Self {
        Self { config }
    }
}

impl Domain for LivingDomain {
    fn name(&self) -> &'static str {
        "living"
    }

    fn owns(&self, fact_type: FactType) -> bool {
        fact_type == schema::BODY_HEAT || fact_type == schema::SIGHT_RANGE
    }

    fn systems(&self) -> Vec<Box<dyn System>> {
        // One instance for the whole world; it iterates every organism it finds in reality.
        let c = self.config;
        vec![Box::new(systems::Thermoregulation::new(
            c.clock.step(c.metabolism_step_seconds.saturating_mul(1000)),
            c.set_point_centi_c,
            c.warm_response_seconds.saturating_mul(1000),
            c.cold_response_seconds.saturating_mul(1000),
        ))]
    }

    fn compose(
        &self,
        fact_type: FactType,
        current: Option<Value>,
        changes: &[Change],
    ) -> Result<Resolved, ResolveError> {
        if fact_type == schema::BODY_HEAT {
            composition::compose_body_heat(current, changes)
        } else if fact_type == schema::SIGHT_RANGE {
            composition::compose_capability(current, changes)
        } else {
            Err(ResolveError::new(
                "living: fact type not owned by this domain",
            ))
        }
    }

    fn validate(&self, fact_type: FactType, value: &Resolved) -> Result<(), ValidationError> {
        if fact_type == schema::BODY_HEAT {
            if let Resolved::Write(Value::Int(centi_c)) = value {
                if *centi_c < schema::BODY_HEAT_FLOOR_CENTI_C {
                    return Err(ValidationError::new(
                        "body heat resolved below absolute zero",
                    ));
                }
            }
        }
        if fact_type == schema::SIGHT_RANGE {
            if let Resolved::Write(Value::Int(cm)) = value {
                if *cm < 0 {
                    return Err(ValidationError::new("sight range resolved negative"));
                }
            }
        }
        Ok(())
    }
}
