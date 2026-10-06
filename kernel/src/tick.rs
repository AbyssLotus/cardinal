//! The tick pipeline — Vol. V Ch. 3 §3.1.
//!
//! A tick advances committed reality by exactly one step through seven ordered stages.
//! Systems evaluate hermetically (committed reads scoped to their declared read set in,
//! proposals out); nothing mutates until `commit` calls the store's single write path
//! (Vol. V Ch. 2 §2.1). A failed tick leaves reality exactly at N-1 (Vol. V Ch. 3 §3.5.5).
//! The narrator runs in `observe` and is never a dependency of the computation
//! (Vol. V Ch. 9 §9.5.2), so a full tick runs with the narrator disabled.

use crate::domain::{Domain, ResolveError, Resolved, ValidationError};
use crate::events::ChronicleEntry;
use crate::fact::{Cardinality, Cause, Fact, FactKey, FactType, Provenance, SystemId};
use crate::proposal::{Change, Proposal};
use crate::store::{CommitBatch, RealityStore, Resolution};
use crate::system::{CommittedView, ScopedView, System, TickContext};
use std::collections::{BTreeMap, BTreeSet};

/// The seven ordered stages of a single tick (Vol. V Ch. 3 §3.1).
///
/// Order is law: proposals are gathered before they are resolved, resolved before they are
/// validated, validated before the single commit, and only committed reality is chronicled
/// and then observed.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum Stage {
    /// 1 — Select which systems run this tick from the cadence calendar (Vol. V Ch. 3 §3.2).
    Schedule,
    /// 2 — Run scheduled systems hermetically over committed reads, gathering proposals.
    Evaluate,
    /// 3 — Reconcile competing proposals into a single coherent set.
    Resolve,
    /// 4 — Check the resolved set against composition and conservation rules.
    Validate,
    /// 5 — Apply the validated set through the store's single write path (Vol. V Ch. 2 §2.1).
    Commit,
    /// 6 — Assemble the append-only, causally-linked chronicle from proposal causes
    ///     (Vol. V Ch. 6 §6.1).
    Chronicle,
    /// 7 — Emit entitled streams to observers and the narrator (Vol. V Ch. 9 §9.5.2).
    Observe,
}

impl Stage {
    /// The seven stages in their canonical, non-negotiable execution order.
    pub const ORDER: [Stage; 7] = [
        Stage::Schedule,
        Stage::Evaluate,
        Stage::Resolve,
        Stage::Validate,
        Stage::Commit,
        Stage::Chronicle,
        Stage::Observe,
    ];
}

/// Why a tick failed. A failed tick never happened: reality stays at N-1
/// (Vol. V Ch. 3 §3.5.5), and the error names the stage that stopped it.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum TickError {
    /// A system read a fact type outside its declared read set (Vol. V Ch. 3 §3.5,
    /// invariant 2) — a hermeticity violation caught in Evaluate. An undeclared read is a
    /// failure, never a silent default: a system that saw an empty world because of a
    /// mis-declared read set would propose confidently wrong facts (Vol. IV Ch. 2,
    /// missing-is-failure).
    UndeclaredRead {
        /// The offending system.
        system: SystemId,
        /// The fact type it read without declaring.
        fact_type: FactType,
    },
    /// A system proposed a change to a fact type outside its declared write set
    /// (Vol. V Ch. 3 §3.5, invariant 2) — a hermeticity violation caught in Evaluate.
    UndeclaredWrite {
        /// The offending system.
        system: SystemId,
        /// The fact type it tried to write without declaring.
        fact_type: FactType,
    },
    /// A proposal declared a basis tick other than the committed tick it must have read
    /// (Vol. V Ch. 3 §3.1, Proposals — conflict detection). A well-formed proposal for
    /// tick N reads reality at N-1 and declares N-1 as its basis; any other basis means the
    /// proposal was computed against state it could not have seen.
    StaleBasis {
        /// The offending system.
        system: SystemId,
        /// The basis every proposal this tick must declare (the committed tick).
        expected: u64,
        /// The basis the proposal actually declared.
        found: u64,
    },
    /// A proposal targeted a fact type no enabled domain owns (Appendix A).
    Unowned(FactType),
    /// The fact's owner does not accept writes from this system (Amendment A-5): it restricts
    /// the fact to its own systems, and the proposer is not one of them.
    Refused {
        /// The system whose proposal was refused.
        system: SystemId,
        /// The restricted fact type it tried to write.
        fact_type: FactType,
    },
    /// Two systems in one tick share an id (Amendment A-5). An id is how an owner recognises
    /// its own systems and how random streams are keyed, so a duplicate could impersonate an
    /// owner and would share another system's randomness.
    DuplicateSystem(SystemId),
    /// Proposals against a fact could not be composed (Resolve stage — Vol. V Ch. 3 §3.1).
    Resolve(ResolveError),
    /// The resolved batch failed a domain coherence check (Validate stage — §3.1).
    Validate(ValidationError),
    /// The tick asked for is not the one after the committed tick (Vol. V Ch. 3: tick N reads
    /// N − 1). Reality advances one tick at a time; it is never skipped, repeated, or rewound.
    NotNext {
        /// The tick reality has committed.
        committed: u64,
        /// The tick that was asked for.
        requested: u64,
    },
    /// A system asked for more new ids in one tick than one system may take (Amendment A-15):
    /// a runaway, refused before anything it proposed could commit.
    IdsExhausted(SystemId),
    /// More systems, or a later tick, than runtime ids can number (Amendment A-15): ids are
    /// `floor + tick·2²⁴ + slot·2¹⁴ + n`, so at most 1,024 systems and ticks below 2³⁸.
    BeyondIds,
}

