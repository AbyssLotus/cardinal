//! # The information layer — Vol. II Ch. 4; Amendment A-8
//!
//! *Reality is universal. Information is local.* This crate owns what each mind observes,
//! believes, and remembers, and nothing else: it turns reality — as Physical Reality and Living
//! Systems publish it, read by their fact ids and never by importing them — into beliefs that
//! belong to one mind each, carry their pedigree, and go stale when they are not refreshed.
//! Decision systems (`domains/minds`) read these beliefs and never reality (Vol. II Ch. 4,
//! invariant 6).
//!
//! **Domains never import domains** (Vol. V Ch. 1 §1.1, rule 2): this crate depends on `kernel`
//! alone. Its first system, [`systems::Perception`], reads what Physical Reality says is in view
//! and what a body feels of itself, and writes beliefs as facts about a pair (Amendment A-7).

pub mod schema;
pub mod systems;

use kernel::domain::{Domain, ResolveError, Resolved, ValidationError};
use kernel::fact::{Cardinality, FactType};
use kernel::proposal::Change;
use kernel::system::System;
use kernel::time::SimClock;
use kernel::value::Value;

/// The information layer's rules, all from the world package (Vol. IV Ch. 2 §2.2).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct InformationConfig {
    /// The world's clock.
    pub clock: SimClock,
    /// How often minds perceive, in seconds of simulated time.
    pub perception_step_seconds: u64,
    /// The least change of warmth, in centidegrees, a mind notices — of the air around it or of
    /// its own body.
    pub warmth_resolution_centi_c: i64,
    /// The least change of fatigue or health, in hundredths of a percent, a mind notices
    /// (Amendment A-10).
    pub need_resolution: i64,
    /// How often fondness grows or fades, in seconds of simulated time (Amendment A-11).
    pub affection_step_seconds: u64,
    /// Fondness gained per hour in another's sight, at full compatibility, in hundredths of a
    /// percent.
    pub affection_per_hour: i64,
    /// Fondness lost per day out of another's sight.
    pub affection_fade_per_day: i64,
}

/// The information layer, plugged into the kernel as the owner of observation, belief, and
/// memory (Appendix A: "Observation, knowledge, memory, belief (individual)").
pub struct InformationDomain {
    config: InformationConfig,
}

impl InformationDomain {
    /// Configure the layer.
    pub fn new(config: InformationConfig) -> Self {
        Self { config }
    }
}

impl Domain for InformationDomain {
    fn name(&self) -> &'static str {
        "information"
    }

    fn owns(&self, fact_type: FactType) -> bool {
        schema::OWNED.contains(&fact_type)
    }

    fn cardinality(&self, fact_type: FactType) -> Cardinality {
        if fact_type == schema::IN_SIGHT
            || fact_type == schema::MADE_OF
            || fact_type == schema::RECIPE_NEEDS
        {
            Cardinality::Many
        } else {
            Cardinality::One
        }
    }

    fn systems(&self) -> Vec<Box<dyn System>> {
        let c = self.config;
        vec![
            Box::new(systems::Perception::new(
                c.clock.step(c.perception_step_seconds.saturating_mul(1000)),
                c.warmth_resolution_centi_c,
                c.need_resolution,
            )),
            // Fondness grows with time in each other's sight (Amendment A-11).
            Box::new(systems::Fondness::new(
                c.clock.step(c.affection_step_seconds.saturating_mul(1000)),
                c.affection_per_hour,
                c.affection_fade_per_day,
            )),
        ]
    }

    fn compose(
        &self,
        _fact_type: FactType,
        _current: Option<Value>,
        changes: &[Change],
    ) -> Result<Resolved, ResolveError> {
        // A mind holds one belief per thing it is about; one perception forms it. Two at once
        // is a conflict, never silently resolved.
        match changes {
            [Change::Set(v) | Change::Create(v)] => Ok(Resolved::Write(*v)),
            [Change::Tombstone] => Ok(Resolved::Tombstone),
            [_] => Err(ResolveError::new(
                "a belief is set or forgotten, never adjusted",
            )),
            _ => Err(ResolveError::new("two competing beliefs about one thing")),
        }
    }

    fn validate_many(&self, fact_type: FactType, values: &[Value]) -> Result<(), ValidationError> {
        let sets = fact_type == schema::IN_SIGHT || fact_type == schema::MADE_OF;
        if sets && values.iter().any(|v| !matches!(v, Value::Entity(_))) {
            return Err(ValidationError::new("one watches things, not numbers"));
        }
        Ok(())
    }
}
