//! # Decision systems — Appendix A, Ruling 14; Amendment A-9
//!
//! Minds that choose from what they believe and act through the intents a player would use. A
//! mind owns what it is committed to — its goal, its plan's next step, its routines — and the
//! trace of why; it reads the information layer's beliefs and its own facts, never reality
//! (Vol. II Ch. 4, invariant 6). The reference mind is deterministic and utility-based
//! (Vol. V Ch. 9 §9.3): see [`think::Think`].
//!
//! **Domains never import domains** (Vol. V Ch. 1 §1.1, rule 2): this crate depends on `kernel`
//! alone, and names beliefs and intents by their published fact ids.

pub mod schema;
pub mod seam;
pub mod think;

use kernel::domain::{Domain, ResolveError, Resolved};
use kernel::fact::FactType;
use kernel::proposal::Change;
use kernel::system::System;
use kernel::time::SimClock;
use kernel::value::Value;
pub use think::MindRules;

/// Decision systems' configuration, all from the world package.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct MindsConfig {
    /// The world's clock.
    pub clock: SimClock,
    /// The length of a day, in seconds — for the hours routines keep.
    pub day_seconds: u64,
    /// The rules every mind decides by.
    pub rules: MindRules,
    /// How a resident crossing into the scene is given a mind (Amendment A-24), if the world has
    /// residents.
    pub residents: Option<seam::ResidentMind>,
}

/// Decision systems, plugged into the kernel as the owner of goals, commitments, routines, and
/// traces (Appendix A, as amended by A-9).
pub struct MindsDomain {
    config: MindsConfig,
}

impl MindsDomain {
    /// Configure the minds.
    pub fn new(config: MindsConfig) -> Self {
        Self { config }
    }
}

impl Domain for MindsDomain {
    fn name(&self) -> &'static str {
        "minds"
    }

    fn owns(&self, fact_type: FactType) -> bool {
        schema::OWNED.contains(&fact_type)
    }

    fn systems(&self) -> Vec<Box<dyn System>> {
        let c = self.config;
        let mut systems: Vec<Box<dyn System>> =
            vec![Box::new(think::Think::new(c.clock, c.day_seconds, c.rules))];
        if let Some(residents) = c.residents {
            systems.push(Box::new(seam::Seam::new(c.day_seconds as i64, residents)));
        }
        systems
    }

    fn compose(
        &self,
        _fact_type: FactType,
        _current: Option<Value>,
        changes: &[Change],
    ) -> Result<Resolved, ResolveError> {
        // One mind, one decision at a time: a commitment is set or let go, never adjusted, and
        // two at once is a conflict.
        match changes {
            [Change::Set(v) | Change::Create(v)] => Ok(Resolved::Write(*v)),
            [Change::Tombstone] => Ok(Resolved::Tombstone),
            [_] => Err(ResolveError::new(
                "a commitment is set or let go, never adjusted",
            )),
            _ => Err(ResolveError::new("two competing decisions for one mind")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn domain() -> MindsDomain {
        MindsDomain::new(MindsConfig {
            clock: SimClock::new(1_000),
            day_seconds: 86_400,
            rules: MindRules {
                think_step_seconds: 5,
                cold_below_centi_c: 2_200,
                trust_half_age_seconds: 86_400,
                hop_cost: 20,
                routine_value: 300,
                switch_margin: 100,
                tired_above: 7_000,
                rested_below: 1_500,
                tired_margin: 200,
                need_above: 3_000,
                need_weight: 20,
                hungry_above: 4_000,
                work_value: 250,
                sleep_from_seconds: 22 * 3600,
                sleep_to_seconds: 6 * 3600,
                bare_hand_hardness: 1000,
                repair_value: 280,
                repair_below: 10_000,
            },
            residents: Some(seam::ResidentMind {
                walk_speed_cm_s: 120,
                work_from_seconds: 8 * 3600,
                work_to_seconds: 17 * 3600,
                hours_spread_seconds: 2_700,
            }),
        })
    }

    #[test]
    fn minds_read_beliefs_and_their_own_commitments_never_reality() {
        // Ruling 14: every fact type a decision system reads is the information layer's or its
        // own. The kernel refuses any read a system did not declare, so this is the wall. The
        // seam is no decider: it makes a mind from the record a person crosses from, and reads
        // that record alone (Amendment A-24).
        for system in domain().systems() {
            if system.id().name() == "minds.seam" {
                for fact in system.reads() {
                    assert!(
                        fact.name().starts_with("society.resident."),
                        "the seam reads {}",
                        fact.name()
                    );
                }
                continue;
            }
            for fact in system.reads() {
                let name = fact.name();
                assert!(
                    name.starts_with("info.") || name.starts_with("mind."),
                    "{} reads {name}",
                    system.id().name()
                );
            }
        }
    }

    #[test]
    fn minds_write_only_their_commitments_and_intents() {
        let intents = [
            "physical.travel.to",
            "physical.travel.speed",
            "physical.act.open",
            "physical.act.take",
            "physical.act.consume",
            "physical.act.pick",
            "physical.act.drop",
            "economy.act.make",
            "economy.act.claim",
            "living.act.rest",
            "physical.act.dig",
            "physical.act.mend",
        ];
        for system in domain().systems() {
            for fact in system.writes() {
                let name = fact.name();
                assert!(
                    name.starts_with("mind.") || intents.contains(&name),
                    "{} writes {name}",
                    system.id().name()
                );
            }
        }
    }
}