/// The most systems runtime ids can tell apart (Amendment A-15): 2¹⁰ slots.
const MAX_SYSTEMS: usize = 1 << 10;
/// The first tick runtime ids cannot number (Amendment A-15): 2³⁸ — at one-second ticks, more
/// than eight thousand years.
const MAX_TICK: u64 = 1 << 38;

/// What one system cost in one tick (Amendment A-21; Vol. V Ch. 8): the time its evaluation took
/// and how many proposals it made. Kept outside the simulation — no system reads it and no fact
/// depends on it, so wall-clock time cannot reach reality.
#[derive(Clone, Copy, Debug)]
pub struct SystemMeter {
    /// The system, or `kernel.commit` for resolving, validating, and committing the tick.
    pub system: SystemId,
    /// Wall-clock time taken, in nanoseconds.
    pub nanos: u128,
    /// Proposals made (for `kernel.commit`, proposals resolved).
    pub proposals: usize,
}

/// Advance committed reality by one tick through the seven ordered stages
/// (Vol. V Ch. 3 §3.1).
///
/// On success the store holds tick `tick` and one [`ChronicleEntry`] is appended per
/// committed change. On any failure the store is left untouched and the returned
/// [`TickError`] names where it stopped (Vol. V Ch. 3 §3.5.5).
///
/// `systems` are the systems to run (typically gathered from `domains`); `domains` supply
/// ownership, cardinality, composition, and validation for each touched fact type.
pub fn run_tick<S: RealityStore>(
    store: &mut S,
    domains: &[&dyn Domain],
    systems: &[Box<dyn System>],
    tick: u64,
    seed: u64,
    chronicle: &mut Vec<ChronicleEntry>,
) -> Result<(), TickError> {
    run_tick_metered(
        store,
        domains,
        systems,
        tick,
        seed,
        chronicle,
        &mut Vec::new(),
    )
}

