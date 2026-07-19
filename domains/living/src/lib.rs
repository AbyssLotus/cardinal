//! # Living Systems domain -- Vol. III Ch. 2
//!
//! Owns (Appendix A): vital state, metabolism, lifecycle, capability, inheritance, death.
//!
//! Must be cleanly absent when disabled (Vol. IV Ch. 2 selection): worlds that switch this
//! domain off carry no trace of it, and Physical Reality runs byte-identically whether or
//! not this domain is present (Vol. III Ch. 12, invariant 7).
//!
//! **Domains never import domains** (Vol. V Ch. 1 §1.1, rule 2; Vol. III Ch. 12, inv. 1).
//! This crate depends on `kernel` and nothing else in the workspace. Its first system,
//! [`systems::Thermoregulation`], *consumes* Physical Reality's temperature fact — but by
//! its published id ([`schema::AMBIENT_TEMPERATURE`]), reading committed reality, never
//! calling or importing the physical crate (Vol. III Ch. 12 §12.1).
//!
//! ## First slice (Vol. V Ch. 10 §10.4)
//! The first cross-domain interaction: organisms whose [`schema::BODY_HEAT`] is defended
//! toward a metabolic set point while the region's temperature pulls it toward ambient —
//! two domains meeting only in the fact store.

pub mod composition;
pub mod schema;
pub mod systems;

use kernel::domain::{Domain, ResolveError, Resolved, ValidationError};
use kernel::fact::FactType;
use kernel::identity::EntityId;
use kernel::proposal::Change;
use kernel::system::System;
use kernel::value::Value;

/// One organism placed in the world: its entity id and the region it inhabits.
///
/// The region link is configuration for now; representing it as a Physical-owned containment
/// fact is a later step (Appendix A, regions and containment).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct OrganismPlacement {
    /// The organism entity.
    pub organism: EntityId,
    /// The region the organism inhabits, whose temperature it senses.
    pub region: EntityId,
}

/// The Living Systems domain, plugged into the kernel (Appendix A owner of vital state).
///
/// Configured over a set of placed organisms sharing one set of metabolic rules. Per-species
/// rules keyed on declared categories are a later refinement (Vol. IV Ch. 2 §2.2).
pub struct LivingDomain {
    organisms: Vec<OrganismPlacement>,
    set_point_centi_c: i64,
    warm_response: i64,
    cold_response: i64,
}

impl LivingDomain {
    /// Configure the domain for a set of organisms and shared metabolic rules.
    pub fn new(
        organisms: Vec<OrganismPlacement>,
        set_point_centi_c: i64,
        warm_response: i64,
        cold_response: i64,
    ) -> Self {
        Self {
            organisms,
            set_point_centi_c,
            warm_response,
            cold_response,
        }
    }
}

impl Domain for LivingDomain {
    fn name(&self) -> &'static str {
        "living"
    }

    fn owns(&self, fact_type: FactType) -> bool {
        fact_type == schema::BODY_HEAT
    }

    fn systems(&self) -> Vec<Box<dyn System>> {
        self.organisms
            .iter()
            .map(|o| {
                Box::new(systems::Thermoregulation::new(
                    o.organism,
                    o.region,
                    self.set_point_centi_c,
                    self.warm_response,
                    self.cold_response,
                )) as Box<dyn System>
            })
            .collect()
    }

    fn compose(
        &self,
        fact_type: FactType,
        current: Option<Value>,
        changes: &[Change],
    ) -> Result<Resolved, ResolveError> {
        if fact_type == schema::BODY_HEAT {
            composition::compose_body_heat(current, changes)
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
        Ok(())
    }
}
