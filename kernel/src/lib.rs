//! # Cardinal Kernel
//!
//! Volume II made executable: facts, ticks, commits, determinism. The kernel knows fact
//! types, proposals, and streams — it has never heard of a deer, a price, or a sword
//! (Vol. IV Ch. 1, Designer Note; zero world content, Vol. IV Ch. 1 §1.5.1).
//!
//! ## Governing spec
//! Vol. V Ch. 1 (layer law), Ch. 2 (Reality Store), Ch. 3 (tick pipeline & scheduler),
//! Ch. 4 (determinism); Vol. II throughout.
//!
//! ## Layer law (Vol. V Ch. 1 §1.1)
//! The kernel is the bottom of the dependency graph: it imports nothing from `domains`,
//! `services`, or `frontends`. Everything above depends on it; it depends on nothing
//! within the workspace. A kernel still gaining features in year three is absorbing
//! someone's domain (Vol. V Ch. 1, Designer Note) — keep it small, keep it law.
//!
//! ## Build order (Vol. V Ch. 10 §10.4)
//! [`hash`] comes first: every determinism test reduces to comparing per-tick state
//! hashes across twin runs (Vol. V Ch. 4 §4.2). The remaining modules are scaffolded to
//! their governing sections and filled in roadmap order.

pub mod hash;
pub mod identity;
pub mod store;
pub mod scheduler;
pub mod rng;
pub mod tick;
pub mod events;
