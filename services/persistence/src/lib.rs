//! # Persistence service -- Vol. V Ch. 7
//!
//! Durability AROUND the world, not inside it: the chronicle tail plus periodic snapshots,
//! with two-road recovery -- replay the tail onto the last snapshot (Vol. V Ch. 7). Operates
//! on committed reality only; it records and restores, it never simulates.
//!
//! Depends on `kernel` (the store's snapshot/history surface). Services sit above domains in
//! the dependency law (Vol. V Ch. 1 §1.1); this one needs only the kernel's persistence surface.
