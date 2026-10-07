//! # Society domain -- Vol. III Ch. 5
//!
//! Owns (Appendix A): kinship, households, settlements, roles, reputation, membership — and,
//! first, bonds between persons (Amendment A-11): two people become lovers when each has grown
//! fond enough of the other, and part when both have cooled; and jobs (Amendment A-18): the kinds
//! of work the world declares, and who holds which.
//!
//! Must be cleanly absent when disabled (Vol. IV Ch. 2 selection): worlds that switch
//! this domain off carry no trace of it -- Pelagia is watching (Vol. IV Ch. 8 §8.3).
//!
//! **Domains never import domains** (Vol. V Ch. 1 §1.1, rule 2). This crate depends on
//! `kernel` and nothing else in the workspace; cross-domain effect happens through
//! committed proposals and events, never direct calls (Vol. III Ch. 12 §12.1). How fond one
//! person is of another is the information layer's (Appendix A, Ruling 2), read here by its
//! published id.

use kernel::domain::{Domain, ResolveError, Resolved, ValidationError};
use kernel::fact::FactType;
use kernel::proposal::Change;
use kernel::system::System;
use kernel::time::SimClock;
use kernel::value::Value;

/// Fact-type declarations owned by the Society domain (Appendix A), and the facts of other
/// owners it reads by id.
pub mod schema {
    use kernel::fact::FactType;

    /// A bond between two persons (Amendment A-11): a fact about a pair, held both ways, whose
    /// value is the kind of bond — [`LOVERS`]. Its history is the chronicle's.
    pub const BOND: FactType = FactType::new("society.bond");

    /// The bond of lovers.
    pub const LOVERS: i64 = 1;

    /// The job a person holds (Amendment A-18): a role, naming a job kind.
    pub const ROLE: FactType = FactType::new("society.role");
    /// A *carry* job's goods: the material it brings to its store.
    pub const JOB_CARRIES: FactType = FactType::new("society.job.carries");
    /// A *make* job's recipe: what it keeps its store supplied with.
    pub const JOB_MAKES: FactType = FactType::new("society.job.makes");
    /// Where a *carry* job gets its goods: a deposit, or a place.
    pub const JOB_FROM: FactType = FactType::new("society.job.from");
    /// A job's store: the place its goods go to.
    pub const JOB_TO: FactType = FactType::new("society.job.to");
    /// How many of its goods a job keeps in its store.
    pub const JOB_KEEP: FactType = FactType::new("society.job.keep");
    /// A job's hours, as seconds into the day `[from, to, 0]`; wrapping past midnight when
    /// `from > to`.
    pub const JOB_HOURS: FactType = FactType::new("society.job.hours");

    /// A resident's home, a place (Amendment A-23): a person at the city tier exists only as a
    /// resident record (Appendix A, Ruling 19).
    pub const RESIDENT_HOME: FactType = FactType::new("society.resident.home");
    /// Where a resident works, a place; absent for one who does not.
    pub const RESIDENT_WORK: FactType = FactType::new("society.resident.work");
    /// What a resident is doing: [`SLEEPING`], [`AT_HOME`], [`WALKING`], [`WORKING`], [`EATING`].
    pub const RESIDENT_DOING: FactType = FactType::new("society.resident.doing");
    /// Where a resident is doing it — while walking, where they are walking to.
    pub const RESIDENT_AT: FactType = FactType::new("society.resident.at");
    /// The tick what a resident is doing ends: an agenda fact (Amendment A-22).
    pub const RESIDENT_UNTIL: FactType = FactType::new("society.resident.until");
    /// How hungry a resident is: a level (Amendment A-20), in hundredths of a percent.
    pub const RESIDENT_HUNGER: FactType = FactType::new("society.resident.hunger");
    /// How tired a resident is: a level, in hundredths of a percent.
    pub const RESIDENT_FATIGUE: FactType = FactType::new("society.resident.fatigue");
    /// How many meals a household's larder holds, on its home.
    pub const LARDER: FactType = FactType::new("society.household.larder");

    /// A place's rung on the residency ladder (Amendment A-24): [`CITY`] or [`SCENE`]. A place
    /// without one takes the rung of the nearest place holding it that has one; with none, it is
    /// in the scene. Policy, set by whoever directs attention; it changes what is computed, never
    /// what is true.
    pub const RESIDENCY: FactType = FactType::new("society.place.residency");
    /// A resident crossing into the scene (Amendment A-24): the place they cross into. Every
    /// domain gives them its own facts the tick after it is set, and Society retires the record.
    pub const RESIDENT_CROSSING: FactType = FactType::new("society.resident.crossing");

