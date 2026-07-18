//! The tick pipeline — Vol. V Ch. 3 §3.1.
//!
//! A tick advances committed reality by exactly one step through seven ordered stages.
//! Systems evaluate hermetically (committed reads in, proposals out); nothing mutates
//! until `commit` calls the store's single write path (Vol. V Ch. 2 §2.1). A failed tick
//! leaves reality exactly at N-1 (Vol. V Ch. 3 §3.5.5). The narrator runs in `observe`
//! and is never a dependency of the computation (Vol. V Ch. 9 §9.5.2), so a full tick
//! runs with the narrator disabled.

/// The seven ordered stages of a single tick (Vol. V Ch. 3 §3.1).
///
/// Order is law: proposals are gathered before they are resolved, resolved before they
/// are validated, validated before the single commit, and only committed reality is
/// chronicled and then observed.
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
