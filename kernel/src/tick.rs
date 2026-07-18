//! The tick pipeline — Vol. V Ch. 3 §3.1.
//!
//! A tick advances committed reality by exactly one step through seven ordered stages.
//! Systems evaluate hermetically (committed reads in, proposals out); nothing mutates until
//! `commit` calls the store's single write path (Vol. V Ch. 2 §2.1). A failed tick leaves
//! reality exactly at N-1 (Vol. V Ch. 3 §3.5.5). The narrator runs in `observe` and is
//! never a dependency of the computation (Vol. V Ch. 9 §9.5.2), so a full tick runs with
//! the narrator disabled.

use crate::domain::{Domain, ResolveError, Resolved, ValidationError};
use crate::events::ChronicleEntry;
use crate::fact::{Cause, Fact, FactKey, FactType, Provenance, SystemId};
use crate::proposal::{Change, Proposal};
use crate::store::{CommitBatch, RealityStore};
use crate::system::{CommittedView, System, TickContext};
use std::collections::BTreeMap;

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
    /// A proposal targeted a fact type no enabled domain owns (Appendix A).
    Unowned(FactType),
    /// Proposals against a fact could not be composed (Resolve stage — Vol. V Ch. 3 §3.1).
    Resolve(ResolveError),
    /// The resolved batch failed a domain coherence check (Validate stage — §3.1).
    Validate(ValidationError),
}

/// Advance committed reality by one tick through the seven ordered stages
/// (Vol. V Ch. 3 §3.1).
///
/// On success the store holds tick `tick` and one [`ChronicleEntry`] is appended per
/// committed change. On any failure the store is left untouched and the returned
/// [`TickError`] names where it stopped (Vol. V Ch. 3 §3.5.5).
///
/// `systems` are the systems to run (typically gathered from `domains`); `domains` supply
/// ownership, composition, and validation for each touched fact type.
pub fn run_tick<S: RealityStore>(
    store: &mut S,
    domains: &[&dyn Domain],
    systems: &[Box<dyn System>],
    tick: u64,
    seed: u64,
    chronicle: &mut Vec<ChronicleEntry>,
) -> Result<(), TickError> {
    // 1. SCHEDULE — due systems in a deterministic order (sorted by id). The full
    //    DAG-from-read/write-sets scheduler is the next refinement (Vol. V Ch. 3 §3.2);
    //    with no declared happens-before edges, a stable id sort is a valid order.
    let mut due: Vec<&dyn System> = systems
        .iter()
        .filter(|s| s.cadence().is_due(tick))
        .map(|b| b.as_ref())
        .collect();
    due.sort_by_key(|s| s.id().name());

    // 2. EVALUATE — hermetic: committed reads in, proposals out (Vol. V Ch. 3 §3.1).
    let mut proposals: Vec<Proposal> = Vec::new();
    {
        let view: &dyn CommittedView = &*store;
        for sys in &due {
            let ctx = TickContext::new(tick, seed, sys.id().code());
            proposals.extend(sys.evaluate(view, &ctx));
        }
    }

    // 3. RESOLVE — group proposals per fact (deterministic key order), apply the owning
    //    domain's declared composition rule (Vol. V Ch. 3 §3.1; Vol. IV Ch. 2).
    let mut grouped: BTreeMap<FactKey, Vec<Proposal>> = BTreeMap::new();
    for p in proposals {
        grouped.entry(p.target).or_default().push(p);
    }

    let mut batch = CommitBatch::new(tick);
    for (key, group) in &grouped {
        let owner = owner_of(domains, key.fact_type).ok_or(TickError::Unowned(key.fact_type))?;

        // Deterministic within-group order before composing (Vol. V Ch. 3 §3.1).
        let mut props = group.clone();
        props.sort_by_key(|p| p.system.name());
        let changes: Vec<Change> = props.iter().map(|p| p.change).collect();
        let current = store.read(*key).map(|f| f.value);

        let resolved = owner
            .compose(key.fact_type, current, &changes)
            .map_err(TickError::Resolve)?;

        // 4. VALIDATE — owner coherence check on the resolved value (Vol. V Ch. 3 §3.1).
        owner
            .validate(key.fact_type, &resolved)
            .map_err(TickError::Validate)?;

        let (system, cause) = attribution(&props);
        match resolved {
            Resolved::Write(value) => {
                let prov = Provenance::new(system, tick, cause);
                batch.writes.push((*key, Fact::new(value, prov)));
            }
            Resolved::Tombstone => batch.tombstones.push(*key),
        }
    }

    // 5. COMMIT — the single mutation path; reality becomes N (Vol. V Ch. 2 §2.1).
    //    Nothing above mutated the store, so any error before here left it at N-1.
    store.apply(batch.clone());

    // 6. CHRONICLE — one entry per committed write, from its proposals' cause
    //    (Vol. V Ch. 6 §6.1).
    for (key, fact) in &batch.writes {
        chronicle.push(ChronicleEntry::new(
            tick,
            key.entity,
            key.fact_type,
            fact.provenance.cause,
        ));
    }

    // 7. OBSERVE — read-only notification hook; nothing on the critical path here yet
    //    (Vol. V Ch. 3 §3.1; Vol. V Ch. 9 §9.5.2).
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