    /// Asleep, at home.
    pub const SLEEPING: i64 = 1;
    /// At home, awake.
    pub const AT_HOME: i64 = 2;
    /// Walking, to [`RESIDENT_AT`].
    pub const WALKING: i64 = 3;
    /// At work.
    pub const WORKING: i64 = 4;
    /// Eating a meal at home.
    pub const EATING: i64 = 5;

    /// The city tier: people here are resident records.
    pub const CITY: i64 = 1;
    /// The scene: people here are organisms, with bodies and minds.
    pub const SCENE: i64 = 2;

    /// Every fact type this domain owns.
    pub const OWNED: &[FactType] = &[
        RESIDENT_HOME,
        RESIDENT_WORK,
        RESIDENT_DOING,
        RESIDENT_AT,
        RESIDENT_UNTIL,
        RESIDENT_HUNGER,
        RESIDENT_FATIGUE,
        RESIDENT_CROSSING,
        RESIDENCY,
        LARDER,
        BOND,
        ROLE,
        JOB_CARRIES,
        JOB_MAKES,
        JOB_FROM,
        JOB_TO,
        JOB_KEEP,
        JOB_HOURS,
    ];

    /// How fond one person is of another — **the information layer's** (a fact about a pair).
    pub const AFFECTION: FactType = FactType::new("info.affection");
    /// Where a thing is placed in its container — **Physical Reality's**.
    pub const POSITION: FactType = FactType::new("physical.space.position");
    /// The container a thing is in — **Physical Reality's**.
    pub const CONTAINED_IN: FactType = FactType::new("physical.space.contained_in");
    /// The facts **Physical Reality's** spatial index mirrors, all read to search it: a thing's
    /// size, facing, and motion, besides its place and position.
    pub const BODY_SIZE: FactType = FactType::new("physical.body.size");
    /// Which way a body faces — **Physical Reality's**.
    pub const HEADING: FactType = FactType::new("physical.body.heading");
    /// Where a moving body is headed — **Physical Reality's**.
    pub const MOTION_TARGET: FactType = FactType::new("physical.motion.target");
    /// When it set off — **Physical Reality's**.
    pub const MOTION_START: FactType = FactType::new("physical.motion.start");
    /// When it arrives — **Physical Reality's**.
    pub const MOTION_END: FactType = FactType::new("physical.motion.end");
}

pub mod residents;
pub mod seam;

/// Hermetic transformations owned by the Society domain (Vol. V Ch. 3 §3.1).
pub mod systems {
    use crate::schema::{AFFECTION, BOND, LOVERS};
    use kernel::fact::{Cause, FactKey, FactType, SystemId};
    use kernel::identity::EntityId;
    use kernel::proposal::{Change, Proposal};
    use kernel::system::{Cadence, CommittedView, System, TickContext};
    use kernel::time::Step;
    use kernel::value::Value;
    use std::collections::BTreeSet;

    const COURTSHIP_READS: &[FactType] = &[AFFECTION, BOND];
    const COURTSHIP_WRITES: &[FactType] = &[BOND];

    /// Courtship (Vol. III Ch. 5; Amendment A-11): two persons become lovers when each is fond of
    /// the other past the world's line, and part when both have cooled below a lower one — so love
    /// does not flicker at the edge. Fondness is each one's own opinion; the bond is the
    /// relationship between them, held both ways.
    pub struct Courtship {
        step: Step,
        bond_above: i64,
        part_below: i64,
    }

    impl Courtship {
        /// Bond past `bond_above`, part below `part_below` (hundredths of a percent of
        /// fondness, on both sides), stepping as `step` says.
        pub const fn new(step: Step, bond_above: i64, part_below: i64) -> Self {
            Self {
                step,
                bond_above,
                part_below,
            }
        }
    }

    fn fondness(view: &dyn CommittedView, of: EntityId, for_: EntityId) -> i64 {
        view.read(FactKey::pair(of, AFFECTION, for_))
            .and_then(|f| f.value.as_int())
            .unwrap_or(0)
    }