/// [`run_tick`], recording what each system cost into `meters` (Amendment A-21): one entry per
/// system evaluated, in the order evaluated, and one for the kernel's own commit.
pub fn run_tick_metered<S: RealityStore>(
    store: &mut S,
    domains: &[&dyn Domain],
    systems: &[Box<dyn System>],
    tick: u64,
    seed: u64,
    chronicle: &mut Vec<ChronicleEntry>,
    meters: &mut Vec<SystemMeter>,
) -> Result<(), TickError> {
    // 0. CONTINUITY — reality advances exactly one tick at a time (Vol. V Ch. 3: tick N reads
    //    N − 1), and within what runtime ids can number (Amendment A-15).
    if tick != store.tick() + 1 {
        return Err(TickError::NotNext {
            committed: store.tick(),
            requested: tick,
        });
    }
    if systems.len() > MAX_SYSTEMS || tick >= MAX_TICK {
        return Err(TickError::BeyondIds);
    }

    // 1. SCHEDULE — due systems in a deterministic order (sorted by id). Under hermetic
    //    evaluation execution order does not change committed reality, so a stable id sort is
    //    a valid order; the DAG scheduler is deferred until parallelism (Vol. V Ch. 3 §3.2).
    let mut due: Vec<&dyn System> = systems
        .iter()
        .filter(|s| s.cadence().is_due(tick))
        .map(|b| b.as_ref())
        .collect();
    due.sort_by_key(|s| s.id().name());
    // Every registered system has its own id — checked over all systems, due or not, so a
    // duplicate is caught the first tick it exists rather than the first tick both run.
    let mut ids: Vec<SystemId> = systems.iter().map(|s| s.id()).collect();
    ids.sort_by_key(|id| id.name());
    if let Some(pair) = ids.windows(2).find(|w| w[0] == w[1]) {
        return Err(TickError::DuplicateSystem(pair[0]));
    }

    // 2. EVALUATE — hermetic: each system reads a view scoped to its declared read set, and
    //    may only propose to facts in its declared write set (Vol. V Ch. 3 §3.1, §3.5). The
    //    basis every proposal must declare is the committed tick it read: N-1 for tick N.
    let expected_basis = tick.saturating_sub(1);
    let mut proposals: Vec<Proposal> = Vec::new();
    {
        let view: &dyn CommittedView = &*store;
        for sys in &due {
            let scoped = ScopedView::new(view, sys.reads());
            let slot = ids.iter().position(|id| *id == sys.id()).unwrap_or(0) as u64;
            let ctx = TickContext::new(tick, seed, sys.id().code(), slot);
            let began = std::time::Instant::now();
            let emitted = sys.evaluate(&scoped, &ctx);
            meters.push(SystemMeter {
                system: sys.id(),
                nanos: began.elapsed().as_nanos(),
                proposals: emitted.len(),
            });
            if ctx.exhausted() {
                return Err(TickError::IdsExhausted(sys.id()));
            }
            // An undeclared read taints the whole evaluation: the system may have acted on a
            // silently-empty view, so its proposals cannot be trusted (Vol. V Ch. 3 §3.5).
            if let Some(fact_type) = scoped.violation() {
                return Err(TickError::UndeclaredRead {
                    system: sys.id(),
                    fact_type,
                });
            }
            for p in &emitted {
                if !sys.writes().contains(&p.target.fact_type) {
                    return Err(TickError::UndeclaredWrite {
                        system: sys.id(),
                        fact_type: p.target.fact_type,
                    });
                }
                if p.basis_tick != expected_basis {
                    return Err(TickError::StaleBasis {
                        system: sys.id(),
                        expected: expected_basis,
                        found: p.basis_tick,
                    });
                }
            }
            proposals.extend(emitted);
        }
    }
    let committing = std::time::Instant::now();
    let resolved = proposals.len();

    // 3-4. RESOLVE + VALIDATE — group proposals per fact (deterministic key order), then
    //      resolve each by its owner's cardinality and rules (Vol. V Ch. 3 §3.1).
    let mut grouped: BTreeMap<FactKey, Vec<Proposal>> = BTreeMap::new();
    for p in proposals {
        grouped.entry(p.target).or_default().push(p);
    }

    let mut batch = CommitBatch::new(tick);
    for (key, group) in &grouped {
        let owner = owner_of(domains, key.fact_type).ok_or(TickError::Unowned(key.fact_type))?;
        // Authority (Amendment A-5): an owner that restricts this fact to its own systems
        // refuses everyone else's proposals, before any composition is attempted.
        if let Some(p) = group
            .iter()
            .find(|p| !owner.accepts(key.fact_type, p.system))
        {
            return Err(TickError::Refused {
                system: p.system,
                fact_type: key.fact_type,
            });
        }

        // Deterministic within-group order before composing (Vol. V Ch. 3 §3.1).
        let mut props = group.clone();
        props.sort_by_key(|p| p.system.name());
        let (system, cause) = attribution(&props);
        let provenance = Provenance::new(system, tick, cause);

        match owner.cardinality(key.fact_type) {
            Cardinality::One => {
                let changes: Vec<Change> = props.iter().map(|p| p.change).collect();
                let current = store.read(*key).map(|f| f.value);
                let resolved = owner
                    .compose(key.fact_type, current, &changes)
                    .map_err(TickError::Resolve)?;
                owner
                    .validate(key.fact_type, &resolved)
                    .map_err(TickError::Validate)?;
                match resolved {
                    Resolved::Write(value) => batch.resolutions.push(Resolution::One {
                        key: *key,
                        fact: Fact::new(value, provenance),
                    }),
                    Resolved::Tombstone => batch.resolutions.push(Resolution::Clear { key: *key }),
                }
            }
            Cardinality::Many => {
                // Set semantics: start from the committed set, apply each Add/Remove. Adding a
                // present value or removing an absent one is a no-op — set-valued composition
                // needs no conflict rule (Vol. V Ch. 2 §2.1, cardinality-many).
                let mut set: BTreeSet<crate::value::Value> =
                    store.read_all(*key).into_iter().map(|f| f.value).collect();
                for p in &props {
                    match p.change {
                        Change::Add(v) => {
                            set.insert(v);
                        }
                        Change::Remove(v) => {
                            set.remove(&v);
                        }
                        _ => {
                            return Err(TickError::Resolve(ResolveError::new(
                                "cardinality-many fact accepts only Add/Remove changes",
                            )))
                        }
                    }
                }
                // The owner validates the whole resolved set, so it can enforce set-level
                // coherence the generic Add/Remove machinery cannot (Vol. V Ch. 3 §3.1,
                // Validate; the cardinality-one path already validates through `compose`).
                let values: Vec<crate::value::Value> = set.into_iter().collect();
                owner
                    .validate_many(key.fact_type, &values)
                    .map_err(TickError::Validate)?;
                let facts = values
                    .into_iter()
                    .map(|v| Fact::new(v, provenance))
                    .collect::<Vec<_>>();
                batch
                    .resolutions
                    .push(Resolution::Many { key: *key, facts });
            }
        }
    }

    // 5. COMMIT — the single mutation path; reality becomes N (Vol. V Ch. 2 §2.1). Nothing
    //    above mutated the store, so any error before here left it at N-1. `apply` is
    //    infallible, so once we reach it the tick is committed; the batch moves in, no clone.
    store.apply(batch);

    // 6. CHRONICLE — one entry per committed proposal, carrying that proposal's own cause
    //    (Vol. V Ch. 6 §6.1). One entry per proposal, not per resolved fact, so every cause
    //    behind a composed value is recorded and a removal leaves a trace: a Tombstone or
    //    Remove change is chronicled exactly like a write (Vol. I, Law 17, traces on death).
    for group in grouped.values() {
        for p in group {
            chronicle.push(ChronicleEntry::new(tick, p.target, p.cause));
        }
    }

    // 7. OBSERVE — read-only notification hook; nothing on the critical path here yet
    //    (Vol. V Ch. 3 §3.1; Vol. V Ch. 9 §9.5.2). The meters are its first reading.
    meters.push(SystemMeter {
        system: SystemId::new("kernel.commit"),
        nanos: committing.elapsed().as_nanos(),
        proposals: resolved,
    });
    Ok(())
}

