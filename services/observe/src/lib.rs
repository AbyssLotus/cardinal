//! # Observe service -- Vol. V Ch. 6 & Ch. 8
//!
//! The read-only observation plane: entitled streams out to frontends, plus meters and the
//! causal-debugger surface for operators (Vol. V Ch. 6 §6.1; Ch. 8). Read-only BY CONSTRUCTION
//! -- a service can watch the world but never write it (services operate around the world).
