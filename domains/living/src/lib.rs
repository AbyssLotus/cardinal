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
    /// How much fatigue an awake organism gathers per hour, in hundredths of a percent
    /// (Amendment A-10).
    pub tire_per_hour: i64,
    /// How much fatigue a resting organism sheds per hour.
    pub rest_per_hour: i64,
    /// Body heat, in centidegrees, below which the cold harms.
    pub hypothermia_below_centi_c: i64,
    /// Health lost per degree below that line, per hour.
    pub cold_harm_per_degree_hour: i64,
    /// The highest fall, in centimetres, that does no harm.
    pub safe_fall_cm: i64,
    /// Health lost per metre fallen beyond the safe drop.
    pub fall_harm_per_metre: i64,
    /// Health recovered per hour while nothing harms the organism.
    pub heal_per_hour: i64,
    /// Dependence on a substance lost per day without it, in hundredths of a percent
    /// (Amendment A-13).
    pub dependence_fade_per_day: i64,
    /// Hunger gathered per hour, in hundredths of a percent (Amendment A-16).
    pub hunger_per_hour: i64,
    /// Hunger past which an organism is starving.
    pub starving_above: i64,
    /// Health lost per hour while starving.
    pub starving_harm_per_hour: i64,
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
        fact_type == schema::BODY_HEAT
            || fact_type == schema::SIGHT_RANGE
            || fact_type == schema::FATIGUE
            || fact_type == schema::HUNGER
            || fact_type == schema::HEALTH
            || fact_type == schema::REST
            || fact_type == schema::FALL_JUDGED
            || schema::NEED_FACTS.contains(&fact_type)
    }

    fn systems(&self) -> Vec<Box<dyn System>> {
        // One instance for the whole world; it iterates every organism it finds in reality.
        let c = self.config;
        let step = c.clock.step(c.metabolism_step_seconds.saturating_mul(1000));
        vec![
            Box::new(systems::Thermoregulation::new(
                step,
                c.set_point_centi_c,
                c.warm_response_seconds.saturating_mul(1000),
                c.cold_response_seconds.saturating_mul(1000),
            )),
            // Tiring and resting, and what harm and healing do to health (Amendment A-10).
            Box::new(systems::Fatigue::new(
                step,
                c.tire_per_hour,
                c.rest_per_hour,
            )),
            // Needs that arise, grow, ease, and end (Amendment A-11).
            Box::new(systems::Needs::new(
                step,
                c.dependence_fade_per_day,
                c.hunger_per_hour,
            )),
            Box::new(systems::Health::new(
                step,
                systems::HarmRules {
                    hypothermia_below_centi_c: c.hypothermia_below_centi_c,
                    cold_harm_per_degree_hour: c.cold_harm_per_degree_hour,
                    safe_fall_cm: c.safe_fall_cm,
                    fall_harm_per_metre: c.fall_harm_per_metre,
                    heal_per_hour: c.heal_per_hour,
                    starving_above: c.starving_above,
                    starving_harm_per_hour: c.starving_harm_per_hour,
                },
            )),
        ]
    }

    fn compose(
        &self,
        fact_type: FactType,
        current: Option<Value>,
        changes: &[Change],
    ) -> Result<Resolved, ResolveError> {
        if fact_type == schema::BODY_HEAT {
            composition::compose_body_heat(current, changes)
        } else if fact_type == schema::SIGHT_RANGE
            || fact_type == schema::HUNGER
            || fact_type == schema::FATIGUE
            || fact_type == schema::HEALTH
            || fact_type == schema::FALL_JUDGED
            || fact_type == schema::REST
            || schema::NEED_FACTS.contains(&fact_type)
        {
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
        let numeric = fact_type == schema::HUNGER
            || fact_type == schema::FATIGUE
            || fact_type == schema::HEALTH
            || fact_type == schema::SIGHT_RANGE
            || fact_type == schema::FALL_JUDGED
            || (schema::NEED_FACTS.contains(&fact_type) && fact_type != schema::NEED_KIND);
        if numeric && matches!(value, Resolved::Write(v) if v.as_int().is_none()) {
            return Err(ValidationError::new("this living fact is a number"));
        }
        if fact_type == schema::FATIGUE
            || fact_type == schema::HUNGER
            || fact_type == schema::HEALTH
            || fact_type == schema::NEED
            || fact_type == schema::DEPENDENCE
        {
            if let Resolved::Write(Value::Int(v)) = value {
                if !(0..=schema::FULL).contains(v) {
                    return Err(ValidationError::new(
                        "a need or health resolved outside 0..=100%",
                    ));
                }
            }
        }
        if fact_type == schema::REST {
            if let Resolved::Write(v) = value {
                if !matches!(v, Value::Bool(_)) {
                    return Err(ValidationError::new("rest is asked for yes or no"));
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