/// The enabled domain that owns `fact_type`, if any (Appendix A: one fact, one owner).
fn owner_of<'a>(domains: &[&'a dyn Domain], fact_type: FactType) -> Option<&'a dyn Domain> {
    for d in domains {
        if d.owns(fact_type) {
            return Some(*d);
        }
    }
    None
}

/// Provenance attribution for a resolved fact. A single proposal keeps its own system and
/// cause; a composed fact is attributed to the first proposal in deterministic order, with
/// its cause. Richer multi-cause provenance is a refinement (Vol. V Ch. 3 §3.1).
fn attribution(props: &[Proposal]) -> (SystemId, Cause) {
    let first = props
        .first()
        .expect("a fact group holds at least one proposal");
    (first.system, first.cause)
}

#[cfg(test)]
mod tests {
    use super::Stage;

    #[test]
    fn order_has_seven_stages_commit_before_chronicle() {
        assert_eq!(Stage::ORDER.len(), 7);
        let commit = Stage::ORDER
            .iter()
            .position(|s| *s == Stage::Commit)
            .unwrap();
        let chronicle = Stage::ORDER
            .iter()
            .position(|s| *s == Stage::Chronicle)
            .unwrap();
        // Only committed reality is chronicled (Vol. V Ch. 3 §3.1).
        assert!(commit < chronicle);
    }
}