    impl System for Courtship {
        fn id(&self) -> SystemId {
            SystemId::new("society.courtship")
        }
        fn reads(&self) -> &'static [FactType] {
            COURTSHIP_READS
        }
        fn writes(&self) -> &'static [FactType] {
            COURTSHIP_WRITES
        }
        fn cadence(&self) -> Cadence {
            self.step.cadence()
        }
        fn evaluate(&self, view: &dyn CommittedView, ctx: &TickContext) -> Vec<Proposal> {
            // Every pair someone is fond of, or bonded to — each pair once, lower id first.
            let mut pairs: BTreeSet<(EntityId, EntityId)> = BTreeSet::new();
            for fact in [AFFECTION, BOND] {
                for a in view.entities_with(fact) {
                    for (b, _) in view.read_about(a, fact) {
                        if a != b {
                            pairs.insert((a.min(b), a.max(b)));
                        }
                    }
                }
            }
            let mut out = Vec::new();
            let mut both_ways = |a: EntityId, b: EntityId, change: Change, why: &'static str| {
                for (x, y) in [(a, b), (b, a)] {
                    out.push(Proposal::new(
                        self.id(),
                        FactKey::pair(x, BOND, y),
                        ctx.basis_tick(),
                        change,
                        Cause::new(why),
                    ));
                }
            };
            for (a, b) in pairs {
                let (ab, ba) = (fondness(view, a, b), fondness(view, b, a));
                let bonded = view.read(FactKey::pair(a, BOND, b)).is_some();
                if !bonded && ab >= self.bond_above && ba >= self.bond_above {
                    both_ways(a, b, Change::Set(Value::Int(LOVERS)), "fell_in_love");
                } else if bonded && ab < self.part_below && ba < self.part_below {
                    both_ways(a, b, Change::Tombstone, "parted");
                }
            }
            out
        }
    }
}

/// The rules Society's bonds follow, all from the world package (Vol. IV Ch. 2 §2.2).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct SocietyConfig {
    /// The world's clock.
    pub clock: SimClock,
    /// How often bonds are made and broken, in seconds of simulated time.
    pub courtship_step_seconds: u64,
    /// Fondness, both ways, past which two persons become lovers (hundredths of a percent).
    pub bond_above: i64,
    /// Fondness, both ways, below which lovers part.
    pub part_below: i64,
    /// The rhythm residents at the city tier live by (Amendment A-23), if the world has any.
    pub rhythm: Option<residents::RhythmRules>,
}

/// The Society domain, plugged into the kernel as the owner of bonds between persons.
pub struct SocietyDomain {
    config: SocietyConfig,
}

impl SocietyDomain {
    /// Configure the domain.
    pub fn new(config: SocietyConfig) -> Self {
        Self { config }
    }
}

impl Domain for SocietyDomain {
    fn name(&self) -> &'static str {
        "society"
    }

    fn owns(&self, fact_type: FactType) -> bool {
        schema::OWNED.contains(&fact_type)
    }

    fn systems(&self) -> Vec<Box<dyn System>> {
        let c = self.config;
        let mut systems: Vec<Box<dyn System>> = vec![Box::new(systems::Courtship::new(
            c.clock.step(c.courtship_step_seconds.saturating_mul(1000)),
            c.bond_above,
            c.part_below,
        ))];
        if let Some(rules) = c.rhythm {
            systems.push(Box::new(residents::Rhythm::new(c.clock, rules)));
            systems.push(Box::new(seam::Crossing));
        }
        systems
    }

    fn agenda(&self) -> Vec<FactType> {
        // A resident is attended to when what they are doing ends (Amendment A-23), and found
        // where they are when attention arrives there (Amendment A-24).
        vec![schema::RESIDENT_UNTIL, schema::RESIDENT_AT]
    }

    fn compose(
        &self,
        _fact_type: FactType,
        _current: Option<Value>,
        changes: &[Change],
    ) -> Result<Resolved, ResolveError> {
        match changes {
            [Change::Set(v) | Change::Create(v)] => Ok(Resolved::Write(*v)),
            [Change::Tombstone] => Ok(Resolved::Tombstone),
            [_] => Err(ResolveError::new(
                "a bond or a role is made or broken, never adjusted",
            )),
            _ => Err(ResolveError::new(
                "two competing changes to one bond or role",
            )),
        }
    }

    fn validate(&self, fact_type: FactType, value: &Resolved) -> Result<(), ValidationError> {
        if fact_type == schema::BOND && matches!(value, Resolved::Write(v) if v.as_int().is_none())
        {
            return Err(ValidationError::new("a bond is of a kind, given by number"));
        }
        if fact_type == schema::ROLE
            && matches!(value, Resolved::Write(v) if !matches!(v, Value::Entity(_)))
        {
            return Err(ValidationError::new("a role names a job"));
        }
        if fact_type == schema::RESIDENCY
            && matches!(value, Resolved::Write(v)
                if !matches!(v, Value::Int(schema::CITY) | Value::Int(schema::SCENE)))
        {
            return Err(ValidationError::new(
                "a place is at the city tier or in the scene",
            ));
        }
        Ok(())
    }
}
