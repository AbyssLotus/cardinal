//! # Packages service -- Vol. IV
//!
//! The world-package loader. Packages arrive as DATA, are validated and version-checked,
//! and are admitted or rejected -- never imported as code (Vol. IV Ch. 1, invariant 6;
//! dependency law rule 3). The loader holds the boundary the whole volume defends: the
//! engine executes rules, the package defines them (Vol. IV Ch. 1 §1.1).
//!
//! This service sits in the `services` layer: it may depend on `kernel` and on the domain
//! crates whose facts a package configures (here, `physical`) -- the first place a domain
//! implementation is wired to package data.
//!
//! [`parse_world`] reads the world-file format into a [`WorldPackage`]; [`load`] turns a
//! package into a runnable [`LoadedWorld`], enforcing the engine-version range and the
//! mandatory Physical Reality domain, and seeding initial committed state.

pub mod loader;
pub mod model;
pub mod parse;
pub mod version;

pub use loader::{engine_version, load, LoadError, LoadedWorld};
pub use model::{Manifest, PhysicalRules, RegionSpec, WorldPackage};
pub use parse::{parse_world, ParseError};
pub use version::{EngineReq, Version, VersionParseError};
