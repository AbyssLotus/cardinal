//! # Packages service -- Vol. IV Ch. 3 & Ch. 7
//!
//! The world-package loader and validation stack. Packages arrive as DATA, are validated
//! against schema and composition rules, and are admitted or rejected -- never imported as
//! code (Vol. IV Ch. 3; dependency law rule 3). Content that fails validation never reaches
//! the store.
//!
//! Scaffold dependency is `kernel` only; per-domain schema-validation wiring (a dependency on
//! each domain whose facts a package may declare) lands with the loader (Vol. IV Ch. 3).
