//! # Physical Reality domain -- Vol. III Ch. 1
//!
//! Owns (Appendix A): space, topology, regions, containment, materials, environment.
//!
//! **Mandatory in every world** (Vol. IV Ch. 2 §2.1): the one domain that may never be
//! disabled -- physical space is the substrate every other domain sits on.
//!
//! **Domains never import domains** (Vol. V Ch. 1 §1.1, rule 2). This crate depends on
//! `kernel` and nothing else in the workspace; cross-domain effect happens through
//! committed proposals and events, never direct calls (Vol. III Ch. 12 §12.1). Adding
//! another domain to this crate's dependencies is the architectural-law violation the crate
//! boundary exists to surface.
//!
//! ## First slice (Vol. V Ch. 10 §10.4)
//! The environment is the first reality made to move: a region's [`schema::TEMPERATURE`]
//! evolved by [`systems::DiurnalCycle`] and [`systems::WeatherNoise`], reconciled by
//! [`composition::compose_temperature`], through the kernel's seven-stage tick.

pub mod composition;
pub mod schema;
pub mod systems;

use kernel::domain::{Domain, ResolveError, Resolved, ValidationError};
use kernel::fact::FactType;
use kernel::identity::EntityId;
use kernel::proposal::Change;
use kernel::system::System;
use kernel::value::Value;

/// The Physical Reality domain, plugged into the kernel (Appendix A owner).
///
/// Configured for a single-region world — the minimal slice that exercises the pipeline.
/// Multi-region scheduling and the rest of Physical's fact types (space, topology,
/// materials) are later steps.
pub struct PhysicalDomain {
    region: EntityId,
    ticks_per_day: u64,
    diurnal_amplitude_centi_c: i64,
    weather_max_swing_centi_c: i64,
}

impl PhysicalDomain {
    /// Configure the domain for one region.
    pub const fn new(
        region: EntityId,
        ticks_per_day: u64,
        diurnal_amplitude_centi_c: i64,
        weather_max_swing_centi_c: i64,
    ) -> Self {
        Self {
            region,
            ticks_per_day,
            diurnal_amplitude_centi_c,
            weather_max_swing_centi_c,
        }
    }
}

impl Domain for PhysicalDomain {
    fn name(&self) -> &'static str {
        "physical"
    }

    fn owns(&self, fact_type: FactType) -> bool {
        fact_type == schema::TEMPERATURE
    }

    fn systems(&self) -> Vec<Box<dyn System>> {
        vec![
            Box::new(systems::DiurnalCycle::new(
                self.region,
                self.ticks_per_day,
                self.diurnal_amplitude_centi_c,
            )),
            Box::new(systems::WeatherNoise::new(
                self.region,
                self.weather_max_swing_centi_c,
            )),
        ]
    }

    fn compose(
        &self,
        fact_type: FactType,
        current: Option<Value>,
        changes: &[Change],
    ) -> Result<Resolved, ResolveError> {
        if fact_type == schema::TEMPERATURE {
            composition::compose_temperature(current, changes)
        } else {
            Err(ResolveError::new(
                "physical: fact type not owned by this domain",
            ))
        }
    }

    fn validate(&self, fact_type: FactType, value: &Resolved) -> Result<(), ValidationError> {
        if fact_type == schema::TEMPERATURE {
            if let Resolved::Write(Value::Int(centi_c)) = value {
                if *centi_c < schema::ABSOLUTE_ZERO_CENTI_C {
                    return Err(ValidationError::new(
                        "temperature resolved below absolute zero",
                    ));
                }
            }
        }
        Ok(())
    }
}
