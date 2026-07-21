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
//! ## Shape
//! [`value`] and [`fact`] are the atoms of committed reality; [`store`] holds them behind
//! its contract; [`system`] and [`proposal`] are how change is proposed; [`domain`] plugs
//! owners in; [`tick`] runs the seven stages; [`hash`] is the determinism digest; [`rng`]
//! the seeded streams; [`events`] the chronicle; [`identity`] permanent ids.

pub mod domain;
pub mod events;
pub mod fact;
pub mod hash;
pub mod hierarchy;
pub mod identity;
pub mod proposal;
pub mod rng;
pub mod scheduler;
pub mod store;
pub mod system;
pub mod tick;
pub mod value;
