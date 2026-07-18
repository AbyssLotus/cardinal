//! # Replay service -- Vol. V Ch. 8
//!
//! Deterministic replay and time-travel tooling: re-run a seed to any tick, splice saves, and
//! diff twin runs by per-tick state hash (Vol. V Ch. 8; Vol. V Ch. 4 §4.2). Replay is only
//! meaningful because the kernel is deterministic -- it stands on that guarantee.
