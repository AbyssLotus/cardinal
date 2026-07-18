//! # Physical Reality domain -- Vol. III Ch. 1
//!
//! Owns (Appendix A): space, topology, regions, containment, materials, environment.
//!
//! **Mandatory in every world** (Vol. IV Ch. 2 §2.1): the one domain that may never
//! be disabled -- physical space is the substrate every other domain sits on.
//!
//! **Domains never import domains** (Vol. V Ch. 1 §1.1, rule 2). This crate depends on
//! `kernel` and nothing else in the workspace; cross-domain effect happens through
//! committed proposals and events, never direct calls (Vol. III Ch. 12 §12.1). Adding
//! another domain to this crate's dependencies is the architectural-law violation the
//! crate boundary exists to surface.
//!
//! Scaffold: the modules below are charters awaiting their fact types, filled in roadmap
//! order (Vol. V Ch. 10 §10.4).

/// Fact-type declarations owned by the Physical Reality domain (Appendix A).
///
/// Every fact type appears exactly once, in its owner's schema -- one fact, one owner
/// (Appendix A). Boundary disputes are settled in Appendix A *before* code, argued from
/// its twelve rulings as precedent.
pub mod schema {}

/// Hermetic transformations owned by the Physical Reality domain (Vol. V Ch. 3 §3.1).
///
/// Each system declares its read/write sets and cadence, reads committed reality, and
/// emits proposals -- mutating nothing directly (Vol. V Ch. 3 §3.1-3.2). Implement causes,
/// never outcomes: no phenomenon systems (Vol. III Ch. 11 §11.3).
pub mod systems {}

/// Proposal-composition validators for Physical Reality-owned fact types (Vol. IV Ch. 2).
///
/// Composition rules decide how competing proposals against an owned fact reconcile before
/// commit (Vol. V Ch. 3, Validate stage). Every tunable threshold lives in the world
/// package, never here (Vol. IV Ch. 2 §2.2).
pub mod composition {}
